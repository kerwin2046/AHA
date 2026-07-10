use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const POLL_INTERVAL_MS: u64 = 300;
const DEBOUNCE_MS: u64 = 800;
/// API 请求最大等待时间，超过则强制释放 BUSY 并通知用户。
const REQUEST_TIMEOUT_SECS: u64 = 60;

static BUSY: AtomicBool = AtomicBool::new(false);

/// Main daemon loop.
pub async fn run_daemon(config: &crate::config::Config) -> Result<()> {
    let _daemon_guard = crate::notify_guard::DaemonGuard::acquire()?;
    let pid = std::process::id();

    println!("ah daemon started (pid {pid}) — watching clipboard...");
    println!("  Poll: {POLL_INTERVAL_MS}ms | Debounce: {DEBOUNCE_MS}ms");
    println!("  Press Ctrl+C to stop.");
    println!();

    let provider_name = std::env::var("TX_PROVIDER").ok();
    let provider = crate::providers::resolve_provider(provider_name.as_deref(), config).await?;
    let provider_name_str = provider.name().to_string();

    let (tx_stop, rx_stop) = tokio::sync::watch::channel(false);
    let tx_stop_clone = tx_stop.clone();
    ctrlc::set_handler(move || {
        let _ = tx_stop_clone.send(true);
    })
    .map_err(|e| anyhow::anyhow!("Failed to set Ctrl+C handler: {e}"))?;
    let mut last_hash: u64 = 0;
    let mut pending: Option<(String, u64, Instant)> = None;
    let mut busy_pending: Option<(String, u64)> = None;


    loop {
        if *rx_stop.borrow() {
            println!("\ndaemon stopped.");
            return Ok(());
        }

        let content = crate::selection::read_clipboard().unwrap_or_default();
        let hash = if content.is_empty() {
            0
        } else {
            crate::notify_guard::content_hash(&content)
        };
        if hash == 0 || hash == last_hash {
            pending = None;
            busy_pending = None;
        } else if BUSY.load(Ordering::Acquire) {
            // Track the latest clipboard content seen while busy.
            if hash != last_hash {
                busy_pending = Some((content.clone(), hash));
            }
        } else {
            // Content that appeared during a busy API call: skip debounce.
            if let Some((busy_text, busy_hash)) = busy_pending.take() {
                if hash == busy_hash && hash != last_hash {
                    pending = Some((busy_text, busy_hash, Instant::now().checked_sub(Duration::from_millis(DEBOUNCE_MS)).unwrap()));
                }
            }

            match &pending {
                Some((_, pending_hash, started))
                    if *pending_hash == hash
                        && started.elapsed() >= Duration::from_millis(DEBOUNCE_MS) =>
                {
                    pending = None;
                    if crate::filter::should_process(&content)
                        && crate::notify_guard::try_claim(&content)
                    {
                        last_hash = hash;
                        process_clipboard(
                            &content,
                            provider.as_ref(),
                            &provider_name_str,
                        )
                        .await;
                    } else {
                        // Skip junk or duplicate, but don't re-trigger for same hash.
                        last_hash = hash;
                    }
                }
                Some((_, pending_hash, _)) if *pending_hash != hash => {
                    pending = Some((content.clone(), hash, Instant::now()));
                }
                None => {
                    pending = Some((content.clone(), hash, Instant::now()));
                }
                _ => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

async fn process_clipboard(
    word: &str,
    provider: &dyn crate::providers::AIProvider,
    provider_name: &str,
) {
    BUSY.store(true, Ordering::Release);
    let result = tokio::time::timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        process_clipboard_inner(word, provider, provider_name),
    )
    .await;
    BUSY.store(false, Ordering::Release);

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let msg = format!("AI 解释失败: {e:#}");
            eprintln!("  {msg}");
            show_notification("ah 出错了", &msg);
        }
        Err(_) => {
            let msg = format!(
                "AI 请求超时（{REQUEST_TIMEOUT_SECS}s），请检查网络或换个 provider"
            );
            eprintln!("  {msg}");
            show_notification("ah 请求超时", &msg);
        }
    }
}

async fn process_clipboard_inner(
    word: &str,
    provider: &dyn crate::providers::AIProvider,
    provider_name: &str,
) -> Result<()> {
    let preview = word
        .lines()
        .next()
        .unwrap_or(word)
        .chars()
        .take(60)
        .collect::<String>();
    let line_hint = if word.contains('\n') {
        format!("  → [{} lines] {preview}…", word.lines().count())
    } else {
        format!("  → {word}")
    };
    println!("{line_hint}");

    let ctx = crate::explain::SourceContext {
        word: word.to_string(),
        to_lang: "中文".to_string(),
        ..Default::default()
    };

    let r = crate::explain::explain(&ctx, provider, false).await?;
    let _ = crate::history::save_query(
        word,
        provider_name,
        &r.translation,
        &r.explanation,
        &r.usage,
        None,
        None,
    );
    let summary = if !r.translation.is_empty() {
        format!("{} — {}", short_title(word), r.translation)
    } else {
        short_title(word)
    };
    let mut body = r.explanation.clone();
    if !r.usage.is_empty() {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&r.usage);
    }
    println!("    翻译: {}", r.translation);
    if !r.explanation.is_empty() {
        for line in r.explanation.lines() {
            println!("    {}", line);
        }
    }
    if !r.usage.is_empty() {
        println!("    用法:");
        for line in r.usage.lines() {
            println!("      {}", line);
        }
    }
    show_notification(&summary, &body);
    Ok(())
}

fn short_title(s: &str) -> String {
    let one_line = s.replace('\n', " ");
    let max = 48;
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut out: String = one_line.chars().take(max).collect();
        out.push('…');
        out
    }
}

pub(crate) fn show_notification(summary: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!(
                "display notification \"{}\" with title \"{}\"",
                body.replace('"', "\\\""),
                summary.replace('"', "\\\""),
            )])
            .output();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = notify_rust::Notification::new()
            .summary(summary)
            .body(body)
            .appname("ah")
            .icon("terminal")
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show();
    }
}
