use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

use crate::providers::AIProvider;

/// A mock provider that returns canned responses for testing.
pub struct MockProvider {
    pub response: String,
    pub delay: Duration,
}

impl MockProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            delay: Duration::ZERO,
        }
    }
}

#[async_trait]
impl AIProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn available(&self) -> bool {
        true
    }

    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        if !self.delay.is_zero() {
            sleep(self.delay).await;
        }
        Ok(self.response.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain;
    use crate::explain::ExplainResult;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_mock_returns_canned_response() {
        let expected = "Hello, world!";
        let provider = MockProvider::new(expected);
        assert!(provider.available().await);
        assert_eq!(provider.name(), "mock");
        let result = provider.complete("any prompt").await.unwrap();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_mock_json_response() {
        let json = r#"{"translation":"测试","explanation":"A test","usage":"test()"}"#;
        let provider = MockProvider::new(json);
        let result = provider.complete("prompt").await.unwrap();
        assert_eq!(result, json);
        let parsed: ExplainResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.translation, "测试");
        assert_eq!(parsed.explanation, "A test");
        assert_eq!(parsed.usage, "test()");
    }

    #[tokio::test]
    async fn test_mock_delay() {
        let provider = MockProvider {
            response: "delayed".to_string(),
            delay: Duration::from_millis(50),
        };
        let start = Instant::now();
        let _ = provider.complete("prompt").await.unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_mock_no_delay() {
        let provider = MockProvider {
            response: "instant".to_string(),
            delay: Duration::ZERO,
        };
        let start = Instant::now();
        let _ = provider.complete("prompt").await.unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_explain_works_with_mock() {
        let json = r#"{"translation":"函数","explanation":"A reusable block of code","usage":"fn foo() {}","full_name":"function"}"#;
        let provider = Box::new(MockProvider::new(json)) as Box<dyn AIProvider>;
        let ctx = explain::SourceContext {
            word: "fn".to_string(),
            to_lang: "中文".to_string(),
            ..Default::default()
        };
        let result = explain::explain(&ctx, provider.as_ref(), false, false)
            .await
            .unwrap();
        assert_eq!(result.translation, "函数");
        assert_eq!(result.explanation, "A reusable block of code");
        assert_eq!(result.usage, "fn foo() {}");
        assert_eq!(result.full_name, "function");
    }

    #[tokio::test]
    async fn test_explain_full_flow() {
        // Full end-to-end: MockProvider with a realistic JSON response
        let response = r#"{"translation":"异步","explanation":"Allows non-blocking execution","usage":"async fn fetch() {}","full_name":"asynchronous"}"#;
        let provider = Box::new(MockProvider::new(response)) as Box<dyn AIProvider>;
        let ctx = explain::SourceContext {
            word: "async".to_string(),
            to_lang: "中文".to_string(),
            ..Default::default()
        };
        let result = explain::explain(&ctx, provider.as_ref(), false, false)
            .await
            .unwrap();

        assert_eq!(result.translation, "异步");
        assert_eq!(result.explanation, "Allows non-blocking execution");
        assert_eq!(result.usage, "async fn fetch() {}");
        assert_eq!(result.full_name, "asynchronous");
    }

    #[tokio::test]
    async fn test_explain_with_minimal_response() {
        // Test that explain handles responses with only translation
        let response = r#"{"translation":"变量"}"#;
        let provider = Box::new(MockProvider::new(response)) as Box<dyn AIProvider>;
        let ctx = explain::SourceContext {
            word: "let".to_string(),
            to_lang: "中文".to_string(),
            ..Default::default()
        };
        let result = explain::explain(&ctx, provider.as_ref(), false, false)
            .await
            .unwrap();

        assert_eq!(result.translation, "变量");
        assert_eq!(result.explanation, "");
        assert_eq!(result.usage, "");
        assert_eq!(result.full_name, "");
    }

    #[tokio::test]
    async fn test_mock_with_resolve_provider() {
        // Test that resolve_provider("mock") returns a working MockProvider
        let config = crate::config::Config {
            provider: crate::config::ProviderConfig {
                default: "mock".to_string(),
                mock: Some(crate::config::MockConfig {
                    response: r#"{"translation":"默认"}"#.to_string(),
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let provider = crate::providers::resolve_provider(Some("mock"), &config)
            .await
            .unwrap();
        assert_eq!(provider.name(), "mock");
        assert!(provider.available().await);
        let result = provider.complete("test").await.unwrap();
        assert_eq!(result, r#"{"translation":"默认"}"#);
    }

    #[tokio::test]
    async fn test_mock_resolve_without_config() {
        // resolve_provider("mock") without MockConfig should create a default one
        let config = crate::config::Config::default();
        let provider = crate::providers::resolve_provider(Some("mock"), &config)
            .await
            .unwrap();
        assert_eq!(provider.name(), "mock");
        let result = provider.complete("test").await.unwrap();
        // Default mock response
        assert!(result.contains("translation"));
    }
}
