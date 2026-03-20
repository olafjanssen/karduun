use karduun_mcp::handlers::{
    CatalogHandler, CuratorHandler, EcoHandler, GaugeHandler, NotaryHandler, PorterHandler,
    ScoutHandler, ScribeHandler, StencilHandler,
};
use karduun_mcp::{state::ServerState, KarduunMcpServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize server state with current directory as repo root
    let repo_root = std::env::current_dir()?;
    let state = ServerState::new(repo_root);

    // Create MCP server
    let server = KarduunMcpServer::new(state);

    // Register all handlers
    server.register_handler("catalog", Box::new(CatalogHandler));
    server.register_handler("curator", Box::new(CuratorHandler));
    server.register_handler("eco", Box::new(EcoHandler));
    server.register_handler("gauge", Box::new(GaugeHandler));
    server.register_handler("notary", Box::new(NotaryHandler));
    server.register_handler("porter", Box::new(PorterHandler));
    server.register_handler("scribe", Box::new(ScribeHandler));
    server.register_handler("scout", Box::new(ScoutHandler));
    server.register_handler("stencil", Box::new(StencilHandler));

    // Start server
    server.serve("127.0.0.1:8080").await?;

    Ok(())
}
