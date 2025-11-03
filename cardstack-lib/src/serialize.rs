use crate::card::Card;
use anyhow::{Context, Result};
use serde_yaml;

/// Deterministic YAML serialization with fixed key order
pub fn deterministic_yaml(card: &Card) -> Result<String, serde_yaml::Error> {
    // Serialize to YAML with specific formatting
    serde_yaml::to_string(card)
}

/// Parse a card from YAML front matter + Markdown body
pub fn parse_card_file(content: &str) -> anyhow::Result<(Card, String)> {
    // Check for YAML front matter delimiter
    if !content.starts_with("---") {
        anyhow::bail!("Card file must start with YAML front matter (---)");
    }

    // Find the second `---` that ends the front matter
    // Handle both `---\n` and `---`
    let delimiter = if content.starts_with("---\n") {
        "---\n"
    } else if content.starts_with("---\r\n") {
        "---\r\n"
    } else {
        // Try to find the next ---
        if let Some(end_pos) = content[3..].find("---") {
            let yaml_end = end_pos + 3;
            let yaml_part = &content[3..yaml_end];
            let markdown_part = content[yaml_end + 3..].trim_start_matches('\n').trim_start_matches('\r').to_string();
            
            let mut card: Card = serde_yaml::from_str(yaml_part)
                .with_context(|| "Failed to parse YAML")?;
            
            if !markdown_part.trim().is_empty() {
                let facets = card.facets.get_or_insert_with(|| crate::card::Facets {
                    content: None,
                    collection: None,
                    template: None,
                });
                facets.content = Some(crate::card::ContentFacet {
                    mime: "text/markdown".to_string(),
                    body: markdown_part.trim().to_string(),
                });
            }
            
            return Ok((card, markdown_part));
        } else {
            anyhow::bail!("Missing closing --- delimiter");
        }
    };
    
    let mut parts = content.splitn(3, delimiter);
    let _ = parts.next(); // Skip the first `---`
    
    let yaml_part = parts.next().context("Missing YAML front matter")?;
    let markdown_part = parts.next().unwrap_or("").to_string();

    // Parse YAML
    let mut card: Card = serde_yaml::from_str(yaml_part)
        .with_context(|| "Failed to parse YAML")?;

    // Set the body in content facet if not already set
    if !markdown_part.trim().is_empty() {
        let facets = card.facets.get_or_insert_with(|| crate::card::Facets {
            content: None,
            collection: None,
            template: None,
        });
        facets.content = Some(crate::card::ContentFacet {
            mime: "text/markdown".to_string(),
            body: markdown_part.trim().to_string(),
        });
    }

    Ok((card, markdown_part))
}

/// Write a card to file format (YAML front matter + Markdown body)
pub fn write_card_file(card: &Card) -> Result<String, serde_yaml::Error> {
    let yaml = deterministic_yaml(card)?;
    let body = card.get_content().unwrap_or("");
    // Always write YAML front matter delimiters, even if body is empty
    Ok(format!("---\n{}\n---\n{}", yaml.trim_end(), body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use crate::uid::generate_uid;

    #[test]
    fn test_parse_card_file() {
        let yaml = r#"---
kind: card
schema: 1
uid: ulid_01TEST
slug: test
title: Test Card
created: 2025-01-01T00:00:00Z
updated: 2025-01-01T00:00:00Z
version: 1
tags: []
keywords: []
fields: {}
links: []
---
# Hello

This is content.
"#;

        let (card, body) = parse_card_file(yaml).expect("Should parse");
        assert_eq!(card.title, "Test Card");
        assert!(body.contains("Hello"));
    }

    #[test]
    fn test_write_card_file() {
        let mut card = Card::new(
            "Test".to_string(),
            "test".to_string(),
            generate_uid(),
        );
        card = card.with_content("# Content\n\nBody here.".to_string());
        
        let output = write_card_file(&card).unwrap();
        assert!(output.contains("kind: card"));
        assert!(output.contains("# Content"));
    }
}

