use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::Card;
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct GaugeHandler;

#[async_trait]
impl KarduunToolHandler for GaugeHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "gauge.analyze" => self.handle_analyze(state, request).await,
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
        // Gauge doesn't handle notifications yet
        Ok(())
    }
}

impl GaugeHandler {
    async fn handle_analyze(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let card_id = params.get("card_id").and_then(|v| v.as_str());

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        if let Some(target_card_id) = card_id {
            // Analyze specific card
            if let Some((_, card)) = cards.into_iter().find(|(_, c)| &c.uid == target_card_id) {
                self.analyze_single_card(card)
            } else {
                Err(KarduunMcpError::CardstackError(format!(
                    "Card not found: {}",
                    target_card_id
                )))
            }
        } else {
            // Analyze all cards and provide summary
            self.analyze_all_cards(cards)
        }
    }

    fn analyze_single_card(&self, card: Card) -> Result<Value, KarduunMcpError> {
        let content = card.get_content().unwrap_or("");

        // Basic text analysis
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();
        let line_count = content.lines().count();

        // Estimate reading time (average 200 words per minute)
        let reading_time_secs = (word_count as f64 / 200.0 * 60.0).round() as u64;

        // Tag analysis
        let tag_count = card.tags.len();

        // Link analysis
        let link_count = card.links.len();
        let link_types: std::collections::HashMap<String, usize> =
            card.links
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, link| {
                    *acc.entry(link.r#type.clone()).or_insert(0) += 1;
                    acc
                });

        Ok(json!({
            "card_id": card.uid,
            "title": card.title,
            "analysis": {
                "content": {
                    "word_count": word_count,
                    "char_count": char_count,
                    "line_count": line_count,
                    "reading_time_seconds": reading_time_secs,
                    "estimated_reading_time": format!("{:.1} minutes", reading_time_secs as f64 / 60.0)
                },
                "metadata": {
                    "tag_count": tag_count,
                    "tags": card.tags,
                    "link_count": link_count,
                    "link_types": link_types,
                    "field_count": card.fields.len()
                },
                "semantic_volume": self.estimate_semantic_volume(&card)
            }
        }))
    }

    fn analyze_all_cards(
        &self,
        cards: Vec<(std::path::PathBuf, Card)>,
    ) -> Result<Value, KarduunMcpError> {
        let total_cards = cards.len();

        if total_cards == 0 {
            return Ok(json!({
                "status": "no_cards_found",
                "message": "No cards found in repository"
            }));
        }

        // Calculate statistics across all cards
        let total_words: usize = cards
            .iter()
            .map(|(_, card)| {
                card.get_content()
                    .map_or(0, |c| c.split_whitespace().count())
            })
            .sum();

        let total_tags: usize = cards.iter().map(|(_, card)| card.tags.len()).sum();

        let total_links: usize = cards.iter().map(|(_, card)| card.links.len()).sum();

        // Average metrics
        let avg_words_per_card = total_words as f64 / total_cards as f64;
        let avg_tags_per_card = total_tags as f64 / total_cards as f64;
        let avg_links_per_card = total_links as f64 / total_cards as f64;

        // Find cards with most/least content
        let mut cards_by_word_count: Vec<_> = cards
            .iter()
            .map(|(_, card)| {
                (
                    card,
                    card.get_content()
                        .map_or(0, |c| c.split_whitespace().count()),
                )
            })
            .collect();

        cards_by_word_count.sort_by(|a, b| b.1.cmp(&a.1));

        let most_verbose = cards_by_word_count.first().map(|(card, count)| {
            json!({
                "card_id": card.uid,
                "title": card.title,
                "word_count": count
            })
        });

        let most_concise = cards_by_word_count.last().map(|(card, count)| {
            json!({
                "card_id": card.uid,
                "title": card.title,
                "word_count": count
            })
        });

        // Tag frequency analysis
        let mut tag_frequency = std::collections::HashMap::new();
        for (_, card) in &cards {
            for tag in &card.tags {
                *tag_frequency.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let top_tags: Vec<_> = tag_frequency
            .iter()
            .map(|(tag, count)| json!({"tag": tag, "count": count}))
            .collect();

        Ok(json!({
            "repository_analysis": {
                "total_cards": total_cards,
                "total_words": total_words,
                "total_tags": total_tags,
                "total_links": total_links,
                "averages": {
                    "words_per_card": avg_words_per_card,
                    "tags_per_card": avg_tags_per_card,
                    "links_per_card": avg_links_per_card
                },
                "content_distribution": {
                    "most_verbose": most_verbose,
                    "most_concise": most_concise
                },
                "tag_analysis": {
                    "unique_tags": tag_frequency.len(),
                    "top_tags": top_tags
                }
            }
        }))
    }

    fn estimate_semantic_volume(&self, card: &Card) -> Value {
        let content = card.get_content().unwrap_or("");
        let word_count = content.split_whitespace().count();

        // Simple heuristic for semantic volume estimation
        // This would be replaced with actual NLP analysis in a real implementation
        let tag_diversity = card.tags.len() as f64;
        let link_complexity = card.links.len() as f64;

        // Estimate semantic volume score (0-100)
        let sv_score = ((word_count as f64 * 0.1).min(50.0)
            + (tag_diversity * 2.0).min(20.0)
            + (link_complexity * 1.5).min(15.0)
            + if card.fields.contains_key("important") {
                10.0
            } else {
                0.0
            })
        .min(100.0)
        .round();

        json!({
            "score": sv_score,
            "interpretation": match sv_score as u32 {
                0..=30 => "Low complexity",
                31..=60 => "Medium complexity",
                61..=80 => "High complexity",
                81..=100 => "Very high complexity",
                _ => "Unknown"
            },
            "factors": {
                "content_length": word_count,
                "tag_diversity": tag_diversity,
                "link_complexity": link_complexity,
                "has_importance_flag": card.fields.contains_key("important")
            }
        })
    }
}
