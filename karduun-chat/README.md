# Karduun Chat - MCP Client TUI

A simple Terminal User Interface (TUI) client for interacting with the Karduun MCP Server.

## Features

- **Minimal TUI Interface**: Built with `ratatui` for a clean, responsive terminal experience
- **MCP Protocol Support**: Connects to Karduun MCP Server using JSON-RPC 2.0
- **Simple Request Parsing**: Easy-to-use syntax for MCP method calls
- **Color-coded Messages**: User messages in green, server responses in blue
- **Real-time Interaction**: Instant feedback and response handling

## Installation

The chat client is part of the Karduun workspace. Build it with:

```bash
cargo build --bin karduun-chat
```

## Usage

### Basic Usage

```bash
cargo run --bin karduun-chat
```

### Connect to Specific Server

```bash
cargo run --bin karduun-chat -- --server http://localhost:8080
```

## Interface

- **Input Area**: Bottom of the screen for typing commands
- **Message History**: Top area showing conversation
- **Navigation**: 
  - `Enter`: Send current input
  - `Backspace`: Delete characters
  - `ESC`: Exit the application

## Command Syntax

The chat client uses a simple syntax for MCP requests:

```
method_name {"param1": "value1", "param2": "value2"}
```

### Examples

```
# Create a new card
scribe.new {"title": "My Card", "slug": "my-card"}

# List cards
scout.list {}

# Get card details
scribe.show {"card_id": "card-uid-here"}

# Search cards
scout.grep {"query": "search term"}
```

## Architecture

- **TUI Framework**: `ratatui` for terminal rendering
- **HTTP Client**: `reqwest` for MCP server communication
- **Async Runtime**: `tokio` for asynchronous operations
- **JSON Handling**: `serde_json` for request/response parsing
- **Terminal Control**: `crossterm` for terminal management

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
ratatui = "0.26.1"
crossterm = "0.27.0"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
```

## Future Enhancements

- Command history and navigation
- Syntax highlighting for JSON
- Auto-completion for method names
- Multi-line input support
- Connection status indicator
- LLM integration for natural language to MCP translation

## License

MIT License - See the main Karduun project for details.