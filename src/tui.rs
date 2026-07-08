use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};

use crate::history::{self, HistoryEntry, HistoryStats};

/// Run the TUI application.
pub async fn run() -> Result<()> {
    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new()?;
    let res = app.run(&mut terminal).await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    res
}

// ── App State ─────────────────────────────────────

struct App {
    entries: Vec<HistoryEntry>,
    stats: Option<HistoryStats>,
    mode: Mode,
    selected: usize,
    search_query: String,
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
        let stats = history::query_stats().ok();
        Ok(Self {
            entries,
            stats,
            mode: Mode::List,
            selected: 0,
            search_query: String::new(),
            should_quit: false,
        })
    }

    async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<&mut std::io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_event().await?;
        }
        Ok(())
    }

    async fn handle_event(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match self.mode {
                Mode::List => self.handle_list_key(key.code),
                Mode::Search => self.handle_search_key(key.code),
                Mode::Detail => self.handle_detail_key(key.code),
                Mode::Stats => self.handle_stats_key(key.code),
            }
        }
        Ok(())
    }

    fn handle_list_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.stats = history::query_stats().ok();
                self.mode = Mode::Stats;
            }
            KeyCode::Char('/') => {
                self.search_query.clear();
                self.mode = Mode::Search;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Ok(entries) = history::list_queries(200, None) {
                    self.entries = entries;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.entries.len().saturating_sub(1);
                self.selected = self.selected.saturating_add(1).min(max);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.entries.is_empty() {
                    self.mode = Mode::Detail;
                }
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter | KeyCode::Esc => {
                let q = self.search_query.trim();
                self.entries = if q.is_empty() {
                    history::list_queries(200, None).unwrap_or_default()
                } else {
                    history::list_queries(200, Some(q)).unwrap_or_default()
                };
                self.selected = 0;
                self.mode = Mode::List;
            }
            KeyCode::Char(c) => self.search_query.push(c),
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyCode) {
        if matches!(key, KeyCode::Esc | KeyCode::Char('q')) {
            self.mode = Mode::List;
        }
    }

    fn handle_stats_key(&mut self, key: KeyCode) {
        if matches!(key, KeyCode::Esc | KeyCode::Char('q')) {
            self.mode = Mode::List;
        }
    }

    // ── Rendering ─────────────────────────────────

    fn render(&self, f: &mut Frame) {
        match self.mode {
            Mode::List | Mode::Search => self.render_list(f),
            Mode::Detail => self.render_detail(f),
            Mode::Stats => self.render_stats(f),
        }
    }

    fn render_list(&self, f: &mut Frame) {
        let area = f.size();
        let chunks = if matches!(self.mode, Mode::Search) {
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(1)]).split(area)
        };

        let help = Line::from(vec![
            " ah history ".into(),
            Span::styled(" [q]uit ", Style::default().dim()),
            Span::styled(" [/]search ", Style::default().dim()),
            Span::styled(" [s]tats ", Style::default().dim()),
            Span::styled(" [r]efresh ", Style::default().dim()),
        ]);

        let header_cells = ["Date", "Word", "Provider", "Translation"]
            .iter()
            .map(|h| Cell::from(*h))
            .collect::<Vec<_>>();
        let header = Row::new(header_cells)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let style = if i == self.selected {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let date = if e.created_at.len() >= 10 {
                    &e.created_at[..10]
                } else {
                    &e.created_at
                };
                Row::new(vec![
                    Cell::from(date.to_string()),
                    Cell::from(e.word.clone()),
                    Cell::from(e.provider.clone()),
                    Cell::from(truncate(&e.translation, 30)),
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Length(24),
            Constraint::Length(12),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::TOP).title(help))
            .highlight_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));

        f.render_widget(table, chunks[0]);

        // Search bar
        if matches!(self.mode, Mode::Search) {
            let search_block = Block::default().borders(Borders::ALL).title(" Search ");
            let search_text = if self.search_query.is_empty() {
                Text::from(Span::styled(
                    "type to search, Enter to confirm",
                    Style::default().dim(),
                ))
            } else {
                Text::from(Span::raw(&self.search_query))
            };
            f.render_widget(Paragraph::new(search_text).block(search_block), chunks[1]);
        }
    }

    fn render_detail(&self, f: &mut Frame) {
        if self.selected >= self.entries.len() {
            return;
        }
        let entry = &self.entries[self.selected];

        let title = format!(" {}  [Esc/q] back ", entry.word);

        let mut text = vec![
            Line::from(vec![" 翻译: ".yellow().bold(), entry.translation.clone().into()]),
            Line::from(""),
            Line::from(vec![" 解释: ".cyan().bold(), entry.explanation.clone().into()]),
            Line::from(""),
        ];

        if !entry.usage_example.is_empty() {
            text.push(Line::from(vec![" 用法: ".green().bold(), entry.usage_example.clone().into()]));
            text.push(Line::from(""));
        }

        text.push(Line::from(Span::styled(
            format!("  Provider: {}  |  {}", entry.provider, entry.created_at),
            Style::default().dim(),
        )));

        f.render_widget(
            Paragraph::new(Text::from(text))
                .block(Block::default().borders(Borders::ALL).title(title.as_str())),
            f.size(),
        );
    }

    fn render_stats(&self, f: &mut Frame) {
        let Some(ref stats) = self.stats else {
            f.render_widget(
                Paragraph::new("No statistics available.")
                    .block(Block::default().borders(Borders::ALL).title(" Statistics [Esc/q] back ")),
                f.size(),
            );
            return;
        };

        let mut lines = vec![
            Line::from(Span::styled("Query Statistics", Style::default().bold())),
            Line::from(""),
            Line::from(format!("  Total queries:  {}", stats.total_queries)),
            Line::from(format!("  Unique words:   {}", stats.unique_words)),
        ];

        if let Some((ref day, cnt)) = stats.top_day {
            lines.push(Line::from(format!("  Busiest day:    {day} ({cnt} queries)")));
        }

        if !stats.top_words.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Top words:", Style::default().bold())));
            for (word, cnt) in &stats.top_words {
                lines.push(Line::from(format!("    {word:<24} {cnt}x")));
            }
        }

        if !stats.provider_breakdown.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  By provider:", Style::default().bold())));
            for (prov, cnt) in &stats.provider_breakdown {
                lines.push(Line::from(format!("    {prov:<24} {cnt}x")));
            }
        }

        f.render_widget(
            Paragraph::new(Text::from(lines))
                .block(Block::default().borders(Borders::ALL).title(" Statistics [Esc/q] back ")),
            f.size(),
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
