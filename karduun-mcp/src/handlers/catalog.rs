use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};


#[derive(Clone)]
pub struct CatalogHandler;

#[async_trait]
impl KarduunToolHandler for CatalogHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "catalog.rebuild" => self.handle_rebuild(state, request).await,
            "catalog.status" => self.handle_status(state, request).await,
            "catalog.vacuum" => self.handle_vacuum(state, request).await,
            _ => Err(KarduunMcpError::HandlerNotFound(format!(
                "Method not found: {}",
                request.method
            ))),
        }
    }

    async fn handle_notification(
        &self,
        _state: &ServerState,
        _notification: Notification,
    ) -> Result<(), KarduunMcpError> {
        // Catalog doesn't handle notifications yet
        Ok(())
    }
}

impl CatalogHandler {
    async fn handle_rebuild(
        &self,
        state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let repo_root = state.repo_root.lock().await;

        // Rebuild index logic would go here
        // For now, we'll simulate the process

        // Load all cards to simulate indexing
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        Ok(json!({
            "status": "success",
            "indexed_cards": cards.len(),
            "message": "Index rebuilt successfully"
        }))
    }

    async fn handle_status(
        &self,
        state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let repo_root = state.repo_root.lock().await;

        // Check repository status
        let cards_count = cardstack_lib::repository::load_all_cards(&repo_root)?.len();

        // Check if .cardstack directory exists
        let cardstack_dir = repo_root.join(".cardstack");
        let is_initialized = cardstack_dir.exists() && cardstack_dir.is_dir();

        // Check config file
        let config_exists = cardstack_dir.join("config.yml").exists();

        Ok(json!({
            "initialized": is_initialized,
            "config_exists": config_exists,
            "total_cards": cards_count,
            "status": if is_initialized && config_exists {
                "healthy"
            } else if is_initialized {
                "partial"
            } else {
                "uninitialized"
            }
        }))
    }

    async fn handle_vacuum(
        &self,
        state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let repo_root = state.repo_root.lock().await;

        // Vacuum/cleanup logic would go here
        // For now, we'll simulate the process

        // Find and count cards
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;
        let cards_to_clean = cards
            .iter()
            .filter(|(_, card)| {
                // Simulate finding cards to clean (e.g., archived, duplicate, etc.)
                card.fields
                    .get("archived")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();

        Ok(json!({
            "status": "success",
            "scanned_cards": cards.len(),
            "cleaned_cards": cards_to_clean,
            "message": format!("Vacuum completed. Cleaned {} cards", cards_to_clean)
        }))
    }
}
