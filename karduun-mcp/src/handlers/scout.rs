use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::{Card, CardEnvelope};
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct ScoutHandler;

#[async_trait]
impl KarduunToolHandler for ScoutHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "scout.list" => self.handle_list(state, request).await,
            "scout.grep" => self.handle_grep(state, request).await,
            "scout.backlinks" => self.handle_backlinks(state, request).await,
            "scout.tree" => self.handle_tree(state, request).await,
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
        // Scout doesn't handle notifications yet
        Ok(())
    }
}

impl ScoutHandler {
    async fn handle_list(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let query = params.get("query").and_then(|v| v.as_str());
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Filter by query if provided
        let filtered_cards: Vec<Card> = if let Some(query_str) = query {
            cards
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
            cards.into_iter().map(|(_, card)| card).collect()
        };

        // Store total count before moving filtered_cards
        let total_count = filtered_cards.len();

        // Apply pagination
        let paginated_cards = filtered_cards
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();

        let envelopes = paginated_cards
            .into_iter()
            .map(CardEnvelope::from)
            .collect::<Vec<_>>();

        Ok(json!({
            "cards": envelopes,
            "count": envelopes.len(),
            "total": total_count
        }))
    }

    async fn handle_grep(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing pattern".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        let matches: Vec<_> = cards
            .into_iter()
            .filter(|(_, card)| {
                card.title.contains(pattern)
                    || card
                        .get_content()
                        .map_or(false, |content| content.contains(pattern))
            })
            .map(|(path, card)| {
                json!({
                    "path": path.to_string_lossy(),
                    "card": CardEnvelope::from(card)
                })
            })
            .collect();

        Ok(json!({
            "matches": matches,
            "count": matches.len()
        }))
    }

    async fn handle_backlinks(
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
        let all_cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        let backlinks: Vec<_> = all_cards
            .into_iter()
            .filter(|(_, card)| card.links.iter().any(|link| link.to == card_id))
            .map(|(_, card)| CardEnvelope::from(card))
            .collect();

        Ok(json!({
            "backlinks": backlinks,
            "count": backlinks.len()
        }))
    }

    async fn handle_tree(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let root_card_id = params.get("root").and_then(|v| v.as_str());
        let max_depth = params.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let repo_root = state.repo_root.lock().await;
        let all_cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Build a map for quick lookup
        let card_map: std::collections::HashMap<String, Card> = all_cards
            .into_iter()
            .map(|(_, card)| (card.uid.clone(), card))
            .collect();

        // Start from root or find root cards (cards with no parents)
        let root_cards: Vec<&Card> = if let Some(root_id) = root_card_id {
            if let Some(card) = card_map.get(root_id) {
                vec![card]
            } else {
                vec![]
            }
        } else {
            card_map
                .values()
                .filter(|card| {
                    !card_map.values().any(|other| {
                        other
                            .links
                            .iter()
                            .any(|link| link.r#type == "contains" && link.to == card.uid)
                    })
                })
                .collect()
        };

        // Build tree structure
        fn build_tree(
            card: &Card,
            card_map: &std::collections::HashMap<String, Card>,
            current_depth: usize,
            max_depth: usize,
            visited: &mut std::collections::HashSet<String>,
        ) -> Value {
            if current_depth > max_depth || visited.contains(&card.uid) {
                return json!({
                    "uid": card.uid,
                    "title": card.title,
                    "truncated": true
                });
            }

            visited.insert(card.uid.clone());

            let children: Vec<Value> = card
                .links
                .iter()
                .filter(|link| link.r#type == "contains")
                .filter_map(|link| card_map.get(&link.to))
                .map(|child_card| {
                    build_tree(child_card, card_map, current_depth + 1, max_depth, visited)
                })
                .collect();

            json!({
                "uid": card.uid,
                "title": card.title,
                "slug": card.slug,
                "children": children,
                "tags": card.tags,
                "links": card.links
            })
        }

        let mut visited = std::collections::HashSet::new();
        let trees: Vec<Value> = root_cards
            .into_iter()
            .map(|card| build_tree(card, &card_map, 0, max_depth, &mut visited))
            .collect();

        Ok(json!({
            "trees": trees,
            "count": trees.len()
        }))
    }
}
