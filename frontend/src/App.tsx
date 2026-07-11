import { useEffect, useState } from "preact/hooks";

type Stats = {
  total_queries: number;
  unique_words: number;
  top_words: [string, number][];
  provider_breakdown: [string, number][];
  top_day: [string, number] | null;
};

type SourceHit = {
  title: string;
  url: string;
  snippet: string;
};

type HistoryEntry = {
  id: number;
  word: string;
  translation: string;
  explanation: string;
  usage_example: string;
  provider: string;
  context_file: string | null;
  context_language: string | null;
  created_at: string;
  sources?: SourceHit[];
};

type WeeklyEntry = {
  day: string;
  words: { word: string; count: number; translation: string }[];
  total: number;
};

type ReviewEntry = {
  word: string;
  translation: string;
  last_seen: string;
  days_ago: number;
};

function api(path: string) {
  return fetch(path).then((r) => r.json());
}

const NAV = [
  { key: "today", label: "今日" },
  { key: "weekly", label: "本周" },
  { key: "review", label: "足迹" },
  { key: "history", label: "历史" },
] as const;

type Tab = (typeof NAV)[number]["key"];

export default function App() {
  const [tab, setTab] = useState<Tab>("today");
  const [stats, setStats] = useState<Stats | null>(null);
  const [today, setToday] = useState<HistoryEntry[]>([]);
  const [weekly, setWeekly] = useState<WeeklyEntry[]>([]);
  const [review, setReview] = useState<ReviewEntry[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [historyQ, setHistoryQ] = useState("");

  useEffect(() => {
    api("/api/stats").then(setStats);
    api("/api/today").then(setToday);
    api("/api/history?limit=200").then(setHistory);
    api("/api/weekly").then(setWeekly).catch(() => {});
    api("/api/review").then(setReview).catch(() => {});
  }, []);

  useEffect(() => {
    const t = setTimeout(() => {
      const url = historyQ
        ? `/api/history?limit=200&search=${encodeURIComponent(historyQ)}`
        : "/api/history?limit=200";
      api(url).then(setHistory);
    }, 300);
    return () => clearTimeout(t);
  }, [historyQ]);

  const counts: Partial<Record<Tab, number>> = {
    today: today.length,
    review: review.length,
    history: history.length,
  };

  return (
    <div class="app">
      <aside class="sidebar">
        <div class="brand">
          <span class="brand-name">ah</span>
          <span class="brand-tag">Stay in flow.</span>
        </div>

        <nav class="nav">
          {NAV.map((n) => (
            <button
              key={n.key}
              class={`nav-item${tab === n.key ? " active" : ""}`}
              onClick={() => setTab(n.key)}
            >
              <span class="nav-label">{n.label}</span>
              {counts[n.key] != null && (
                <span class="nav-count">{counts[n.key]}</span>
              )}
            </button>
          ))}
        </nav>

        {stats && (
          <div class="side-stats">
            <div class="stat">
              <span class="stat-num">{stats.total_queries}</span>
              <span class="stat-label">总查询</span>
            </div>
            <div class="stat">
              <span class="stat-num">{stats.unique_words}</span>
              <span class="stat-label">不重复</span>
            </div>
            <div class="stat">
              <span class="stat-num">{today.length}</span>
              <span class="stat-label">今日</span>
            </div>
          </div>
        )}
      </aside>

      <main class="main">
        {tab === "today" && <TodayView entries={today} />}
        {tab === "weekly" && <WeeklyView entries={weekly} />}
        {tab === "review" && <ReviewView entries={review} />}
        {tab === "history" && (
          <HistoryView
            entries={history}
            query={historyQ}
            onQuery={setHistoryQ}
          />
        )}
      </main>
    </div>
  );
}

function TodayView({ entries }: { entries: HistoryEntry[] }) {
  return (
    <section>
      <header class="page-head">
        <div>
          <h1 class="page-title">今日</h1>
          <p class="page-sub">今天查过的词与解释</p>
        </div>
        <span class="page-meta">{entries.length} 次</span>
      </header>
      {!entries.length ? (
        <p class="empty">
          还没有记录。选中文字后复制，或运行 <strong>ah grab</strong>
        </p>
      ) : (
        <Feed entries={entries} />
      )}
    </section>
  );
}

function WeeklyView({ entries }: { entries: WeeklyEntry[] }) {
  const total = entries.reduce((sum, d) => sum + d.total, 0);
  return (
    <section>
      <header class="page-head">
        <div>
          <h1 class="page-title">本周</h1>
          <p class="page-sub">按天回顾学习痕迹</p>
        </div>
        <span class="page-meta">
          {total} 次 · {entries.length} 天
        </span>
      </header>
      {!entries.length ? (
        <p class="empty">本周暂无数据</p>
      ) : (
        <div class="week-list">
          {entries.map((day) => (
            <div key={day.day} class="week-day">
              <div class="week-day-head">
                <span class="week-day-date">{day.day.slice(5)}</span>
                <span class="week-day-total">{day.total} 次</span>
              </div>
              <div class="week-words">
                {day.words.slice(0, 8).map((w) => (
                  <div key={w.word} class="week-word">
                    <span class="week-word-name">{w.word}</span>
                    <span class="week-word-trans">{w.translation}</span>
                    <span class="week-word-cnt">{w.count}</span>
                  </div>
                ))}
                {day.words.length > 8 && (
                  <div class="week-word-cnt">+{day.words.length - 8} 更多</div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function ReviewView({ entries }: { entries: ReviewEntry[] }) {
  return (
    <section>
      <header class="page-head">
        <div>
          <h1 class="page-title">足迹</h1>
          <p class="page-sub">久未再见的「啊？」时刻</p>
        </div>
        <span class="page-meta">{entries.length} 个</span>
      </header>
      {!entries.length ? (
        <p class="empty">暂时没有久未见面的词</p>
      ) : (
        <div class="review-list">
          {entries.map((e) => (
            <div key={e.word} class="review-item">
              <span class="review-word">{e.word}</span>
              <span class="review-trans">{e.translation}</span>
              <span
                class={`review-ago${
                  e.days_ago > 14 ? " old" : e.days_ago > 7 ? " stale" : ""
                }`}
              >
                {e.days_ago} 天前
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function HistoryView({
  entries,
  query,
  onQuery,
}: {
  entries: HistoryEntry[];
  query: string;
  onQuery: (s: string) => void;
}) {
  return (
    <section>
      <header class="page-head">
        <div>
          <h1 class="page-title">历史</h1>
          <p class="page-sub">在本地记录里查找</p>
        </div>
        <span class="page-meta">{entries.length} 条</span>
      </header>
      <input
        class="search-box"
        type="search"
        placeholder="搜索词、翻译、解释…"
        value={query}
        onInput={(e) => onQuery((e.target as HTMLInputElement).value)}
      />
      {!entries.length ? (
        <p class="empty">无匹配结果</p>
      ) : (
        <Feed entries={entries} />
      )}
    </section>
  );
}

function Feed({ entries }: { entries: HistoryEntry[] }) {
  const [expanded, setExpanded] = useState<number | null>(null);

  return (
    <div class="feed">
      {entries.map((e) => {
        const open = expanded === e.id;
        const sources = e.sources || [];
        return (
          <div key={e.id} class="feed-item">
            <button
              class="feed-row"
              onClick={() => setExpanded(open ? null : e.id)}
              aria-expanded={open}
            >
              <span class="feed-word">{e.word}</span>
              <span class="feed-trans">{e.translation}</span>
              <span class="feed-provider">
                {e.provider}
                {sources.length > 0 ? ` · ${sources.length} 源` : ""}
              </span>
              <span class="feed-time">{e.created_at.slice(5, 16)}</span>
            </button>
            {open && (
              <div class="feed-detail">
                {e.explanation && (
                  <div class="feed-detail-block">
                    <div class="feed-detail-label">解释</div>
                    <div class="feed-detail-text">{e.explanation}</div>
                  </div>
                )}
                {e.usage_example && (
                  <div class="feed-detail-block">
                    <div class="feed-detail-label">用法</div>
                    <pre class="feed-detail-code">{e.usage_example}</pre>
                  </div>
                )}
                {e.context_file && (
                  <div class="feed-detail-block">
                    <div class="feed-detail-label">文件</div>
                    <div class="feed-detail-text">
                      {e.context_file}
                      {e.context_language ? ` · ${e.context_language}` : ""}
                    </div>
                  </div>
                )}
                {sources.length > 0 && (
                  <div class="feed-detail-block">
                    <div class="feed-detail-label">来源</div>
                    <div class="web-list">
                      {sources.map((s) => (
                        <a
                          key={s.url}
                          class="web-card"
                          href={s.url}
                          target="_blank"
                          rel="noreferrer"
                        >
                          <div class="web-title">{s.title}</div>
                          <div class="web-url">{s.url}</div>
                          {s.snippet && (
                            <div class="web-snippet">{s.snippet}</div>
                          )}
                        </a>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
