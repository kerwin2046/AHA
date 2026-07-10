mod cli;
mod config;
mod explain;
mod providers;
mod error;
mod tui;
mod daemon;
mod grab;
mod history;
mod render;
mod filter;
mod notify_guard;
mod selection;
mod web;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load();
    let result = match cli.command {
        Command::Explain {
            word,
            pipe,
            file,
            provider,
            context,
            expand,
            json,
            format,
            to,
        } => {
            let fmt = if json { "json".to_string() } else { format };
            cmd_explain(word, pipe, file, provider, context, expand, fmt, to, &config).await
        }
        Command::Ask => cmd_ask(&config).await,
        Command::Daemon => daemon::run_daemon(&config).await,
        Command::Grab {
            source,
            expand,
            quiet,
        } => grab::run_grab(&config, &source, expand, quiet).await,
        Command::Config => {
            println!("Config path: {}", Config::path().display());
            println!("{:#?}", config);
            Ok(())
        }
        Command::Init => cmd_init().await,
        Command::History {
            limit,
            search,
            stats,
            clear,
            json,
        } => cmd_history(limit, search, stats, clear, json).await,
        Command::Tui => tui::run().await,
        Command::Web { port } => web::serve(port).await,
    };

    if let Err(e) = result {
        error::print_error(&e);
        std::process::exit(1);
    }

    Ok(())
}
async fn cmd_explain(
    word: Option<String>,
    pipe: bool,
    file: Option<String>,
    provider_name: Option<String>,
    context_lines: usize,
    expand: bool,
    format: String,
    to_lang: String,
    config: &Config,
) -> Result<()> {
    let ctx = resolve_input(word, pipe, file, context_lines, &to_lang)?;

    let provider = providers::resolve_provider(provider_name.as_deref(), config).await?;
    let provider_name = provider.name().to_string();

    let result = explain::explain(&ctx, provider.as_ref(), expand).await?;

    let _ = history::save_query(
        &ctx.word,
        &provider_name,
        &result.translation,
        &result.explanation,
        &result.usage,
        ctx.file.as_deref(),
        ctx.language.as_deref(),
    );

    render::render_explain(&result, &format);

    Ok(())
}

fn resolve_input(
    word: Option<String>,
    pipe: bool,
    file: Option<String>,
    context_lines: usize,
    to_lang: &str,
) -> Result<explain::SourceContext> {
    // Priority: --file > --pipe > positional word
    if let Some(path) = file {
        return build_file_context(&path, context_lines, to_lang);
    }

    if pipe {
        let text = read_word_from_stdin()?;
        return Ok(explain::SourceContext {
            word: text,
            to_lang: to_lang.to_string(),
            ..Default::default()
        });
    }

    let word = word
        .ok_or_else(|| anyhow::anyhow!("Provide a word: ah explain <word> or use --pipe"))?;
    Ok(explain::SourceContext {
        word,
        to_lang: to_lang.to_string(),
        ..Default::default()
    })
}

fn read_word_from_stdin() -> Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("Empty input from stdin");
    }
    Ok(trimmed)
}

fn build_file_context(spec: &str, context_lines: usize, to_lang: &str) -> Result<explain::SourceContext> {
    let (path, line) = parse_file_spec(spec);
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let language = explain::detect_language(path).map(|s| s.to_string());

    if let Some(n) = line {
        if n == 0 || n > lines.len() {
            anyhow::bail!("Line {n} out of range (file has {} lines)", lines.len());
        }
        let line_content = lines[n - 1].trim();
        let word = extract_identifier(line_content).unwrap_or(line_content).to_string();
        let surrounding = surrounding_lines(&lines, n, context_lines);

        Ok(explain::SourceContext {
            word,
            file: Some(path.to_string()),
            line: Some(n),
            language,
            surrounding,
            to_lang: to_lang.to_string(),
        })
    } else {
        let word = first_identifier_in_file(&lines).unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .to_string()
        });

        Ok(explain::SourceContext {
            word,
            file: Some(path.to_string()),
            language,
            to_lang: to_lang.to_string(),
            ..Default::default()
        })
    }
}

fn parse_file_spec(spec: &str) -> (&str, Option<usize>) {
    if let Some(idx) = spec.rfind(':') {
        let line_part = &spec[idx + 1..];
        if let Ok(n) = line_part.parse::<usize>() {
            return (&spec[..idx], Some(n));
        }
    }
    (spec, None)
}

fn extract_identifier(line: &str) -> Option<&str> {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|s| !s.is_empty())
}

fn first_identifier_in_file(lines: &[&str]) -> Option<String> {
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        if let Some(word) = extract_identifier(trimmed) {
            return Some(word.to_string());
        }
    }
    None
}

fn surrounding_lines(lines: &[&str], line_num: usize, context_lines: usize) -> Vec<String> {
    let idx = line_num - 1;
    let start = idx.saturating_sub(context_lines);
    let end = (idx + context_lines + 1).min(lines.len());
    lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>4} | {}", start + i + 1, line))
        .collect()
}

async fn cmd_ask(config: &Config) -> Result<()> {
    println!("ah interactive mode. Type a word to explain, or Ctrl+C to exit.");
    println!();

    loop {
        let mut input = String::new();
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input)?;
        let word = input.trim();
        if word.is_empty() {
            continue;
        }

        let provider = providers::resolve_provider(None, config).await?;
        let provider_name = provider.name().to_string();
        match explain::explain(
            &explain::SourceContext {
                word: word.to_string(),
                to_lang: "中文".to_string(),
                ..Default::default()
            },
            provider.as_ref(),
            false,
        )
        .await {
            Ok(result) => {
                let _ = history::save_query(
                    word,
                    &provider_name,
                    &result.translation,
                    &result.explanation,
                    &result.usage,
                    None,
                    None,
                );
                render::render_explain(&result, "plain");
            }
            Err(e) => crate::error::print_error(&e),
        }
        println!();
    }
}

async fn cmd_init() -> Result<()> {
    let config_path = Config::path();
    if config_path.exists() {
        println!("Config already exists at: {}", config_path.display());
        return Ok(());
    }

    // Create parent dir
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Simple interactive setup
    println!("ah - First-time setup");
    println!();

    // Detect local ollama
    let has_ollama = reqwest::get("http://localhost:11434/api/tags")
        .await
        .is_ok();

    let config_content = if has_ollama {
        println!("✓ Detected local Ollama instance.");
        DEFAULT_CONFIG_WITH_OLLAMA
    } else {
        println!("No local Ollama detected.");
        println!("You can configure OpenAI, DeepSeek, or Anthropic API keys.");
        DEFAULT_CONFIG_CLOUD
    };

    std::fs::write(&config_path, config_content)?;
    println!();
    println!("Config written to: {}", config_path.display());
    println!("Edit it to add API keys or change providers.");
    println!();
    println!("Quick start:");
    println!("  ./start.sh          # 安装并启动 daemon");
    println!("  选中文字 → Ctrl+C   # 自动弹出解释");
    println!("  ah explain map");
    println!("  ah ask");

    Ok(())
}

async fn cmd_history(
    limit: usize,
    search: Option<String>,
    stats: bool,
    clear: bool,
    json: bool,
) -> Result<()> {
    if clear {
        let count = history::clear_history()?;
        println!("Cleared {count} history entries.");
        return Ok(());
    }

    if stats {
        let s = history::query_stats()?;
        if json {
            println!("{}", serde_json::to_string_pretty(&s)?);
        } else {
            println!("📊 Query Statistics");
            println!("  Total queries:  {}", s.total_queries);
            println!("  Unique words:   {}", s.unique_words);
            if let Some((day, cnt)) = &s.top_day {
                println!("  Busiest day:    {day} ({cnt} queries)");
            }
            if !s.top_words.is_empty() {
                println!("\n  Top words:");
                for (word, cnt) in &s.top_words {
                    println!("    {word:<20} {cnt}x");
                }
            }
            if !s.provider_breakdown.is_empty() {
                println!("\n  By provider:");
                for (prov, cnt) in &s.provider_breakdown {
                    println!("    {prov:<20} {cnt}x");
                }
            }
        }
        return Ok(());
    }

    let entries = history::list_queries(limit, search.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No history yet. Run `ah explain <word>` first.");
        return Ok(());
    }

    for entry in &entries {
        use colored::*;
        println!(
            "{}  {}  {}  {}",
            entry.created_at.dimmed(),
            entry.word.yellow().bold(),
            entry.provider.cyan(),
            entry.translation
        );
    }
    println!();
    println!("{} entries shown. Use --stats for summary.", entries.len());

    Ok(())
}

const DEFAULT_CONFIG_WITH_OLLAMA: &str = r#"
[provider]
default = "ollama"

[provider.ollama]
model = "llama3.2"
url = "http://localhost:11434"

[display]
theme = "auto"
"#;

const DEFAULT_CONFIG_CLOUD: &str = r#"
[provider]
default = "auto"

[provider.ollama]
model = "llama3.2"
url = "http://localhost:11434"

[provider.openai]
model = "gpt-4o-mini"
# Set TX_OPENAI_KEY env var, or uncomment:
# api_key = "sk-..."

[provider.deepseek]
model = "deepseek-chat"
url = "https://api.deepseek.com/v1"
# Set TX_DEEPSEEK_KEY env var, or uncomment:
# api_key = "sk-..."

[provider.anthropic]
model = "claude-3-haiku-20240307"
url = "https://api.anthropic.com/v1"
# Set TX_ANTHROPIC_KEY env var, or uncomment:
# api_key = "sk-..."

[display]
theme = "auto"
"#;

#[cfg(test)]
mod file_context_tests {
    use super::*;

    #[test]
    fn test_parse_file_spec_with_line() {
        let (path, line) = parse_file_spec("src/main.rs:42");
        assert_eq!(path, "src/main.rs");
        assert_eq!(line, Some(42));
    }

    #[test]
    fn test_parse_file_spec_without_line() {
        let (path, line) = parse_file_spec("src/main.rs");
        assert_eq!(path, "src/main.rs");
        assert_eq!(line, None);
    }

    #[test]
    fn test_surrounding_lines() {
        let lines = vec!["a", "b", "c", "d", "e"];
        let surrounding = surrounding_lines(&lines, 3, 1);
        assert_eq!(surrounding.len(), 3);
        assert!(surrounding[0].contains("| b"));
        assert!(surrounding[1].contains("| c"));
        assert!(surrounding[2].contains("| d"));
    }

    #[test]
    fn test_extract_identifier() {
        assert_eq!(extract_identifier("fn main()"), Some("fn"));
        assert_eq!(extract_identifier("let x = 1"), Some("let"));
    }
}
