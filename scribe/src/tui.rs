use anyhow::Result;
use cardstack_lib::{
    card::{Card, CollectionFacet, CollectionMode, Facets},
    repository::{load_all_cards, save_card},
    uid,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
use std::path::PathBuf;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum ViewMode {
    Cards,
    Decks,
    DeckMembers,
    Form,
    Confirm,
}

#[derive(Clone)]
enum FormItem {
    Header(String),
    TextField { label: &'static str, buf_idx: usize },
    CustomField { idx: usize },
    AddCustomField,
    BodyField,
}

// ── App ──────────────────────────────────────────────────────────────────────

struct App {
    repo: PathBuf,
    all_cards: Vec<(PathBuf, Card)>,

    // Cards view
    view_mode: ViewMode,
    card_sel: usize,
    filter_active: bool,
    filter_buf: String,
    card_indices: Vec<usize>,

    // Decks view
    deck_sel: usize,
    deck_indices: Vec<usize>,

    // Deck members view
    current_deck: usize,
    member_sel: usize,
    member_indices: Vec<usize>,
    add_member_mode: bool,
    add_member_buf: String,

    // Form (new / edit card or deck)
    form_is_new: bool,
    form_is_deck: bool,
    form_card_uid: Option<String>,
    form_items: Vec<FormItem>,
    form_nav: usize,
    form_basic: Vec<String>, // [title, slug, tags, template] or [name, mode, query] for deck
    form_custom: Vec<(String, String)>,
    form_body: String,
    form_add_mode: bool,
    form_add_buf: String,
    form_body_mode: bool,
    form_body_prev: String,

    // Confirmation (archive)
    confirm_uid: String,
    confirm_title: String,

    should_quit: bool,
    status: String,
}

fn card_type_icon(card: &Card) -> &'static str {
    if card.has_collection() {
        "⬡"
    } else if card.facets.as_ref().and_then(|f| f.template.as_ref()).is_some() {
        "⊞"
    } else {
        "·"
    }
}

fn is_archived(card: &Card) -> bool {
    card.fields
        .get("archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

fn title_to_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

impl App {
    fn new(repo: PathBuf, all_cards: Vec<(PathBuf, Card)>) -> Self {
        let mut app = Self {
            repo,
            all_cards,
            view_mode: ViewMode::Cards,
            card_sel: 0,
            filter_active: false,
            filter_buf: String::new(),
            card_indices: Vec::new(),
            deck_sel: 0,
            deck_indices: Vec::new(),
            current_deck: 0,
            member_sel: 0,
            member_indices: Vec::new(),
            add_member_mode: false,
            add_member_buf: String::new(),
            form_is_new: true,
            form_is_deck: false,
            form_card_uid: None,
            form_items: Vec::new(),
            form_nav: 0,
            form_basic: Vec::new(),
            form_custom: Vec::new(),
            form_body: String::new(),
            form_add_mode: false,
            form_add_buf: String::new(),
            form_body_mode: false,
            form_body_prev: String::new(),
            confirm_uid: String::new(),
            confirm_title: String::new(),
            should_quit: false,
            status: String::new(),
        };
        app.rebuild_card_indices();
        app.rebuild_deck_indices();
        app
    }

    fn reload(&mut self) {
        match load_all_cards(&self.repo) {
            Ok(cards) => {
                self.all_cards = cards;
                self.rebuild_card_indices();
                self.rebuild_deck_indices();
                // Re-resolve deck members if in that view
                if self.view_mode == ViewMode::DeckMembers {
                    self.resolve_current_deck_members();
                }
                // Clamp selections
                self.card_sel =
                    self.card_sel.min(self.card_indices.len().saturating_sub(1));
                self.deck_sel =
                    self.deck_sel.min(self.deck_indices.len().saturating_sub(1));
            }
            Err(e) => self.status = format!("Error reloading: {}", e),
        }
    }

    fn rebuild_card_indices(&mut self) {
        let q = self.filter_buf.to_lowercase();
        self.card_indices = (0..self.all_cards.len())
            .filter(|&i| {
                let card = &self.all_cards[i].1;
                if q.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    card.title.to_lowercase(),
                    card.slug.to_lowercase(),
                    card.uid.to_lowercase(),
                    card.tags.join(" ").to_lowercase()
                );
                haystack.contains(&q)
            })
            .collect();
        self.card_sel = self.card_sel.min(self.card_indices.len().saturating_sub(1));
    }

    fn rebuild_deck_indices(&mut self) {
        self.deck_indices = (0..self.all_cards.len())
            .filter(|&i| self.all_cards[i].1.has_collection())
            .collect();
        self.deck_sel = self.deck_sel.min(self.deck_indices.len().saturating_sub(1));
    }

    fn resolve_current_deck_members(&mut self) {
        let Some(&deck_data_idx) = self.deck_indices.get(self.current_deck) else {
            self.member_indices = Vec::new();
            return;
        };
        let deck_card = &self.all_cards[deck_data_idx].1;
        self.member_indices = deck_card
            .facets
            .as_ref()
            .and_then(|f| f.collection.as_ref())
            .map(|col| {
                col.members
                    .iter()
                    .filter_map(|uid| self.all_cards.iter().position(|(_, c)| &c.uid == uid))
                    .collect()
            })
            .unwrap_or_default();
        self.member_sel = self.member_sel.min(self.member_indices.len().saturating_sub(1));
    }

    fn current_card(&self) -> Option<&Card> {
        self.card_indices
            .get(self.card_sel)
            .and_then(|&i| self.all_cards.get(i))
            .map(|(_, c)| c)
    }

    fn current_deck_card(&self) -> Option<&Card> {
        self.deck_indices
            .get(self.deck_sel)
            .and_then(|&i| self.all_cards.get(i))
            .map(|(_, c)| c)
    }

    fn current_member_card(&self) -> Option<&Card> {
        self.member_indices
            .get(self.member_sel)
            .and_then(|&i| self.all_cards.get(i))
            .map(|(_, c)| c)
    }

    // ── Form helpers ─────────────────────────────────────────────────────────

    fn enter_new_card_form(&mut self) {
        self.form_is_new = true;
        self.form_is_deck = false;
        self.form_card_uid = None;
        self.form_basic = vec![
            String::new(), // title
            String::new(), // slug
            String::new(), // tags
            String::new(), // template
        ];
        self.form_custom = Vec::new();
        self.form_body = String::new();
        self.form_add_mode = false;
        self.form_add_buf = String::new();
        self.form_body_mode = false;
        self.form_nav = 0;
        self.view_mode = ViewMode::Form;
        self.rebuild_form_items();
    }

    fn enter_edit_card_form(&mut self) {
        let Some(card) = self.current_card().cloned() else {
            return;
        };
        self.form_is_new = false;
        self.form_is_deck = false;
        self.form_card_uid = Some(card.uid.clone());
        self.form_basic = vec![
            card.title.clone(),
            card.slug.clone(),
            card.tags.join(", "),
            String::new(), // template (not stored directly)
        ];
        self.form_custom = card
            .fields
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), val)
            })
            .collect();
        self.form_custom.sort_by(|a, b| a.0.cmp(&b.0));
        self.form_body = card.get_content().unwrap_or("").to_string();
        self.form_add_mode = false;
        self.form_add_buf = String::new();
        self.form_body_mode = false;
        self.form_nav = 0;
        self.view_mode = ViewMode::Form;
        self.rebuild_form_items();
    }

    fn enter_new_deck_form(&mut self) {
        self.form_is_new = true;
        self.form_is_deck = true;
        self.form_card_uid = None;
        self.form_basic = vec![
            String::new(), // name
            "static".to_string(), // mode
            String::new(), // query
        ];
        self.form_custom = Vec::new();
        self.form_body = String::new();
        self.form_add_mode = false;
        self.form_add_buf = String::new();
        self.form_body_mode = false;
        self.form_nav = 0;
        self.view_mode = ViewMode::Form;
        self.rebuild_form_items();
    }

    fn rebuild_form_items(&mut self) {
        let mut items: Vec<FormItem> = Vec::new();
        if self.form_is_deck {
            items.push(FormItem::Header("── Deck ──────────────────────────────────".to_string()));
            items.push(FormItem::TextField { label: "Name", buf_idx: 0 });
            items.push(FormItem::TextField { label: "Mode (static/query/hybrid)", buf_idx: 1 });
            items.push(FormItem::TextField { label: "Query", buf_idx: 2 });
        } else {
            items.push(FormItem::Header("── Basic ─────────────────────────────────".to_string()));
            items.push(FormItem::TextField { label: "Title", buf_idx: 0 });
            items.push(FormItem::TextField { label: "Slug", buf_idx: 1 });
            items.push(FormItem::TextField { label: "Tags (comma-sep)", buf_idx: 2 });
            items.push(FormItem::TextField { label: "Template", buf_idx: 3 });
            items.push(FormItem::Header("── Custom Fields ─────────────────────────".to_string()));
            for (idx, _) in self.form_custom.iter().enumerate() {
                items.push(FormItem::CustomField { idx });
            }
            items.push(FormItem::AddCustomField);
            items.push(FormItem::Header("── Body ──────────────────────────────────".to_string()));
            items.push(FormItem::BodyField);
        }
        self.form_items = items;
        let max = self.form_navigable().len().saturating_sub(1);
        self.form_nav = self.form_nav.min(max);
    }

    fn form_navigable(&self) -> Vec<usize> {
        self.form_items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item, FormItem::Header(_)))
            .map(|(i, _)| i)
            .collect()
    }

    fn form_current_item(&self) -> Option<&FormItem> {
        let nav = self.form_navigable();
        nav.get(self.form_nav).and_then(|&i| self.form_items.get(i))
    }

    fn save_form(&mut self) -> Result<()> {
        if self.form_is_deck {
            let name = self.form_basic[0].trim().to_string();
            if name.is_empty() {
                self.status = "Name is required".to_string();
                return Ok(());
            }
            let slug = title_to_slug(&name);
            let deck_uid = uid::generate_uid();
            let mut card = Card::new(name.clone(), slug, deck_uid);
            let mode_str = self.form_basic[1].trim();
            let mode = match mode_str {
                "query" => CollectionMode::Query,
                "hybrid" => CollectionMode::Hybrid,
                _ => CollectionMode::Static,
            };
            let query_str = self.form_basic.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let query_val = if !query_str.is_empty() {
                match cardstack_lib::query::parse_query_shorthand(&query_str) {
                    Ok(q) => {
                        let mut obj = serde_json::Map::new();
                        if let Some(filter) = &q.filter {
                            let mut f = serde_json::Map::new();
                            let (op, preds) = match filter {
                                cardstack_lib::query::Filter::All(p) => ("all", p),
                                cardstack_lib::query::Filter::Any(p) => ("any", p),
                                cardstack_lib::query::Filter::None(p) => ("none", p),
                            };
                            f.insert("op".into(), op.into());
                            f.insert(
                                "predicates".into(),
                                serde_json::Value::Array(
                                    preds.iter().map(|p| p.clone().into()).collect(),
                                ),
                            );
                            obj.insert("filter".into(), serde_json::Value::Object(f));
                        }
                        Some(serde_json::Value::Object(obj))
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            card.facets = Some(Facets {
                content: None,
                collection: Some(CollectionFacet {
                    mode,
                    members: Vec::new(),
                    query: query_val,
                    view: None,
                }),
                template: None,
            });
            save_card(&self.repo, &mut card)?;
            self.status = format!("Created deck: {}", name);
        } else {
            // Card form
            let title = self.form_basic[0].trim().to_string();
            if title.is_empty() {
                self.status = "Title is required".to_string();
                return Ok(());
            }
            let slug = if self.form_basic[1].trim().is_empty() {
                title_to_slug(&title)
            } else {
                self.form_basic[1].trim().to_string()
            };

            let mut card = if let Some(uid) = &self.form_card_uid {
                match cardstack_lib::repository::load_card(&self.repo, uid) {
                    Ok(c) => c,
                    Err(_) => Card::new(title.clone(), slug.clone(), uid.clone()),
                }
            } else {
                Card::new(title.clone(), slug.clone(), uid::generate_uid())
            };

            card.title = title.clone();
            card.slug = slug;
            card.tags = self.form_basic[2]
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            // Apply template defaults if specified
            let template_slug = self.form_basic.get(3).map(|s| s.trim().to_string()).unwrap_or_default();
            if !template_slug.is_empty() && self.form_is_new {
                if let Ok(tmpl) = cardstack_lib::repository::load_card(&self.repo, &template_slug) {
                    if let Some(facets) = &tmpl.facets {
                        if let Some(tf) = &facets.template {
                            for (k, v) in &tf.defaults {
                                card.fields.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                        }
                    }
                }
            }

            // Custom fields
            card.fields.clear();
            for (k, v) in &self.form_custom {
                if !k.is_empty() {
                    card.fields.insert(k.clone(), serde_json::Value::String(v.clone()));
                }
            }

            // Body
            if !self.form_body.is_empty() {
                card = card.with_content(self.form_body.clone());
            }

            save_card(&self.repo, &mut card)?;
            self.status = format!(
                "{}: {}",
                if self.form_is_new { "Created" } else { "Updated" },
                title
            );
        }

        self.reload();
        Ok(())
    }

    // ── Run ──────────────────────────────────────────────────────────────────

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.ui(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    // ── UI ───────────────────────────────────────────────────────────────────

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ].as_ref())
            .split(f.size());

        // Title
        let title_text = match self.view_mode {
            ViewMode::Cards => "Karduun Scribe — Cards",
            ViewMode::Decks => "Karduun Scribe — Decks",
            ViewMode::DeckMembers => "Karduun Scribe — Deck Members",
            ViewMode::Form => {
                if self.form_is_new && self.form_is_deck { "Karduun Scribe — New Deck" }
                else if self.form_is_new { "Karduun Scribe — New Card" }
                else { "Karduun Scribe — Edit Card" }
            }
            ViewMode::Confirm => "Karduun Scribe — Confirm Archive",
        };
        let title = Paragraph::new(title_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main area
        match self.view_mode {
            ViewMode::Cards => self.render_cards(f, chunks[1]),
            ViewMode::Decks => self.render_decks(f, chunks[1]),
            ViewMode::DeckMembers => self.render_deck_members(f, chunks[1]),
            ViewMode::Form => self.render_form(f, chunks[1]),
            ViewMode::Confirm => self.render_confirm(f, chunks[1]),
        }

        // Help / status
        let help = if !self.status.is_empty() {
            Paragraph::new(self.status.as_str()).style(Style::default().fg(Color::Green))
        } else {
            let text = match self.view_mode {
                ViewMode::Cards => if self.filter_active {
                    "Type to filter  Enter: apply  Esc: clear"
                } else {
                    "↑/↓: Navigate  n:New  e:Edit  a:Archive  d:Decks  /:Filter  q:Quit"
                },
                ViewMode::Decks => "↑/↓: Navigate  Enter:Members  n:New deck  c:Cards  q:Quit",
                ViewMode::DeckMembers => {
                    if self.add_member_mode {
                        "Type card uid/slug  Enter:add  Esc:cancel"
                    } else {
                        "↑/↓: Navigate  a:Add  Del:Remove  d:Back to decks  q:Quit"
                    }
                }
                ViewMode::Form => {
                    if self.form_body_mode {
                        "Type body text  Enter:newline  Ctrl+S:save  Esc:cancel"
                    } else if self.form_add_mode {
                        "Type key=value  Enter:add  Esc:cancel"
                    } else {
                        "↑/↓/Tab: Navigate  Enter:Action  Del:Remove  Ctrl+S:Save  Esc:Cancel"
                    }
                }
                ViewMode::Confirm => "y:Archive  n/Esc:Cancel",
            };
            Paragraph::new(text).style(Style::default().fg(Color::DarkGray))
        };
        f.render_widget(help, chunks[2]);
    }

    fn render_cards(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        // Filter bar above list
        let list_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(split[0]);

        let filter_text = if self.filter_active {
            format!("/{}_", self.filter_buf)
        } else if self.filter_buf.is_empty() {
            "(press / to filter)".to_string()
        } else {
            format!("/{} ✓", self.filter_buf)
        };
        let filter_style = if self.filter_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let filter_bar = Paragraph::new(filter_text)
            .style(filter_style)
            .block(Block::default().borders(Borders::ALL).title("Filter"));
        f.render_widget(filter_bar, list_chunks[0]);

        // Card list
        let items: Vec<ListItem> = self
            .card_indices
            .iter()
            .enumerate()
            .map(|(display_i, &data_i)| {
                let card = &self.all_cards[data_i].1;
                let is_sel = display_i == self.card_sel;
                let icon = card_type_icon(card);
                let archived = is_archived(card);
                let title = truncate(&card.title, 30);
                let tags = if card.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", card.tags.join(", "))
                };

                let fg = if archived {
                    Color::DarkGray
                } else if is_sel {
                    Color::Yellow
                } else {
                    Color::White
                };

                let prefix = if is_sel { "▶ " } else { "  " };
                let arch_mark = if archived { " ✗" } else { "" };
                let line = format!("{}{} {}{}{}", prefix, icon, title, arch_mark, tags);
                ListItem::new(Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(fg)
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                )))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            "Cards ({}/{})",
            self.card_indices.len(),
            self.all_cards.len()
        )));
        f.render_widget(list, list_chunks[1]);

        // Right: card detail
        if let Some(card) = self.current_card() {
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    truncate(&card.title, 44),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("uid:  {}", card.uid),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    format!("slug: {}", card.slug),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    format!(
                        "created: {}  updated: {}",
                        card.created.format("%Y-%m-%d"),
                        card.updated.format("%Y-%m-%d")
                    ),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ];

            if !card.tags.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("tags:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(card.tags.join(", "), Style::default().fg(Color::Blue)),
                ]));
            }

            if !card.fields.is_empty() {
                lines.push(Line::from(Span::styled(
                    "fields:",
                    Style::default().fg(Color::DarkGray),
                )));
                let mut fields: Vec<_> = card.fields.iter().collect();
                fields.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in fields.iter().take(8) {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {}: ", k),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(truncate(&val, 32), Style::default().fg(Color::White)),
                    ]));
                }
            }

            if !card.links.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "links:",
                    Style::default().fg(Color::DarkGray),
                )));
                for link in card.links.iter().take(5) {
                    lines.push(Line::from(Span::styled(
                        format!("  [{}] → {}", link.r#type, truncate(&link.to, 30)),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            if let Some(body) = card.get_content() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "── body ──",
                    Style::default().fg(Color::DarkGray),
                )));
                let preview = if body.len() > 400 {
                    format!("{}…", &body[..400])
                } else {
                    body.to_string()
                };
                lines.push(Line::from(Span::styled(
                    preview,
                    Style::default().fg(Color::White),
                )));
            }

            let para = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Details"));
            f.render_widget(para, split[1]);
        }
    }

    fn render_decks(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let items: Vec<ListItem> = self
            .deck_indices
            .iter()
            .enumerate()
            .map(|(display_i, &data_i)| {
                let card = &self.all_cards[data_i].1;
                let is_sel = display_i == self.deck_sel;
                let mode = card
                    .facets
                    .as_ref()
                    .and_then(|f| f.collection.as_ref())
                    .map(|c| match c.mode {
                        CollectionMode::Static => "static",
                        CollectionMode::Query => "query",
                        CollectionMode::Hybrid => "hybrid",
                    })
                    .unwrap_or("?");
                let members = card
                    .facets
                    .as_ref()
                    .and_then(|f| f.collection.as_ref())
                    .map(|c| c.members.len())
                    .unwrap_or(0);
                let title = truncate(&card.title, 32);
                let prefix = if is_sel { "▶ " } else { "  " };
                let line =
                    format!("{}{:<32} [{}] {}m", prefix, title, mode, members);
                ListItem::new(Line::from(Span::styled(
                    line,
                    Style::default().fg(if is_sel { Color::Yellow } else { Color::White }).add_modifier(
                        if is_sel { Modifier::BOLD } else { Modifier::empty() },
                    ),
                )))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            "Decks ({})",
            self.deck_indices.len()
        )));
        f.render_widget(list, split[0]);

        // Right: deck details
        if let Some(card) = self.current_deck_card() {
            let col = card.facets.as_ref().and_then(|f| f.collection.as_ref());
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    truncate(&card.title, 44),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("uid: {}", card.uid),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ];

            if let Some(col) = col {
                lines.push(Line::from(vec![
                    Span::styled("mode:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:?}", col.mode),
                        Style::default().fg(Color::White),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("members: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        col.members.len().to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]));
                if let Some(q) = &col.query {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "query:",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(Span::styled(
                        truncate(&q.to_string(), 50),
                        Style::default().fg(Color::White),
                    )));
                }
                if !col.members.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "static members:",
                        Style::default().fg(Color::DarkGray),
                    )));
                    for uid in col.members.iter().take(10) {
                        let title = self
                            .all_cards
                            .iter()
                            .find(|(_, c)| &c.uid == uid)
                            .map(|(_, c)| c.title.as_str())
                            .unwrap_or("(not found)");
                        lines.push(Line::from(Span::styled(
                            format!("  · {}", truncate(title, 40)),
                            Style::default().fg(Color::White),
                        )));
                    }
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Enter: view members",
                Style::default().fg(Color::DarkGray),
            )));

            let para = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Deck Info"));
            f.render_widget(para, split[1]);
        }
    }

    fn render_deck_members(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let deck_name = self
            .deck_indices
            .get(self.current_deck)
            .and_then(|&i| self.all_cards.get(i))
            .map(|(_, c)| c.title.as_str())
            .unwrap_or("?");

        let mut items: Vec<ListItem> = self
            .member_indices
            .iter()
            .enumerate()
            .map(|(display_i, &data_i)| {
                let card = &self.all_cards[data_i].1;
                let is_sel = display_i == self.member_sel;
                let title = truncate(&card.title, 36);
                let prefix = if is_sel { "▶ " } else { "  " };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", prefix, title),
                    Style::default()
                        .fg(if is_sel { Color::Yellow } else { Color::White })
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                )))
            })
            .collect();

        // Add member row
        let add_row = if self.add_member_mode {
            format!("  > {}_", self.add_member_buf)
        } else {
            "  + Add card...".to_string()
        };
        items.push(ListItem::new(Line::from(Span::styled(
            add_row,
            Style::default().fg(Color::Green),
        ))));

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            "Members of: {}  ({})",
            truncate(deck_name, 24),
            self.member_indices.len()
        )));
        f.render_widget(list, split[0]);

        // Right: member detail
        if let Some(card) = self.current_member_card() {
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    truncate(&card.title, 44),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("uid:  {}", card.uid),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    format!("slug: {}", card.slug),
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            if !card.tags.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("tags: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(card.tags.join(", "), Style::default().fg(Color::Blue)),
                ]));
            }
            if !card.fields.is_empty() {
                lines.push(Line::from(""));
                let mut fields: Vec<_> = card.fields.iter().collect();
                fields.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in fields.iter().take(6) {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}: ", k), Style::default().fg(Color::DarkGray)),
                        Span::styled(truncate(&val, 32), Style::default().fg(Color::White)),
                    ]));
                }
            }
            let para = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Card Details"));
            f.render_widget(para, split[1]);
        }
    }

    fn render_form(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
            .split(area);

        // Left: form fields
        let nav_indices = self.form_navigable();
        let current_item_idx = nav_indices.get(self.form_nav).copied();

        let items: Vec<ListItem> = self
            .form_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_sel = current_item_idx == Some(i);
                match item {
                    FormItem::Header(h) => ListItem::new(Line::from(Span::styled(
                        h.clone(),
                        Style::default().fg(Color::DarkGray),
                    ))),
                    FormItem::TextField { label, buf_idx } => {
                        let val = self.form_basic.get(*buf_idx).map(|s| s.as_str()).unwrap_or("");
                        let display = if is_sel {
                            format!("  {}: {}_", label, val)
                        } else {
                            format!("  {}: {}", label, val)
                        };
                        ListItem::new(Line::from(Span::styled(
                            display,
                            Style::default().fg(if is_sel { Color::Yellow } else { Color::White }),
                        )))
                    }
                    FormItem::CustomField { idx } => {
                        let (k, v) = self
                            .form_custom
                            .get(*idx)
                            .map(|(a, b)| (a.as_str(), b.as_str()))
                            .unwrap_or(("?", "?"));
                        let prefix = if is_sel { "▶ " } else { "  " };
                        ListItem::new(Line::from(Span::styled(
                            format!("{}{}={}", prefix, k, v),
                            Style::default().fg(if is_sel { Color::Yellow } else { Color::White }),
                        )))
                    }
                    FormItem::AddCustomField => {
                        let label = if self.form_add_mode && is_sel {
                            format!("  > {}_", self.form_add_buf)
                        } else {
                            let prefix = if is_sel { "▶ " } else { "  " };
                            format!("{}+ Add field", prefix)
                        };
                        ListItem::new(Line::from(Span::styled(
                            label,
                            Style::default().fg(if is_sel { Color::Green } else { Color::DarkGray }),
                        )))
                    }
                    FormItem::BodyField => {
                        let preview = if self.form_body.is_empty() {
                            "(no body)".to_string()
                        } else {
                            let first_line = self.form_body.lines().next().unwrap_or("");
                            format!("{} …", truncate(first_line, 28))
                        };
                        let prefix = if is_sel { "▶ " } else { "  " };
                        ListItem::new(Line::from(Span::styled(
                            format!("{}Body: {}", prefix, preview),
                            Style::default().fg(if is_sel { Color::Yellow } else { Color::White }),
                        )))
                    }
                }
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(if self.form_is_new { "New" } else { "Edit" }),
        );
        f.render_widget(list, split[0]);

        // Right panel
        if self.form_body_mode {
            // Body editor
            let para = Paragraph::new(format!("{}_", self.form_body))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Body Editor (Ctrl+S: save  Esc: cancel)"),
                );
            f.render_widget(para, split[1]);
        } else {
            // Card preview
            let title = self.form_basic.first().map(|s| s.as_str()).unwrap_or("(untitled)");
            let slug = self.form_basic.get(1).filter(|s| !s.is_empty())
                .map(|s| s.clone())
                .unwrap_or_else(|| title_to_slug(title));
            let tags = self.form_basic.get(2).map(|s| s.as_str()).unwrap_or("");
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    "── Preview ──",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    truncate(title, 44),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("slug: {}", slug),
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            if !tags.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("tags: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(tags.to_string(), Style::default().fg(Color::Blue)),
                ]));
            }
            if !self.form_custom.is_empty() {
                lines.push(Line::from(""));
                for (k, v) in &self.form_custom {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}: ", k), Style::default().fg(Color::DarkGray)),
                        Span::styled(truncate(v, 32), Style::default().fg(Color::White)),
                    ]));
                }
            }
            if !self.form_body.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "── body ──",
                    Style::default().fg(Color::DarkGray),
                )));
                let preview = if self.form_body.len() > 300 {
                    format!("{}…", &self.form_body[..300])
                } else {
                    self.form_body.clone()
                };
                lines.push(Line::from(Span::styled(
                    preview,
                    Style::default().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Ctrl+S: save  Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )));

            let para = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Preview"));
            f.render_widget(para, split[1]);
        }
    }

    fn render_confirm(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let text = format!(
            "\nArchive card?\n\n  \"{}\"\n  ({})\n\nThis marks the card as archived.\n\ny: confirm  n/Esc: cancel",
            self.confirm_title, self.confirm_uid
        );
        let para = Paragraph::new(text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Confirm Archive"));
        f.render_widget(para, area);
    }

    // ── Events ───────────────────────────────────────────────────────────────

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(std::time::Duration::from_millis(16))? {
            return Ok(());
        }
        if let Event::Key(key) = event::read()? {
            self.status.clear();
            match self.view_mode {
                ViewMode::Cards => self.handle_cards_key(key.code, key.modifiers)?,
                ViewMode::Decks => self.handle_decks_key(key.code)?,
                ViewMode::DeckMembers => self.handle_members_key(key.code)?,
                ViewMode::Form => self.handle_form_key(key.code, key.modifiers)?,
                ViewMode::Confirm => self.handle_confirm_key(key.code),
            }
        }
        Ok(())
    }

    fn handle_cards_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<()> {
        if self.filter_active {
            match code {
                KeyCode::Esc => {
                    self.filter_active = false;
                    self.filter_buf.clear();
                    self.rebuild_card_indices();
                }
                KeyCode::Enter => self.filter_active = false,
                KeyCode::Backspace => {
                    self.filter_buf.pop();
                    self.rebuild_card_indices();
                }
                KeyCode::Char(c) => {
                    self.filter_buf.push(c);
                    self.rebuild_card_indices();
                }
                _ => {}
            }
            return Ok(());
        }

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('/') => self.filter_active = true,
            KeyCode::Char('d') => self.view_mode = ViewMode::Decks,
            KeyCode::Char('n') => self.enter_new_card_form(),
            KeyCode::Char('e') | KeyCode::Enter => self.enter_edit_card_form(),
            KeyCode::Char('a') => {
                if let Some((uid, title)) = self.current_card().map(|c| (c.uid.clone(), c.title.clone())) {
                    self.confirm_uid = uid;
                    self.confirm_title = title;
                    self.view_mode = ViewMode::Confirm;
                }
            }
            KeyCode::Up => {
                if self.card_sel > 0 {
                    self.card_sel -= 1;
                }
            }
            KeyCode::Down => {
                if self.card_sel < self.card_indices.len().saturating_sub(1) {
                    self.card_sel += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_decks_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') => self.view_mode = ViewMode::Cards,
            KeyCode::Char('n') => self.enter_new_deck_form(),
            KeyCode::Up => {
                if self.deck_sel > 0 {
                    self.deck_sel -= 1;
                }
            }
            KeyCode::Down => {
                if self.deck_sel < self.deck_indices.len().saturating_sub(1) {
                    self.deck_sel += 1;
                }
            }
            KeyCode::Enter => {
                if !self.deck_indices.is_empty() {
                    self.current_deck = self.deck_sel;
                    self.member_sel = 0;
                    self.resolve_current_deck_members();
                    self.view_mode = ViewMode::DeckMembers;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_members_key(&mut self, code: KeyCode) -> Result<()> {
        if self.add_member_mode {
            match code {
                KeyCode::Esc => {
                    self.add_member_mode = false;
                    self.add_member_buf.clear();
                }
                KeyCode::Enter => {
                    let identifier = self.add_member_buf.trim().to_string();
                    if !identifier.is_empty() {
                        self.add_card_to_deck(&identifier)?;
                    }
                    self.add_member_mode = false;
                    self.add_member_buf.clear();
                }
                KeyCode::Backspace => {
                    self.add_member_buf.pop();
                }
                KeyCode::Char(c) => {
                    self.add_member_buf.push(c);
                }
                _ => {}
            }
            return Ok(());
        }

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') | KeyCode::Esc => self.view_mode = ViewMode::Decks,
            KeyCode::Char('a') => {
                self.add_member_mode = true;
                self.add_member_buf.clear();
            }
            KeyCode::Up => {
                if self.member_sel > 0 {
                    self.member_sel -= 1;
                }
            }
            KeyCode::Down => {
                if self.member_sel < self.member_indices.len().saturating_sub(1) {
                    self.member_sel += 1;
                }
            }
            KeyCode::Delete => {
                self.remove_card_from_deck()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_form_key(&mut self, code: KeyCode, mods: KeyModifiers) -> Result<()> {
        // Body editing sub-mode
        if self.form_body_mode {
            match code {
                KeyCode::Esc => {
                    self.form_body = self.form_body_prev.clone();
                    self.form_body_mode = false;
                }
                KeyCode::Char('s') if mods.contains(KeyModifiers::CONTROL) => {
                    self.form_body_mode = false;
                }
                KeyCode::Enter => self.form_body.push('\n'),
                KeyCode::Backspace => {
                    self.form_body.pop();
                }
                KeyCode::Char(c) => self.form_body.push(c),
                _ => {}
            }
            return Ok(());
        }

        // Add field sub-mode
        if self.form_add_mode {
            match code {
                KeyCode::Esc => {
                    self.form_add_mode = false;
                    self.form_add_buf.clear();
                }
                KeyCode::Enter => {
                    if let Some((k, v)) = self.form_add_buf.split_once('=') {
                        let k = k.trim().to_string();
                        let v = v.trim().to_string();
                        if !k.is_empty() {
                            self.form_custom.push((k, v));
                            self.rebuild_form_items();
                        }
                    }
                    self.form_add_mode = false;
                    self.form_add_buf.clear();
                }
                KeyCode::Backspace => {
                    self.form_add_buf.pop();
                }
                KeyCode::Char(c) => self.form_add_buf.push(c),
                _ => {}
            }
            return Ok(());
        }

        // Ctrl+S: save
        if code == KeyCode::Char('s') && mods.contains(KeyModifiers::CONTROL) {
            self.save_form()?;
            self.view_mode = if self.form_is_deck { ViewMode::Decks } else { ViewMode::Cards };
            return Ok(());
        }

        if code == KeyCode::Esc {
            self.view_mode = if self.form_is_deck { ViewMode::Decks } else { ViewMode::Cards };
            return Ok(());
        }

        let on_text_field = matches!(self.form_current_item(), Some(FormItem::TextField { .. }));

        if on_text_field {
            match code {
                KeyCode::Backspace => {
                    if let Some(FormItem::TextField { buf_idx, .. }) =
                        self.form_current_item().cloned()
                    {
                        if let Some(buf) = self.form_basic.get_mut(buf_idx) {
                            buf.pop();
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(FormItem::TextField { buf_idx, .. }) =
                        self.form_current_item().cloned()
                    {
                        if let Some(buf) = self.form_basic.get_mut(buf_idx) {
                            buf.push(c);
                        }
                        // Auto-fill slug from title
                        if buf_idx == 0 && self.form_is_new && !self.form_is_deck {
                            let auto_slug = title_to_slug(
                                self.form_basic.first().map(|s| s.as_str()).unwrap_or(""),
                            );
                            if let Some(slug) = self.form_basic.get_mut(1) {
                                *slug = auto_slug;
                            }
                        }
                    }
                }
                KeyCode::Tab | KeyCode::Down => self.form_nav_next(),
                KeyCode::Up => self.form_nav_prev(),
                _ => {}
            }
        } else {
            match code {
                KeyCode::Up => self.form_nav_prev(),
                KeyCode::Down | KeyCode::Tab => self.form_nav_next(),
                KeyCode::Enter => match self.form_current_item().cloned() {
                    Some(FormItem::AddCustomField) => {
                        self.form_add_mode = true;
                        self.form_add_buf.clear();
                    }
                    Some(FormItem::BodyField) => {
                        self.form_body_prev = self.form_body.clone();
                        self.form_body_mode = true;
                    }
                    _ => {}
                },
                KeyCode::Delete => {
                    if let Some(FormItem::CustomField { idx }) =
                        self.form_current_item().cloned()
                    {
                        if idx < self.form_custom.len() {
                            self.form_custom.remove(idx);
                            self.rebuild_form_items();
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn form_nav_next(&mut self) {
        let max = self.form_navigable().len().saturating_sub(1);
        if self.form_nav < max {
            self.form_nav += 1;
        }
    }

    fn form_nav_prev(&mut self) {
        if self.form_nav > 0 {
            self.form_nav -= 1;
        }
    }

    fn handle_confirm_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') => {
                let uid = self.confirm_uid.clone();
                self.archive_card(&uid);
                self.view_mode = ViewMode::Cards;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.view_mode = ViewMode::Cards;
            }
            _ => {}
        }
    }

    // ── Card operations ──────────────────────────────────────────────────────

    fn archive_card(&mut self, uid: &str) {
        match cardstack_lib::repository::load_card(&self.repo, uid) {
            Ok(mut card) => {
                card.fields.insert(
                    "archived".to_string(),
                    serde_json::Value::Bool(true),
                );
                card.fields.insert(
                    "archived_at".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                match save_card(&self.repo, &mut card) {
                    Ok(_) => {
                        self.status = format!("Archived: {}", card.title);
                        self.reload();
                    }
                    Err(e) => self.status = format!("Error: {}", e),
                }
            }
            Err(e) => self.status = format!("Error: {}", e),
        }
    }

    fn add_card_to_deck(&mut self, identifier: &str) -> Result<()> {
        let Some(&deck_data_idx) = self.deck_indices.get(self.current_deck) else {
            self.status = "No deck selected".to_string();
            return Ok(());
        };
        let deck_uid = self.all_cards[deck_data_idx].1.uid.clone();

        let target_card = match cardstack_lib::repository::load_card(&self.repo, identifier) {
            Ok(c) => c,
            Err(_) => {
                self.status = format!("Card not found: {}", identifier);
                return Ok(());
            }
        };

        let mut deck_card =
            cardstack_lib::repository::load_card(&self.repo, &deck_uid)?;
        let facets = deck_card.facets.get_or_insert_with(|| Facets {
            content: None,
            collection: Some(CollectionFacet {
                mode: CollectionMode::Static,
                members: Vec::new(),
                query: None,
                view: None,
            }),
            template: None,
        });
        let col = facets.collection.get_or_insert_with(|| CollectionFacet {
            mode: CollectionMode::Static,
            members: Vec::new(),
            query: None,
            view: None,
        });

        if !col.members.contains(&target_card.uid) {
            col.members.push(target_card.uid.clone());
            deck_card.links.push(cardstack_lib::card::Link {
                r#type: "contains".to_string(),
                to: target_card.uid.clone(),
            });
            save_card(&self.repo, &mut deck_card)?;
            self.status = format!("Added: {}", target_card.title);
            self.reload();
        } else {
            self.status = "Card already in deck".to_string();
        }
        Ok(())
    }

    fn remove_card_from_deck(&mut self) -> Result<()> {
        let Some(&member_data_idx) = self.member_indices.get(self.member_sel) else {
            return Ok(());
        };
        let member_uid = self.all_cards[member_data_idx].1.uid.clone();
        let member_title = self.all_cards[member_data_idx].1.title.clone();

        let Some(&deck_data_idx) = self.deck_indices.get(self.current_deck) else {
            return Ok(());
        };
        let deck_uid = self.all_cards[deck_data_idx].1.uid.clone();
        let mut deck_card =
            cardstack_lib::repository::load_card(&self.repo, &deck_uid)?;

        if let Some(facets) = &mut deck_card.facets {
            if let Some(col) = &mut facets.collection {
                col.members.retain(|m| m != &member_uid);
                deck_card
                    .links
                    .retain(|l| !(l.r#type == "contains" && l.to == member_uid));
            }
        }
        save_card(&self.repo, &mut deck_card)?;
        self.status = format!("Removed: {}", member_title);
        self.reload();
        Ok(())
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run_tui(repo: PathBuf) -> Result<()> {
    let all_cards = load_all_cards(&repo)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(repo, all_cards);
    app.run(&mut terminal)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}
