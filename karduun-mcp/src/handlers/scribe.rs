use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::{Card, CardEnvelope};
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct ScribeHandler;

#[async_trait]
impl KarduunToolHandler for ScribeHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "scribe.new" => self.handle_new(state, request).await,
            "scribe.show" => self.handle_show(state, request).await,
            "scribe.edit" => self.handle_edit(state, request).await,
            "scribe.archive" => self.handle_archive(state, request).await,
            "scribe.fork" => self.handle_fork(state, request).await,
            "scribe.merge" => self.handle_merge(state, request).await,
            "scribe.link" => self.handle_link(state, request).await,
            "scribe.unlink" => self.handle_unlink(state, request).await,
            "scribe.deck.new" => self.handle_deck_new(state, request).await,
            "scribe.deck.show" => self.handle_deck_show(state, request).await,
            "scribe.deck.add" => self.handle_deck_add(state, request).await,
            "scribe.deck.remove" => self.handle_deck_remove(state, request).await,
            "scribe.deck.snapshot" => self.handle_deck_snapshot(state, request).await,
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
        // Scribe doesn't handle notifications yet
        Ok(())
    }
}

impl ScribeHandler {
    async fn handle_new(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing title".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let mut card = Card::new(
            title.to_string(),
            params
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            cardstack_lib::uid::generate_uid(),
        );

        let _card_path = cardstack_lib::repository::save_card(&repo_root, &mut card)?;
        let envelope = CardEnvelope::from(card);

        Ok(json!(envelope))
    }

    async fn handle_show(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let identifier = params
            .get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing identifier".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let card = cardstack_lib::repository::load_card(&repo_root, identifier)?;
        let envelope = CardEnvelope::from(card);

        Ok(json!(envelope))
    }

    // Implement other handler methods...
    async fn handle_edit(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "edit not implemented".to_string(),
        ))
    }

    async fn handle_archive(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "archive not implemented".to_string(),
        ))
    }

    async fn handle_fork(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "fork not implemented".to_string(),
        ))
    }

    async fn handle_merge(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "merge not implemented".to_string(),
        ))
    }

    async fn handle_link(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "link not implemented".to_string(),
        ))
    }

    async fn handle_unlink(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "unlink not implemented".to_string(),
        ))
    }

    async fn handle_deck_new(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "deck.new not implemented".to_string(),
        ))
    }

    async fn handle_deck_show(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "deck.show not implemented".to_string(),
        ))
    }

    async fn handle_deck_add(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "deck.add not implemented".to_string(),
        ))
    }

    async fn handle_deck_remove(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "deck.remove not implemented".to_string(),
        ))
    }

    async fn handle_deck_snapshot(
        &self,
        _state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        Err(KarduunMcpError::HandlerNotFound(
            "deck.snapshot not implemented".to_string(),
        ))
    }
}
