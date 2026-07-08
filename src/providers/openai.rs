use anyhow::{Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use crate::config::OpenAIConfig;
use crate::providers::AIProvider;

/// API 请求超时，与 daemon 保持一致。
const REQUEST_TIMEOUT_SECS: u64 = 60;

pub struct OpenAIProvider {
    model: String,
    url: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(cfg: &OpenAIConfig) -> Self {
        let api_key = if !cfg.api_key.is_empty() {
            cfg.api_key.clone()
        } else {
            // Try env var based on URL pattern
            if cfg.url.contains("openai") {
                std::env::var("TX_OPENAI_KEY").unwrap_or_default()
            } else if cfg.url.contains("deepseek") {
                std::env::var("TX_DEEPSEEK_KEY").unwrap_or_default()
            } else if cfg.url.contains("anthropic") {
                std::env::var("TX_ANTHROPIC_KEY").unwrap_or_default()
            } else {
                String::new()
            }
        };

        Self {
            model: cfg.model.clone(),
            url: cfg.url.trim_end_matches('/').to_string(),
            api_key,
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .build()
                .expect("reqwest Client::builder() failed"),
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        if self.url.contains("deepseek") {
            "deepseek"
        } else if self.url.contains("anthropic") {
            "anthropic"
        } else {
            "openai"
        }
    }

    async fn available(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn complete(&self, prompt: &str) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ],
            "response_format": { "type": "json_object" },
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .context("OpenAI API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error ({status}): {text}");
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
