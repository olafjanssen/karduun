use anyhow::Result;
use cardstack_lib::repository::load_all_cards;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
    Terminal,
};
use std::{
    io,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::EcosystemState;

// Ecosystem configuration constants
const DAILY_PRINT_QUOTA: u32 = 50;
const WEEKLY_PRINT_QUOTA: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Overview,
    Cards,
    Printing,
    Maturation,
}

impl AppTab {
    fn titles(&self) -> [&str; 4] {
        ["Overview", "Cards", "Printing", "Maturation"]
    }

    fn next(&self) -> Self {
        match self {
            AppTab::Overview => AppTab::Cards,
            AppTab::Cards => AppTab::Printing,
            AppTab::Printing => AppTab::Maturation,
            AppTab::Maturation => AppTab::Overview,
        }
    }

    fn previous(&self) -> Self {
        match self {
            AppTab::Overview => AppTab::Maturation,
            AppTab::Cards => AppTab::Overview,
            AppTab::Printing => AppTab::Cards,
            AppTab::Maturation => AppTab::Printing,
        }
    }
}

#[derive(Debug, Clone)]
struct CardInfo {
    uid: String,
    title: String,
    resonance: f64,
    print_count: u32,
}

#[derive(Debug, Clone)]
struct AppState {
    current_tab: AppTab,
    ecosystem_state: EcosystemState,
    cards: Vec<CardInfo>,
    selected_card_index: Option<usize>,
    quit: bool,
}

impl AppState {
    fn new(repo_root: &PathBuf) -> Result<Self> {
        let ecosystem_state = EcosystemState::load_or_new()?;
        let cards = load_cards_with_resonance(repo_root, &ecosystem_state)?;

        Ok(Self {
            current_tab: AppTab::Overview,
            ecosystem_state,
            cards,
            selected_card_index: None,
            quit: false,
        })
    }
}

fn load_cards_with_resonance(repo_root: &PathBuf, state: &EcosystemState) -> Result<Vec<CardInfo>> {
    let cards = load_all_cards(repo_root)?;
    let mut card_infos = Vec::new();

    for (_, card) in &cards {
        card_infos.push(CardInfo {
            uid: card.uid.clone(),
            title: card.title.clone(),
            resonance: state.get_resonance(&card.uid),
            print_count: state.card_print_counts.get(&card.uid).copied().unwrap_or(0),
        });
    }

    // Sort by resonance (descending)
    card_infos.sort_by(|a, b| b.resonance.partial_cmp(&a.resonance).unwrap());

    Ok(card_infos)
}

pub fn run_tui(repo_root: &PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app_state = AppState::new(repo_root)?;

    // Setup event channel
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(250);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::ZERO);

            if event::poll(timeout).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        tx.send(Event::Key(key)).unwrap();
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Key(KeyCode::Null.into())).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    // Main loop
    while !app_state.quit {
        terminal.draw(|f| ui(f, &app_state))?;

        match rx.recv()? {
            Event::Key(key) => {
                handle_key_event(key, &mut app_state);
            }
            _ => {}
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(frame: &mut ratatui::Frame, app_state: &AppState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main content
        ])
        .split(frame.size());

    // Render header
    render_header(frame, main_layout[0]);

    // Render tabs
    render_tabs(frame, app_state, main_layout[1]);

    // Render main content based on current tab
    match app_state.current_tab {
        AppTab::Overview => render_overview(frame, app_state, main_layout[2]),
        AppTab::Cards => render_cards(frame, app_state, main_layout[2]),
        AppTab::Printing => render_printing(frame, app_state, main_layout[2]),
        AppTab::Maturation => render_maturation(frame, app_state, main_layout[2]),
    }
}

fn render_header(frame: &mut ratatui::Frame, area: Rect) {
    let header = Paragraph::new("🌱 ECO SYSTEM - Card Ecosystem Dynamics")
        .block(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::default().fg(Color::Green).bold()),
        )
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(header, area);
}

fn render_tabs(frame: &mut ratatui::Frame, app_state: &AppState, area: Rect) {
    let titles = app_state.current_tab.titles();
    let tabs = Tabs::new(titles)
        .select(app_state.current_tab as usize)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Green).bold());

    frame.render_widget(tabs, area);
}

fn render_overview(frame: &mut ratatui::Frame, app_state: &AppState, area: Rect) {
    let state = &app_state.ecosystem_state;

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Summary
            Constraint::Length(10), // Resonance chart
            Constraint::Min(0),     // Card list
        ])
        .split(area);

    // Summary block
    let summary = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Ecosystem Summary",
            Style::default().bold(),
        )]),
        Line::from(vec![
            Span::styled("Total Cards: ", Style::default().fg(Color::Blue)),
            Span::styled(app_state.cards.len().to_string(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Daily Quota: ", Style::default().fg(Color::Blue)),
            Span::styled(
                format!("{}/{}", state.daily_prints, DAILY_PRINT_QUOTA),
                if state.daily_prints >= DAILY_PRINT_QUOTA {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().bold()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Weekly Quota: ", Style::default().fg(Color::Blue)),
            Span::styled(
                format!("{}/{}", state.weekly_prints, WEEKLY_PRINT_QUOTA),
                if state.weekly_prints >= WEEKLY_PRINT_QUOTA {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().bold()
                },
            ),
        ]),
    ])
    .block(Block::default().title("Summary").borders(Borders::ALL));

    frame.render_widget(summary, layout[0]);

    // Resonance distribution chart
    let mut high_resonance = 0;
    let mut medium_resonance = 0;
    let mut low_resonance = 0;

    for card in &app_state.cards {
        if card.resonance > 0.7 {
            high_resonance += 1;
        } else if card.resonance > 0.4 {
            medium_resonance += 1;
        } else {
            low_resonance += 1;
        }
    }

    let chart = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Resonance Distribution",
            Style::default().bold(),
        )]),
        Line::from(vec![
            Span::styled("High (>0.7): ", Style::default().fg(Color::Green)),
            Span::styled(high_resonance.to_string(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Medium (0.4-0.7): ", Style::default().fg(Color::Yellow)),
            Span::styled(medium_resonance.to_string(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Low (<0.4): ", Style::default().fg(Color::Red)),
            Span::styled(low_resonance.to_string(), Style::default().bold()),
        ]),
    ])
    .block(Block::default().title("Resonance").borders(Borders::ALL));

    frame.render_widget(chart, layout[1]);

    // Top cards table
    let rows = app_state.cards.iter().take(10).map(|card| {
        Row::new(vec![
            card.title.clone(),
            format!("{:.2}", card.resonance),
            card.print_count.to_string(),
        ])
    });

    let table = Table::new(
        rows,
        vec![
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(Row::new(vec!["Card", "Resonance", "Prints"]))
    .block(Block::default().title("Top Cards").borders(Borders::ALL));

    frame.render_widget(table, layout[2]);
}

fn render_cards(frame: &mut ratatui::Frame, app_state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Card list
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Header
    let header = Paragraph::new("All Cards (Use ↑/↓ to navigate, Enter to select)")
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, layout[0]);

    // Card list
    let items: Vec<Line> = app_state
        .cards
        .iter()
        .enumerate()
        .map(|(i, card)| {
            let resonance_style = if card.resonance > 0.7 {
                Style::default().fg(Color::Green)
            } else if card.resonance > 0.4 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };

            let line = Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().dim()),
                Span::styled(
                    &card.title,
                    if Some(i) == app_state.selected_card_index {
                        Style::default().fg(Color::Cyan).bold()
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!(" (resonance: {:.2})", card.resonance),
                    resonance_style,
                ),
            ]);

            line
        })
        .collect();

    let paragraph = Paragraph::new(items).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, layout[1]);

    // Controls
    let controls = Paragraph::new("↑/↓: Navigate  Enter: Select  S: Scan  P: Print  Q: Quit")
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().dim());

    frame.render_widget(controls, layout[2]);
}

fn render_printing(frame: &mut ratatui::Frame, app_state: &AppState, area: Rect) {
    let state = &app_state.ecosystem_state;

    let daily_used = state.daily_prints;
    let daily_total = DAILY_PRINT_QUOTA;
    let weekly_used = state.weekly_prints;
    let weekly_total = WEEKLY_PRINT_QUOTA;

    let daily_percent = (daily_used as f64 / daily_total as f64) * 100.0;
    let weekly_percent = (weekly_used as f64 / weekly_total as f64) * 100.0;

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Daily quota
            Constraint::Length(10), // Weekly quota
            Constraint::Min(0),     // Print history
        ])
        .split(area);

    // Daily quota
    let daily_quota = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Daily Print Quota",
            Style::default().bold(),
        )]),
        Line::from(vec![
            Span::styled("Used: ", Style::default()),
            Span::styled(
                format!("{}/{}", daily_used, daily_total),
                Style::default().bold(),
            ),
            Span::styled(
                format!(" ({:.1}%)", daily_percent),
                if daily_percent > 80.0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Remaining: ", Style::default()),
            Span::styled(
                (daily_total - daily_used).to_string(),
                if daily_used >= daily_total {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().fg(Color::Green).bold()
                },
            ),
        ]),
    ])
    .block(Block::default().title("Daily").borders(Borders::ALL));

    frame.render_widget(daily_quota, layout[0]);

    // Weekly quota
    let weekly_quota = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Weekly Print Quota",
            Style::default().bold(),
        )]),
        Line::from(vec![
            Span::styled("Used: ", Style::default()),
            Span::styled(
                format!("{}/{}", weekly_used, weekly_total),
                Style::default().bold(),
            ),
            Span::styled(
                format!(" ({:.1}%)", weekly_percent),
                if weekly_percent > 80.0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Remaining: ", Style::default()),
            Span::styled(
                (weekly_total - weekly_used).to_string(),
                if weekly_used >= weekly_total {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().fg(Color::Green).bold()
                },
            ),
        ]),
    ])
    .block(Block::default().title("Weekly").borders(Borders::ALL));

    frame.render_widget(weekly_quota, layout[1]);

    // Print history
    let print_history: Vec<Row> = state
        .card_print_counts
        .iter()
        .map(|(uid, count)| {
            let card_title = app_state
                .cards
                .iter()
                .find(|c| c.uid == *uid)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| uid.clone());

            Row::new(vec![card_title, count.to_string()])
        })
        .collect();

    let history_table = Table::new(
        print_history,
        vec![Constraint::Percentage(80), Constraint::Percentage(20)],
    )
    .header(Row::new(vec!["Card", "Print Count"]))
    .block(
        Block::default()
            .title("Print History")
            .borders(Borders::ALL),
    );

    frame.render_widget(history_table, layout[2]);
}

fn render_maturation(frame: &mut ratatui::Frame, app_state: &AppState, area: Rect) {
    let high_resonance_cards: Vec<_> = app_state
        .cards
        .iter()
        .filter(|c| c.resonance > 0.7)
        .collect();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Maturation info
            Constraint::Min(0),    // Potential clusters
        ])
        .split(area);

    // Maturation info
    let info = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Maturation Analysis",
            Style::default().bold(),
        )]),
        Line::from(vec![
            Span::styled("High-resonance cards: ", Style::default()),
            Span::styled(
                high_resonance_cards.len().to_string(),
                Style::default().bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Minimum cluster size: ", Style::default()),
            Span::styled("3", Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Similarity threshold: ", Style::default()),
            Span::styled("0.8", Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default()),
            Span::styled(
                if high_resonance_cards.len() >= 3 {
                    "Ready for maturation"
                } else {
                    "Not enough high-resonance cards"
                },
                if high_resonance_cards.len() >= 3 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
    ])
    .block(Block::default().title("Analysis").borders(Borders::ALL));

    frame.render_widget(info, layout[0]);

    // Potential clusters
    if high_resonance_cards.len() >= 3 {
        let cluster_items: Vec<Line> = high_resonance_cards
            .iter()
            .take(5)
            .map(|card| {
                Line::from(vec![
                    Span::styled("• ", Style::default()),
                    Span::styled(&card.title, Style::default().bold()),
                    Span::styled(
                        format!(" (resonance: {:.2})", card.resonance),
                        Style::default().fg(Color::Green),
                    ),
                ])
            })
            .collect();

        let clusters = Paragraph::new(cluster_items).block(
            Block::default()
                .title("Potential Clusters")
                .borders(Borders::ALL),
        );

        frame.render_widget(clusters, layout[1]);
    } else {
        let no_clusters = Paragraph::new("Need at least 3 high-resonance cards for maturation.")
            .block(
                Block::default()
                    .title("Potential Clusters")
                    .borders(Borders::ALL),
            )
            .style(Style::default().dim());

        frame.render_widget(no_clusters, layout[1]);
    }
}

fn handle_key_event(key: event::KeyEvent, app_state: &mut AppState) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app_state.quit = true;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app_state.current_tab = app_state.current_tab.previous();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app_state.current_tab = app_state.current_tab.next();
        }
        KeyCode::Up => {
            if let Some(current) = app_state.selected_card_index {
                if current > 0 {
                    app_state.selected_card_index = Some(current - 1);
                }
            } else {
                app_state.selected_card_index = Some(0);
            }
        }
        KeyCode::Down => {
            if let Some(current) = app_state.selected_card_index {
                if current < app_state.cards.len() - 1 {
                    app_state.selected_card_index = Some(current + 1);
                }
            } else {
                app_state.selected_card_index = Some(0);
            }
        }
        KeyCode::Enter => {
            // TODO: Implement card selection action
        }
        KeyCode::Char('s') => {
            // TODO: Implement scan action
        }
        KeyCode::Char('p') => {
            // TODO: Implement print action
        }
        _ => {}
    }
}
