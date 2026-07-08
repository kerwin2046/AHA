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
    fn rejects_pure_punctuation() {
        assert!(!should_process("!!!"));
        assert!(!should_process("..."));
        assert!(!should_process("  ,  "));
    }
}
