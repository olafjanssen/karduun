use anyhow::Result;
use cardstack_lib::card::Card;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io::{self, Stdout};
use std::path::PathBuf;


pub struct App {
    all_cards: Vec<(PathBuf, Card)>,
    filtered_cards: Vec<(PathBuf, Card)>,
    selected_index: usize,
    should_quit: bool,
    search_query: String,
    search_mode: bool,
    view_mode: ViewMode,
}

#[derive(PartialEq)]
enum ViewMode {
    List,
    Details,
    Publications,
}

impl App {
    pub fn new(all_cards: Vec<(PathBuf, Card)>) -> Self {
        Self {
            all_cards: all_cards.clone(),
            filtered_cards: all_cards,
            selected_index: 0,
            should_quit: false,
            search_query: String::new(),
            search_mode: false,
            view_mode: ViewMode::List,
        }
    }

    fn get_selected_card(&self) -> Option<&(PathBuf, Card)> {
        self.filtered_cards.get(self.selected_index)
    }

    fn filter_cards(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_cards = self.all_cards.clone();
        } else {
            self.filtered_cards = self
                .all_cards
                .clone()
                .into_iter()
                .filter(|(_, card)| {
                    card.title
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                        || card
                            .uid
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                })
                .collect();
        }
        self.selected_index = 0;
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
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
        let title = Paragraph::new("Karduun Publisher - manage card publications")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Search bar
        let search_prefix = if self.search_mode { ">> " } else { "/ " };
        let search_text = if self.search_mode {
            format!("{}{}", search_prefix, self.search_query)
        } else {
            format!(
                "{}{}",
                search_prefix,
                if self.search_query.is_empty() {
                    "Search..."
                } else {
                    &self.search_query
                }
            )
        };

        let search_bar = Paragraph::new(search_text)
            .style(Style::default().fg(if self.search_mode {
                Color::Yellow
            } else {
                Color::Gray
            }))
            .block(Block::default().borders(Borders::ALL).title("Search"));
        f.render_widget(search_bar, chunks[1]);

        // Card list
        let items: Vec<ListItem> = self
            .filtered_cards
            .iter()
            .enumerate()
            .map(|(_i, (_, card))| {
                let content = vec![
                    Span::raw(format!("{} ", card.uid)),
                    Span::styled(
                        card.title.clone(),
                        Style::default().fg(if _i == self.selected_index {
                            Color::Yellow
                        } else {
                            Color::White
                        }),
                    ),
                ];
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            "Cards ({}/{})",
            self.filtered_cards.len(),
            self.all_cards.len()
        )));
        f.render_widget(list, chunks[2]);

        // Main content area with horizontal split
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[2]);

        // Left panel - Card list
        let items: Vec<ListItem> = self
            .filtered_cards
            .iter()
            .enumerate()
            .map(|(_i, (_, card))| {
                let content = vec![
                    Span::raw(format!("{} ", card.uid)),
                    Span::styled(
                        card.title.clone(),
                        Style::default().fg(if _i == self.selected_index {
                            Color::Yellow
                        } else {
                            Color::White
                        }),
                    ),
                ];
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            "Cards ({}/{})",
            self.filtered_cards.len(),
            self.all_cards.len()
        )));
        f.render_widget(list, horizontal_chunks[0]);

        // Right panel - Dynamic content based on view mode
        if let Some((_, selected_card)) = self.get_selected_card() {
            let right_content = match self.view_mode {
                ViewMode::Details => {
                    let detail_lines = vec![
                        format!("UID: {}", selected_card.uid),
                        format!("Title: {}", selected_card.title),
                        format!(
                            "Created: {}",
                            selected_card.created.format("%Y-%m-%d %H:%M")
                        ),
                        format!(
                            "Updated: {}",
                            selected_card.updated.format("%Y-%m-%d %H:%M")
                        ),
                        format!("Tags: {}", selected_card.tags.join(", ")),
                    ];
                    let detail_text = detail_lines.join("\n");
                    Paragraph::new(detail_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Card Details"))
                }
                ViewMode::Publications => {
                    let pub_lines: Vec<String> = selected_card
                        .publications
                        .iter()
                        .map(|album| format!("• {}", album))
                        .collect();
                    let pub_text = if pub_lines.is_empty() {
                        "No publications".to_string()
                    } else {
                        pub_lines.join("\n")
                    };
                    Paragraph::new(pub_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Publications"))
                }
                ViewMode::List => {
                    Paragraph::new("Select a card and press:\n• D: Details\n• P: Publications")
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Info"))
                }
            };
            f.render_widget(right_content, horizontal_chunks[1]);
        } else {
            let empty_right = Paragraph::new("No card selected")
                .block(Block::default().borders(Borders::ALL).title("Info"));
            f.render_widget(empty_right, horizontal_chunks[1]);
        }

        // Help text
        let help = Paragraph::new(
            "↑/↓: Navigate  /: Search  Esc: Exit  q: Quit  D: Details  P: Publications",
        )
        .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[3]);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('/') => {
                        self.search_mode = true;
                        self.search_query.clear();
                        self.filter_cards();
                    }
                    KeyCode::Esc => {
                        if self.search_mode {
                            self.search_mode = false;
                            if self.search_query.is_empty() {
                                self.filter_cards();
                            }
                        } else {
                            self.should_quit = true;
                        }
                    }
                    KeyCode::Backspace => {
                        if self.search_mode {
                            self.search_query.pop();
                            self.filter_cards();
                        }
                    }
                    KeyCode::Enter => {
                        if self.search_mode {
                            self.search_mode = false;
                        }
                    }
                    KeyCode::Char('d') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::Details;
                        }
                    }
                    KeyCode::Char('p') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::Publications;
                        }
                    }
                    KeyCode::Char('l') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::List;
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.search_mode {
                            self.search_query.push(c);
                            self.filter_cards();
                        }
                    }
                    KeyCode::Up => {
                        if !self.search_mode {
                            if self.selected_index > 0 {
                                self.selected_index -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !self.search_mode {
                            if self.selected_index < self.filtered_cards.len().saturating_sub(1) {
                                self.selected_index += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub fn run_tui(all_cards: Vec<(PathBuf, Card)>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new(all_cards);
    app.run(&mut terminal)?;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}
