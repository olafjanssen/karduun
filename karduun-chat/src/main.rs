use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use serde_json::{json, Value};
use std::io;
use tokio::sync::mpsc;

mod mcp_client;
use crate::mcp_client::send_mcp_request;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// MCP Server URL (default: ws://127.0.0.1:8080)
    #[arg(short, long, default_value = "ws://127.0.0.1:8080")]
    server: String,
}

#[derive(Debug, Clone)]
struct Message {
    content: String,
    is_user: bool,
}

struct App {
    messages: Vec<Message>,
    input: String,
    server_url: String,
    response_receiver: Option<mpsc::Receiver<Result<Value, String>>>,
}

impl App {
    fn new(server_url: String) -> Self {
        Self {
            messages: vec![Message {
                content: "Welcome to Karduun Chat! Type your MCP requests...\n\nExample: scribe.new {\"title\": \"My Card\", \"slug\": \"my-card\"}".to_string(),
                is_user: false,
            }],
            input: String::new(),
            server_url,
            response_receiver: None,
        }
    }

    async fn process_input(&mut self) -> Option<mpsc::Sender<Result<Value, String>>> {
        if self.input.trim().is_empty() {
            return None;
        }

        // Add user message
        self.messages.push(Message {
            content: self.input.clone(),
            is_user: true,
        });

        // Create channel for response
        let (sender, receiver) = mpsc::channel(1);
        self.response_receiver = Some(receiver);

        let input = self.input.clone();
        self.input.clear();

        // Start task to handle MCP request
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            let result = parse_and_execute_mcp(&input).await;
            let _ = sender_clone.send(result).await;
        });

        Some(sender)
    }

    fn check_response(&mut self) {
        if let Some(receiver) = &mut self.response_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(response) => {
                        self.messages.push(Message {
                            content: format!("Success: {}", response),
                            is_user: false,
                        });
                    }
                    Err(error) => {
                        self.messages.push(Message {
                            content: format!("Error: {}", error),
                            is_user: false,
                        });
                    }
                }
                self.response_receiver = None;
            }
        }
    }
}

async fn parse_and_execute_mcp(input: &str) -> Result<Value, String> {
    // Simple parsing: method.name params
    let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();

    if parts.len() < 1 {
        return Err("No method specified".to_string());
    }

    let method = parts[0];
    let params = if parts.len() > 1 {
        parts[1].trim()
    } else {
        "{}"
    };

    // Parse params as JSON
    let params_json: Value =
        serde_json::from_str(params).map_err(|e| format!("Invalid JSON params: {}", e))?;

    send_mcp_request(method, params_json).await
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(args.server);

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Check for responses
        app.check_response();

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            let _ = app.process_input().await;
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]);

    let [messages_area, input_area] = vertical.areas(frame.size());

    // Messages
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|msg| {
            let content = Line::from(msg.content.clone());
            if msg.is_user {
                ListItem::new(content).style(Style::default().fg(Color::Green))
            } else {
                ListItem::new(content).style(Style::default().fg(Color::Blue))
            }
        })
        .collect();

    let messages_list =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Karduun Chat"));

    frame.render_widget(messages_list, messages_area);

    // Input
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Input"));

    frame.render_widget(input, input_area);

    // Set cursor position
    frame.set_cursor(input_area.x + app.input.len() as u16 + 1, input_area.y + 1);
}
