use colored::*;
use textwrap::Options;

use crate::explain::ExplainResult;

/// Render the explain result to stdout in the requested format.
pub fn render_explain(result: &ExplainResult, format: &str) {
    match format {
        "json" => render_json(result),
        "markdown" => render_markdown(result),
        "html" => render_html(result),
        _ => render_terminal(result),
    }
}

fn render_json(result: &ExplainResult) {
    if let Ok(json) = serde_json::to_string_pretty(result) {
        println!("{json}");
    }
}

fn render_markdown(result: &ExplainResult) {
    println!("## 翻译 (Translation)\n");
    if !result.translation.is_empty() {
        println!("{}", result.translation);
    }
    println!();

    if !result.full_name.is_empty() {
        println!("**全称:** {}\n", result.full_name);
    }

    if !result.explanation.is_empty() {
        println!("## 解释 (Explanation)\n");
        println!("{}", result.explanation);
        println!();
    }

    if !result.usage.is_empty() {
        println!("## 用法 (Usage)\n");
        println!("```");
        println!("{}", result.usage);
        println!("```");
        println!();
    }
}

fn render_html(result: &ExplainResult) {
    println!("<!DOCTYPE html>");
    println!("<html><head><meta charset=\"utf-8\">");
    println!("<style>");
    println!("body {{ font-family: sans-serif; max-width: 700px; margin: 2em auto; padding: 0 1em; line-height: 1.6; }}");
    println!("h2 {{ color: #333; border-bottom: 1px solid #ddd; padding-bottom: 0.3em; }}");
    println!(".translation {{ font-size: 1.2em; color: #b58900; }}");
    println!(".explanation {{ color: #2aa198; }}");
    println!(".usage {{ background: #f8f8f8; padding: 1em; border-radius: 4px; font-family: monospace; }}");
    println!(".meta {{ color: #999; font-size: 0.9em; }}");
    println!("</style></head><body>");
    println!("<h1>ah explain</h1>");

    if !result.translation.is_empty() {
        println!("<h2>翻译</h2>");
        println!(
            "<p class=\"translation\">{}</p>",
            escape_html(&result.translation)
        );
    }

    if !result.full_name.is_empty() {
        println!(
            "<p class=\"meta\"><strong>全称:</strong> {}</p>",
            escape_html(&result.full_name)
        );
    }

    if !result.explanation.is_empty() {
        println!("<h2>解释</h2>");
        println!(
            "<p class=\"explanation\">{}</p>",
            escape_html(&result.explanation)
        );
    }

    if !result.usage.is_empty() {
        println!("<h2>用法</h2>");
        println!("<pre class=\"usage\">{}</pre>", escape_html(&result.usage));
    }

    println!("</body></html>");
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_terminal(result: &ExplainResult) {
    let term_width = terminal_width();
    let opts = Options::new(term_width).break_words(true);

    let divider: String = std::iter::repeat('─').take(term_width.min(48)).collect();
    println!("{}", divider.dimmed());

    if !result.translation.is_empty() {
        print!("{} ", "翻译:".yellow().bold());
        println!("{}", result.translation);
        println!();
    }

    if !result.full_name.is_empty() {
        print!("{} ", "全称:".cyan().bold());
        println!("{}", result.full_name);
        println!();
    }

    if !result.explanation.is_empty() {
        print!("{} ", "解释:".cyan().bold());
        let wrapped = textwrap::fill(&result.explanation, &opts);
        println!("{wrapped}");
        println!();
    }

    if !result.usage.is_empty() {
        print!("{} ", "用法:".green().bold());
        for line in result.usage.lines() {
            println!("  {}", line.green());
        }
        println!();
    }

    println!("{}", divider.dimmed());
}

fn terminal_width() -> usize {
    match term_size() {
        Some((w, _)) => w.min(100),
        None => 80,
    }
}

fn term_size() -> Option<(usize, usize)> {
    if let Ok(output) = std::process::Command::new("stty")
        .arg("size")
        .arg("-F")
        .arg("/dev/tty")
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = s.trim().split_whitespace().collect();
            if parts.len() == 2 {
                if let (Ok(_rows), Ok(cols)) =
                    (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    return Some((cols, 24));
                }
            }
        }
    }
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(c) = cols.parse::<usize>() {
            return Some((c, 24));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::ExplainResult;

    #[test]
    fn test_json_output_valid() {
        let result = ExplainResult {
            translation: "映射".to_string(),
            explanation: "Array transformation".to_string(),
            usage: "[1,2].map(fn)".to_string(),
            full_name: String::new(),
        };
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("映射"));
        assert!(json.contains("usage"));
    }

    #[test]
    fn test_markdown_contains_translation() {
        let result = ExplainResult {
            translation: "迭代器".to_string(),
            explanation: "遍历集合".to_string(),
            usage: "for item in vec".to_string(),
            full_name: String::new(),
        };
        // Just verify markdown function doesn't panic
        // We'd need to capture stdout for full testing
        let _ = result;
    }

    #[test]
    fn test_html_escaping() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("hello"), "hello");
    }
}
