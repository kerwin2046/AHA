use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use crate::config::OllamaConfig;
use crate::providers::AIProvider;

/// API 请求超时。
const REQUEST_TIMEOUT_SECS: u64 = 60;

pub struct OllamaProvider {
    model: String,
    url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(cfg: &OllamaConfig) -> Self {
        Self {
            model: cfg.model.clone(),
            url: cfg.url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .build()
                .expect("reqwest Client::builder() failed"),
        }
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, prompt: &str) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        Ok(data["response"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
