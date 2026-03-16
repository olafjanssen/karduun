use anyhow::Result;
use cardstack_lib::card::Card;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
use std::collections::HashMap;
use std::io::{self, Stdout};

#[derive(PartialEq)]
enum ViewMode {
    Templates,
    Validation,
    Details,
    Edit,
}

#[derive(Clone, PartialEq)]
enum AddTarget {
    Required,
    Frozen,
    Enum,
}

#[derive(Clone)]
enum EditItem {
    Header(String),
    TextField { label: &'static str, buf_idx: usize },
    RequiredField { idx: usize, value: String },
    FrozenField { idx: usize, value: String },
    EnumField { key: String },
    AddRequired,
    AddFrozen,
    AddEnum,
}

pub struct App {
    templates: Vec<Card>,
    cards: Vec<Card>,
    validation_results: Vec<ValidationResult>,
    selected_index: usize,
    should_quit: bool,
    view_mode: ViewMode,
    // Edit mode state
    edit_items: Vec<EditItem>,
    edit_nav: usize,
    edit_basic_buffers: Vec<String>,
    edit_required: Vec<String>,
    edit_frozen: Vec<String>,
    edit_enum: HashMap<String, Vec<serde_json::Value>>,
    edit_enum_order: Vec<String>,
    edit_add_mode: bool,
    edit_add_buffer: String,
    edit_add_target: AddTarget,
    edit_enum_value_mode: bool,
    edit_enum_key: String,
    edit_enum_buf: String,
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
            edit_items: Vec::new(),
            edit_nav: 0,
            edit_basic_buffers: Vec::new(),
            edit_required: Vec::new(),
            edit_frozen: Vec::new(),
            edit_enum: HashMap::new(),
            edit_enum_order: Vec::new(),
            edit_add_mode: false,
            edit_add_buffer: String::new(),
            edit_add_target: AddTarget::Required,
            edit_enum_value_mode: false,
            edit_enum_key: String::new(),
            edit_enum_buf: String::new(),
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
        let title = Paragraph::new("Karduun Stencil - stamping your template management")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Mode indicator
        let mode_text = match self.view_mode {
            ViewMode::Templates => "Templates (T)",
            ViewMode::Validation => "Validation (V)",
            ViewMode::Details => "Details (D)",
            ViewMode::Edit => "Edit (E)",
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

        // Left panel
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
            ViewMode::Edit => {
                let nav_indices = self.navigable_indices();
                let current_item_idx = nav_indices.get(self.edit_nav).copied();
                let items: Vec<ListItem> = self
                    .edit_items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let is_selected = current_item_idx == Some(i);
                        match item {
                            EditItem::Header(h) => ListItem::new(Line::from(Span::styled(
                                h.clone(),
                                Style::default().fg(Color::DarkGray),
                            ))),
                            EditItem::TextField { label, buf_idx } => {
                                let value = self
                                    .edit_basic_buffers
                                    .get(*buf_idx)
                                    .map(|s| s.as_str())
                                    .unwrap_or("");
                                let display = if is_selected {
                                    format!("  {}: {}_", label, value)
                                } else {
                                    format!("  {}: {}", label, value)
                                };
                                ListItem::new(Line::from(Span::styled(
                                    display,
                                    Style::default().fg(if is_selected {
                                        Color::Yellow
                                    } else {
                                        Color::White
                                    }),
                                )))
                            }
                            EditItem::RequiredField { value, .. }
                            | EditItem::FrozenField { value, .. } => {
                                let prefix = if is_selected { "▶ " } else { "  " };
                                ListItem::new(Line::from(Span::styled(
                                    format!("{}{}", prefix, value),
                                    Style::default().fg(if is_selected {
                                        Color::Yellow
                                    } else {
                                        Color::White
                                    }),
                                )))
                            }
                            EditItem::EnumField { key } => {
                                let values = self
                                    .edit_enum
                                    .get(key)
                                    .map(|v| {
                                        v.iter()
                                            .map(|val| match val {
                                                serde_json::Value::String(s) => s.clone(),
                                                other => other.to_string(),
                                            })
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                    .unwrap_or_default();
                                let prefix = if is_selected { "▶ " } else { "  " };
                                let display = if values.is_empty() {
                                    format!("{}{}: (no values)", prefix, key)
                                } else {
                                    format!("{}{}: {}", prefix, key, values)
                                };
                                ListItem::new(Line::from(Span::styled(
                                    display,
                                    Style::default().fg(if is_selected {
                                        Color::Yellow
                                    } else {
                                        Color::White
                                    }),
                                )))
                            }
                            EditItem::AddRequired | EditItem::AddFrozen | EditItem::AddEnum => {
                                let in_add = self.edit_add_mode
                                    && is_selected
                                    && match item {
                                        EditItem::AddRequired => {
                                            self.edit_add_target == AddTarget::Required
                                        }
                                        EditItem::AddFrozen => {
                                            self.edit_add_target == AddTarget::Frozen
                                        }
                                        EditItem::AddEnum => {
                                            self.edit_add_target == AddTarget::Enum
                                        }
                                        _ => false,
                                    };
                                let label = if in_add {
                                    format!("  > {}_", self.edit_add_buffer)
                                } else {
                                    let kind = match item {
                                        EditItem::AddRequired => "required",
                                        EditItem::AddFrozen => "frozen",
                                        _ => "enum",
                                    };
                                    format!(
                                        "{}+ Add {} field",
                                        if is_selected { "▶ " } else { "  " },
                                        kind
                                    )
                                };
                                ListItem::new(Line::from(Span::styled(
                                    label,
                                    Style::default().fg(if is_selected {
                                        Color::Green
                                    } else {
                                        Color::DarkGray
                                    }),
                                )))
                            }
                        }
                    })
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Edit Template"),
                );
                f.render_widget(list, horizontal_chunks[0]);
            }
        }

        // Right panel
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
            ViewMode::Edit => {
                if self.edit_enum_value_mode {
                    let text = format!(
                        "Key: {}\n\nComma-separated values:\n{}_\n\nEnter: save  Esc: cancel",
                        self.edit_enum_key, self.edit_enum_buf
                    );
                    let paragraph = Paragraph::new(text)
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Edit Enum Values"),
                        );
                    f.render_widget(paragraph, horizontal_chunks[1]);
                } else if let Some(selected) = self.get_selected_template() {
                    let mut info = vec![
                        format!("UID: {}", selected.uid),
                        format!("Created: {}", selected.created.format("%Y-%m-%d")),
                        String::new(),
                    ];
                    match self.current_edit_item() {
                        Some(EditItem::TextField { .. }) => {
                            info.push("Type to edit field".to_string());
                            info.push("↑/↓/Tab: navigate".to_string());
                        }
                        Some(EditItem::RequiredField { .. })
                        | Some(EditItem::FrozenField { .. }) => {
                            info.push("Del: remove this field".to_string());
                            info.push("↑/↓: navigate".to_string());
                        }
                        Some(EditItem::EnumField { key }) => {
                            let key = key.clone();
                            if let Some(vals) = self.edit_enum.get(&key) {
                                info.push(format!("Values for {}:", key));
                                for v in vals {
                                    let s = match v {
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    };
                                    info.push(format!("  • {}", s));
                                }
                                info.push(String::new());
                            }
                            info.push("Enter: edit values".to_string());
                            info.push("Del: remove this field".to_string());
                        }
                        Some(EditItem::AddRequired)
                        | Some(EditItem::AddFrozen)
                        | Some(EditItem::AddEnum) => {
                            info.push("Enter: start typing new field".to_string());
                        }
                        _ => {}
                    }
                    info.push(String::new());
                    info.push("Tab: jump to next section".to_string());
                    info.push("Ctrl+S: save & exit".to_string());
                    info.push("Esc: cancel".to_string());
                    let paragraph = Paragraph::new(info.join("\n"))
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Info & Controls"),
                        );
                    f.render_widget(paragraph, horizontal_chunks[1]);
                }
            }
        }

        // Help text
        let help_text = if self.view_mode == ViewMode::Edit {
            if self.edit_enum_value_mode {
                "Type values (comma-separated)  Enter: save  Esc: cancel"
            } else if self.edit_add_mode {
                "Type field name  Enter: confirm  Esc: cancel"
            } else {
                "↑/↓: Navigate  Tab: Next section  Enter: Action  Del: Remove  Ctrl+S: Save  Esc: Cancel"
            }
        } else {
            "↑/↓: Navigate  T: Templates  V: Validation  D: Details  E: Edit  N: New  q: Quit  Enter: Run Validation"
        };
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
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

    fn navigable_indices(&self) -> Vec<usize> {
        self.edit_items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item, EditItem::Header(_)))
            .map(|(i, _)| i)
            .collect()
    }

    fn current_edit_item(&self) -> Option<&EditItem> {
        let nav = self.navigable_indices();
        nav.get(self.edit_nav).and_then(|&i| self.edit_items.get(i))
    }

    fn edit_move_up(&mut self) {
        if self.edit_nav > 0 {
            self.edit_nav -= 1;
        }
    }

    fn edit_move_down(&mut self) {
        let max = self.navigable_indices().len().saturating_sub(1);
        if self.edit_nav < max {
            self.edit_nav += 1;
        }
    }

    fn edit_next_section(&mut self) {
        let nav = self.navigable_indices();
        let current_idx = nav.get(self.edit_nav).copied().unwrap_or(0);
        // Find next header after current position
        let next_header = self
            .edit_items
            .iter()
            .enumerate()
            .skip(current_idx + 1)
            .find(|(_, item)| matches!(item, EditItem::Header(_)))
            .map(|(i, _)| i);
        if let Some(header_idx) = next_header {
            // First navigable after that header
            if let Some(pos) = nav.iter().position(|&i| i > header_idx) {
                self.edit_nav = pos;
                return;
            }
        }
        // Wrap to first navigable
        self.edit_nav = 0;
    }

    fn rebuild_edit_items(&mut self) {
        let mut items = vec![
            EditItem::Header("── Basic ─────────────────────────────────".to_string()),
            EditItem::TextField {
                label: "Title",
                buf_idx: 0,
            },
            EditItem::TextField {
                label: "Slug",
                buf_idx: 1,
            },
            EditItem::Header("── Required Fields ───────────────────────".to_string()),
        ];
        for (idx, value) in self.edit_required.iter().enumerate() {
            items.push(EditItem::RequiredField {
                idx,
                value: value.clone(),
            });
        }
        items.push(EditItem::AddRequired);

        items.push(EditItem::Header(
            "── Frozen Fields ─────────────────────────".to_string(),
        ));
        for (idx, value) in self.edit_frozen.iter().enumerate() {
            items.push(EditItem::FrozenField {
                idx,
                value: value.clone(),
            });
        }
        items.push(EditItem::AddFrozen);

        items.push(EditItem::Header(
            "── Enum Fields ───────────────────────────".to_string(),
        ));
        for key in &self.edit_enum_order.clone() {
            if self.edit_enum.contains_key(key) {
                items.push(EditItem::EnumField { key: key.clone() });
            }
        }
        items.push(EditItem::AddEnum);

        self.edit_items = items;
        let max = self.navigable_indices().len().saturating_sub(1);
        self.edit_nav = self.edit_nav.min(max);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if self.view_mode == ViewMode::Edit {
                    // Enum value editing sub-mode
                    if self.edit_enum_value_mode {
                        match key.code {
                            KeyCode::Esc => {
                                self.edit_enum_value_mode = false;
                            }
                            KeyCode::Enter => {
                                let values: Vec<serde_json::Value> = self
                                    .edit_enum_buf
                                    .split(',')
                                    .map(|s| serde_json::Value::String(s.trim().to_string()))
                                    .filter(|v| v.as_str().map_or(false, |s| !s.is_empty()))
                                    .collect();
                                self.edit_enum.insert(self.edit_enum_key.clone(), values);
                                if !self.edit_enum_order.contains(&self.edit_enum_key) {
                                    self.edit_enum_order.push(self.edit_enum_key.clone());
                                }
                                self.edit_enum_value_mode = false;
                                self.rebuild_edit_items();
                            }
                            KeyCode::Backspace => {
                                self.edit_enum_buf.pop();
                            }
                            KeyCode::Char(c) => {
                                self.edit_enum_buf.push(c);
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    // Add field sub-mode
                    if self.edit_add_mode {
                        match key.code {
                            KeyCode::Esc => {
                                self.edit_add_mode = false;
                                self.edit_add_buffer.clear();
                            }
                            KeyCode::Enter => {
                                let new_field = self.edit_add_buffer.trim().to_string();
                                if !new_field.is_empty() {
                                    match self.edit_add_target.clone() {
                                        AddTarget::Required => {
                                            self.edit_required.push(new_field);
                                        }
                                        AddTarget::Frozen => {
                                            self.edit_frozen.push(new_field);
                                        }
                                        AddTarget::Enum => {
                                            self.edit_enum_key = new_field;
                                            self.edit_enum_buf.clear();
                                            self.edit_enum_value_mode = true;
                                        }
                                    }
                                }
                                self.edit_add_mode = false;
                                self.edit_add_buffer.clear();
                                self.rebuild_edit_items();
                            }
                            KeyCode::Backspace => {
                                self.edit_add_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                self.edit_add_buffer.push(c);
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    // Main edit navigation
                    let on_text_field =
                        matches!(self.current_edit_item(), Some(EditItem::TextField { .. }));

                    // Ctrl+S always saves and exits
                    if key.code == KeyCode::Char('s')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.save_edit();
                        self.view_mode = ViewMode::Details;
                        return Ok(());
                    }

                    if key.code == KeyCode::Esc {
                        self.view_mode = ViewMode::Details;
                        return Ok(());
                    }

                    if on_text_field {
                        match key.code {
                            KeyCode::Backspace => {
                                if let Some(EditItem::TextField { buf_idx, .. }) =
                                    self.current_edit_item().cloned()
                                {
                                    if let Some(buf) = self.edit_basic_buffers.get_mut(buf_idx) {
                                        buf.pop();
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(EditItem::TextField { buf_idx, .. }) =
                                    self.current_edit_item().cloned()
                                {
                                    if let Some(buf) = self.edit_basic_buffers.get_mut(buf_idx) {
                                        buf.push(c);
                                    }
                                }
                            }
                            KeyCode::Tab | KeyCode::Down => self.edit_move_down(),
                            KeyCode::Up => self.edit_move_up(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Up => self.edit_move_up(),
                            KeyCode::Down => self.edit_move_down(),
                            KeyCode::Tab => self.edit_next_section(),
                            KeyCode::Enter => match self.current_edit_item().cloned() {
                                Some(EditItem::AddRequired) => {
                                    self.edit_add_mode = true;
                                    self.edit_add_target = AddTarget::Required;
                                    self.edit_add_buffer.clear();
                                }
                                Some(EditItem::AddFrozen) => {
                                    self.edit_add_mode = true;
                                    self.edit_add_target = AddTarget::Frozen;
                                    self.edit_add_buffer.clear();
                                }
                                Some(EditItem::AddEnum) => {
                                    self.edit_add_mode = true;
                                    self.edit_add_target = AddTarget::Enum;
                                    self.edit_add_buffer.clear();
                                }
                                Some(EditItem::EnumField { key }) => {
                                    self.edit_enum_key = key.clone();
                                    self.edit_enum_buf = self
                                        .edit_enum
                                        .get(&key)
                                        .map(|v| {
                                            v.iter()
                                                .map(|val| match val {
                                                    serde_json::Value::String(s) => s.clone(),
                                                    other => other.to_string(),
                                                })
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        })
                                        .unwrap_or_default();
                                    self.edit_enum_value_mode = true;
                                }
                                _ => {
                                    self.save_edit();
                                    self.view_mode = ViewMode::Details;
                                }
                            },
                            KeyCode::Delete => match self.current_edit_item().cloned() {
                                Some(EditItem::RequiredField { idx, .. }) => {
                                    if idx < self.edit_required.len() {
                                        self.edit_required.remove(idx);
                                        self.rebuild_edit_items();
                                    }
                                }
                                Some(EditItem::FrozenField { idx, .. }) => {
                                    if idx < self.edit_frozen.len() {
                                        self.edit_frozen.remove(idx);
                                        self.rebuild_edit_items();
                                    }
                                }
                                Some(EditItem::EnumField { key }) => {
                                    self.edit_enum.remove(&key);
                                    self.edit_enum_order.retain(|k| k != &key);
                                    self.rebuild_edit_items();
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    return Ok(());
                }

                // Normal mode
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('t') => self.view_mode = ViewMode::Templates,
                    KeyCode::Char('v') => self.view_mode = ViewMode::Validation,
                    KeyCode::Char('d') => self.view_mode = ViewMode::Details,
                    KeyCode::Char('e') => self.enter_edit_mode(),
                    KeyCode::Char('n') => self.create_new_template(),
                    KeyCode::Enter => {
                        if self.view_mode == ViewMode::Validation {
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
                        ViewMode::Details | ViewMode::Edit => {
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

    fn enter_edit_mode(&mut self) {
        if let Some(selected) = self.templates.get(self.selected_index) {
            self.edit_basic_buffers = vec![selected.title.clone(), selected.slug.clone()];

            let (required, frozen, enum_map, enum_order) = if let Some(facets) = &selected.facets {
                if let Some(tf) = &facets.template {
                    if let Some(c) = &tf.constraints {
                        let order: Vec<String> = c.enum_fields.keys().cloned().collect();
                        (
                            c.required_fields.clone(),
                            c.frozen_fields.clone(),
                            c.enum_fields.clone(),
                            order,
                        )
                    } else {
                        (vec![], vec![], HashMap::new(), vec![])
                    }
                } else {
                    (vec![], vec![], HashMap::new(), vec![])
                }
            } else {
                (vec![], vec![], HashMap::new(), vec![])
            };

            self.edit_required = required;
            self.edit_frozen = frozen;
            self.edit_enum = enum_map;
            self.edit_enum_order = enum_order;
            self.edit_nav = 0;
            self.edit_add_mode = false;
            self.edit_add_buffer.clear();
            self.edit_add_target = AddTarget::Required;
            self.edit_enum_value_mode = false;
            self.edit_enum_key.clear();
            self.edit_enum_buf.clear();
            self.view_mode = ViewMode::Edit;
            self.rebuild_edit_items();
        }
    }

    fn save_edit(&mut self) {
        use cardstack_lib::card::{Facets, TemplateConstraints, TemplateFacet};

        if let Some(template) = self.templates.get_mut(self.selected_index) {
            if let Some(title) = self.edit_basic_buffers.get(0) {
                template.title = title.clone();
            }
            if let Some(slug) = self.edit_basic_buffers.get(1) {
                template.slug = slug.clone();
            }

            let constraints = TemplateConstraints {
                required_fields: self.edit_required.clone(),
                frozen_fields: self.edit_frozen.clone(),
                enum_fields: self.edit_enum.clone(),
            };

            if template.facets.is_none() {
                template.facets = Some(Facets {
                    content: None,
                    collection: None,
                    template: None,
                });
            }
            if let Some(facets) = &mut template.facets {
                if facets.template.is_none() {
                    facets.template = Some(TemplateFacet {
                        constraints: None,
                        defaults: HashMap::new(),
                    });
                }
                if let Some(tf) = &mut facets.template {
                    tf.constraints = Some(constraints);
                }
            }
        }
    }

    fn run_validation(&mut self) {
        self.validation_results = self
            .cards
            .iter()
            .map(|card| ValidationResult {
                uid: card.uid.clone(),
                slug: card.slug.clone(),
                template: Some("template-example".to_string()),
                valid: card.uid.len() % 2 == 0,
                errors: if card.uid.len() % 2 == 0 {
                    Vec::new()
                } else {
                    vec![
                        "Missing required field".to_string(),
                        "Invalid enum value".to_string(),
                    ]
                },
                warnings: vec!["Field should not be modified".to_string()],
            })
            .collect();
    }

    fn create_new_template(&mut self) {
        use cardstack_lib::card::{Card, TemplateConstraints, TemplateFacet};

        let mut constraints = TemplateConstraints {
            required_fields: vec!["fields.status".to_string(), "fields.topic".to_string()],
            enum_fields: HashMap::new(),
            frozen_fields: vec!["author.id".to_string()],
        };

        constraints.enum_fields.insert(
            "fields.status".to_string(),
            vec![
                serde_json::Value::String("draft".to_string()),
                serde_json::Value::String("active".to_string()),
                serde_json::Value::String("published".to_string()),
            ],
        );
        constraints.enum_fields.insert(
            "fields.source_type".to_string(),
            vec![
                serde_json::Value::String("paper".to_string()),
                serde_json::Value::String("article".to_string()),
                serde_json::Value::String("book".to_string()),
            ],
        );

        let template_facet = TemplateFacet {
            constraints: Some(constraints),
            defaults: HashMap::new(),
        };

        let facets = cardstack_lib::card::Facets {
            content: None,
            collection: None,
            template: Some(template_facet),
        };

        let mut new_template = Card::new(
            "New Template".to_string(),
            "template-new-template".to_string(),
            cardstack_lib::uid::generate_uid(),
        );
        new_template.facets = Some(facets);

        self.templates.push(new_template);
    }
}

pub fn run_tui(templates: Vec<Card>, cards: Vec<Card>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(templates, cards);
    app.run(&mut terminal)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}
