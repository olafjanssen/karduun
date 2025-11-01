use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Canonical Query representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Query {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Filter {
    All(Vec<String>),
    Any(Vec<String>),
    None(Vec<String>),
}

/// Parse shorthand query DSL into canonical Query JSON
/// 
/// Examples:
/// - `status=draft tag:design` → `{filter: {all: ["fields.status = \"draft\"", "tags contains \"design\""]}}`
/// - `status=draft sort:-updated,title limit:50`
pub fn parse_query_shorthand(shorthand: &str) -> anyhow::Result<Query> {
    let mut query = Query {
        filter: None,
        sort: Vec::new(),
        limit: None,
    };

    let mut predicates = Vec::new();
    let parts: Vec<&str> = shorthand.split_whitespace().collect();

    for part in parts {
        if part.starts_with("sort:") {
            let sorts = part.strip_prefix("sort:").unwrap();
            query.sort = sorts.split(',').map(|s| s.to_string()).collect();
        } else if part.starts_with("limit:") {
            let limit_str = part.strip_prefix("limit:").unwrap();
            query.limit = Some(limit_str.parse().with_context(|| format!("Invalid limit: {}", limit_str))?);
        } else if part.starts_with("tag:") {
            let tag = part.strip_prefix("tag:").unwrap();
            predicates.push(format!("tags contains \"{}\"", tag));
        } else if part.contains('=') {
            let (field, value) = part.split_once('=').unwrap();
            if field.starts_with("fields.") {
                let field_name = field.strip_prefix("fields.").unwrap();
                predicates.push(format!("fields.{} = \"{}\"", field_name, value));
            } else {
                predicates.push(format!("fields.{} = \"{}\"", field, value));
            }
        } else if part.starts_with("has:") {
            let facet = part.strip_prefix("has:").unwrap();
            predicates.push(format!("has:{}", facet));
        } else if part.starts_with("link:") {
            // link:contains>ulid_01 or link:contains
            let link_expr = part.strip_prefix("link:").unwrap();
            if let Some((link_type, target)) = link_expr.split_once('>') {
                predicates.push(format!("link:{}>{}", link_type, target));
            } else {
                predicates.push(format!("link:{}", link_expr));
            }
        } else {
            anyhow::bail!("Unrecognized query part: {}", part);
        }
    }

    if !predicates.is_empty() {
        query.filter = Some(Filter::All(predicates));
    }

    Ok(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let query = parse_query_shorthand("status=draft tag:design").unwrap();
        match query.filter {
            Some(Filter::All(preds)) => {
                assert!(preds.contains(&"fields.status = \"draft\"".to_string()));
                assert!(preds.contains(&"tags contains \"design\"".to_string()));
            }
            _ => panic!("Expected All filter"),
        }
    }

    #[test]
    fn test_parse_with_sort() {
        let query = parse_query_shorthand("status=draft sort:-updated,title limit:50").unwrap();
        assert_eq!(query.sort, vec!["-updated", "title"]);
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_parse_tag_query() {
        let query = parse_query_shorthand("tag:research tag:design").unwrap();
        match query.filter {
            Some(Filter::All(preds)) => {
                assert_eq!(preds.len(), 2);
            }
            _ => panic!(),
        }
    }
}

