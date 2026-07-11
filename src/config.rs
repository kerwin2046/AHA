use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct ProviderConfig {
    #[serde(default = "default_provider")]
    pub default: String,

    pub ollama: Option<OllamaConfig>,
    pub openai: Option<OpenAIConfig>,
    pub deepseek: Option<OpenAIConfig>,
    pub anthropic: Option<OpenAIConfig>,
    pub mock: Option<MockConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_model")]
    pub model: String,
    #[serde(default = "default_ollama_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIConfig {
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_openai_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MockConfig {
    pub response: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct CaptureConfig {
    #[serde(default = "default_context_lines")]
    pub context_lines: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SearchConfig {
    /// SearXNG instance URL, e.g. http://localhost:8888
    #[serde(default = "default_searxng_url")]
    pub searxng_url: String,
    /// Max results to fetch per query
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            searxng_url: default_searxng_url(),
            max_results: default_search_results(),
        }
    }
}

fn default_searxng_url() -> String {
    "http://localhost:8888".to_string()
}
fn default_search_results() -> usize {
    5
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct DisplayConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_provider() -> String {
    "auto".to_string()
}
fn default_ollama_model() -> String {
    "llama3.2".to_string()
}
fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_openai_url() -> String {
    "https://api.openai.com/v1".to_string()
}
fn default_context_lines() -> usize {
    5
}
fn default_format() -> String {
    "plain".to_string()
}
fn default_theme() -> String {
    "auto".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                default: default_provider(),
                ollama: Some(OllamaConfig {
                    model: default_ollama_model(),
                    url: default_ollama_url(),
                }),
                openai: None,
                deepseek: None,
                anthropic: None,
                mock: None,
            },
            capture: CaptureConfig {
                context_lines: default_context_lines(),
            },
            display: DisplayConfig {
                format: default_format(),
                theme: default_theme(),
            },
            search: SearchConfig {
                searxng_url: default_searxng_url(),
                max_results: default_search_results(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Config::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        });
        base.join("ah").join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.provider.default, "auto");
        assert!(cfg.provider.ollama.is_some());
        assert_eq!(cfg.capture.context_lines, 5);
    }

    #[test]
    fn test_config_path() {
        let path = Config::path();
        assert!(path.ends_with("ah/config.toml"));
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let cfg = Config::load();
        // Should not panic; returns default config
        assert_eq!(cfg.provider.default, "auto");
    }

    #[test]
    fn test_ollama_config_defaults() {
        let cfg = Config::default();
        let ollama = cfg.provider.ollama.unwrap();
        assert_eq!(ollama.model, "llama3.2");
        assert_eq!(ollama.url, "http://localhost:11434");
    }
}
