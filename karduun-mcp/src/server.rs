use crate::error::KarduunMcpError;
use crate::state::ServerState;
use async_trait::async_trait;
use mcp_sdk_rs::error::ErrorCode;
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
    handlers: Arc<Mutex<HashMap<String, Box<dyn KarduunToolHandler + Send + Sync>>>>,
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
        handlers.insert(tool_name.to_string(), handler);
    }

    pub async fn serve(&self, addr: &str) -> Result<(), KarduunMcpError> {
        // For now, implement a simple TCP server
        // TODO: Use proper MCP transport layer
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Karduun MCP Server listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await?;
            let server = self.clone();

            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(socket).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        &self,
        _socket: tokio::net::TcpStream,
    ) -> Result<(), KarduunMcpError> {
        // TODO: Implement proper MCP protocol handling
        Ok(())
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
        let tool_name = method.split('.').next().unwrap_or("");

        // Create a simple request-like structure for our handlers
        let simple_request = Request {
            id: RequestId::Number(0),
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        // Simple approach: execute handler while holding the lock
        // This is not ideal for performance but avoids complex lifetime issues
        let handlers = self.handlers.lock().unwrap();
        if let Some(handler) = handlers.get(tool_name) {
            // Execute handler synchronously within the lock
            let state = self.state.clone();
            let simple_request = simple_request.clone();

            // Use block_on to execute the async handler
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { handler.handle_request(&state, simple_request).await })
            });

            result.map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))
        } else {
            Err(Error::protocol(
                ErrorCode::MethodNotFound,
                format!("Method not found: {}", method),
            ))
        }
    }
}
