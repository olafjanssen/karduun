use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The unified Card model - everything is a Card
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Card {
    pub kind: String,
    pub schema: u32,
    pub uid: String,
    pub slug: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facets: Option<Facets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign: Option<Signature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<Computed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub r#type: String,
    pub to: String,
}

/// Optional facets that add capabilities to a card
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Facets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentFacet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<CollectionFacet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateFacet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentFacet {
    #[serde(default = "default_mime")]
    pub mime: String,
    pub body: String,
}

fn default_mime() -> String {
    "text/markdown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionFacet {
    pub mode: CollectionMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<serde_json::Value>, // Query DSL in canonical JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CollectionMode {
    Static,
    Query,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewSettings {
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
}

fn default_layout() -> String {
    "list".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateFacet {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub defaults: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<TemplateConstraints>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateConstraints {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub enum_fields: HashMap<String, Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frozen_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    pub algo: String,
    pub by: String,
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Computed {
    pub tokens: Option<u32>,
    pub nid_bpt: Option<f64>,
    pub cohesion: Option<f64>,
    pub bandwidth: Option<u32>,
    pub redundancy: Option<f64>,
    pub link_density: Option<f64>,
    pub sv: Option<f64>,
    pub last_analyzed: Option<DateTime<Utc>>,
}

impl Card {
    pub fn new(title: String, slug: String, uid: String) -> Self {
        let now = Utc::now();
        Card {
            kind: "card".to_string(),
            schema: 1,
            uid,
            slug,
            title,
            author: None,
            created: now,
            updated: now,
            version: 1,
            tags: Vec::new(),
            keywords: Vec::new(),
            fields: HashMap::new(),
            links: Vec::new(),
            facets: None,
            sign: None,
            computed: None,
        }
    }

    pub fn with_content(mut self, body: String) -> Self {
        let mut facets = self.facets.unwrap_or_else(|| Facets {
            content: None,
            collection: None,
            template: None,
        });
        facets.content = Some(ContentFacet {
            mime: "text/markdown".to_string(),
            body,
        });
        self.facets = Some(facets);
        self
    }

    pub fn get_content(&self) -> Option<&str> {
        self.facets
            .as_ref()
            .and_then(|f| f.content.as_ref())
            .map(|c| c.body.as_str())
    }

    pub fn has_collection(&self) -> bool {
        self.facets
            .as_ref()
            .map(|f| f.collection.is_some())
            .unwrap_or(false)
    }

    pub fn has_template(&self) -> bool {
        self.facets
            .as_ref()
            .map(|f| f.template.is_some())
            .unwrap_or(false)
    }
}

/// CardEnvelope for JSONL streams between tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardEnvelope {
    pub r#type: String,
    pub uid: String,
    pub slug: String,
    pub title: String,
    pub path: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links_out: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facets: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<Computed>,
}

impl From<Card> for CardEnvelope {
    fn from(card: Card) -> Self {
        CardEnvelope {
            r#type: "card".to_string(),
            uid: card.uid.clone(),
            slug: card.slug.clone(),
            title: card.title.clone(),
            path: format!(
                "cards/{}/{}/{}--{}.yaml",
                card.created.format("%Y"),
                card.created.format("%m"),
                card.uid,
                card.slug
            ),
            created: card.created,
            updated: card.updated,
            tags: card.tags.clone(),
            fields: card.fields.clone(),
            links_out: card.links.clone(),
            facets: card
                .facets
                .as_ref()
                .map(|f| serde_json::to_value(f).ok())
                .flatten(),
            computed: card.computed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_new() {
        let card = Card::new(
            "Test".to_string(),
            "test".to_string(),
            "ulid_01".to_string(),
        );
        assert_eq!(card.kind, "card");
        assert_eq!(card.schema, 1);
        assert_eq!(card.title, "Test");
    }

    #[test]
    fn test_card_with_content() {
        let card = Card::new(
            "Test".to_string(),
            "test".to_string(),
            "ulid_01".to_string(),
        )
        .with_content("# Hello\n\nWorld".to_string());
        assert_eq!(card.get_content(), Some("# Hello\n\nWorld"));
    }
}
