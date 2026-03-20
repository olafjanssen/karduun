use crate::error::KarduunMcpError;
use crate::state::ServerState;
use async_trait::async_trait;
use mcp_sdk_rs::protocol::RequestId;
use mcp_sdk_rs::server::ServerHandler;
use mcp_sdk_rs::{
    ClientCapabilities, Error, Implementation, Notification, Request, ServerCapabilities,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct KarduunMcpServer {
    state: ServerState,
    handlers: Arc<Mutex<HashMap<String, Arc<dyn KarduunToolHandler + Send + Sync>>>>,
}

impl KarduunMcpServer {
    pub fn new(state: ServerState) -> Self {
        Self {
            state,
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_handler(
        &self,
        tool_name: &str,
        handler: Box<dyn KarduunToolHandler + Send + Sync>,
    ) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(tool_name.to_string(), Arc::from(handler));
    }

    pub async fn serve(&self, addr: &str) -> Result<(), KarduunMcpError> {
        use std::sync::Arc;
        use tokio_tungstenite::accept_async;

        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Karduun MCP Server (WebSocket) listening on ws://{}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let server_handler = self.clone();

            tokio::spawn(async move {
                // Perform WebSocket handshake
                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        eprintln!("WebSocket handshake error: {}", e);
                        return;
                    }
                };

                // Create WebSocket transport
                let transport =
                    mcp_sdk_rs::transport::websocket::WebSocketTransport::from_stream(ws_stream);

                // Create MCP server with transport and handler
                let server =
                    mcp_sdk_rs::server::Server::new(Arc::new(transport), Arc::new(server_handler));

                // Start the server for this connection
                if let Err(e) = server.start().await {
                    eprintln!("WebSocket connection error: {}", e);
                }
            });
        }
    }

    pub async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, Error> {
        let tool_name = method.split('.').next().unwrap_or("");

        // Create a simple request-like structure for our handlers
        let simple_request = Request {
            id: RequestId::Number(0),
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        // Simple approach: execute handler while holding the lock
        // Get handler while holding the lock
        let handler = {
            let handlers = self.handlers.lock().unwrap();
            handlers.get(tool_name).cloned()
        };

        // Execute handler outside the lock
        if let Some(handler) = handler {
            let state = self.state.clone();
            // Convert our error type to the MCP SDK error type
            handler
                .handle_request(&state, simple_request)
                .await
                .map_err(|e| {
                    Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InternalError,
                        format!("Handler error: {}", e),
                    )
                })
        } else {
            Err(Error::protocol(
                mcp_sdk_rs::error::ErrorCode::MethodNotFound,
                format!("Method {} not found", method),
            ))
        }
    }
}

impl Clone for KarduunMcpServer {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            handlers: self.handlers.clone(),
        }
    }
}

#[async_trait]
pub trait KarduunToolHandler: Send + Sync {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError>;
    async fn handle_notification(
        &self,
        state: &ServerState,
        notification: Notification,
    ) -> Result<(), KarduunMcpError>;
}

#[async_trait]
impl ServerHandler for KarduunMcpServer {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, Error> {
        Ok(ServerCapabilities::default())
    }

    async fn shutdown(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, Error> {
        self.handle_method(method, params).await
    }
}
