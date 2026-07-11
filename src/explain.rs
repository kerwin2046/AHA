use anyhow::Result;
use lru::LruCache;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::LazyLock;

/// In-memory LRU cache for query results.
static QUERY_CACHE: LazyLock<Mutex<LruCache<String, ExplainResult>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())));
/// Structured response from the AI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExplainResult {
    pub translation: String,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub usage: String,
    #[serde(default)]
    pub full_name: String,
}

/// Context about where the word was found.
#[derive(Debug, Default)]
pub struct SourceContext {
    pub word: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub language: Option<String>,
    pub surrounding: Vec<String>,
    /// Target translation language, e.g. "中文", "日本語", "English"
    pub to_lang: String,
    /// Active app context (auto-detected), e.g. "Code", "Terminal"
    pub app_context: Option<String>,
}

/// Build the AI prompt.
pub fn build_prompt(ctx: &SourceContext, expand: bool) -> String {
    if expand {
        build_detailed_prompt(ctx)
    } else {
        build_short_prompt(ctx)
    }
}

fn append_file_context(prompt: &mut String, ctx: &SourceContext) {
    if let Some(lang) = &ctx.language {
        prompt.push_str(&format!("Language: {}\n", lang));
    }
    if let Some(file) = &ctx.file {
        prompt.push_str(&format!("File: {}\n", file));
        if let Some(line) = ctx.line {
            prompt.push_str(&format!("Line: {}\n", line));
        }
    }
    if !ctx.surrounding.is_empty() {
        prompt.push_str("Nearby code:\n```\n");
        for line in &ctx.surrounding {
            prompt.push_str(line);
            prompt.push('\n');
        }
        prompt.push_str("```\n");
    }
}

fn build_short_prompt(ctx: &SourceContext) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code assistant. The user copied this from their editor:\n\n");
    prompt.push_str(&format!("```\n{}\n```\n\n", ctx.word));

    if let Some(ref app) = ctx.app_context {
        prompt.push_str(&format!("App: {app}\n"));
    }
    append_file_context(&mut prompt, ctx);

    prompt.push_str(
        &format!(
            r#"If it's a single word or short identifier (e.g. map, replicate, topk):
  - Translate to {target}
  - Briefly explain what it means in code

If it's a code snippet (function, struct, trait, expression, etc.):
  - Explain what it does in plain {target}
  - Describe parameters, return type, purpose
  - No word-by-word translation needed

Respond in JSON:
{{
  "translation": "if single word: translation, else: what this code does (1 short phrase)",
  "explanation": "meaning/purpose in {target}",
  "usage": "example if applicable, else empty string"
}}
"#,
            target = ctx.to_lang
        ),
    );
    prompt
}

fn build_detailed_prompt(ctx: &SourceContext) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code assistant. The user wants a detailed explanation of this code:\n\n");
    prompt.push_str(&format!("```\n{}\n```\n\n", ctx.word));

    if let Some(ref app) = ctx.app_context {
        prompt.push_str(&format!("App: {app}\n"));
    }
    append_file_context(&mut prompt, ctx);

    prompt.push_str(
        &format!(
            r#"If it's a single word or short identifier:
  - Translate to {target}
  - Full name if abbreviation
  - Explain in context

If it's a code snippet:
  - Explain what it does in {target}
  - Describe each part (parameters, return, logic)
  - No word-by-word translation

Respond in JSON:
{{
  "translation": "if word: translation, if code: one-line summary",
  "explanation": "detailed explanation in {target} (3-5 sentences)",
  "full_name": "full name if abbreviation, else empty string",
  "usage": "example if applicable, else empty string"
}}
"#,
            target = ctx.to_lang
        ),
    );
    prompt
}

/// Parse the AI response into ExplainResult.
pub fn parse_response(text: &str) -> Result<ExplainResult> {
    // Try direct JSON parse
    if let Ok(r) = serde_json::from_str::<ExplainResult>(text) {
        return Ok(r);
    }

    // Try to extract JSON from a markdown code block
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            let json = &text[start..=end];
            if let Ok(r) = serde_json::from_str::<ExplainResult>(json) {
                return Ok(r);
            }
        }
    }

    // Fallback: treat the whole response as explanation
    Ok(ExplainResult {
        translation: String::new(),
        explanation: text.to_string(),
        usage: String::new(),
        full_name: String::new(),
    })
}

/// Detect likely programming language from a file path.
pub fn detect_language(file: &str) -> Option<&'static str> {
    let ext = file.rsplit('.').next()?;
    Some(match ext {
        "rs" => "Rust",
        "ts" => "TypeScript",
        "tsx" => "TypeScript React",
        "js" => "JavaScript",
        "jsx" => "JavaScript React",
        "py" => "Python",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "c" => "C",
        "h" => "C",
        "cpp" | "cc" | "cxx" => "C++",
        "hpp" => "C++",
        "cs" => "C#",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "scala" => "Scala",
        "zig" => "Zig",
        "sh" | "bash" => "Bash",
        "zsh" => "Zsh",
        "lua" => "Lua",
        "pl" => "Perl",
        "php" => "PHP",
        "r" => "R",
        "sql" => "SQL",
        "yaml" | "yml" => "YAML",
        "json" => "JSON",
        "toml" => "TOML",
        "md" => "Markdown",
        "html" => "HTML",
        "css" => "CSS",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "clj" | "cljs" => "Clojure",
        "hs" => "Haskell",
        _ => return None,
    })
}

/// The main explain operation.
pub async fn explain(
    ctx: &SourceContext,
    provider: &dyn crate::providers::AIProvider,
    expand: bool,
) -> Result<ExplainResult> {
    let cache_key = format!(
        "{}:{}:{}:{:?}:{:?}",
        ctx.word, provider.name(), ctx.to_lang, ctx.file, ctx.line
    );

    // Check cache
    {
        let mut cache = QUERY_CACHE.lock();
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }

    let prompt = build_prompt(ctx, expand);
    let response = provider.complete(&prompt).await?;
    let result = parse_response(&response)?;

    // Store in cache
    {
        let mut cache = QUERY_CACHE.lock();
        cache.put(cache_key, result.clone());
    }

    Ok(result)
}
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[test]
    fn test_parse_response_direct_json() {
        let json = r#"{"translation":"映射","explanation":"transforms array items","usage":"[1,2].map(x=>x*2)"}"#;
        let r = parse_response(json).unwrap();
        assert_eq!(r.translation, "映射");
        assert_eq!(r.explanation, "transforms array items");
        assert_eq!(r.usage, "[1,2].map(x=>x*2)");
    }

    #[test]
    fn test_parse_response_codeblock() {
        let text = "Here is the result:\n```json\n{\"translation\":\"命名空间\"}\n```\n";
        let r = parse_response(text).unwrap();
        assert_eq!(r.translation, "命名空间");
    }

    #[test]
    fn test_parse_response_text_fallback() {
        let text = "Just a plain text explanation";
        let r = parse_response(text).unwrap();
        assert_eq!(r.translation, "");
        assert_eq!(r.explanation, "Just a plain text explanation");
        assert_eq!(r.usage, "");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), Some("Rust"));
        assert_eq!(detect_language("app.tsx"), Some("TypeScript React"));
        assert_eq!(detect_language("Dockerfile"), None);
        assert_eq!(detect_language("test.py"), Some("Python"));
    }
}
