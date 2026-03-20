use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::{Card, CardEnvelope};
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct EcoHandler;

#[async_trait]
impl KarduunToolHandler for EcoHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "eco.scan" => self.handle_scan(state, request).await,
            "eco.resonance" => self.handle_resonance(state, request).await,
            "eco.print" => self.handle_print(state, request).await,
            "eco.mature" => self.handle_mature(state, request).await,
            "eco.status" => self.handle_status(state, request).await,
            "eco.evolve" => self.handle_evolve(state, request).await,
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
        // Eco doesn't handle notifications yet
        Ok(())
    }
}

impl EcoHandler {
    async fn handle_scan(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params
            .get("card_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing card_id".to_string()))?;

        let resonance_increase = params
            .get("resonance_increase")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let repo_root = state.repo_root.lock().await;
        let mut card = cardstack_lib::repository::load_card(&repo_root, card_id)?;

        // Update resonance (simulating a scan event)
        let current_resonance = card
            .fields
            .get("resonance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let new_resonance = (current_resonance + resonance_increase).max(0.0);

        card.fields
            .insert("resonance".to_string(), json!(new_resonance));
        card.fields.insert(
            "last_scanned".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );

        // Increment scan count
        let scan_count = card
            .fields
            .get("scan_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            + 1;
        card.fields
            .insert("scan_count".to_string(), json!(scan_count));

        // Save the updated card
        cardstack_lib::repository::save_card(&repo_root, &mut card)?;

        let envelope = CardEnvelope::from(card);

        Ok(json!({
            "status": "success",
            "card_id": card_id,
            "old_resonance": current_resonance,
            "new_resonance": new_resonance,
            "resonance_increase": resonance_increase,
            "scan_count": scan_count,
            "updated_card": envelope,
            "message": "Card scanned and resonance updated"
        }))
    }

    async fn handle_resonance(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params.get("card_id").and_then(|v| v.as_str());

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        if let Some(target_card_id) = card_id {
            // Get resonance for specific card
            if let Some((_, card)) = cards.into_iter().find(|(_, c)| &c.uid == target_card_id) {
                let resonance = card
                    .fields
                    .get("resonance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let scan_count = card
                    .fields
                    .get("scan_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                Ok(json!({
                    "status": "success",
                    "card_id": card.uid,
                    "resonance": resonance,
                    "scan_count": scan_count,
                    "last_scanned": card.fields.get("last_scanned"),
                    "message": "Resonance data retrieved"
                }))
            } else {
                Err(KarduunMcpError::CardstackError(format!(
                    "Card not found: {}",
                    target_card_id
                )))
            }
        } else {
            // Get resonance for all cards
            let mut resonance_data: Vec<_> = cards
                .into_iter()
                .map(|(_, card)| {
                    let resonance = card
                        .fields
                        .get("resonance")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let scan_count = card
                        .fields
                        .get("scan_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    json!({
                        "card_id": card.uid,
                        "title": card.title,
                        "resonance": resonance,
                        "scan_count": scan_count,
                        "last_scanned": card.fields.get("last_scanned")
                    })
                })
                .collect();

            // Sort by resonance (highest first)
            resonance_data.sort_by(|a, b| {
                let a_res = a["resonance"].as_f64().unwrap_or(0.0);
                let b_res = b["resonance"].as_f64().unwrap_or(0.0);
                b_res
                    .partial_cmp(&a_res)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Ok(json!({
                "status": "success",
                "total_cards": resonance_data.len(),
                "resonance_data": resonance_data,
                "message": "Resonance data for all cards retrieved"
            }))
        }
    }

    async fn handle_print(
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

        // Simulate printing (in a real system, this would interface with a printing service)
        let current_print_count = card
            .fields
            .get("print_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let new_print_count = current_print_count + 1;

        card.fields
            .insert("print_count".to_string(), json!(new_print_count));
        card.fields.insert(
            "last_printed".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );

        // Save the updated card
        cardstack_lib::repository::save_card(&repo_root, &mut card)?;

        let envelope = CardEnvelope::from(card);

        Ok(json!({
            "status": "success",
            "card_id": card_id,
            "print_count": new_print_count,
            "last_printed": chrono::Utc::now().to_rfc3339(),
            "updated_card": envelope,
            "message": "Card print recorded successfully"
        }))
    }

    async fn handle_mature(
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

        // Check if card meets maturation criteria
        let resonance = card
            .fields
            .get("resonance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let scan_count = card
            .fields
            .get("scan_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let print_count = card
            .fields
            .get("print_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Simple maturation criteria
        let is_mature = resonance >= 5.0 && (scan_count >= 3 || print_count >= 1);

        if is_mature {
            // Mark card as mature
            card.fields.insert("is_mature".to_string(), json!(true));
            card.fields.insert(
                "matured_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );

            // Save the updated card
            cardstack_lib::repository::save_card(&repo_root, &mut card)?;

            let envelope = CardEnvelope::from(card);

            Ok(json!({
                "status": "mature",
                "card_id": card_id,
                "is_mature": true,
                "resonance": resonance,
                "scan_count": scan_count,
                "print_count": print_count,
                "matured_at": chrono::Utc::now().to_rfc3339(),
                "matured_card": envelope,
                "message": "Card has matured!"
            }))
        } else {
            Ok(json!({
                "status": "immature",
                "card_id": card_id,
                "is_mature": false,
                "resonance": resonance,
                "scan_count": scan_count,
                "print_count": print_count,
                "requirements": {
                    "minimum_resonance": 5.0,
                    "minimum_scans": 3,
                    "minimum_prints": 1
                },
                "message": "Card does not yet meet maturation criteria"
            }))
        }
    }

    async fn handle_status(
        &self,
        state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Calculate ecosystem statistics
        let total_cards = cards.len();

        let mature_cards = cards
            .iter()
            .filter(|(_, card)| {
                card.fields
                    .get("is_mature")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();

        let total_resonance: f64 = cards
            .iter()
            .filter_map(|(_, card)| card.fields.get("resonance").and_then(|v| v.as_f64()))
            .sum();

        let avg_resonance = if total_cards > 0 {
            total_resonance / total_cards as f64
        } else {
            0.0
        };

        let total_scans: u64 = cards
            .iter()
            .filter_map(|(_, card)| card.fields.get("scan_count").and_then(|v| v.as_u64()))
            .sum();

        let total_prints: u64 = cards
            .iter()
            .filter_map(|(_, card)| card.fields.get("print_count").and_then(|v| v.as_u64()))
            .sum();

        // Find most resonant cards
        let mut cards_by_resonance: Vec<_> = cards
            .iter()
            .filter_map(|(_, card)| {
                card.fields
                    .get("resonance")
                    .and_then(|v| v.as_f64())
                    .map(|res| (card, res))
            })
            .collect();

        cards_by_resonance
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_cards: Vec<_> = cards_by_resonance
            .into_iter()
            .take(5)
            .map(|(card, resonance)| {
                json!({
                    "card_id": card.uid,
                    "title": card.title,
                    "resonance": resonance
                })
            })
            .collect();

        Ok(json!({
            "status": "success",
            "ecosystem_stats": {
                "total_cards": total_cards,
                "mature_cards": mature_cards,
                "maturity_rate": if total_cards > 0 {
                    (mature_cards as f64 / total_cards as f64) * 100.0
                } else {
                    0.0
                },
                "total_resonance": total_resonance,
                "average_resonance": avg_resonance,
                "total_scans": total_scans,
                "total_prints": total_prints,
                "engagement_score": self.calculate_engagement_score(
                    total_cards, total_scans, total_prints, total_resonance
                )
            },
            "top_cards": top_cards,
            "message": "Ecosystem status retrieved successfully"
        }))
    }

    async fn handle_evolve(
        &self,
        state: &ServerState,
        _request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Find mature cards that can evolve
        let mature_cards: Vec<&Card> = cards
            .iter()
            .filter(|(_, card)| {
                card.fields
                    .get("is_mature")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .map(|(_, card)| card)
            .collect();

        if mature_cards.is_empty() {
            return Ok(json!({
                "status": "no_evolution",
                "message": "No mature cards available for evolution"
            }));
        }

        // Group mature cards by similarity (simple implementation)
        let mut evolution_clusters: Vec<Vec<&Card>> = Vec::new();

        for card in &mature_cards {
            // Find a cluster with similar cards or create a new one
            let mut found_cluster = false;
            for cluster in &mut evolution_clusters {
                if self.are_cards_similar(card, cluster[0]) {
                    cluster.push(card);
                    found_cluster = true;
                    break;
                }
            }

            if !found_cluster {
                evolution_clusters.push(vec![card]);
            }
        }

        // Create evolution opportunities
        let mut evolution_opportunities = Vec::new();

        for (i, cluster) in evolution_clusters.into_iter().enumerate() {
            if cluster.len() >= 2 {
                // Need at least 2 cards to evolve
                let cluster_resonance: f64 = cluster
                    .iter()
                    .filter_map(|card| card.fields.get("resonance").and_then(|v| v.as_f64()))
                    .sum();

                let evolution_potential =
                    (cluster_resonance / cluster.len() as f64) * (cluster.len() as f64 * 0.5);

                evolution_opportunities.push(json!({
                    "cluster_id": i,
                    "card_count": cluster.len(),
                    "total_resonance": cluster_resonance,
                    "evolution_potential": evolution_potential,
                    "cards": cluster.iter().map(|card| json!({
                        "card_id": card.uid,
                        "title": card.title,
                        "resonance": card.fields.get("resonance").and_then(|v| v.as_f64())
                    })).collect::<Vec<_>>(),
                    "suggested_action": if evolution_potential > 10.0 {
                        "High potential for new concept evolution"
                    } else if evolution_potential > 5.0 {
                        "Moderate potential for evolution"
                    } else {
                        "Low potential, consider merging"
                    }
                }));
            }
        }

        // Sort by evolution potential
        evolution_opportunities.sort_by(|a, b| {
            let a_pot = a["evolution_potential"].as_f64().unwrap_or(0.0);
            let b_pot = b["evolution_potential"].as_f64().unwrap_or(0.0);
            b_pot
                .partial_cmp(&a_pot)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(json!({
            "status": "success",
            "mature_cards_analyzed": mature_cards.len(),
            "evolution_clusters": evolution_opportunities.len(),
            "evolution_opportunities": evolution_opportunities,
            "message": format!(
                "Found {} evolution opportunities from {} mature cards",
                evolution_opportunities.len(),
                mature_cards.len()
            )
        }))
    }

    fn calculate_engagement_score(
        &self,
        total_cards: usize,
        total_scans: u64,
        total_prints: u64,
        total_resonance: f64,
    ) -> f64 {
        // Simple engagement score calculation
        let scan_score = (total_scans as f64 / total_cards.max(1) as f64) * 0.4;
        let print_score = (total_prints as f64 / total_cards.max(1) as f64) * 0.3;
        let resonance_score = (total_resonance / total_cards.max(1) as f64) * 0.3;

        (scan_score + print_score + resonance_score).min(100.0)
    }

    fn are_cards_similar(&self, card1: &Card, card2: &Card) -> bool {
        // Simple similarity heuristic
        // In production, you might use NLP or other advanced techniques

        // Check if they share tags
        let shared_tags = card1
            .tags
            .iter()
            .filter(|tag| card2.tags.contains(tag))
            .count();

        // Check content similarity (very basic)
        let content1 = card1.get_content().unwrap_or("");
        let content2 = card2.get_content().unwrap_or("");

        let common_words = content1
            .split_whitespace()
            .filter(|word| content2.split_whitespace().any(|w| w == *word))
            .count();

        // Simple scoring
        let tag_score = (shared_tags as f64 / card1.tags.len().max(1) as f64) * 50.0;
        let content_score =
            (common_words as f64 / content1.split_whitespace().count().max(1) as f64) * 50.0;

        (tag_score + content_score) > 30.0
    }
}
