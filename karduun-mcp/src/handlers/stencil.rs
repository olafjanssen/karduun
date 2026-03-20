use crate::{error::KarduunMcpError, server::KarduunToolHandler, state::ServerState};
use async_trait::async_trait;
use cardstack_lib::card::{Card, CardEnvelope};
use mcp_sdk_rs::{Notification, Request};
use serde_json::{json, Value};


#[derive(Clone)]
pub struct StencilHandler;

#[async_trait]
impl KarduunToolHandler for StencilHandler {
    async fn handle_request(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        match request.method.as_str() {
            "stencil.new" => self.handle_new(state, request).await,
            "stencil.list" => self.handle_list(state, request).await,
            "stencil.show" => self.handle_show(state, request).await,
            "stencil.validate" => self.handle_validate(state, request).await,
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
        // Stencil doesn't handle notifications yet
        Ok(())
    }
}

impl StencilHandler {
    async fn handle_new(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing name".to_string()))?;

        let slug = params
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                name.to_lowercase()
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>()
            });

        let repo_root = state.repo_root.lock().await;

        // Create a template card
        let mut template_card = Card::new(
            format!("Template: {}", name),
            slug,
            cardstack_lib::uid::generate_uid(),
        );

        // Add template-specific fields
        template_card
            .fields
            .insert("template_type".to_string(), json!("stencil"));
        template_card
            .fields
            .insert("is_template".to_string(), json!(true));

        // Add template constraints if provided
        if let Some(constraints) = params.get("constraints") {
            template_card
                .fields
                .insert("template_constraints".to_string(), constraints.clone());
        }

        // Add template fields if provided
        if let Some(fields) = params.get("fields").and_then(|v| v.as_object()) {
            for (key, value) in fields {
                template_card
                    .fields
                    .insert(format!("template_field_{}", key), value.clone());
            }
        }

        // Save the template card
        let card_path = cardstack_lib::repository::save_card(&repo_root, &mut template_card)?;
        let envelope = CardEnvelope::from(template_card);

        Ok(json!({
            "status": "success",
            "template": envelope,
            "path": card_path.to_string_lossy(),
            "message": "Template created successfully"
        }))
    }

    async fn handle_list(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let template_type = params.get("type").and_then(|v| v.as_str());

        let repo_root = state.repo_root.lock().await;
        let cards = cardstack_lib::repository::load_all_cards(&repo_root)?;

        // Filter for template cards
        let templates: Vec<_> = cards
            .into_iter()
            .filter(|(_, card)| {
                card.fields
                    .get("is_template")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .filter(|(_, card)| {
                template_type.map_or(true, |t| {
                    card.fields.get("template_type").and_then(|v| v.as_str()) == Some(t)
                })
            })
            .map(|(_, card)| CardEnvelope::from(card))
            .collect();

        Ok(json!({
            "templates": templates,
            "count": templates.len(),
            "message": format!("Found {} templates", templates.len())
        }))
    }

    async fn handle_show(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let template_id = params
            .get("template_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing template_id".to_string()))?;

        let repo_root = state.repo_root.lock().await;
        let card = cardstack_lib::repository::load_card(&repo_root, template_id)?;

        // Verify it's actually a template
        if !card
            .fields
            .get("is_template")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(KarduunMcpError::CardstackError(format!(
                "Card {} is not a template",
                template_id
            )));
        }

        let envelope = CardEnvelope::from(card);

        // Extract template-specific information
        let template_info = json!({
            "template_id": card.uid,
            "name": card.title,
            "type": card.fields.get("template_type").and_then(|v| v.as_str()),
            "constraints": card.fields.get("template_constraints"),
            "fields": card.fields.iter()
                .filter(|(k, _)| k.starts_with("template_field_"))
                .map(|(k, v)| (k.strip_prefix("template_field_").unwrap(), v.clone()))
                .collect::<serde_json::Map<String, serde_json::Value>>(),
            "created": card.created,
            "updated": card.updated
        });

        Ok(json!({
            "status": "success",
            "template": envelope,
            "template_info": template_info,
            "message": "Template details retrieved"
        }))
    }

    async fn handle_validate(
        &self,
        state: &ServerState,
        request: Request,
    ) -> Result<Value, KarduunMcpError> {
        let params = request.params.unwrap_or_default();
        let template_id = params
            .get("template_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KarduunMcpError::InvalidRequest("Missing template_id".to_string()))?;

        let card_data = params.get("card_data").ok_or_else(|| {
            KarduunMcpError::InvalidRequest("Missing card_data to validate".to_string())
        })?;

        let repo_root = state.repo_root.lock().await;

        // Load the template
        let template_card = cardstack_lib::repository::load_card(&repo_root, template_id)?;

        // Verify it's a template
        if !template_card
            .fields
            .get("is_template")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(KarduunMcpError::CardstackError(format!(
                "Card {} is not a template",
                template_id
            )));
        }

        // Perform validation
        let mut validation_results = Vec::new();
        let mut is_valid = true;

        // Check required fields
        if let Some(constraints) = template_card
            .fields
            .get("template_constraints")
            .and_then(|v| v.as_object())
        {
            if let Some(required_fields) = constraints
                .get("required_fields")
                .and_then(|v| v.as_array())
            {
                for required_field in required_fields {
                    if let Some(field_name) = required_field.as_str() {
                        if !card_data.get(field_name).is_some() {
                            validation_results.push(json!({
                                "field": field_name,
                                "status": "error",
                                "message": format!("Required field '{}' is missing", field_name)
                            }));
                            is_valid = false;
                        } else {
                            validation_results.push(json!({
                                "field": field_name,
                                "status": "ok",
                                "message": "Required field present"
                            }));
                        }
                    }
                }
            }
        }

        // Check field types if specified
        if let Some(constraints) = template_card
            .fields
            .get("template_constraints")
            .and_then(|v| v.as_object())
        {
            if let Some(field_types) = constraints.get("field_types").and_then(|v| v.as_object()) {
                for (field_name, expected_type) in field_types {
                    if let Some(actual_value) = card_data.get(field_name) {
                        let type_match = match expected_type.as_str() {
                            Some("string") => actual_value.is_string(),
                            Some("number") => actual_value.is_number(),
                            Some("boolean") => actual_value.is_boolean(),
                            Some("array") => actual_value.is_array(),
                            Some("object") => actual_value.is_object(),
                            _ => false,
                        };

                        if !type_match {
                            validation_results.push(json!({
                                "field": field_name,
                                "status": "error",
                                "message": format!("Field '{}' should be {} but is {}", field_name, expected_type, actual_value)
                            }));
                            is_valid = false;
                        } else {
                            validation_results.push(json!({
                                "field": field_name,
                                "status": "ok",
                                "message": format!("Field '{}' has correct type", field_name)
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!({
            "status": if is_valid { "valid" } else { "invalid" },
            "is_valid": is_valid,
            "template_id": template_id,
            "validation_results": validation_results,
            "message": if is_valid {
                "Card data is valid according to template"
            } else {
                "Card data has validation errors"
            }
        }))
    }
}
