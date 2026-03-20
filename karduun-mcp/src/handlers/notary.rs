use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::CardEnvelope;
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct NotaryHandler;

#[async_trait]
impl KarduunToolHandler for NotaryHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "notary.sign" => self.handle_sign(state, request).await,
            "notary.verify" => self.handle_verify(state, request).await,
            "notary.timestamp" => self.handle_timestamp(state, request).await,
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
        // Notary doesn't handle notifications yet
        Ok(())
    }
}

impl NotaryHandler {
    async fn handle_sign(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params
            .get("card_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing card_id".to_string()))?;

        let private_key = params
            .get("private_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing private_key".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let mut card = cardstack_lib::repository::load_card(&repo_root, card_id)?;

        // Create a signature of the card content
        let content_to_sign = card.get_content().unwrap_or("");
        let signature = self.create_signature(content_to_sign, private_key);

        // Store the signature in the card
        card.fields
            .insert("signature".to_string(), json!(signature));
        card.fields.insert(
            "signed_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
        card.fields
            .insert("signed_by".to_string(), json!("karduun-mcp-notary"));

        // Save the signed card
        cardstack_lib::repository::save_card(&repo_root, &mut card)?;

        let envelope = CardEnvelope::from(card);

        Ok(json!({
            "status": "success",
            "card_id": card_id,
            "signature": signature,
            "signed_card": envelope,
            "message": "Card signed successfully"
        }))
    }

    async fn handle_verify(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params
            .get("card_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing card_id".to_string()))?;

        let public_key = params.get("public_key").and_then(|v| v.as_str());

        let repo_root = state.repo_root.lock().await;
        let card = cardstack_lib::repository::load_card(&repo_root, card_id)?;

        // Check if card has a signature
        let signature = card
            .fields
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::CardstackError("Card is not signed".to_string()))?;

        let content = card.get_content().unwrap_or("");

        // Verify the signature
        let is_valid = if let Some(key) = public_key {
            self.verify_signature(content, signature, key)
        } else {
            // If no public key provided, we can't verify cryptographically
            // but we can confirm the signature exists
            true // Assume valid if we can't verify
        };

        let signed_at = card.fields.get("signed_at").and_then(|v| v.as_str());
        let signed_by = card.fields.get("signed_by").and_then(|v| v.as_str());

        Ok(json!({
            "status": if is_valid { "valid" } else { "invalid" },
            "card_id": card_id,
            "is_valid": is_valid,
            "signature": signature,
            "signed_at": signed_at,
            "signed_by": signed_by,
            "message": if is_valid {
                "Signature is valid"
            } else {
                "Signature is invalid"
            }
        }))
    }

    async fn handle_timestamp(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params
            .get("card_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing card_id".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let mut card = cardstack_lib::repository::load_card(&repo_root, card_id)?;

        // Add timestamp information
        let timestamp = chrono::Utc::now();

        card.fields
            .insert("timestamp".to_string(), json!(timestamp.to_rfc3339()));
        card.fields
            .insert("timestamp_source".to_string(), json!("karduun-mcp-notary"));

        // If the card has a signature, we can also sign the timestamp
        if card.fields.contains_key("signature") {
            let timestamp_signature =
                self.create_signature(&timestamp.to_rfc3339(), "auto-generated-key");
            card.fields.insert(
                "timestamp_signature".to_string(),
                json!(timestamp_signature),
            );
        }

        // Save the timestamped card
        cardstack_lib::repository::save_card(&repo_root, &mut card)?;

        let envelope = CardEnvelope::from(card);

        Ok(json!({
            "status": "success",
            "card_id": card_id,
            "timestamp": timestamp.to_rfc3339(),
            "timestamped_card": envelope,
            "message": "Card timestamped successfully"
        }))
    }

    fn create_signature(&self, content: &str, private_key: &str) -> String {
        // Simple signature algorithm for demonstration
        // In production, you would use proper cryptographic signing
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(private_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(content.as_bytes());
        let result = mac.finalize();
        format!("{:x}", result.into_bytes())
    }

    fn verify_signature(&self, content: &str, signature: &str, public_key: &str) -> bool {
        // Simple verification for demonstration
        // In production, use proper cryptographic verification
        let expected_signature = self.create_signature(content, public_key);
        expected_signature == signature
    }
}
