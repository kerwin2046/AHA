use anyhow::Result;
use lru::LruCache;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

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

struct CacheEntry {
    at: Instant,
    results: Vec<SearchResult>,
}

static SEARCH_CACHE: LazyLock<Mutex<LruCache<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap())));

const CACHE_TTL: Duration = Duration::from_secs(600);

/// Build a short, searchable query from selected text (errors, identifiers, snippets).
pub fn build_search_query(text: &str, language: Option<&str>) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut parts: Vec<String> = Vec::new();

    // Rust: error[E0425]
    if let Some(cap) = extract_rust_error_code(trimmed) {
        parts.push(cap);
        parts.push("Rust".into());
    }

    // Generic "error CS...." / "TS2345" style
    if let Some(code) = extract_generic_error_code(trimmed) {
        if !parts.iter().any(|p| p == &code) {
            parts.push(code);
        }
    }

    // First meaningful line, cleaned
    if let Some(line) = first_meaningful_line(trimmed) {
        let cleaned = clean_error_line(&line);
        if !cleaned.is_empty() && !parts.iter().any(|p| cleaned.contains(p) || p.contains(&cleaned))
        {
            parts.push(cleaned);
        }
    }

    // Language / ecosystem hint
    if let Some(lang) = language.map(str::trim).filter(|s| !s.is_empty()) {
        let lang_l = lang.to_lowercase();
        if !parts.iter().any(|p| p.eq_ignore_ascii_case(lang)) {
            // Avoid duplicating "Rust" if already added from error code
            if !(lang_l == "rust" && parts.iter().any(|p| p == "Rust")) {
                parts.push(lang.to_string());
            }
        }
    } else if looks_like_python(trimmed) && !parts.iter().any(|p| p == "Python") {
        parts.push("Python".into());
    } else if looks_like_js(trimmed) && !parts.iter().any(|p| p.eq_ignore_ascii_case("javascript"))
    {
        parts.push("JavaScript".into());
    }

    let mut q = parts.join(" ");
    // Hard cap — SearXNG hates huge queries
    if q.chars().count() > 120 {
        q = q.chars().take(120).collect();
    }
    if q.trim().is_empty() {
        trimmed.chars().take(80).collect()
    } else {
        q
    }
}

fn extract_rust_error_code(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(6) {
        if bytes[i] == b'E'
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 4].is_ascii_digit()
        {
            // Prefer error[E0425] form
            return Some(format!(
                "error[{}]",
                &text[i..i + 5]
            ));
        }
    }
    // Also match error[E0425] already present
    if let Some(start) = text.find("error[") {
        if let Some(end) = text[start..].find(']') {
            return Some(text[start..start + end + 1].to_string());
        }
    }
    None
}

fn extract_generic_error_code(text: &str) -> Option<String> {
    // TS2345, CS0246, ECONNREFUSED-like tokens on first lines
    for line in text.lines().take(4) {
        for token in line.split(|c: char| !c.is_ascii_alphanumeric()) {
            let t = token.trim();
            if t.len() >= 4
                && t.len() <= 16
                && t.chars().any(|c| c.is_ascii_digit())
                && t.chars().any(|c| c.is_ascii_alphabetic())
                && (t.starts_with("TS")
                    || t.starts_with("CS")
                    || t.starts_with("ERR_")
                    || (t.starts_with('E') && t[1..].chars().all(|c| c.is_ascii_digit())))
            {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn first_meaningful_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && !l.starts_with("-->")
                && !l.starts_with("= note")
                && !l.starts_with('|')
                && !l.chars().all(|c| c == '-' || c == '^' || c.is_whitespace())
        })
        .map(|s| s.to_string())
}

fn clean_error_line(line: &str) -> String {
    let mut s = line.to_string();
    // Strip common prefixes
    for prefix in ["error: ", "Error: ", "fatal error: ", "FAILED: "] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    // Drop "error[E0425]: " prefix body already has code separately
    if let Some(idx) = s.find("]: ") {
        if s[..idx].contains('[') {
            s = s[idx + 3..].to_string();
        }
    }
    // Remove absolute/relative path noise
    s = s
        .split_whitespace()
        .filter(|w| {
            !w.contains('/') || w.starts_with('`') || w.ends_with('`') || w.contains("::")
        })
        .collect::<Vec<_>>()
        .join(" ");
    s.chars().take(80).collect::<String>().trim().to_string()
}

fn looks_like_python(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("traceback") || t.contains("modulenotfounderror") || t.contains("nameerror")
}

fn looks_like_js(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("typeerror") || t.contains("referenceerror") || t.contains("syntaxerror")
}

/// Query SearXNG, rank by domain quality, dedupe, cache.
pub async fn search_searxng(
    url: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        anyhow::bail!("empty search query");
    }

    let max_results = max_results.clamp(1, 5);
    let cache_key = format!("{url}|{query}|{max_results}");

    if let Some(hit) = SEARCH_CACHE.lock().get(&cache_key) {
        if hit.at.elapsed() < CACHE_TTL {
            return Ok(hit.results.clone());
        }
    }

    let base = url.trim_end_matches('/');
    let endpoint = format!("{base}/search");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;

    let resp = client
        .get(&endpoint)
        .header("X-Forwarded-For", "127.0.0.1")
        .query(&[("q", query), ("format", "json"), ("language", "en")])
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

    let mut ranked: Vec<(i32, SearchResult)> = data
        .results
        .into_iter()
        .filter_map(|r| {
            let title = r.title?.trim().to_string();
            let url = r.url?.trim().to_string();
            let snippet = r.content.unwrap_or_default().trim().to_string();
            if title.is_empty() || url.is_empty() || snippet.is_empty() {
                return None;
            }
            if is_low_quality_url(&url) {
                return None;
            }
            let score = domain_score(&url);
            Some((
                score,
                SearchResult {
                    title,
                    url,
                    snippet: truncate_snippet(&snippet, 220),
                },
            ))
        })
        .collect();

    ranked.sort_by(|a, b| b.0.cmp(&a.0));

    let mut results = Vec::new();
    let mut seen_hosts = Vec::new();
    for (_, r) in ranked {
        let host = host_of(&r.url);
        if seen_hosts.iter().any(|h| h == &host) {
            continue;
        }
        seen_hosts.push(host);
        results.push(r);
        if results.len() >= max_results {
            break;
        }
    }

    if results.is_empty() {
        anyhow::bail!("SearXNG returned no useful results");
    }

    SEARCH_CACHE.lock().put(
        cache_key,
        CacheEntry {
            at: Instant::now(),
            results: results.clone(),
        },
    );

    Ok(results)
}

fn truncate_snippet(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .trim_start_matches("www.")
        .to_lowercase()
}

fn is_low_quality_url(url: &str) -> bool {
    let u = url.to_lowercase();
    const BLOCK: &[&str] = &[
        "pinterest.",
        "facebook.com",
        "twitter.com",
        "x.com/",
        "instagram.com",
        "tiktok.com",
        "quora.com/unanswered",
    ];
    BLOCK.iter().any(|b| u.contains(b))
}

/// Higher is better. Official docs and Q&A beat random blogs.
fn domain_score(url: &str) -> i32 {
    let host = host_of(url);
    const HIGH: &[&str] = &[
        "doc.rust-lang.org",
        "docs.rs",
        "docs.python.org",
        "developer.mozilla.org",
        "nodejs.org",
        "go.dev",
        "docs.oracle.com",
        "learn.microsoft.com",
        "stackoverflow.com",
        "stackexchange.com",
        "github.com",
    ];
    const MID: &[&str] = &[
        "reddit.com",
        "dev.to",
        "medium.com",
        "baeldung.com",
        "realpython.com",
        "css-tricks.com",
    ];

    if HIGH.iter().any(|d| host == *d || host.ends_with(&format!(".{d}"))) {
        100
    } else if host.contains("docs.") || host.starts_with("docs") {
        80
    } else if MID.iter().any(|d| host == *d || host.ends_with(&format!(".{d}"))) {
        40
    } else {
        10
    }
}

/// Format search results as a prompt appendix with citation markers.
pub fn format_search_results(results: &[SearchResult]) -> String {
    let mut out = String::from("\n\n## Web Search Results (cite as [1], [2], …)\n\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "[{}] {}\n    {}\n    {}\n\n",
            i + 1,
            r.title,
            r.snippet,
            r.url
        ));
    }
    out.push_str(
        "When a claim comes from these results, add the citation like [1] or [1][2] inline in explanation.\n",
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
        assert!(formatted.contains("[1]"));
        assert!(formatted.contains("Rust serde"));
        assert!(formatted.contains("https://serde.rs"));
    }

    #[test]
    fn test_build_query_rust_error() {
        let text = "error[E0425]: cannot find value `unknown_variable` in this scope\n --> src/main.rs:10:5";
        let q = build_search_query(text, Some("Rust"));
        assert!(q.contains("E0425") || q.contains("error[E0425]"));
        assert!(q.contains("Rust"));
        assert!(q.contains("unknown_variable") || q.contains("cannot find"));
        assert!(q.chars().count() <= 120);
    }

    #[test]
    fn test_build_query_python() {
        let text = "Traceback (most recent call last):\n  File \"a.py\", line 1\nNameError: name 'foo' is not defined";
        let q = build_search_query(text, None);
        assert!(q.contains("Python") || q.to_lowercase().contains("nameerror"));
    }

    #[test]
    fn test_domain_score_docs_beat_random() {
        assert!(domain_score("https://doc.rust-lang.org/error_codes/E0425.html")
            > domain_score("https://random-blog.example/post"));
        assert!(domain_score("https://stackoverflow.com/questions/1")
            > domain_score("https://example.com/x"));
    }

    #[test]
    fn test_host_dedupe_key() {
        assert_eq!(host_of("https://www.Docs.Python.org/3/"), "docs.python.org");
    }
}
