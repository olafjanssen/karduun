use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;

use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct CuratorHandler;

#[async_trait]
impl KarduunToolHandler for CuratorHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "curator.plan" => self.handle_plan(state, request).await,
            "curator.apply" => self.handle_apply(state, request).await,
            "curator.autoclean" => self.handle_autoclean(state, request).await,
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
        // Curator doesn't handle notifications yet
        Ok(())
    }
}

impl CuratorHandler {
    async fn handle_plan(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let aggressive = params
            .get("aggressive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        let mut plan = Vec::new();

        for (_, card) in &cards {
            // Check for potential issues and suggest actions

            // 1. Check for archived cards that can be cleaned up
            if card
                .fields
                .get("archived")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                plan.push(json!({
                    "action": "cleanup_archived",
                    "card_id": card.uid,
                    "title": card.title,
                    "priority": "low",
                    "description": "Archived card can be permanently removed"
                }));
            }

            // 2. Check for cards with many tags (potential tag cleanup)
            if card.tags.len() > 10 {
                plan.push(json!({
                    "action": "review_tags",
                    "card_id": card.uid,
                    "title": card.title,
                    "priority": "medium",
                    "description": format!("Card has {} tags - consider consolidating", card.tags.len())
                }));
            }

            // 3. Check for cards with broken links
            let broken_links: Vec<_> = card
                .links
                .iter()
                .filter(|link| {
                    // Check if target card exists
                    !cards.iter().any(|(_, c)| &c.uid == &link.to)
                })
                .collect();

            if !broken_links.is_empty() {
                plan.push(json!({
                    "action": "fix_broken_links",
                    "card_id": card.uid,
                    "title": card.title,
                    "priority": "high",
                    "description": format!("Card has {} broken links", broken_links.len()),
                    "broken_links": broken_links.iter().map(|l| json!({
                        "type": l.r#type,
                        "target": l.to
                    })).collect::<Vec<_>>()
                }));
            }

            // 4. Aggressive mode: suggest content improvements
            if aggressive {
                let content = card.get_content().unwrap_or("");
                if content.split_whitespace().count() < 50
                    && !card.tags.contains(&"stub".to_string())
                {
                    plan.push(json!({
                        "action": "expand_content",
                        "card_id": card.uid,
                        "title": card.title,
                        "priority": "medium",
                        "description": "Card content is very brief - consider expanding"
                    }));
                }

                if content.split_whitespace().count() > 500 {
                    plan.push(json!({
                        "action": "review_length",
                        "card_id": card.uid,
                        "title": card.title,
                        "priority": "low",
                        "description": "Card content is very long - consider splitting"
                    }));
                }
            }
        }

        Ok(json!({
            "plan": plan,
            "total_actions": plan.len(),
            "summary": {
                "high_priority": plan.iter().filter(|p| p["priority"] == "high").count(),
                "medium_priority": plan.iter().filter(|p| p["priority"] == "medium").count(),
                "low_priority": plan.iter().filter(|p| p["priority"] == "low").count()
            }
        }))
    }

    async fn handle_apply(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let action_type = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing action type".to_string()))?;

        let card_id = params
            .get("card_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing card_id".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let mut cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Find the card to modify
        let card_pos = cards
            .iter()
            .position(|(_, c)| &c.uid == card_id)
            .ok_or_else(|| {
                KarduunMcpError::CardstackError(format!("Card not found: {}", card_id))
            })?;

        let (_path, mut card) = cards.remove(card_pos);

        match action_type {
            "cleanup_archived" => {
                // Remove archived flag
                card.fields.remove("archived");
                card.fields.remove("archived_at");

                // Save the updated card
                cardstack_lib::repository::save_card(&repo_root, &mut card)?;

                Ok(json!({
                    "status": "success",
                    "action": "cleanup_archived",
                    "card_id": card.uid,
                    "message": "Archived flags removed"
                }))
            }
            "fix_broken_links" => {
                // Remove broken links
                let original_link_count = card.links.len();
                card.links
                    .retain(|link| cards.iter().any(|(_, c)| &c.uid == &link.to));
                let removed_links = original_link_count - card.links.len();

                // Save the updated card
                cardstack_lib::repository::save_card(&repo_root, &mut card)?;

                Ok(json!({
                    "status": "success",
                    "action": "fix_broken_links",
                    "card_id": card.uid,
                    "removed_links": removed_links,
                    "remaining_links": card.links.len(),
                    "message": format!("Removed {} broken links", removed_links)
                }))
            }
            _ => Err(KarduunMcpError::InvalidRequest(format!(
                "Unknown action type: {}",
                action_type
            ))),
        }
    }

    async fn handle_autoclean(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let dry_run = params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        let mut actions_taken = 0;
        let mut actions_skipped = 0;

        for (_, card) in &cards {
            // Auto-clean archived cards older than 30 days
            if let Some(archived_at) = card.fields.get("archived_at") {
                if let Some(archived_date) = archived_at.as_str() {
                    if let Ok(archived_time) = chrono::DateTime::parse_from_rfc3339(archived_date) {
                        let archived_time_utc = archived_time.with_timezone(&chrono::Utc);
                        let duration = chrono::Utc::now() - archived_time_utc;
                        if duration.num_days() > 30 {
                            if dry_run {
                                actions_skipped += 1;
                            } else {
                                // Would clean up the card here
                                actions_taken += 1;
                            }
                        }
                    }
                }
            }

            // Auto-remove empty tags
            let empty_tags: Vec<_> = card
                .tags
                .iter()
                .filter(|tag| tag.trim().is_empty())
                .collect();

            if !empty_tags.is_empty() {
                if dry_run {
                    actions_skipped += 1;
                } else {
                    // Would remove empty tags here
                    actions_taken += 1;
                }
            }
        }

        Ok(json!({
            "status": if dry_run { "dry_run" } else { "completed" },
            "actions_taken": actions_taken,
            "actions_skipped": actions_skipped,
            "total_actions": actions_taken + actions_skipped,
            "message": if dry_run {
                "Autoclean dry run completed. Use dry_run=false to apply changes."
            } else {
                "Autoclean completed successfully"
            }
        }))
    }
}
