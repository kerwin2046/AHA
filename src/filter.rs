/// Whether clipboard/selection content is worth sending to the AI.
pub fn should_process(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() < 2 {
        return false;
    }
    if is_standalone_url(trimmed) {
        return false;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_punctuation() || c.is_whitespace())
    {
        return false;
    }
    true
}

/// Only reject when the entire selection is a single URL, not code that mentions one.
fn is_standalone_url(text: &str) -> bool {
    if text.contains('\n') {
        return false;
    }
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://") || t.starts_with("ftp://"))
        && t.split_whitespace().count() == 1
}

/// Detect if text looks like a compiler error, stack trace, or runtime error.
pub fn looks_like_error(text: &str) -> bool {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > 30 {
        return false; // too long, probably regular code
    }

    let first = lines.first().map(|l| l.to_lowercase()).unwrap_or_default();

    // Compiler error patterns: error[E0425], error: ..., error CS...
    if first.starts_with("error[") || first.starts_with("error ") || first.starts_with("error:") {
        return true;
    }
    if first.starts_with("fatal error") || first.starts_with("fatal:") {
        return true;
    }

    // Rust panic
    if first.contains("panicked") || first.contains("thread '") && first.contains("panicked") {
        return true;
    }

    // Stack trace patterns
    if lines
        .iter()
        .any(|l| l.trim().starts_with("at ") && l.contains(':'))
    {
        // at src/main.rs:42:5
        return true;
    }

    // Python traceback
    if first.starts_with("traceback") || first.contains("traceback (most recent call last)") {
        return true;
    }

    // Java/C# exception
    if lines.iter().any(|l| {
        l.contains("Exception") && (l.ends_with(':') || l.ends_with(": ") || *l == l.trim())
    }) || first.ends_with("Exception")
        || first.ends_with("Error")
    {
        return true;
    }

    // Common error prefixes
    let error_prefixes = [
        "failed:",
        "unable to",
        "could not",
        "cannot find",
        "no such",
        "is not",
        "unresolved",
        "undefined",
        "expected ",
        "unexpected",
        "mismatched",
        "missing ",
        "duplicate",
        "invalid",
        "unknown ",
    ];
    if lines.len() <= 5 && error_prefixes.iter().any(|p| first.starts_with(p)) {
        return true;
    }

    // File:line patterns common in tool output (e.g. grep, linter)
    if lines.len() <= 8
        && lines.iter().any(|l| {
            // src/main.rs:42 or src/main.rs:42:5
            l.chars().filter(|&c| c == ':').count() >= 2 && l.contains('/')
        })
        && !text.contains("fn ")
        && !text.contains("impl ")
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_words() {
        assert!(should_process("map"));
        assert!(should_process("useEffect"));
        assert!(should_process("fn main()"));
    }

    #[test]
    fn accepts_multiline_code() {
        let code = "pub fn foo() {\n    bar()\n}";
        assert!(should_process(code));
    }

    #[test]
    fn accepts_multiline_with_url_inside() {
        let code = "let url = \"https://example.com\";\nprintln!(\"{url}\");";
        assert!(should_process(code));
    }

    #[test]
    fn rejects_too_short() {
        assert!(!should_process(""));
        assert!(!should_process("a"));
    }

    #[test]
    fn rejects_urls() {
        assert!(!should_process("https://example.com"));
        assert!(!should_process("http://localhost:8080"));
    }

    #[test]
    fn rejects_pure_digits() {
        assert!(!should_process("42"));
        assert!(!should_process("12345"));
    }

    #[test]
    fn detects_rust_compiler_error() {
        let err = "error[E0425]: cannot find value `x` in this scope\n --> src/main.rs:10:5";
        assert!(looks_like_error(err));
    }

    #[test]
    fn detects_rust_panic() {
        let err = "thread 'main' panicked at 'index out of bounds', src/main.rs:10";
        assert!(looks_like_error(err));
    }

    #[test]
    fn detects_python_traceback() {
        let err = "Traceback (most recent call last):\n  File \"test.py\", line 5, in <module>\n    foo()";
        assert!(looks_like_error(err));
    }

    #[test]
    fn detects_type_error() {
        let err = "TypeError: 'int' object is not iterable";
        assert!(looks_like_error(err));
    }

    #[test]
    fn rejects_normal_code() {
        assert!(!looks_like_error("fn map<T>(x: T) -> T { x }"));
        assert!(!looks_like_error("let x = 42;"));
        assert!(!looks_like_error("pub struct Config { name: String }"));
    }

    #[test]
    fn rejects_long_text() {
        let long = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!looks_like_error(&long));
    }

    #[test]
    fn rejects_pure_punctuation() {
        assert!(!should_process("!!!"));
        assert!(!should_process("..."));
        assert!(!should_process("  ,  "));
    }
}
