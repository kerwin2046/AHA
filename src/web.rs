use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use rust_embed::Embed;
use serde::Deserialize;
use std::sync::Arc;

struct AppState;

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
    search: Option<String>,
}

#[derive(Deserialize)]
struct WebSearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

/// Preact dashboard built by `cd frontend && npm run build`.
#[derive(Embed)]
#[folder = "assets/dashboard/"]
struct Assets;

pub async fn serve(port: u16) -> Result<()> {
    let state = Arc::new(AppState);

    let app = Router::new()
        .route("/api/history", get(api_history))
        .route("/api/stats", get(api_stats))
        .route("/api/today", get(api_today))
        .route("/api/weekly", get(api_weekly))
        .route("/api/review", get(api_review))
        .route("/api/search", get(api_search))
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
        [(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache_control(path))],
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
    State(_): State<Arc<AppState>>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100);
    match crate::history::list_queries(limit, q.search.as_deref()) {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_stats(State(_): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::history::query_stats() {
        Ok(stats) => Json(serde_json::json!(stats)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_today(State(_): State<Arc<AppState>>) -> impl IntoResponse {
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
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_weekly(State(_): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::history::query_weekly() {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_review(State(_): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::history::query_review() {
        Ok(entries) => Json(serde_json::json!(entries)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_search(Query(q): Query<WebSearchQuery>) -> impl IntoResponse {
    let query = q.q.unwrap_or_default();
    let query = query.trim();
    if query.is_empty() {
        return Json(serde_json::json!({
            "query": "",
            "results": [],
        }))
        .into_response();
    }

    let config = crate::config::Config::load();
    let limit = q
        .limit
        .unwrap_or(config.search.max_results)
        .clamp(1, 20);

    match crate::search::search_searxng(&config.search.searxng_url, query, limit).await {
        Ok(results) => Json(serde_json::json!({
            "query": query,
            "results": results,
        }))
        .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "query": query,
                "error": e.to_string(),
                "results": [],
            })),
        )
            .into_response(),
    }
}

const FALLBACK_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh"><head><meta charset="UTF-8"><title>ah</title></head>
<body style="font-family:sans-serif;background:#0d1117;color:#e6edf3;padding:2rem">
<h1>ah dashboard</h1>
<p>UI assets missing. Run <code>cd frontend && npm install && npm run build</code>, then rebuild ah.</p>
</body></html>"#;
