// Schema validation module
// JSON Schemas will be stored in /schemas/ directory

use crate::card::Card;
use jsonschema::JSONSchema;
use serde_json::Value;

/// Validate a card against JSON Schema
pub fn validate_card(card: &Card, schema: &JSONSchema) -> Result<(), Vec<String>> {
    let card_value: Value = serde_json::to_value(card)
        .map_err(|e| vec![format!("Serialization error: {}", e)])?;
    
    let validation_result = schema.validate(&card_value);
    
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        Err(error_messages)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use crate::uid::generate_uid;

    #[test]
    fn test_basic_card_structure() {
        let card = Card::new(
            "Test".to_string(),
            "test".to_string(),
            generate_uid(),
        );
        // Basic structure validation - JSON Schema would be more comprehensive
        assert_eq!(card.kind, "card");
        assert_eq!(card.schema, 1);
    }
}

