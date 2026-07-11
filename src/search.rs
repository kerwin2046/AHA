use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A search result from SearXNG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Deserialize)]
struct SearxngResult {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
}

/// Query a SearXNG instance and return top results.
pub async fn search_searxng(
    url: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>> {
    let base = url.trim_end_matches('/');
    let endpoint = format!("{base}/search");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;

    let resp = client
        .get(&endpoint)
        .header("X-Forwarded-For", "127.0.0.1")
        .query(&[
            ("q", query),
            ("format", "json"),
            ("language", "en"),
        ])
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("SearXNG request failed ({endpoint}): {e}"))?;

    if !resp.status().is_success() {
        anyhow::bail!("SearXNG returned HTTP {}", resp.status());
    }

    let data: SearxngResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("SearXNG JSON parse error: {e}"))?;

    let results: Vec<SearchResult> = data
        .results
        .into_iter()
        .filter_map(|r| {
            Some(SearchResult {
                title: r.title?.trim().to_string(),
                url: r.url?.trim().to_string(),
                snippet: r.content.unwrap_or_default().trim().to_string(),
            })
        })
        .filter(|r| !r.snippet.is_empty())
        .take(max_results)
        .collect();

    if results.is_empty() {
        anyhow::bail!("SearXNG returned no results");
    }

    Ok(results)
}

/// Format search results as a prompt appendix.
pub fn format_search_results(results: &[SearchResult]) -> String {
    let mut out = String::from("\n\n## Web Search Results\n\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "{}. **{}**\n   {}\n   URL: {}\n\n",
            i + 1,
            r.title,
            r.snippet,
            r.url
        ));
    }
    out.push_str(
        "Use these search results to provide an up-to-date answer. Cite sources where relevant.\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_search_results() {
        let results = vec![SearchResult {
            title: "Rust serde".to_string(),
            url: "https://serde.rs".to_string(),
            snippet: "Serde is a framework for serializing Rust data structures.".to_string(),
        }];
        let formatted = format_search_results(&results);
        assert!(formatted.contains("Rust serde"));
        assert!(formatted.contains("https://serde.rs"));
        assert!(formatted.contains("Serde is a framework"));
    }
}
