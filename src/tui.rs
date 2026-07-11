use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::history::{self, HistoryEntry, HistoryStats};

/// Run the TUI application.
pub async fn run() -> Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut stdout = std::io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new()?;
    app.run(&mut terminal).await
}

// ── Terminal Guard ────────────────────────────────

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        if let Err(e) = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = disable_raw_mode();
    }
}

// ── App State ─────────────────────────────────────

struct App {
    entries: Vec<HistoryEntry>,
    filtered: Vec<usize>,
    total: usize,
    stats: Option<HistoryStats>,
    mode: Mode,
    selected: usize,
    search_query: String,
    scroll: usize,
    should_quit: bool,
}

enum Mode {
    List,
    Search,
    Detail,
    Stats,
}

impl App {
    fn new() -> Result<Self> {
        let entries = history::list_queries(200, None).unwrap_or_default();
        let total = entries.len();
        Ok(Self {
            filtered: (0..total).collect(),
            total,
            stats: None,
            mode: Mode::List,
            selected: 0,
            search_query: String::new(),
            scroll: 0,
            should_quit: false,
            entries,
        })
    }

    fn visible_count(&self) -> usize {
        self.filtered.len()
    }

    fn current_entry(&self) -> Option<&HistoryEntry> {
        let idx = *self.filtered.get(self.selected)?;
        self.entries.get(idx)
    }

    async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<&mut std::io::Stdout>>,
    ) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_event().await?;
        }
        Ok(())
    }

    async fn handle_event(&mut self) -> Result<()> {
        let event = event::read()?;
        if let Event::Key(k) = &event {
            if k.kind != KeyEventKind::Press {
                return Ok(());
            }
        }
        match &self.mode {
            Mode::List => self.handle_list_event(event).await,
            Mode::Search => self.handle_search_event(event).await,
            Mode::Detail => self.handle_detail_event(event).await,
            Mode::Stats => self.handle_stats_event(event).await,
        }
    }

    // ── List mode ──

    async fn handle_list_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(k) = &event {
            match k.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.stats = history::query_stats().ok();
                    self.scroll = 0;
                    self.mode = Mode::Stats;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if let Ok(entries) = history::list_queries(200, None) {
                        self.entries = entries;
                        let total = self.entries.len();
                        self.total = total;
                        self.filtered = (0..total).collect();
                        self.selected = self.selected.min(total.saturating_sub(1));
                    }
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if let Some(e) = self.current_entry() {
                        let _ = copy_text(&e.translation);
                    }
                }
                KeyCode::Char('/') => {
                    self.search_query.clear();
                    self.mode = Mode::Search;
                }
                KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char('D') => {
                    if self.current_entry().is_some() {
                        self.scroll = 0;
                        self.mode = Mode::Detail;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let n = self.visible_count();
                    if n > 0 {
                        self.selected = (self.selected + 1).min(n.saturating_sub(1));
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected = self.selected.saturating_sub(1);
                }
                KeyCode::Char('g') => self.selected = 0,
                KeyCode::Char('G') => {
                    self.selected = self.visible_count().saturating_sub(1);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Search mode ──

    async fn handle_search_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(k) = &event {
            match k.code {
                KeyCode::Esc => {
                    self.search_query.clear();
                    self.filtered = (0..self.total).collect();
                    self.selected = self.selected.min(self.visible_count().saturating_sub(1));
                    self.mode = Mode::List;
                }
                KeyCode::Enter => {
                    let q = self.search_query.trim().to_string();
                    self.apply_filter(&q);
                    self.selected = 0;
                    self.mode = Mode::List;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    let q = self.search_query.trim().to_string();
                    self.apply_filter(&q);
                    self.selected = 0;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    let q = self.search_query.trim().to_string();
                    self.apply_filter(&q);
                    self.selected = 0;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn apply_filter(&mut self, q: &str) {
        if q.is_empty() {
            self.filtered = (0..self.total).collect();
            return;
        }
        let q_lower = q.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.word.to_lowercase().contains(&q_lower)
                    || e.translation.to_lowercase().contains(&q_lower)
                    || e.provider.to_lowercase().contains(&q_lower)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = self.selected.min(self.visible_count().saturating_sub(1));
    }

    // ── Detail mode ──

    async fn handle_detail_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(k) = &event {
            match k.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.scroll = 0;
                    self.mode = Mode::List;
                }
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if let Some(e) = self.current_entry() {
                        let _ = copy_text(&e.translation);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Stats mode ──

    async fn handle_stats_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(k) = &event {
            match k.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.scroll = 0;
                    self.mode = Mode::List;
                }
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
                _ => {}
            }
        }
        Ok(())
    }

    // ── Rendering ──

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.size());

        self.render_header(f, chunks[0]);
        match &self.mode {
            Mode::List | Mode::Search => self.render_list(f, chunks[1]),
            Mode::Detail => self.render_detail(f, chunks[1]),
            Mode::Stats => self.render_stats(f, chunks[1]),
        }
        self.render_footer(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let count = self.visible_count();
        let title = Line::from(vec![
            Span::styled(
                " ah",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" history \u{2014} "),
            Span::styled(format!("{count} shown"), Style::default().fg(Color::Green)),
            Span::raw(" / "),
            Span::raw(format!("{} total", self.total)),
        ]);
        f.render_widget(Paragraph::new(title), area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let hints = match &self.mode {
            Mode::Search => " type to filter \u{2014} Esc cancel  Enter confirm",
            Mode::Detail => " j/k scroll  PgUp/PgDn page  c copy  Esc/q back",
            Mode::Stats => " j/k scroll  PgUp/PgDn page  Esc/q back",
            Mode::List => {
                " j/k \u{2191}\u{2193} navigate  / search  Enter detail  s stats  c copy  r refresh  q quit"
            }
        };
        let style = Style::default().fg(Color::DarkGray);
        f.render_widget(Paragraph::new(Text::from(Span::styled(hints, style))), area);
    }

    fn render_list(&self, f: &mut Frame, area: Rect) {
        let count = self.visible_count();
        if count == 0 {
            let msg = if self.mode.is_search() {
                " No matching entries found."
            } else {
                " No history entries yet. Select a word with \u{2328} \u{2192} ah grab to start."
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    msg,
                    Style::default().fg(Color::DarkGray),
                ))),
                area,
            );
            return;
        }

        let narrow = area.width < 70;

        let header_cells: Vec<Cell> = if narrow {
            vec![
                Cell::from(Span::styled(
                    "Date",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Word",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Translation",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            ]
        } else {
            vec![
                Cell::from(Span::styled(
                    "Date",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Word",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Provider",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Translation",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            ]
        };
        let header_row = Row::new(header_cells);

        let col_widths: Vec<Constraint> = if narrow {
            vec![
                Constraint::Length(10),
                Constraint::Length(20),
                Constraint::Min(10),
            ]
        } else {
            let word_max = (area.width as usize)
                .saturating_sub(10 + 14 + 1 + 1 + 1)
                .min(30)
                .max(10) as u16;
            vec![
                Constraint::Length(10),
                Constraint::Length(word_max),
                Constraint::Length(14),
                Constraint::Min(10),
            ]
        };

        let rows: Vec<Row> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &entry_idx)| {
                let entry = &self.entries[entry_idx];
                let selected = i == self.selected;
                let st = if selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else if i % 2 == 1 {
                    Style::default().bg(Color::DarkGray).dim()
                } else {
                    Style::default()
                };

                let mut cells = vec![
                    Cell::from(truncate(&entry.created_at, 10)),
                    Cell::from(truncate(&entry.word, 20)),
                ];
                if !narrow {
                    cells.push(Cell::from(truncate(&entry.provider, 14)));
                }
                cells.push(Cell::from(truncate(&entry.translation, 40)));
                Row::new(cells).style(st)
            })
            .collect();

        let table = Table::new(rows, col_widths).header(header_row);
        f.render_widget(table, area);
    }

    fn render_detail(&self, f: &mut Frame, area: Rect) {
        let Some(entry) = self.current_entry() else {
            return;
        };

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled(" Word: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &entry.word,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" Provider: ", Style::default().fg(Color::Cyan)),
            Span::raw(&entry.provider),
            Span::raw("  "),
            Span::styled("Date: ", Style::default().fg(Color::Cyan)),
            Span::raw(&entry.created_at),
        ]));
        if let Some(ref cf) = entry.context_file {
            lines.push(Line::from(vec![
                Span::styled(" File: ", Style::default().fg(Color::Cyan)),
                Span::raw(cf),
            ]));
        }
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            " Translation",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for line in entry.translation.lines() {
            lines.push(Line::from(Span::raw(line)));
        }
        lines.push(Line::from(""));

        if !entry.explanation.is_empty() {
            lines.push(Line::from(Span::styled(
                " Explanation",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in entry.explanation.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
            lines.push(Line::from(""));
        }

        if !entry.usage_example.is_empty() {
            lines.push(Line::from(Span::styled(
                " Usage Example",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in entry.usage_example.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        if !entry.sources.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Sources",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for (i, s) in entry.sources.iter().enumerate() {
                lines.push(Line::from(format!("  [{}] {}", i + 1, s.title)));
                if !s.snippet.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("     {}", s.snippet),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(Span::styled(
                    format!("     {}", s.url),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll as u16, 0))
            .block(Block::default().borders(Borders::ALL).title(" Detail "));

        f.render_widget(paragraph, area);
    }

    fn render_stats(&self, f: &mut Frame, area: Rect) {
        let lines: Vec<Line> = if let Some(ref s) = self.stats {
            let mut v = vec![
                Line::from(Span::styled(
                    " Statistics",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!("  Total queries:   {}", s.total_queries)),
                Line::from(format!("  Unique words:    {}", s.unique_words)),
            ];
            if let Some((ref day, count)) = s.top_day {
                v.push(Line::from(format!("  Busiest day:     {day} ({count})")));
            }
            v.push(Line::from(""));
            v.push(Line::from(Span::styled(
                " Top Words",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            for (word, count) in &s.top_words {
                v.push(Line::from(format!("  {count:>4}  {word}")));
            }
            v.push(Line::from(""));
            v.push(Line::from(Span::styled(
                " Providers",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            for (provider, count) in &s.provider_breakdown {
                v.push(Line::from(format!("  {count:>4}  {provider}")));
            }
            v
        } else {
            vec![Line::from(Span::styled(
                " Failed to load statistics.",
                Style::default().fg(Color::Red),
            ))]
        };

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll as u16, 0))
            .block(Block::default().borders(Borders::ALL).title(" Statistics "));

        f.render_widget(paragraph, area);
    }
}

// ── Utilities ─────────────────────────────────────

/// Truncate by terminal display width (CJK = 2 columns), never mid-char.
fn truncate(s: &str, max: usize) -> String {
    if s.width() <= max {
        return s.to_string();
    }
    let ellipsis = '…';
    let ellipsis_w = ellipsis.width().unwrap_or(1);
    let budget = max.saturating_sub(ellipsis_w);
    let mut width = 0;
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let w = c.width().unwrap_or(0);
        if width + w > budget {
            break;
        }
        width += w;
        end = i + c.len_utf8();
    }
    format!("{}{ellipsis}", &s[..end])
}

fn copy_text(text: &str) -> Result<()> {
    #[cfg(not(target_os = "linux"))]
    {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            let _ = ctx.set_text(text);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            let _ = ctx.set_text(text);
        } else {
            let _ = std::process::Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut c| {
                    use std::io::Write;
                    c.stdin
                        .take()
                        .map(|mut s| s.write_all(text.as_bytes()))
                        .transpose()
                });
        }
    }
    Ok(())
}

/// Extension: adds `is_search()` to Mode enum.
impl Mode {
    fn is_search(&self) -> bool {
        matches!(self, Mode::Search)
    }
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_ascii() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn truncate_cjk_on_char_boundary() {
        use unicode_width::UnicodeWidthStr;
        let s = "TUI模式集成—在TUI里加一个快捷开关";
        let out = truncate(s, 20);
        assert!(out.ends_with('…'));
        // Must be valid UTF-8 / char-boundary slice
        assert!(out.is_char_boundary(out.len() - '…'.len_utf8()));
        assert!(out.width() <= 20);
    }
}
