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

// Import query functionality from main module
use super::grep_filter;

pub struct App {
    cards: Vec<Card>,
    filtered_cards: Vec<Card>,
    selected_index: usize,
    should_quit: bool,
    search_query: String,
    search_mode: bool,
    view_mode: ViewMode,
}

#[derive(PartialEq)]
enum ViewMode {
    List,
    Tree,
    Backlinks,
    Details,
}

impl App {
    pub fn new(cards: Vec<Card>) -> Self {
        Self {
            cards: cards.clone(),
            filtered_cards: cards,
            selected_index: 0,
            should_quit: false,
            search_query: String::new(),
            search_mode: false,
            view_mode: ViewMode::List,
        }
    }

    fn get_selected_card(&self) -> Option<&Card> {
        self.filtered_cards.get(self.selected_index)
    }

    fn build_tree_view(&self, card: &Card, cards_map: &std::collections::HashMap<String, &Card>) -> Vec<String> {
        let mut lines = vec![];

        // Find parent links
        for (_, potential_parent) in cards_map {
            if potential_parent.links.iter().any(|l| l.to == card.uid && (l.r#type == "parent-of" || l.r#type == "contains")) {
                lines.push(format!("└─ {} - {}", potential_parent.uid, potential_parent.title));
            }
        }

        // Find child links
        for link in &card.links {
            if link.r#type == "parent-of" || link.r#type == "contains" {
                if let Some(child) = cards_map.get(&link.to) {
                    lines.push(format!("├─ {} - {}", child.uid, child.title));
                }
            }
        }

        lines
    }

    fn build_backlinks_view(&self, card: &Card, cards_map: &std::collections::HashMap<String, &Card>) -> Vec<String> {
        let mut lines = vec![];

        // Find cards that link to this card
        for (_, potential_linker) in cards_map {
            if potential_linker.links.iter().any(|l| l.to == card.uid) {
                lines.push(format!("• {} - {}", potential_linker.uid, potential_linker.title));
            }
        }

        if lines.is_empty() {
            lines.push("No backlinks found".to_string());
        }

        lines
    }

    fn build_details_view(&self, card: &Card) -> Vec<String> {
        let mut lines = vec![];
        lines.push(format!("UID: {}", card.uid));
        lines.push(format!("Title: {}", card.title));
        lines.push(format!("Created: {}", card.created.format("%Y-%m-%d %H:%M")));
        lines.push(format!("Updated: {}", card.updated.format("%Y-%m-%d %H:%M")));
        let tags_text = if card.tags.is_empty() {
            "None".to_string()
        } else {
            card.tags.join(", ")
        };
        lines.push(format!("Tags: {}", tags_text));

        if let Some(content) = card.get_content() {
            // Show full content with word wrapping (let ratatui handle it)
            lines.push("Content:".to_string());
            lines.push(content.to_string());
        } else {
            lines.push("Content: No content available".to_string());
        }

        lines
    }

    fn filter_cards(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_cards = self.cards.clone();
        } else {
            // Use grep_filter from query_utils for content search
            self.filtered_cards = grep_filter(self.cards.clone(), &self.search_query);
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
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1)
            ].as_ref())
            .split(f.size());

        // Title
        let title = Paragraph::new("Karduun Scout --- quering cards")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Search bar
        let search_prefix = if self.search_mode { ">> " } else { "/ " };
        let search_text = if self.search_mode {
            format!("{}{}", search_prefix, self.search_query)
        } else {
            format!("{}{}", search_prefix, if self.search_query.is_empty() { "Search..." } else { &self.search_query })
        };

        let search_bar = Paragraph::new(search_text)
            .style(Style::default().fg(if self.search_mode { Color::Yellow } else { Color::Gray }))
            .block(Block::default().borders(Borders::ALL).title("Search"));
        f.render_widget(search_bar, chunks[1]);

        // Card list
        let items: Vec<ListItem> = self.filtered_cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                let content = vec![
                    Span::raw(format!("{} ", card.uid)),
                    Span::styled(
                        card.title.clone(),
                        Style::default().fg(if i == self.selected_index {
                            Color::Yellow
                        } else {
                            Color::White
                        }),
                    ),
                ];
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("Cards ({}/{})", self.filtered_cards.len(), self.cards.len())));
        f.render_widget(list, chunks[2]);

        // Main content area with horizontal split
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[2]);

        // Left panel - Card list (no wrapping, truncate long titles)
        let items: Vec<ListItem> = self.filtered_cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                // Truncate long titles to prevent overflow into right panel (dynamic based on panel width)
                // Create content with UID and full title
                // Calculate available width for title (account for UID + borders + padding)
                // Simple approach: use full title and let ratatui handle display
                let content = vec![
                    Span::raw(format!("{} ", card.uid)),
                    Span::styled(
                        card.title.clone(),
                        Style::default().fg(if i == self.selected_index {
                            Color::Yellow
                        } else {
                            Color::White
                        }),
                    ),
                ];
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("Cards ({}/{})", self.filtered_cards.len(), self.cards.len())));
        f.render_widget(list, horizontal_chunks[0]);

        // Right panel - Dynamic content based on view mode
        if let Some(selected_card) = self.get_selected_card() {
            let cards_map: std::collections::HashMap<String, &Card> = self.cards.iter()
                .map(|card| (card.uid.clone(), card))
                .collect();

            let right_content = match self.view_mode {
                ViewMode::Tree => {
                    let tree_lines = self.build_tree_view(selected_card, &cards_map);
                    let tree_text = tree_lines.join("\n");
                    Paragraph::new(tree_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Tree View"))
                }
                ViewMode::Backlinks => {
                    let backlink_lines = self.build_backlinks_view(selected_card, &cards_map);
                    let backlink_text = backlink_lines.join("\n");
                    Paragraph::new(backlink_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Backlinks"))
                }
                ViewMode::Details => {
                    let detail_lines = self.build_details_view(selected_card);
                    let detail_text = detail_lines.join("\n");
                    Paragraph::new(detail_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Card Details"))
                }
                ViewMode::List => {
                    Paragraph::new("Select a card and press:\n• T: Tree View\n• B: Backlinks\n• D: Details")
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
        let help = Paragraph::new("↑/↓: Navigate  /: Search  Esc: Exit  q: Quit  T: Tree  B: Backlinks  D: Details")
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
                    KeyCode::Char('t') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::Tree;
                        }
                    }
                    KeyCode::Char('b') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::Backlinks;
                        }
                    }
                    KeyCode::Char('d') => {
                        if !self.search_mode {
                            self.view_mode = ViewMode::Details;
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

pub fn run_tui(cards: Vec<Card>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new(cards);
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
