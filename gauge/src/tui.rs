use anyhow::Result;
use cardstack_lib::card::{Card, Computed};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io::{self, Stdout};

pub struct CardAnalysis {
    pub card: Card,
    pub computed: Computed,
    pub suggestion: String,
    pub rationale: String,
}

#[derive(PartialEq, Clone, Copy)]
enum SortMode {
    Title,
    Sv,
    Tokens,
    Suggestion,
}

#[derive(PartialEq)]
enum AppMode {
    Browse,
    Query,
}

struct App {
    analyses: Vec<CardAnalysis>,
    filtered_indices: Vec<usize>,
    selected: usize,
    sort_mode: SortMode,
    app_mode: AppMode,
    query_buf: String,
    should_quit: bool,
}

fn suggestion_color(s: &str) -> Color {
    match s {
        "ok" => Color::Green,
        "split" | "consider-split" => Color::Yellow,
        "merge" | "consider-merge" => Color::Cyan,
        "prune" => Color::Red,
        "refactor" => Color::Magenta,
        _ => Color::White,
    }
}

fn suggestion_icon(s: &str) -> &'static str {
    match s {
        "ok" => "✓",
        "split" => "⚡",
        "consider-split" => "↗",
        "merge" => "⬡",
        "consider-merge" => "↘",
        "prune" => "✗",
        "refactor" => "⟳",
        _ => "?",
    }
}

fn sv_bar(sv: f64, width: usize) -> String {
    let filled = ((sv / 2.0).clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s.chars().take(max - 1).collect::<String>())
    }
}

impl App {
    fn new(analyses: Vec<CardAnalysis>) -> Self {
        let filtered_indices: Vec<usize> = (0..analyses.len()).collect();
        let mut app = Self {
            analyses,
            filtered_indices,
            selected: 0,
            sort_mode: SortMode::Title,
            app_mode: AppMode::Browse,
            query_buf: String::new(),
            should_quit: false,
        };
        app.apply_sort();
        app
    }

    fn apply_filter(&mut self) {
        let q = self.query_buf.to_lowercase();
        self.filtered_indices = (0..self.analyses.len())
            .filter(|&i| {
                if q.is_empty() {
                    return true;
                }
                let a = &self.analyses[i];
                if let Some(tag) = q.strip_prefix("tag:") {
                    return a.card.tags.iter().any(|t| t.to_lowercase().contains(tag));
                }
                if let Some(sug) = q.strip_prefix("suggestion:") {
                    return a.suggestion.contains(sug);
                }
                let haystack = format!(
                    "{} {} {} {}",
                    a.card.title.to_lowercase(),
                    a.card.slug.to_lowercase(),
                    a.card.uid.to_lowercase(),
                    a.card.tags.join(" ").to_lowercase()
                );
                haystack.contains(&q)
            })
            .collect();
        self.apply_sort();
        self.selected = 0;
    }

    fn apply_sort(&mut self) {
        let analyses = &self.analyses;
        let mode = self.sort_mode;
        self.filtered_indices.sort_by(|&a, &b| match mode {
            SortMode::Title => analyses[a].card.title.cmp(&analyses[b].card.title),
            SortMode::Sv => {
                let sv_a = analyses[a].computed.sv.unwrap_or(0.0);
                let sv_b = analyses[b].computed.sv.unwrap_or(0.0);
                sv_b.partial_cmp(&sv_a).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortMode::Tokens => {
                let t_a = analyses[a].computed.tokens.unwrap_or(0);
                let t_b = analyses[b].computed.tokens.unwrap_or(0);
                t_b.cmp(&t_a)
            }
            SortMode::Suggestion => analyses[a].suggestion.cmp(&analyses[b].suggestion),
        });
    }

    fn cycle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Title => SortMode::Sv,
            SortMode::Sv => SortMode::Tokens,
            SortMode::Tokens => SortMode::Suggestion,
            SortMode::Suggestion => SortMode::Title,
        };
        self.apply_sort();
        self.selected = 0;
    }

    fn current_analysis(&self) -> Option<&CardAnalysis> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.analyses.get(i))
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.ui(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        // Title
        let title = Paragraph::new("Karduun Gauge — measuring cups of semantic volume")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Query / filter bar
        let sort_label = match self.sort_mode {
            SortMode::Title => "title",
            SortMode::Sv => "sv↓",
            SortMode::Tokens => "tokens↓",
            SortMode::Suggestion => "action",
        };
        let query_content = if self.app_mode == AppMode::Query {
            format!("/{}_  [sort: {}]", self.query_buf, sort_label)
        } else if self.query_buf.is_empty() {
            format!("(press / to filter)  [sort: {}]", sort_label)
        } else {
            format!("/{} ✓  [sort: {}]", self.query_buf, sort_label)
        };
        let query_style = if self.app_mode == AppMode::Query {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let query_bar = Paragraph::new(query_content)
            .style(query_style)
            .block(Block::default().borders(Borders::ALL).title("Filter"));
        f.render_widget(query_bar, chunks[1]);

        // Main split
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)].as_ref())
            .split(chunks[2]);

        // Card list
        let header_title = format!(
            "Cards ({}/{})",
            self.filtered_indices.len(),
            self.analyses.len()
        );
        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(display_i, &data_i)| {
                let a = &self.analyses[data_i];
                let is_selected = display_i == self.selected;
                let sv = a.computed.sv.unwrap_or(0.0);
                let tokens = a.computed.tokens.unwrap_or(0);
                let icon = suggestion_icon(&a.suggestion);
                let color = suggestion_color(&a.suggestion);
                let title = truncate(&a.card.title, 28);

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{:<28}", title),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {:>4}t", tokens),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(format!(" {:>5.2}", sv), Style::default().fg(Color::White)),
                        Span::styled(
                            format!(" {} {}", icon, a.suggestion),
                            Style::default().fg(color),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{:<28}", title), Style::default().fg(Color::White)),
                        Span::styled(
                            format!(" {:>4}t", tokens),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!(" {:>5.2}", sv),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!(" {} {}", icon, a.suggestion),
                            Style::default().fg(color),
                        ),
                    ]))
                }
            })
            .collect();

        let list =
            List::new(items).block(Block::default().borders(Borders::ALL).title(header_title));
        f.render_widget(list, main_chunks[0]);

        // Detail panel
        if let Some(a) = self.current_analysis() {
            let sv = a.computed.sv.unwrap_or(0.0);
            let tokens = a.computed.tokens.unwrap_or(0);
            let nid = a.computed.nid_bpt.unwrap_or(0.0);
            let link_d = a.computed.link_density.unwrap_or(0.0);
            let color = suggestion_color(&a.suggestion);
            let icon = suggestion_icon(&a.suggestion);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    truncate(&a.card.title, 40),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("slug: {}", a.card.slug),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    format!("uid:  {}", a.card.uid),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Tokens:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}", tokens), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("NID b/tok: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.2}", nid), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Link den:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.2}", link_d), Style::default().fg(Color::White)),
                ]),
            ];

            if let Some(c) = a.computed.cohesion {
                lines.push(Line::from(vec![
                    Span::styled("Cohesion:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.2}", c), Style::default().fg(Color::White)),
                ]));
            }
            if let Some(b) = a.computed.bandwidth {
                lines.push(Line::from(vec![
                    Span::styled("Bandwidth: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}", b), Style::default().fg(Color::White)),
                ]));
            }
            if let Some(r) = a.computed.redundancy {
                lines.push(Line::from(vec![
                    Span::styled("Redundancy:", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.2}", r), Style::default().fg(Color::White)),
                ]));
            }

            let sv_color = if sv > 1.6 || sv < 0.5 {
                Color::Red
            } else if sv > 1.3 || sv < 0.65 {
                Color::Yellow
            } else {
                Color::Green
            };
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("SV: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.2} ", sv), Style::default().fg(sv_color)),
                Span::styled(sv_bar(sv, 10), Style::default().fg(sv_color)),
            ]));
            lines.push(Line::from(Span::styled(
                "     0.5       1.6",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} {}", icon, a.suggestion.to_uppercase()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                &a.rationale,
                Style::default().fg(Color::DarkGray),
            )));

            if !a.card.tags.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("tags: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(a.card.tags.join(", "), Style::default().fg(Color::Blue)),
                ]));
            }

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Metrics"));
            f.render_widget(paragraph, main_chunks[1]);
        }

        // Help
        let help_text = if self.app_mode == AppMode::Query {
            "Type to filter (tag:xxx / suggestion:xxx)  Enter: apply  Esc: clear"
        } else {
            "↑/↓: Navigate  /: Filter  s: Sort  q: Quit"
        };
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[3]);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if self.app_mode == AppMode::Query {
                    match key.code {
                        KeyCode::Esc => {
                            self.query_buf.clear();
                            self.app_mode = AppMode::Browse;
                            self.apply_filter();
                        }
                        KeyCode::Enter => {
                            self.app_mode = AppMode::Browse;
                        }
                        KeyCode::Backspace => {
                            self.query_buf.pop();
                            self.apply_filter();
                        }
                        KeyCode::Char(c) => {
                            self.query_buf.push(c);
                            self.apply_filter();
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('/') => self.app_mode = AppMode::Query,
                    KeyCode::Char('s') => self.cycle_sort(),
                    KeyCode::Up => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected < self.filtered_indices.len().saturating_sub(1) {
                            self.selected += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub fn run_tui(analyses: Vec<CardAnalysis>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(analyses);
    app.run(&mut terminal)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}
