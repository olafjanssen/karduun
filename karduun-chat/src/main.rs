use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use serde_json::Value;
use std::io;
use tokio::sync::mpsc;

mod llm;
mod mcp_client;
use crate::llm::{create_mcp_prompt, extract_mcp_commands, simple_nl_to_mcp, LLMBackend};
use crate::mcp_client::send_mcp_request;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// MCP Server URL (default: ws://127.0.0.1:8080)
    #[arg(short, long, default_value = "ws://127.0.0.1:8080")]
    server: String,

    /// LLM Backend (ollama, openai, mistral, none)
    #[arg(short, long, default_value = "ollama")]
    llm: String,

    /// LLM Model (default: llama3 for Ollama)
    #[arg(short, long, default_value = "llama3")]
    model: String,

    /// Ollama URL (default: http://localhost:11434)
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// OpenAI API Key (if using OpenAI)
    #[arg(long)]
    openai_key: Option<String>,
}

#[derive(Debug, Clone)]
struct Message {
    content: String,
    is_user: bool,
}

struct App {
    messages: Vec<Message>,
    input: String,
    llm_backend: LLMBackend,
    response_receiver: Option<mpsc::Receiver<Result<Value, String>>>,
    llm_response_receiver: Option<mpsc::Receiver<Result<String, String>>>,
}

impl App {
    fn new(llm_backend: LLMBackend) -> Self {
        Self {
            messages: vec![Message {
                content: format!(
                    "Welcome to Karduun Chat with LLM support! 🚀\n\n{}\n\nOptions:\n- Type natural language queries (LLM will translate to MCP commands)\n- Type direct MCP commands (e.g., scribe.new {{title: \"My Card\"}})\n- Press ESC, 'q', or 'Q' to quit",
                    if matches!(llm_backend, LLMBackend::None) {
                        "LLM backend not configured - using direct MCP mode"
                    } else {
                        "Powered by LLM backend - try natural language!"
                    }
                ),
                is_user: false,
            }],
            input: String::new(),
            llm_backend,
            response_receiver: None,
            llm_response_receiver: None,
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

        let input = self.input.clone();
        self.input.clear();

        // Check if input looks like an MCP command
        if input
            .trim_start()
            .starts_with(|c: char| c.is_ascii_alphabetic())
            && input.contains('.')
            && (input.contains('{') || input.contains('['))
        {
            // Direct MCP command
            let (sender, receiver) = mpsc::channel(1);
            self.response_receiver = Some(receiver);

            // Clone sender for the async task
            let task_sender = sender.clone();
            tokio::spawn(async move {
                let result = parse_and_execute_mcp(&input).await;
                let _ = task_sender.send(result).await;
            });

            return Some(sender);
        } else {
            // Use LLM to process natural language
            self.process_with_llm(input).await;
            return None;
        }
    }

    async fn process_with_llm(&mut self, input: String) {
        // Add thinking message
        self.messages.push(Message {
            content: "🤖 Thinking... (using LLM)".to_string(),
            is_user: false,
        });

        let llm_backend = self.llm_backend.clone();
        let (llm_sender, llm_receiver) = mpsc::channel(1);
        self.llm_response_receiver = Some(llm_receiver);

        tokio::spawn(async move {
            // Try simple NL to MCP mapping first
            if let Some(mcp_command) = simple_nl_to_mcp(&input) {
                let result = parse_and_execute_mcp(&mcp_command).await;
                match result {
                    Ok(response) => {
                        let _ = llm_sender
                            .send(Ok(format!(
                                "Executed: {}\n\nResult: {}",
                                mcp_command, response
                            )))
                            .await;
                    }
                    Err(error) => {
                        let _ = llm_sender.send(Err(error)).await;
                    }
                }
                return;
            }

            // Use LLM to generate response
            let prompt = create_mcp_prompt(&input);
            match llm_backend.generate(&prompt).await {
                Ok(llm_response) => {
                    // Extract MCP commands from LLM response
                    let commands = extract_mcp_commands(&llm_response);

                    if commands.is_empty() {
                        // No MCP commands, just show LLM response
                        let _ = llm_sender.send(Ok(llm_response)).await;
                    } else {
                        // Execute MCP commands
                        let mut combined_result = String::new();
                        combined_result.push_str(&format!("LLM Response:\n{}\n\n", llm_response));
                        combined_result.push_str("Executing MCP commands:\n");

                        for command in commands {
                            combined_result.push_str(&format!("- {}\n", command));
                            match parse_and_execute_mcp(&command).await {
                                Ok(response) => {
                                    combined_result.push_str(&format!("  Result: {}\n", response));
                                }
                                Err(error) => {
                                    combined_result.push_str(&format!("  Error: {}\n", error));
                                }
                            }
                        }

                        let _ = llm_sender.send(Ok(combined_result)).await;
                    }
                }
                Err(error) => {
                    let _ = llm_sender.send(Err(error)).await;
                }
            }
        });
    }

    fn check_response(&mut self) {
        // Check MCP responses
        if let Some(receiver) = &mut self.response_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(response) => {
                        self.messages.push(Message {
                            content: format!("✅ Success: {}", response),
                            is_user: false,
                        });
                    }
                    Err(error) => {
                        self.messages.push(Message {
                            content: format!("❌ Error: {}", error),
                            is_user: false,
                        });
                    }
                }
                self.response_receiver = None;
            }
        }

        // Check LLM responses
        if let Some(receiver) = &mut self.llm_response_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(response) => {
                        self.messages.push(Message {
                            content: format!("🤖 LLM: {}", response),
                            is_user: false,
                        });
                    }
                    Err(error) => {
                        self.messages.push(Message {
                            content: format!("⚠️  LLM Error: {}", error),
                            is_user: false,
                        });
                    }
                }
                self.llm_response_receiver = None;
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

    // Send MCP request using SDK
    send_mcp_request(method, params_json).await
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // Initialize LLM backend using the llm crate
    let llm_backend = LLMBackend::from_args(
        &args.llm,
        &args.model,
        &args.ollama_url,
        args.openai_key.as_deref(),
    );

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(llm_backend);

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
