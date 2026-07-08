
/// Print a user-friendly error message to stderr.
pub fn print_error(err: &anyhow::Error) {
    eprintln!("{}", friendly_error(err));
}

/// Format an error as a user-friendly Chinese message.
pub fn friendly_error(err: &anyhow::Error) -> String {
    let msg = format!("{:#}", err);
    friendly(&msg)
}

fn friendly(msg: &str) -> String {
    // Ollama-specific: check before generic connection errors
    if msg.contains("ollama") && msg.contains("connection") {
        return format!("🦙 Ollama 未运行\n   请先启动: ollama serve");
    }

    // Network / connection errors
    if msg.contains("error sending request")
        || msg.contains("connection refused")
        || msg.contains("connection reset")
        || msg.contains("dns error")
        || msg.contains("timed out")
        || msg.contains("No route to host")
    {
        return format!("⚠ 网络连接失败，请检查网络或 API 地址配置");
    }

    // HTTP status errors
    if msg.contains("401") || msg.contains("unauthorized") || msg.contains("Unauthorized") {
        return format!(
            "🔑 API Key 无效或未设置\n   设置 TX_DEEPSEEK_KEY 或运行 ah init 配置"
        );
    }
    if msg.contains("402") || msg.contains("insufficient") || msg.contains("quota") {
        return format!("💰 API 余额不足，请检查账户配额");
    }
    if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") {
        return format!("⏳ 请求过于频繁，请稍后再试");
    }
    if msg.contains("500") || msg.contains("503") || msg.contains("server error") {
        return format!("🔧 AI 服务暂时不可用，请稍后重试");
    }

    // Provider/Config errors
    if msg.contains("No available AI provider") {
        return format!(
            "⚙ 未配置 AI provider\n   运行 ah init 配置，或设置 TX_DEEPSEEK_KEY 环境变量"
        );
    }

    // JSON / parse errors
    if msg.contains("EOF") || msg.contains("unexpected end") {
        return format!("📭 AI 返回为空，请重试");
    }

    // Input errors
    if msg.contains("Provide a word") || msg.contains("Empty input") {
        return format!(
            "📝 请输入要查询的词\n   用法: ah explain <word> 或 echo word | ah explain --pipe"
        );
    }

    // Line out of range
    if msg.contains("out of range") || msg.contains("Line") {
        return format!("📄 指定的行号超出文件范围");
    }

    // Fallback
    format!("✖ {}", msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error() {
        let e = anyhow::anyhow!("error sending request: connection refused");
        assert!(friendly_error(&e).contains("网络连接失败"));
    }

    #[test]
    fn test_unauthorized() {
        let e = anyhow::anyhow!("HTTP 401 Unauthorized");
        assert!(friendly_error(&e).contains("API Key 无效"));
    }

    #[test]
    fn test_rate_limit() {
        let e = anyhow::anyhow!("429 too many requests");
        assert!(friendly_error(&e).contains("请求过于频繁"));
    }

    #[test]
    fn test_no_provider() {
        let e = anyhow::anyhow!("No available AI provider");
        assert!(friendly_error(&e).contains("未配置 AI provider"));
    }

    #[test]
    fn test_ollama_offline() {
        let e = anyhow::anyhow!("ollama: connection refused at localhost:11434");
        assert!(friendly_error(&e).contains("Ollama 未运行"));
    }

    #[test]
    fn test_empty_input() {
        let e = anyhow::anyhow!("Empty input from stdin");
        assert!(friendly_error(&e).contains("请输入要查询的词"));
    }

    #[test]
    fn test_server_error() {
        let e = anyhow::anyhow!("HTTP 503 Service Unavailable");
        assert!(friendly_error(&e).contains("暂时不可用"));
    }
}
