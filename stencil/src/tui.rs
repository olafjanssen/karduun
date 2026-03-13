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

#[derive(PartialEq)]
enum ViewMode {
    Templates,
    Validation,
    Details,
}

pub struct App {
    templates: Vec<Card>,
    cards: Vec<Card>,
    validation_results: Vec<ValidationResult>,
    selected_index: usize,
    should_quit: bool,
    view_mode: ViewMode,
}

#[derive(Clone, Debug)]
struct ValidationResult {
    uid: String,
    slug: String,
    template: Option<String>,
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl App {
    pub fn new(templates: Vec<Card>, cards: Vec<Card>) -> Self {
        Self {
            templates,
            cards,
            validation_results: Vec::new(),
            selected_index: 0,
            should_quit: false,
            view_mode: ViewMode::Templates,
        }
    }

    fn get_selected_template(&self) -> Option<&Card> {
        self.templates.get(self.selected_index)
    }

    fn get_selected_validation(&self) -> Option<&ValidationResult> {
        self.validation_results.get(self.selected_index)
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
        let title = Paragraph::new("Stencil TUI - Template Management")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Mode indicator
        let mode_text = match self.view_mode {
            ViewMode::Templates => "Templates (T)",
            ViewMode::Validation => "Validation (V)",
            ViewMode::Details => "Details (D)",
        };
        let mode_bar = Paragraph::new(mode_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Mode"));
        f.render_widget(mode_bar, chunks[1]);

        // Main content area with horizontal split
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[2]);

        // Left panel - Dynamic based on view mode
        match self.view_mode {
            ViewMode::Templates => {
                let items: Vec<ListItem> = self
                    .templates
                    .iter()
                    .enumerate()
                    .map(|(i, template)| {
                        let content = vec![
                            Span::raw(format!("{} ", template.uid)),
                            Span::styled(
                                template.title.clone(),
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

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Templates ({})", self.templates.len())),
                );
                f.render_widget(list, horizontal_chunks[0]);
            }
            ViewMode::Validation => {
                let items: Vec<ListItem> = self
                    .validation_results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let status = if result.valid { "✓" } else { "✗" };
                        let content = vec![
                            Span::raw(format!("{} {} ", status, result.uid)),
                            Span::styled(
                                result.slug.clone(),
                                Style::default().fg(if i == self.selected_index {
                                    Color::Yellow
                                } else if !result.valid {
                                    Color::Red
                                } else {
                                    Color::Green
                                }),
                            ),
                        ];
                        ListItem::new(Line::from(content))
                    })
                    .collect();

                let list =
                    List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
                        "Validation ({}/{})",
                        self.validation_results.iter().filter(|r| r.valid).count(),
                        self.validation_results.len()
                    )));
                f.render_widget(list, horizontal_chunks[0]);
            }
            ViewMode::Details => {
                if let Some(selected) = self.get_selected_template() {
                    let details = vec![
                        format!("UID: {}", selected.uid),
                        format!("Title: {}", selected.title),
                        format!("Slug: {}", selected.slug),
                        format!("Created: {}", selected.created.format("%Y-%m-%d")),
                    ];

                    let details_text = details.join("\n");
                    let paragraph = Paragraph::new(details_text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Template Details"),
                    );
                    f.render_widget(paragraph, horizontal_chunks[0]);
                }
            }
        }

        // Right panel - Dynamic content based on view mode
        match self.view_mode {
            ViewMode::Templates => {
                if let Some(selected) = self.get_selected_template() {
                    let constraints_info = self.get_template_constraints_info(selected);
                    let paragraph = Paragraph::new(constraints_info)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Template Info"),
                        );
                    f.render_widget(paragraph, horizontal_chunks[1]);
                }
            }
            ViewMode::Validation => {
                if let Some(selected) = self.get_selected_validation() {
                    let mut details = vec![
                        format!("UID: {}", selected.uid),
                        format!("Slug: {}", selected.slug),
                        format!(
                            "Status: {}",
                            if selected.valid {
                                "Valid ✓"
                            } else {
                                "Invalid ✗"
                            }
                        ),
                    ];

                    if let Some(template) = &selected.template {
                        details.push(format!("Template: {}", template));
                    }

                    if !selected.errors.is_empty() {
                        details.push("\nErrors:".to_string());
                        for error in &selected.errors {
                            details.push(format!("  • {}", error));
                        }
                    }

                    if !selected.warnings.is_empty() {
                        details.push("\nWarnings:".to_string());
                        for warning in &selected.warnings {
                            details.push(format!("  • {}", warning));
                        }
                    }

                    let details_text = details.join("\n");
                    let paragraph = Paragraph::new(details_text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Validation Details"),
                        );
                    f.render_widget(paragraph, horizontal_chunks[1]);
                }
            }
            ViewMode::Details => {
                if let Some(selected) = self.get_selected_template() {
                    if let Some(body) = selected.get_content() {
                        let content_preview = if body.len() > 500 {
                            format!("{}...", &body[..500])
                        } else {
                            body.to_string()
                        };

                        let paragraph = Paragraph::new(content_preview)
                            .wrap(ratatui::widgets::Wrap { trim: true })
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .title("Template Body"),
                            );
                        f.render_widget(paragraph, horizontal_chunks[1]);
                    }
                }
            }
        }

        // Help text
        let help = Paragraph::new("↑/↓: Navigate  T: Templates  V: Validation  D: Details  q: Quit  Enter: Run Validation")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[3]);
    }

    fn get_template_constraints_info(&self, template: &Card) -> String {
        let mut info = vec![format!("Template: {}", template.title)];

        if let Some(facets) = &template.facets {
            if let Some(template_facet) = &facets.template {
                if let Some(constraints) = &template_facet.constraints {
                    if !constraints.required_fields.is_empty() {
                        info.push("Required Fields:".to_string());
                        for field in &constraints.required_fields {
                            info.push(format!("  • {}", field));
                        }
                    }

                    if !constraints.enum_fields.is_empty() {
                        info.push("Enum Fields:".to_string());
                        for (field, values) in &constraints.enum_fields {
                            info.push(format!("  • {}: {:?}", field, values));
                        }
                    }

                    if !constraints.frozen_fields.is_empty() {
                        info.push("Frozen Fields:".to_string());
                        for field in &constraints.frozen_fields {
                            info.push(format!("  • {}", field));
                        }
                    }
                }
            }
        }

        if info.len() == 1 {
            info.push("(No constraints defined)".to_string());
        }

        info.join("\n")
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('t') => self.view_mode = ViewMode::Templates,
                    KeyCode::Char('v') => self.view_mode = ViewMode::Validation,
                    KeyCode::Char('d') => self.view_mode = ViewMode::Details,
                    KeyCode::Enter => {
                        if self.view_mode == ViewMode::Validation {
                            // Run validation
                            self.run_validation();
                        }
                    }
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => match self.view_mode {
                        ViewMode::Templates => {
                            if self.selected_index < self.templates.len().saturating_sub(1) {
                                self.selected_index += 1;
                            }
                        }
                        ViewMode::Validation => {
                            if self.selected_index < self.validation_results.len().saturating_sub(1)
                            {
                                self.selected_index += 1;
                            }
                        }
                        ViewMode::Details => {
                            if self.selected_index < self.templates.len().saturating_sub(1) {
                                self.selected_index += 1;
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn run_validation(&mut self) {
        // This would be implemented to call the actual validation logic
        // For now, we'll create some dummy data
        self.validation_results = self
            .cards
            .iter()
            .map(|card| {
                ValidationResult {
                    uid: card.uid.clone(),
                    slug: card.slug.clone(),
                    template: Some("template-example".to_string()),
                    valid: card.uid.len() % 2 == 0, // Alternate valid/invalid for demo
                    errors: if card.uid.len() % 2 == 0 {
                        Vec::new()
                    } else {
                        vec![
                            "Missing required field".to_string(),
                            "Invalid enum value".to_string(),
                        ]
                    },
                    warnings: vec!["Field should not be modified".to_string()],
                }
            })
            .collect();
    }
}

pub fn run_tui(templates: Vec<Card>, cards: Vec<Card>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new(templates, cards);
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
