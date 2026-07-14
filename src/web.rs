use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode, Uri},
    response::sse::{Event, Sse, KeepAlive},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use futures_util::stream::{Stream, StreamExt};
use rust_embed::Embed;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

#[derive(Clone)]
struct AppState;

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
    search: Option<String>,
}

/// Preact dashboard built by `cd frontend && npm run build`.
#[derive(Embed)]
#[folder = "assets/dashboard/"]
struct Assets;

pub async fn serve(port: u16) -> Result<()> {
    let state = AppState;

    let app = Router::new()
        .route("/api/history", get(api_history))
        .route("/api/history/trends", get(api_trends))
        .route("/api/stats", get(api_stats))
        .route("/api/today", get(api_today))
        .route("/api/weekly", get(api_weekly))
        .route("/api/review", get(api_review))
        .route("/api/events", get(api_events))
        .fallback(static_handler)
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    println!("🌐 Web dashboard: http://{addr}");
    println!("   Press Ctrl+C to stop.");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = Assets::get(path) {
        return embed_response(path, file.data.as_ref());
    }
    // SPA-style fallback for client routes
    if let Some(file) = Assets::get("index.html") {
        return embed_response("index.html", file.data.as_ref());
    }
    Html(FALLBACK_HTML).into_response()
}

fn embed_response(path: &str, data: &[u8]) -> Response {
    let mime = content_type(path);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache_control(path)),
        ],
        data.to_vec(),
    )
        .into_response()
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "json" => "application/json",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn cache_control(path: &str) -> &'static str {
    if path == "index.html" || path.ends_with(".html") {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    }
}

async fn api_history(
    Query(q): Query<HistoryQuery>,
    State(_): State<AppState>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100);
    match crate::history::list_queries(limit, q.search.as_deref()) {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_stats(State(_): State<AppState>) -> impl IntoResponse {
    match crate::history::query_stats() {
        Ok(stats) => Json(serde_json::json!(stats)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_today(State(_): State<AppState>) -> impl IntoResponse {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match crate::history::list_queries(1000, None) {
        Ok(entries) => {
            let today_entries: Vec<_> = entries
                .into_iter()
                .filter(|e| e.created_at.starts_with(&today))
                .collect();
            Json(serde_json::json!(today_entries)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_weekly(State(_): State<AppState>) -> impl IntoResponse {
    match crate::history::query_weekly() {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_review(State(_): State<AppState>) -> impl IntoResponse {
    match crate::history::query_review() {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_trends(State(_): State<AppState>) -> impl IntoResponse {
    match crate::history::query_daily_trend(30) {
        Ok(trends) => Json(serde_json::json!(trends)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_events(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures_util::stream::unfold(
        0i64,
        |mut last_id| async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let current_max = crate::history::query_max_id().unwrap_or(None).unwrap_or(0);
            if current_max <= last_id {
                return Some((
                    futures_util::stream::iter(vec![Ok(Event::default().data("ping"))]),
                    last_id,
                ));
            }
            let entries =
                crate::history::list_queries_since(last_id, 50).unwrap_or_default();
            let new_last = entries.last().map(|e| e.id).unwrap_or(current_max);
            let events: Vec<Result<Event, Infallible>> = entries
                .into_iter()
                .map(|e| Ok(Event::default().json_data(e).unwrap_or_default()))
                .collect();
            Some((futures_util::stream::iter(events), new_last))
        },
    )
    .flatten();
    Sse::new(stream).keep_alive(KeepAlive::default())
}

const FALLBACK_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh"><head><meta charset="UTF-8"><title>ah</title></head>
<body style="font-family:sans-serif;background:#0d1117;color:#e6edf3;padding:2rem">
<h1>ah dashboard</h1>
<p>UI assets missing. Run <code>cd frontend && npm install && npm run build</code>, then rebuild ah.</p>
</body></html>"#;
