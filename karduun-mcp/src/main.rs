use karduun_mcp::handlers::ScribeHandler;
use karduun_mcp::{state::ServerState, KarduunMcpServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize server state with current directory as repo root
    let repo_root = std::env::current_dir()?;
    let state = ServerState::new(repo_root);

    // Create MCP server
    let server = KarduunMcpServer::new(state);

    // Register handlers
    server.register_handler("scribe", Box::new(ScribeHandler));

    // Start server
    server.serve("127.0.0.1:8080").await?;

    Ok(())
}
