use async_trait::async_trait;

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn available(&self) -> bool;
    async fn complete(&self, prompt: &str) -> anyhow::Result<String>;
}

pub mod mock;
pub mod ollama;
pub mod openai;

/// Resolve a provider by name, falling back to config default or auto-detection.
pub async fn resolve_provider(
    name: Option<&str>,
    config: &crate::config::Config,
) -> anyhow::Result<Box<dyn AIProvider>> {
    let preferred = name.unwrap_or(&config.provider.default);

    // exact match first
    if preferred == "ollama" {
        if let Some(cfg) = &config.provider.ollama {
            return Ok(Box::new(ollama::OllamaProvider::new(cfg)));
        }
    }
    if preferred == "openai" {
        if let Some(cfg) = &config.provider.openai {
            return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
        }
    }
    if preferred == "deepseek" {
        if let Some(cfg) = &config.provider.deepseek {
            return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
        }
        // Fallback: create default DeepSeek config from env var
        if std::env::var("TX_DEEPSEEK_KEY").is_ok() {
            let cfg = crate::config::OpenAIConfig {
                model: "deepseek-chat".to_string(),
                api_key: String::new(),
                url: "https://api.deepseek.com/v1".to_string(),
            };
            return Ok(Box::new(openai::OpenAIProvider::new(&cfg)));
        }
    }
    if preferred == "anthropic" {
        if let Some(cfg) = &config.provider.anthropic {
            return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
        }
        if std::env::var("TX_ANTHROPIC_KEY").is_ok() {
            let cfg = crate::config::OpenAIConfig {
                model: "claude-3-haiku-20240307".to_string(),
                api_key: String::new(),
                url: "https://api.anthropic.com/v1".to_string(),
            };
            return Ok(Box::new(openai::OpenAIProvider::new(&cfg)));
        }
    }
    if preferred == "mock" {
        if let Some(cfg) = &config.provider.mock {
            return Ok(Box::new(mock::MockProvider::new(&cfg.response)));
        }
        return Ok(Box::new(mock::MockProvider::new(
            r#"{"translation":"测试","explanation":"A test explanation"}"#,
        )));
    }

    // auto-detect: try ollama first, then configured cloud providers
    if preferred == "auto" {
        if let Some(cfg) = &config.provider.ollama {
            let p = ollama::OllamaProvider::new(cfg);
            if p.available().await {
                return Ok(Box::new(p));
            }
        }
        if let Some(cfg) = &config.provider.openai {
            if !cfg.api_key.is_empty() || std::env::var("TX_OPENAI_KEY").is_ok() {
                return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
            }
        }
        if let Some(cfg) = &config.provider.deepseek {
            if !cfg.api_key.is_empty() || std::env::var("TX_DEEPSEEK_KEY").is_ok() {
                return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
            }
        }
        if let Some(cfg) = &config.provider.anthropic {
            if !cfg.api_key.is_empty() || std::env::var("TX_ANTHROPIC_KEY").is_ok() {
                return Ok(Box::new(openai::OpenAIProvider::new(cfg)));
            }
        }
    }

    anyhow::bail!(
        "No available AI provider. Configure one in {} or set environment variables.",
        crate::config::Config::path().display()
    );
}
