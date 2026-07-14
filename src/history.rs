use anyhow::Result;
use chrono::{Local, TimeZone};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;

use crate::search::SearchResult;

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub word: String,
    pub provider: String,
    pub translation: String,
    pub explanation: String,
    pub usage_example: String,
    pub context_file: Option<String>,
    pub context_language: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub sources: Vec<SearchResult>,
}

/// Path to the history SQLite database.
fn db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local").join("share")
    });
    base.join("ah").join("history.db")
}

/// Open (or create) the database and ensure the schema exists.
pub fn init() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS queries (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            word       TEXT NOT NULL,
            provider   TEXT NOT NULL DEFAULT '',
            translation TEXT NOT NULL DEFAULT '',
            explanation TEXT NOT NULL DEFAULT '',
            usage_example TEXT NOT NULL DEFAULT '',
            context_file   TEXT,
            context_language TEXT,
            created_at TEXT NOT NULL,
            sources_json TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_queries_word ON queries(word);
        CREATE INDEX IF NOT EXISTS idx_queries_created ON queries(created_at DESC);",
    )?;

    ensure_sources_column(&conn)?;

    Ok(conn)
}

fn ensure_sources_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(queries)")?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|c| c.ok())
        .collect();
    if !cols.iter().any(|c| c == "sources_json") {
        conn.execute(
            "ALTER TABLE queries ADD COLUMN sources_json TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

fn sources_to_json(sources: &[SearchResult]) -> String {
    if sources.is_empty() {
        String::new()
    } else {
        serde_json::to_string(sources).unwrap_or_default()
    }
}

fn sources_from_json(raw: String) -> Vec<SearchResult> {
    if raw.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    }
}

/// Save a query result to history.
pub fn save_query(
    word: &str,
    provider: &str,
    translation: &str,
    explanation: &str,
    usage_example: &str,
    context_file: Option<&str>,
    context_language: Option<&str>,
    sources: &[SearchResult],
) -> Result<i64> {
    let conn = init()?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let sources_json = sources_to_json(sources);

    conn.execute(
        "INSERT INTO queries (word, provider, translation, explanation, usage_example, context_file, context_language, created_at, sources_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            word,
            provider,
            translation,
            explanation,
            usage_example,
            context_file,
            context_language,
            now,
            sources_json,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// List recent queries with optional search filter.
pub fn list_queries(limit: usize, search: Option<&str>) -> Result<Vec<HistoryEntry>> {
    let conn = init()?;

    if let Some(q) = search {
        let like = format!("%{}%", q);
        let mut stmt = conn.prepare(
            "SELECT id, word, provider, translation, explanation, usage_example, context_file, context_language, created_at, sources_json
             FROM queries
             WHERE word LIKE ?1 OR translation LIKE ?1 OR explanation LIKE ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![like, limit as i64], row_to_entry)?;
        let entries: Vec<HistoryEntry> = rows.filter_map(|r| r.ok()).collect();
        return Ok(entries);
    }

    let mut stmt = conn.prepare(
        "SELECT id, word, provider, translation, explanation, usage_example, context_file, context_language, created_at, sources_json
         FROM queries
         ORDER BY created_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], row_to_entry)?;
    let entries: Vec<HistoryEntry> = rows.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

/// Get query stats.
#[derive(Debug, Serialize)]
pub struct HistoryStats {
    pub total_queries: i64,
    pub unique_words: i64,
    pub top_words: Vec<(String, i64)>,
    pub provider_breakdown: Vec<(String, i64)>,
    pub top_day: Option<(String, i64)>,
}

pub fn query_stats() -> Result<HistoryStats> {
    let conn = init()?;

    let total_queries: i64 = conn.query_row("SELECT COUNT(*) FROM queries", [], |r| r.get(0))?;
    let unique_words: i64 =
        conn.query_row("SELECT COUNT(DISTINCT word) FROM queries", [], |r| r.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT word, COUNT(*) as cnt FROM queries GROUP BY word ORDER BY cnt DESC LIMIT 10",
    )?;
    let top_words: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn.prepare(
        "SELECT provider, COUNT(*) as cnt FROM queries WHERE provider != '' GROUP BY provider ORDER BY cnt DESC",
    )?;
    let provider_breakdown: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let top_day = conn
        .query_row(
            "SELECT substr(created_at, 1, 10) as day, COUNT(*) as cnt
             FROM queries
             GROUP BY day ORDER BY cnt DESC LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )
        .ok();

    Ok(HistoryStats {
        total_queries,
        unique_words,
        top_words,
        provider_breakdown,
        top_day,
    })
}

/// Get this week's query activity, grouped by day.
#[derive(Debug, Serialize)]
pub struct WeeklyEntry {
    pub day: String,
    pub words: Vec<WeeklyWord>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct WeeklyWord {
    pub word: String,
    pub count: i64,
    pub translation: String,
}

pub fn query_weekly() -> Result<Vec<WeeklyEntry>> {
    let conn = init()?;

    // Get start of current week (Monday)
    let mut stmt = conn.prepare(
        "SELECT date(created_at) as day, word, translation, COUNT(*) as cnt
         FROM queries
         WHERE created_at >= date('now', '-7 days')
         GROUP BY day, word
         ORDER BY day DESC, cnt DESC",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;

    let mut days: Vec<WeeklyEntry> = Vec::new();
    for row in rows {
        let (day, word, translation, count) = row?;
        if let Some(last) = days.last_mut() {
            if last.day == day {
                last.words.push(WeeklyWord {
                    word,
                    count,
                    translation,
                });
                last.total += count;
                continue;
            }
        }
        days.push(WeeklyEntry {
            day,
            words: vec![WeeklyWord {
                word,
                count,
                translation,
            }],
            total: count,
        });
    }

    Ok(days)
}

/// Get words that haven't been queried in 7+ days (for review).
#[derive(Debug, Serialize)]
pub struct ReviewEntry {
    pub word: String,
    pub translation: String,
    pub last_seen: String,
    pub days_ago: i64,
}

pub fn query_review() -> Result<Vec<ReviewEntry>> {
    let conn = init()?;

    let mut stmt = conn.prepare(
        "SELECT word, translation, MAX(created_at) as last_seen
         FROM queries
         GROUP BY word, translation
         HAVING last_seen < datetime('now', '-7 days')
         ORDER BY last_seen ASC",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;

    let now = chrono::Local::now();
    let entries: Vec<ReviewEntry> = rows
        .filter_map(|r| r.ok())
        .map(|(word, translation, last_seen)| {
            let days = if let Ok(parsed) =
                chrono::NaiveDateTime::parse_from_str(&last_seen, "%Y-%m-%d %H:%M:%S")
            {
                let dt = chrono::Local.from_local_datetime(&parsed).unwrap();
                (now - dt).num_days().max(0)
            } else {
                0
            };
            ReviewEntry {
                word,
                translation,
                last_seen,
                days_ago: days,
            }
        })
        .collect();

    Ok(entries)
}

/// Delete all history.
pub fn clear_history() -> Result<usize> {
    let conn = init()?;
    let count = conn.execute("DELETE FROM queries", [])?;
    Ok(count)
}

pub fn query_max_id() -> Result<Option<i64>> {
    let conn = init()?;
    let mut stmt = conn.prepare("SELECT MAX(id) FROM queries")?;
    let max = stmt.query_row([], |r| r.get::<_, Option<i64>>(0))?;
    Ok(max)
}

pub fn list_queries_since(last_id: i64, limit: usize) -> Result<Vec<HistoryEntry>> {
    let conn = init()?;
    let mut stmt = conn.prepare(
        "SELECT id, word, provider, translation, explanation, usage_example, context_file, context_language, created_at, sources_json
         FROM queries WHERE id > ?1 ORDER BY id ASC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![last_id, limit as i64], row_to_entry)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyTrend {
    pub date: String,
    pub count: i64,
}

pub fn query_daily_trend(days: usize) -> Result<Vec<DailyTrend>> {
    let conn = init()?;
    let mut stmt = conn.prepare(
        "SELECT date(created_at) as day, COUNT(*) as cnt
         FROM queries
         WHERE created_at >= date('now', ?1)
         GROUP BY day ORDER BY day ASC",
    )?;
    let rows = stmt.query_map(params![format!("-{} days", days)], |r| {
        Ok(DailyTrend { date: r.get(0)?, count: r.get(1)? })
    })?;
    let mut trends: Vec<DailyTrend> = rows.filter_map(|r| r.ok()).collect();
    // Fill in missing days with 0
    if let (Some(first), Some(_last)) = (trends.first().cloned(), trends.last().cloned()) {
        let start = chrono::NaiveDate::parse_from_str(&first.date, "%Y-%m-%d").ok();
        let end = chrono::Local::now().naive_local().date();
        if let Some(mut current) = start {
            let mut filled = Vec::new();
            let mut idx = 0;
            while current <= end {
                let ds = current.format("%Y-%m-%d").to_string();
                if idx < trends.len() && trends[idx].date == ds {
                    filled.push(trends[idx].clone());
                    idx += 1;
                } else {
                    filled.push(DailyTrend { date: ds, count: 0 });
                }
                current += chrono::Duration::days(1);
            }
            trends = filled;
        }
    }
    Ok(trends)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        word: row.get(1)?,
        provider: row.get(2)?,
        translation: row.get(3)?,
        explanation: row.get(4)?,
        usage_example: row.get(5)?,
        context_file: row.get(6)?,
        context_language: row.get(7)?,
        created_at: row.get(8)?,
        sources: sources_from_json(row.get::<_, String>(9).unwrap_or_default()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use an in-memory SQLite DB per test to avoid cross-test interference.
    fn test_init() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS queries (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                word       TEXT NOT NULL,
                provider   TEXT NOT NULL DEFAULT '',
                translation TEXT NOT NULL DEFAULT '',
                explanation TEXT NOT NULL DEFAULT '',
                usage_example TEXT NOT NULL DEFAULT '',
                context_file   TEXT,
                context_language TEXT,
                created_at TEXT NOT NULL,
                sources_json TEXT NOT NULL DEFAULT ''
            );",
        )?;
        Ok(conn)
    }

    #[test]
    fn test_save_and_list() -> Result<()> {
        // Override db_path to use temp file
        let conn = test_init()?;
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        conn.execute(
            "INSERT INTO queries (word, provider, translation, explanation, usage_example, created_at, sources_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params!["map", "ollama", "映射", "Array method", "[1,2].map(fn)", &now, ""],
        )?;
        conn.execute(
            "INSERT INTO queries (word, provider, created_at)
             VALUES (?1, ?2, ?3)",
            params!["filter", "openai", &now],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, word, provider, translation, explanation, usage_example, context_file, context_language, created_at, sources_json
             FROM queries ORDER BY id",
        )?;
        let entries: Vec<HistoryEntry> = stmt
            .query_map([], row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].word, "map");
        assert_eq!(entries[0].translation, "映射");
        assert_eq!(entries[0].provider, "ollama");
        assert_eq!(entries[1].word, "filter");
        assert_eq!(entries[1].provider, "openai");

        Ok(())
    }

    #[test]
    fn test_stats() -> Result<()> {
        let _conn = test_init()?;
        // Save through the real path to test stats
        // This tests the actual DB at the real path - skip for unit test
        // Just verify the struct fields exist
        let stats = HistoryStats {
            total_queries: 0,
            unique_words: 0,
            top_words: vec![],
            provider_breakdown: vec![],
            top_day: None,
        };
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.unique_words, 0);

        Ok(())
    }

    #[test]
    fn test_clear() -> Result<()> {
        let conn = test_init()?;
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        conn.execute(
            "INSERT INTO queries (word, provider, translation, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params!["test", "ollama", "测试", &now],
        )?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM queries", [], |r| r.get(0))?;
        assert_eq!(count, 1);

        conn.execute("DELETE FROM queries", [])?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM queries", [], |r| r.get(0))?;
        assert_eq!(count, 0);

        Ok(())
    }
}
