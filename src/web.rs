use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

struct AppState;

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
    search: Option<String>,
}

pub async fn serve(port: u16) -> Result<()> {
    let state = Arc::new(AppState);

    let app = Router::new()
        .route("/", get(index))
        .route("/api/history", get(api_history))
        .route("/api/stats", get(api_stats))
        .route("/api/today", get(api_today))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    println!("🌐 Web dashboard: http://{addr}");
    println!("   Press Ctrl+C to stop.");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(HTML)
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

async fn api_stats(
    State(_): State<Arc<AppState>>,
) -> impl IntoResponse {
    match crate::history::query_stats() {
        Ok(stats) => Json(serde_json::json!(stats)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_today(
    State(_): State<Arc<AppState>>,
) -> impl IntoResponse {
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

const HTML: &str = r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>ah — 历史记录</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:#0d1117;color:#e6edf3;min-height:100vh}
.container{max-width:960px;margin:0 auto;padding:24px 16px}
h1{font-size:24px;font-weight:600;margin-bottom:24px;color:#f0f6fc}
h1 span{color:#58a6ff}
.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin-bottom:24px}
.stat-card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;text-align:center}
.stat-card .num{font-size:28px;font-weight:700;color:#58a6ff}
.stat-card .label{font-size:12px;color:#8b949e;margin-top:4px}
.section-title{font-size:16px;font-weight:600;margin-bottom:12px;color:#f0f6fc}
.search-bar{margin-bottom:16px}
.search-bar input{width:100%;padding:10px 14px;background:#0d1117;border:1px solid #30363d;border-radius:6px;color:#e6edf3;font-size:14px;outline:none;transition:border .2s}
.search-bar input:focus{border-color:#58a6ff}
table{width:100%;border-collapse:collapse;font-size:14px}
th,td{padding:10px 12px;text-align:left;border-bottom:1px solid #21262d}
th{color:#8b949e;font-weight:600;font-size:12px;text-transform:uppercase;letter-spacing:.05em}
td.word{color:#58a6ff;font-weight:500;cursor:pointer;max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
td.word:hover{text-decoration:underline}
td.trans{color:#e6edf3;max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
td.provider{color:#8b949e;font-size:12px}
td.time{color:#8b949e;font-size:12px;white-space:nowrap}
tr:hover td{background:#161b22}
.expand-row{display:none}
.expand-row td{padding:0}
.expand-row.show{display:table-row}
.expand-content{padding:12px 16px 16px;background:#161b22;border-bottom:1px solid #30363d;font-size:13px;line-height:1.6;color:#c9d1d9}
.expand-content .label{color:#8b949e;font-size:11px;text-transform:uppercase;margin-bottom:2px}
.expand-content .explanation{margin-bottom:8px;white-space:pre-wrap;word-break:break-word}
.expand-content .usage{background:#1c2128;padding:8px 12px;border-radius:6px;font-family:"SF Mono","Fira Code","Consolas",monospace;font-size:12px;white-space:pre-wrap;word-break:break-word;color:#7ee787}
.empty{padding:32px;text-align:center;color:#8b949e;font-size:14px}
.loading{text-align:center;padding:32px;color:#8b949e}
@keyframes pulse{0%,100%{opacity:.4}50%{opacity:1}}
.loading::after{content:"...";animation:pulse 1.5s ease-in-out infinite}
.time-today{color:#3fb950}
.top-word{display:inline-block;background:#1c2128;border:1px solid #30363d;border-radius:4px;padding:2px 8px;margin:2px;font-size:12px;color:#e6edf3}
.top-word .cnt{color:#8b949e;margin-left:4px}
</style>
</head>
<body>
<div class="container">
<h1>ah <span>dashboard</span></h1>
<div class="stats" id="stats">
<div class="stat-card"><div class="num loading" id="total">—</div><div class="label">总查询</div></div>
<div class="stat-card"><div class="num loading" id="unique">—</div><div class="label">不同词</div></div>
<div class="stat-card"><div class="num loading" id="today">—</div><div class="label">今日</div></div>
<div class="stat-card"><div class="num loading" id="top-day">—</div><div class="label">最活跃日</div></div>
</div>

<div class="section-title">📋 今日查询</div>
<table id="today-table"><tbody id="today-body"><tr><td colspan="4" class="loading">加载中</td></tr></tbody></table>
<br>

<div class="section-title">📜 历史记录</div>
<div class="search-bar"><input type="text" id="search" placeholder="搜索词、翻译、解释..." oninput="loadHistory()"></div>
<table><thead><tr><th>词</th><th>翻译</th><th>Provider</th><th>时间</th></tr></thead><tbody id="history-body"><tr><td colspan="4" class="loading">加载中</td></tr></tbody></table>
</div>

<script>
async function loadStats() {
  let r=await fetch('/api/stats');let d=await r.json();
  document.getElementById('total').textContent=d.total_queries;document.getElementById('total').className='num';
  document.getElementById('unique').textContent=d.unique_words;document.getElementById('unique').className='num';
  document.getElementById('top-day').textContent=d.top_day?d.top_day[0].slice(5):'-';document.getElementById('top-day').className='num';
}
async function loadToday() {
  let r=await fetch('/api/today');let d=await r.json();
  let tbody=document.getElementById('today-body');
  if(!d.length){tbody.innerHTML='<tr><td colspan="4" class="empty">今天还没有查询</td></tr>';document.getElementById('today').textContent='0';document.getElementById('today').className='num';return}
  document.getElementById('today').textContent=d.length;document.getElementById('today').className='num';
  tbody.innerHTML=d.map(e=>renderRow(e)).join('');
}
let expandId=0;
function renderRow(e){
  let id=++expandId;
  let cls=e.created_at&&e.created_at.startsWith(new Date().toISOString().slice(0,10))?'time-today':'';
  return `<tr onclick="toggle(${id})"><td class="word" title="${esc(e.word)}">${esc(e.word)}</td><td class="trans">${esc(e.translation)}</td><td class="provider">${esc(e.provider)}</td><td class="time ${cls}">${e.created_at?e.created_at.slice(5,16):'-'}</td></tr><tr class="expand-row" id="row-${id}"><td colspan="4"><div class="expand-content">${e.explanation?`<div class="label">解释</div><div class="explanation">${esc(e.explanation)}</div>`:''}${e.usage_example?`<div class="label">用法</div><div class="usage">${esc(e.usage_example)}</div>`:''}${e.context_file?`<div class="label">文件</div><div>${esc(e.context_file)}${e.context_language?' ('+esc(e.context_language)+')':''}</div>`:''}</div></td></tr>`;
}
function toggle(id){
  document.getElementById('row-'+id).classList.toggle('show');
}
function esc(s){if(!s)return'';let d=document.createElement('div');d.textContent=s;return d.innerHTML}
let searchTimer;
async function loadHistory(){
  clearTimeout(searchTimer);searchTimer=setTimeout(async()=>{
    let q=document.getElementById('search').value;
    let url='/api/history?limit=200'+(q?'&search='+encodeURIComponent(q):'');
    let r=await fetch(url);let d=await r.json();
    let tbody=document.getElementById('history-body');
    if(!d.length){tbody.innerHTML='<tr><td colspan="4" class="empty">还没有查询记录</td></tr>';return}
    tbody.innerHTML=d.map(e=>renderRow(e)).join('');
  },300);
}
loadStats();loadToday();loadHistory();
</script>
</body></html>"#;
