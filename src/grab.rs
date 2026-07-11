use anyhow::Result;

use crate::config::Config;
use crate::selection::Source;

/// Run the grab command: read selection, explain it, notify + optionally print.
pub async fn run_grab(config: &Config, source: &str, expand: bool, quiet: bool) -> Result<()> {
    // Hotkeys can fire multiple overlapping instances; only one should run.
    let Some(_lock) = crate::notify_guard::try_grab_lock() else {
        return Ok(());
    };

    let source = Source::parse(source)?;
    let word = crate::selection::read_selection(source)?;

    // Suppress duplicate hotkey triggers (notify + terminal print + API).
    if !crate::notify_guard::try_claim(&word) {
        return Ok(());
    }

    let provider = crate::providers::resolve_provider(None, config).await?;
    let provider_name = provider.name().to_string();

    let active_ctx = crate::context::detect_context();
    let diagnose = crate::filter::looks_like_error(&word);

    let mut search_results = Vec::new();
    if diagnose {
        let url = &config.search.searxng_url;
        let max = config.search.max_results;
        if let Ok(results) = crate::search::search_searxng(url, &word, max).await {
            search_results = results;
        }
    }

    let result = crate::explain::explain(
        &crate::explain::SourceContext {
            word: word.clone(),
            to_lang: "中文".to_string(),
            language: active_ctx.language.clone(),
            app_context: active_ctx.app_name.clone(),
            search_results: search_results.clone(),
            ..Default::default()
        },
        provider.as_ref(),
        expand,
        diagnose,
    )
    .await?;

    let _ = crate::history::save_query(
        &word,
        &provider_name,
        &result.translation,
        &result.explanation,
        &result.usage,
        None,
        None,
        &search_results,
    );

    // Desktop notification is the primary output for hotkey use.
    let summary = if !result.translation.is_empty() {
        format!("{} — {}", short_title(&word), result.translation)
    } else {
        short_title(&word)
    };
    let mut body = result.explanation.clone();
    if !result.usage.is_empty() {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&result.usage);
    }
    crate::daemon::show_notification(&summary, &body);

    // Also print when run from an interactive terminal.
    if !quiet {
        crate::render::render_explain(&result, "plain", &search_results);
    }

    Ok(())
}

/// Shorten a selection for use in a notification title.
fn short_title(s: &str) -> String {
    let one_line = s.replace('\n', " ");
    let max = 48;
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut out: String = one_line.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_title_short() {
        assert_eq!(short_title("map"), "map");
    }

    #[test]
    fn test_short_title_multiline_collapsed() {
        assert_eq!(short_title("foo\nbar"), "foo bar");
    }

    #[test]
    fn test_short_title_truncated() {
        let long = "a".repeat(100);
        let t = short_title(&long);
        assert_eq!(t.chars().count(), 49); // 48 chars + ellipsis
        assert!(t.ends_with('…'));
    }
}
