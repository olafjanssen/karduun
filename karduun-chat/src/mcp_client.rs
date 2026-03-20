use mcp_sdk_rs::client::{Client, Session};
use mcp_sdk_rs::transport::websocket::WebSocketTransport;
use mcp_sdk_rs::types::Implementation;
use serde_json::Value;
use std::sync::Arc;

pub async fn send_mcp_request(method: &str, params: Value) -> Result<Value, String> {
    // Create WebSocket transport
    let transport = match WebSocketTransport::new("ws://127.0.0.1:8080").await {
        Ok(transport) => Arc::new(transport),
        Err(e) => return Err(format!("Transport error: {}", e)),
    };

    // Create mpsc communication channels
    let (request_tx, response_rx) = tokio::sync::mpsc::unbounded_channel();
    let (response_tx, request_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create a session and start listening for requests and notifications
    let session = Session::new(transport, response_tx, request_rx, None);
    if let Err(e) = session.start().await {
        return Err(format!("Session error: {}", e));
    }

    // Create MCP client
    let client = Client::new(request_tx, response_rx);

    // Initialize client
    let implementation = Implementation {
        name: "karduun-chat".to_string(),
        version: "0.1.0".to_string(),
    };

    match client.initialize(implementation, None).await {
        Ok(_) => {}
        Err(e) => return Err(format!("Initialization error: {}", e)),
    };

    // Send request
    match client.request(&method.to_string(), Some(params)).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("MCP request error: {}", e)),
    }
}
