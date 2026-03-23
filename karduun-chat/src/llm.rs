use llm::{
    backends::ollama::Ollama,
    chat::{ChatMessage, ChatProvider},
};

/// Unified LLM Backend using the `llm` crate
#[derive(Clone)]
pub enum LLMBackend {
    Ollama(Ollama),
    None,
}

impl LLMBackend {
    pub fn from_args(
        backend: &str,
        model: &str,
        ollama_url: &str,
        _openai_key: Option<&str>,
    ) -> Self {
        match backend {
            "ollama" => {
                // Use the base Ollama URL - the llm crate will construct the full API path
                let api_url = ollama_url.to_string();

                LLMBackend::Ollama(Ollama::new(
                    api_url,
                    None,                    // api_key
                    Some(model.to_string()), // model
                    None,                    // max_tokens
                    None,                    // temperature
                    None,                    // timeout_seconds
                    None,                    // system
                    None,                    // top_p
                    None,                    // top_k
                    None,                    // json_schema
                    None,                    // tools
                ))
            }
            _ => LLMBackend::None,
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, String> {
        match self {
            LLMBackend::Ollama(model) => {
                let message = ChatMessage::user().content(prompt).build();
                let response = model
                    .chat(&[message])
                    .await
                    .map_err(|e| format!("Ollama error: {}", e))?;
                Ok(response.text().unwrap_or_default())
            }
            LLMBackend::None => {
                Ok("LLM backend not configured. Use direct MCP commands.".to_string())
            }
        }
    }
}

/// Simple LLM prompt templates for MCP tool usage
pub fn create_mcp_prompt(user_input: &str) -> String {
    format!(
        "You are an AI assistant with access to Karduun MCP tools.
    Analyze the user's request and determine if you need to use any MCP tools.

    Available tools:
    - scribe.*: Create and manage cards
    - scout.*: Search and query cards
    - catalog.*: Manage indexes
    - gauge.*: Analytics
    - curator.*: Organization
    - stencil.*: Templates
    - porter.*: Import/export
    - notary.*: Signing
    - eco.*: Ecosystem

    User request: {}

    Respond with either:
    1. Direct answer if no tools needed
    2. MCP command to execute if tools needed (format: TOOL_NAME {{params: ...}})
    3. Follow-up questions if clarification needed",
        user_input
    )
}

/// Parse LLM response to extract MCP commands
pub fn extract_mcp_commands(response: &str) -> Vec<String> {
    let mut commands = Vec::new();

    // Simple pattern matching for MCP commands
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("scribe.")
            || trimmed.starts_with("scout.")
            || trimmed.starts_with("catalog.")
            || trimmed.starts_with("gauge.")
            || trimmed.starts_with("curator.")
            || trimmed.starts_with("stencil.")
            || trimmed.starts_with("porter.")
            || trimmed.starts_with("notary.")
            || trimmed.starts_with("eco.")
        {
            commands.push(trimmed.to_string());
        }
    }

    commands
}

/// Simple natural language to MCP command mapping
pub fn simple_nl_to_mcp(user_input: &str) -> Option<String> {
    let lower_input = user_input.to_lowercase();

    if lower_input.contains("create card") || lower_input.contains("new card") {
        Some("scribe.new {\"title\": \"New Card\", \"slug\": \"new-card\"}".to_string())
    } else if lower_input.contains("list cards") || lower_input.contains("show cards") {
        Some("scout.list {}".to_string())
    } else if lower_input.contains("search") {
        // Extract search term
        let search_term = user_input
            .replace("search", "")
            .replace("for", "")
            .trim()
            .to_string();
        Some(format!("scout.grep {{\"query\": \"{}\"}}", search_term))
    } else if lower_input.contains("status") || lower_input.contains("health") {
        Some("catalog.status {}".to_string())
    } else {
        None
    }
}
