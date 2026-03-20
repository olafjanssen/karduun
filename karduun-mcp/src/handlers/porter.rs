use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::{Card, CardEnvelope};
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct PorterHandler;

#[async_trait]
impl KarduunToolHandler for PorterHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "porter.export" => self.handle_export(state, request).await,
            "porter.import" => self.handle_import(state, request).await,
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
        // Porter doesn't handle notifications yet
        Ok(())
    }
}

impl PorterHandler {
    async fn handle_export(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");
        let query = params.get("query").and_then(|v| v.as_str());
        let card_ids = params.get("card_ids").and_then(|v| v.as_array());

        let repo_root = state.repo_root.lock().await;
        let all_cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Filter cards based on query or card_ids
        let cards_to_export: Vec<_> = if let Some(ids) = card_ids {
            // Filter by specific card IDs
            ids.iter()
                .filter_map(|id| id.as_str())
                .filter_map(|id| {
                    all_cards
                        .iter()
                        .find(|(_, card)| &card.uid == id)
                        .map(|(_, card)| card)
                })
                .cloned()
                .collect()
        } else if let Some(query_str) = query {
            // Filter by query
            all_cards
                .into_iter()
                .filter(|(_, card)| {
                    card.title
                        .to_lowercase()
                        .contains(&query_str.to_lowercase())
                        || card.get_content().map_or(false, |content| {
                            content.to_lowercase().contains(&query_str.to_lowercase())
                        })
                })
                .map(|(_, card)| card)
                .collect()
        } else {
            // Export all cards
            all_cards.into_iter().map(|(_, card)| card).collect()
        };

        match format {
            "json" => {
                let envelopes: Vec<_> = cards_to_export
                    .into_iter()
                    .map(CardEnvelope::from)
                    .collect();

                Ok(json!({
                    "status": "success",
                    "format": "json",
                    "exported_cards": envelopes.len(),
                    "data": envelopes,
                    "message": format!("Exported {} cards in JSON format", envelopes.len())
                }))
            }
            "jsonl" => {
                let jsonl_lines: Vec<String> = cards_to_export
                    .into_iter()
                    .map(|card| {
                        let envelope = CardEnvelope::from(card);
                        serde_json::to_string(&envelope).unwrap_or_default()
                    })
                    .collect();

                let jsonl_data = jsonl_lines.join("\n");

                Ok(json!({
                    "status": "success",
                    "format": "jsonl",
                    "exported_cards": jsonl_lines.len(),
                    "data": jsonl_data,
                    "message": format!("Exported {} cards in JSONL format", jsonl_lines.len())
                }))
            }
            "markdown" => {
                let cards_to_export_len = cards_to_export.len();
                let markdown_output = cards_to_export
                    .into_iter()
                    .map(|card| {
                        format!("# {}", card.title)
                            + &format!("\n\n**UID**: {}", card.uid)
                            + &format!("\n**Slug**: {}", card.slug)
                            + &if !card.tags.is_empty() {
                                format!("\n**Tags**: {}", card.tags.join(", "))
                            } else {
                                String::new()
                            }
                            + &if let Some(content) = card.get_content() {
                                format!("\n\n---\n\n{}", content)
                            } else {
                                String::new()
                            }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n");

                Ok(json!({
                    "status": "success",
                    "format": "markdown",
                    "exported_cards": cards_to_export_len,
                    "data": markdown_output,
                    "message": format!("Exported {} cards in Markdown format", cards_to_export_len)
                }))
            }
            _ => Err(KarduunMcpError::InvalidRequest(format!(
                "Unsupported export format: {}",
                format
            ))),
        }
    }

    async fn handle_import(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");
        let data = params
            .get("data")
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing data to import".to_string()))?;

        let repo_root = state.repo_root.lock().await;

        let cards_to_import = match format {
            "json" => {
                // Expect an array of card envelopes or cards
                if let Some(array) = data.as_array() {
                    array
                        .iter()
                        .filter_map(|value| {
                            if let Ok(envelope) =
                                serde_json::from_value::<CardEnvelope>(value.clone())
                            {
                                Some(Card {
                                    kind: "card".to_string(),
                                    schema: 1,
                                    uid: envelope.uid.clone(),
                                    slug: envelope.slug.clone(),
                                    title: envelope.title.clone(),
                                    author: None,
                                    created: envelope.created,
                                    updated: envelope.updated,
                                    version: 1,
                                    tags: envelope.tags.clone(),
                                    keywords: Vec::new(),
                                    fields: envelope.fields.clone(),
                                    links: envelope.links_out.clone(),
                                    facets: None,
                                    sign: None,
                                    publications: Vec::new(),
                                    computed: None,
                                })
                            } else if let Ok(card) = serde_json::from_value::<Card>(value.clone()) {
                                Some(card)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    // Try single card
                    if let Ok(envelope) = serde_json::from_value::<CardEnvelope>(data.clone()) {
                        vec![Card {
                            kind: "card".to_string(),
                            schema: 1,
                            uid: envelope.uid.clone(),
                            slug: envelope.slug.clone(),
                            title: envelope.title.clone(),
                            author: None,
                            created: envelope.created,
                            updated: envelope.updated,
                            version: 1,
                            tags: envelope.tags.clone(),
                            keywords: Vec::new(),
                            fields: envelope.fields.clone(),
                            links: envelope.links_out.clone(),
                            facets: None,
                            sign: None,
                            publications: Vec::new(),
                            computed: None,
                        }]
                    } else if let Ok(card) = serde_json::from_value::<Card>(data.clone()) {
                        vec![card]
                    } else {
                        return Err(KarduunMcpError::InvalidRequest(
                            "Invalid card data format".to_string(),
                        ));
                    }
                }
            }
            "jsonl" => {
                // Expect a string with JSONL data
                if let Some(jsonl_string) = data.as_str() {
                    jsonl_string
                        .lines()
                        .filter_map(|line| {
                            if line.trim().is_empty() {
                                return None;
                            }
                            if let Ok(envelope) = serde_json::from_str::<CardEnvelope>(line) {
                                Some(Card {
                                    kind: "card".to_string(),
                                    schema: 1,
                                    uid: envelope.uid.clone(),
                                    slug: envelope.slug.clone(),
                                    title: envelope.title.clone(),
                                    author: None,
                                    created: envelope.created,
                                    updated: envelope.updated,
                                    version: 1,
                                    tags: envelope.tags.clone(),
                                    keywords: Vec::new(),
                                    fields: envelope.fields.clone(),
                                    links: envelope.links_out.clone(),
                                    facets: None,
                                    sign: None,
                                    publications: Vec::new(),
                                    computed: None,
                                })
                            } else if let Ok(card) = serde_json::from_str::<Card>(line) {
                                Some(card)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    return Err(KarduunMcpError::InvalidRequest(
                        "JSONL data should be a string".to_string(),
                    ));
                }
            }
            _ => {
                return Err(KarduunMcpError::InvalidRequest(format!(
                    "Unsupported import format: {}",
                    format
                )))
            }
        };

        let mut imported_count = 0;
        let skipped_count = 0;
        let mut error_count = 0;

        for mut card in cards_to_import {
            // Generate UID if not present
            if card.uid.is_empty() {
                card.uid = cardstack_lib::uid::generate_uid();
            }

            // Generate slug if not present
            if card.slug.is_empty() {
                card.slug = card
                    .title
                    .to_lowercase()
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .collect();
            }

            // Save the card
            match cardstack_lib::repository::save_card(&repo_root, &mut card) {
                Ok(_) => imported_count += 1,
                Err(e) => {
                    error_count += 1;
                    eprintln!("Failed to import card {}: {}", card.uid, e);
                }
            }
        }

        Ok(json!({
            "status": "completed",
            "format": format,
            "imported_cards": imported_count,
            "skipped_cards": skipped_count,
            "error_count": error_count,
            "total_processed": imported_count + skipped_count + error_count,
            "message": format!(
                "Import completed: {} cards imported, {} skipped, {} errors",
                imported_count, skipped_count, error_count
            )
        }))
    }
}
