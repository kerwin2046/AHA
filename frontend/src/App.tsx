import { useEffect, useState } from "preact/hooks";
import type { JSX } from "preact";

type Stats = {
  total_queries: number;
  unique_words: number;
  top_words: [string, number][];
  provider_breakdown: [string, number][];
  top_day: [string, number] | null;
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
  // Same-origin when served by `ah web`; Vite proxies /api in dev.
  return fetch(path).then((r) => r.json());
}

const NAV = [
  { key: "today", label: "今日", icon: "◉" },
  { key: "weekly", label: "本周", icon: "○" },
  { key: "review", label: "复习", icon: "◎" },
  { key: "history", label: "历史", icon: "⊞" },
] as const;

type Tab = (typeof NAV)[number]["key"];

export default function App() {
  const [tab, setTab] = useState<Tab>("today");
  const [stats, setStats] = useState<Stats | null>(null);
  const [today, setToday] = useState<HistoryEntry[]>([]);
  const [weekly, setWeekly] = useState<WeeklyEntry[]>([]);
  const [review, setReview] = useState<ReviewEntry[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [search, setSearch] = useState("");

  useEffect(() => {
    api("/api/stats").then(setStats);
    api("/api/today").then(setToday);
    api("/api/history?limit=200").then(setHistory);
    api("/api/weekly").then(setWeekly).catch(() => {});
    api("/api/review").then(setReview).catch(() => {});
  }, []);

  useEffect(() => {
    const t = setTimeout(() => {
      const url = search
        ? `/api/history?limit=200&search=${encodeURIComponent(search)}`
        : "/api/history?limit=200";
      api(url).then(setHistory);
    }, 300);
    return () => clearTimeout(t);
  }, [search]);

  return (
    <div style={s.layout}>
      {/* LEFT SIDEBAR */}
      <aside style={s.sidebar}>
        <div style={s.logo}>
          <span style={s.logoAccent}>ah</span>
          <span style={s.logoDim}>/dashboard</span>
        </div>

        <nav style={s.nav}>
          {NAV.map((n) => (
            <button
              key={n.key}
              onClick={() => setTab(n.key)}
              style={{
                ...s.navItem,
                ...(tab === n.key ? s.navItemActive : {}),
              }}
            >
              <span style={s.navIcon}>{n.icon}</span>
              {n.label}
            </button>
          ))}
        </nav>

        {stats && (
          <div style={s.sideStats}>
            <StatLine num={stats.total_queries} label="总查询" />
            <StatLine num={stats.unique_words} label="不重复" />
            <StatLine num={stats.top_day?.[1] ?? 0} label="单日最高" />
          </div>
        )}

        <div style={s.sideFooter}>
          <div style={s.providerList}>
            {stats?.provider_breakdown.map(([p, c]) => (
              <span key={p} style={s.providerBadge}>
                {p} <span style={s.providerCnt}>{c}</span>
              </span>
            ))}
          </div>
        </div>
      </aside>

      {/* MAIN CONTENT */}
      <main style={s.main}>
        {tab === "today" && <TodayView entries={today} />}
        {tab === "weekly" && <WeeklyView entries={weekly} />}
        {tab === "review" && <ReviewView entries={review} />}
        {tab === "history" && (
          <HistoryView
            entries={history}
            search={search}
            onSearch={setSearch}
          />
        )}
      </main>
    </div>
  );
}

function StatLine({ num, label }: { num: number; label: string }) {
  return (
    <div style={s.statLine}>
      <span style={s.statNum}>{num}</span>
      <span style={s.statLabel}>{label}</span>
    </div>
  );
}

function TodayView({ entries }: { entries: HistoryEntry[] }) {
  const [expanded, setExpanded] = useState<number | null>(null);

  return (
    <div>
      <div style={s.pageHeader}>
        <h2 style={s.pageTitle}>今日查询</h2>
        <span style={s.badge}>{entries.length} 次</span>
      </div>
      {!entries.length ? (
        <p style={s.empty}>还没有记录</p>
      ) : (
        <Table entries={entries} expanded={expanded} onToggle={setExpanded} />
      )}
    </div>
  );
}

function WeeklyView({ entries }: { entries: WeeklyEntry[] }) {
  const total = entries.reduce((s, d) => s + d.total, 0);
  return (
    <div>
      <div style={s.pageHeader}>
        <h2 style={s.pageTitle}>本周学习</h2>
        <span style={s.badge}>{total} 次 / {entries.length} 天</span>
      </div>
      {!entries.length ? (
        <p style={s.empty}>暂无数据</p>
      ) : (
        <div style={s.weekGrid}>
          {entries.map((day) => (
            <div key={day.day} style={s.dayCard}>
              <div style={s.dayHeader}>{day.day.slice(5)}</div>
              <div style={s.dayTotal}>{day.total} 次</div>
              {day.words.slice(0, 8).map((w) => (
                <div key={w.word} style={s.dayWord}>
                  <span style={s.dayWordName}>{w.word}</span>
                  <span style={s.dayWordTrans}>{w.translation}</span>
                  <span style={s.dayWordCnt}>{w.count}</span>
                </div>
              ))}
              {day.words.length > 8 && (
                <div style={s.dayMore}>+{day.words.length - 8} 更多</div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ReviewView({ entries }: { entries: ReviewEntry[] }) {
  return (
    <div>
      <div style={s.pageHeader}>
        <h2 style={s.pageTitle}>需要复习</h2>
        <span style={s.badge}>{entries.length} 个词</span>
      </div>
      {!entries.length ? (
        <p style={s.empty}>全部掌握！</p>
      ) : (
        <div style={s.reviewGrid}>
          {entries.map((e) => (
            <div key={e.word} style={s.reviewCard}>
              <div style={s.reviewWord}>{e.word}</div>
              <div style={s.reviewTrans}>{e.translation}</div>
              <div style={s.reviewMeta}>
                <span
                  style={{
                    fontSize: 11,
                    color:
                      e.days_ago > 14
                        ? "#f87171"
                        : e.days_ago > 7
                          ? "#fbbf24"
                          : "#64748b",
                    fontWeight: e.days_ago > 14 ? 600 : 400,
                  }}
                >
                  {e.days_ago} 天前
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function HistoryView({
  entries,
  search,
  onSearch,
}: {
  entries: HistoryEntry[];
  search: string;
  onSearch: (s: string) => void;
}) {
  const [expanded, setExpanded] = useState<number | null>(null);

  return (
    <div>
      <div style={s.pageHeader}>
        <h2 style={s.pageTitle}>历史记录</h2>
        <span style={s.badge}>{entries.length} 条</span>
      </div>
      <input
        type="text"
        placeholder="搜索词、翻译、解释..."
        value={search}
        onInput={(e) => onSearch((e.target as HTMLInputElement).value)}
        style={s.search}
      />
      {!entries.length ? (
        <p style={s.empty}>无匹配</p>
      ) : (
        <Table entries={entries} expanded={expanded} onToggle={setExpanded} />
      )}
    </div>
  );
}

function Table({
  entries,
  expanded,
  onToggle,
}: {
  entries: HistoryEntry[];
  expanded: number | null;
  onToggle: (id: number | null) => void;
}) {
  return (
    <div style={s.table}>
      {entries.map((e) => (
        <div key={e.id}>
          <div
            style={s.row}
            onClick={() => onToggle(expanded === e.id ? null : e.id)}
          >
            <span style={s.cellWord}>{e.word}</span>
            <span style={s.cellTrans}>{e.translation}</span>
            <span style={s.cellProvider}>{e.provider}</span>
            <span style={s.cellTime}>{e.created_at.slice(5, 16)}</span>
          </div>
          {expanded === e.id && (
            <div style={s.expanded}>
              {e.explanation && (
                <>
                  <div style={s.expLabel}>解释</div>
                  <div style={s.expText}>{e.explanation}</div>
                </>
              )}
              {e.usage_example && (
                <>
                  <div style={s.expLabel}>用法</div>
                  <pre style={s.expCode}>{e.usage_example}</pre>
                </>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

const s: Record<string, JSX.CSSProperties> = {
  layout: {
    display: "flex",
    height: "100vh",
    background: "#0b0e14",
    color: "#e2e8f0",
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  sidebar: {
    width: 220,
    flexShrink: 0,
    background: "#11161e",
    borderRight: "1px solid #1e2530",
    display: "flex",
    flexDirection: "column",
    padding: "24px 12px",
    gap: 24,
  },
  logo: { fontSize: 20, fontWeight: 700, padding: "0 8px" },
  logoAccent: { color: "#60a5fa" },
  logoDim: { color: "#475569", fontWeight: 400 },
  nav: { display: "flex", flexDirection: "column", gap: 2 },
  navItem: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "10px 12px",
    border: "none",
    borderRadius: 8,
    background: "transparent",
    color: "#94a3b8",
    fontSize: 14,
    cursor: "pointer",
    textAlign: "left",
    width: "100%",
    transition: "all .15s",
  },
  navItemActive: {
    background: "#1e293b",
    color: "#e2e8f0",
    fontWeight: 600,
  },
  navIcon: { width: 20, textAlign: "center" as const, fontSize: 14 },
  sideStats: {
    display: "flex",
    flexDirection: "column",
    gap: 8,
    padding: "16px 12px",
    background: "#0b0e14",
    borderRadius: 8,
    border: "1px solid #1e2530",
  },
  statLine: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
  },
  statNum: { fontSize: 18, fontWeight: 700, color: "#60a5fa" },
  statLabel: { fontSize: 12, color: "#64748b" },
  sideFooter: { marginTop: "auto" },
  providerList: { display: "flex", flexWrap: "wrap", gap: 6 },
  providerBadge: {
    fontSize: 11,
    padding: "4px 8px",
    background: "#1e293b",
    borderRadius: 4,
    color: "#94a3b8",
  },
  providerCnt: { color: "#60a5fa", fontWeight: 600 },

  main: {
    flex: 1,
    overflow: "auto",
    padding: "32px 40px",
    minWidth: 0,
  },
  pageHeader: {
    display: "flex",
    alignItems: "center",
    gap: 12,
    marginBottom: 24,
  },
  pageTitle: { fontSize: 22, fontWeight: 700, color: "#f1f5f9", margin: 0 },
  badge: {
    fontSize: 12,
    padding: "4px 10px",
    background: "#1e293b",
    borderRadius: 12,
    color: "#94a3b8",
  },
  search: {
    width: "100%",
    padding: "10px 14px",
    background: "#0b0e14",
    border: "1px solid #1e2530",
    borderRadius: 8,
    color: "#e2e8f0",
    fontSize: 14,
    outline: "none",
    marginBottom: 16,
    boxSizing: "border-box",
  },

  // Weekly
  weekGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))",
    gap: 16,
  },
  dayCard: {
    background: "#11161e",
    border: "1px solid #1e2530",
    borderRadius: 10,
    padding: 16,
  },
  dayHeader: { fontSize: 14, fontWeight: 600, color: "#f1f5f9", marginBottom: 4 },
  dayTotal: { fontSize: 12, color: "#64748b", marginBottom: 12 },
  dayWord: {
    display: "flex",
    gap: 8,
    padding: "4px 0",
    fontSize: 13,
    alignItems: "center",
  },
  dayWordName: { color: "#60a5fa", fontWeight: 500, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  dayWordTrans: { color: "#94a3b8", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 12 },
  dayWordCnt: {
    color: "#475569",
    fontSize: 11,
    background: "#1e293b",
    padding: "1px 6px",
    borderRadius: 4,
  },
  dayMore: { fontSize: 12, color: "#475569", marginTop: 8 },

  // Review
  reviewGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
    gap: 12,
  },
  reviewCard: {
    background: "#11161e",
    border: "1px solid #1e2530",
    borderRadius: 10,
    padding: 16,
  },
  reviewWord: { fontSize: 15, fontWeight: 600, color: "#60a5fa", marginBottom: 4 },
  reviewTrans: { fontSize: 13, color: "#94a3b8", marginBottom: 8 },
  reviewMeta: {},
  // Table
  table: { borderRadius: 8, overflow: "hidden", border: "1px solid #1e2530" },
  row: {
    display: "flex",
    gap: 12,
    padding: "10px 14px",
    borderBottom: "1px solid #1e2530",
    fontSize: 14,
    cursor: "pointer",
    alignItems: "center",
    background: "#11161e",
  },
  cellWord: { color: "#60a5fa", fontWeight: 500, minWidth: 140, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  cellTrans: { color: "#e2e8f0", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 300 },
  cellProvider: { color: "#64748b", fontSize: 12, minWidth: 60 },
  cellTime: { color: "#475569", fontSize: 12, marginLeft: "auto", whiteSpace: "nowrap" },
  expanded: {
    padding: "14px 16px",
    background: "#0b0e14",
    borderBottom: "1px solid #1e2530",
    fontSize: 13,
    lineHeight: 1.6,
  },
  expLabel: { color: "#64748b", fontSize: 11, textTransform: "uppercase" as const, marginBottom: 4 },
  expText: { color: "#e2e8f0", marginBottom: 8, whiteSpace: "pre-wrap" as const, wordBreak: "break-word" as const },
  expCode: {
    background: "#0b0e14",
    padding: "8px 12px",
    borderRadius: 6,
    fontSize: 12,
    color: "#4ade80",
    whiteSpace: "pre-wrap" as const,
    wordBreak: "break-word" as const,
    border: "1px solid #1e2530",
  },
  empty: { padding: 40, textAlign: "center" as const, color: "#475569", fontSize: 14 },
};
