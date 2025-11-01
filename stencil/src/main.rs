use anyhow::{Context, Result};
use cardstack_lib::{card::{Card, Facets, TemplateFacet, TemplateConstraints}, serialize, uid};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "stencil")]
#[command(about = "Template management and validation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    
    #[arg(long, global = true)]
    json: bool,
    
    #[arg(long, global = true)]
    jsonl: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new template card
    New {
        name: String,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        required_field: Vec<String>,
        #[arg(long)]
        enum_field: Vec<String>,
        #[arg(long)]
        frozen_field: Vec<String>,
    },
    /// List all template cards
    List,
    /// Display a template
    Show {
        slug: String,
    },
    /// Validate cards against template constraints
    Validate {
        #[arg(long)]
        uid: Option<String>,
        #[arg(long)]
        query: Option<String>,
    },
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let cardstack_dir = current.join(".cardstack");
        if cardstack_dir.exists() && cardstack_dir.is_dir() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn get_repo_root(repo_override: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(repo) = repo_override {
        if repo.join(".cardstack").exists() {
            return Ok(repo);
        }
        anyhow::bail!("Not a cardstack repository: {:?}", repo);
    }
    
    let cwd = std::env::current_dir()?;
    find_repo_root(&cwd)
        .context("Not in a cardstack repository. Run 'scribe init' first.")
}

fn load_card(repo: &Path, identifier: &str) -> Result<Card> {
    let cards_dir = repo.join("cards");
    if !cards_dir.exists() {
        anyhow::bail!("Cards directory not found");
    }
    
    for entry in WalkDir::new(&cards_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok((card, _)) = serialize::parse_card_file(&content) {
                    if card.uid == identifier || card.slug == identifier {
                        return Ok(card);
                    }
                }
            }
        }
    }
    
    anyhow::bail!("Card not found: {}", identifier)
}

fn load_all_cards(repo: &Path) -> Result<Vec<Card>> {
    let cards_dir = repo.join("cards");
    if !cards_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut cards = Vec::new();
    for entry in WalkDir::new(&cards_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok((card, _)) = serialize::parse_card_file(&content) {
                    cards.push(card);
                }
            }
        }
    }
    
    Ok(cards)
}

fn save_card(repo: &Path, card: &mut Card) -> Result<PathBuf> {
    card.updated = chrono::Utc::now();
    card.version += 1;
    
    let year = card.created.format("%Y").to_string();
    let month = card.created.format("%m").to_string();
    let dir = repo.join("cards").join(&year).join(&month);
    fs::create_dir_all(&dir)?;
    
    let filename = format!("{}--{}.yaml", card.uid, card.slug);
    let file_path = dir.join(&filename);
    
    let content = serialize::write_card_file(card)?;
    fs::write(&file_path, content)?;
    
    Ok(file_path)
}

#[derive(serde::Serialize)]
struct ValidationResult {
    uid: String,
    slug: String,
    template: Option<String>,
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_card_against_template(card: &Card, template: &Card) -> (bool, Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    if let Some(facets) = &template.facets {
        if let Some(template_facet) = &facets.template {
            // Check required fields
            if let Some(constraints) = &template_facet.constraints {
                for required in &constraints.required_fields {
                    if required == "title" {
                        if card.title.is_empty() {
                            errors.push(format!("Missing required field: {}", required));
                        }
                    } else if required.starts_with("fields.") {
                        let field_name = required.strip_prefix("fields.").unwrap();
                        if !card.fields.contains_key(field_name) {
                            errors.push(format!("Missing required field: {}", required));
                        }
                    } else if !card.fields.contains_key(required) {
                        errors.push(format!("Missing required field: {}", required));
                    }
                }
                
                // Check enum fields
                for (field_path, allowed_values) in &constraints.enum_fields {
                    let field_value: Option<serde_json::Value> = if field_path == "title" {
                        Some(serde_json::Value::String(card.title.clone()))
                    } else if field_path.starts_with("fields.") {
                        card.fields.get(field_path.strip_prefix("fields.").unwrap()).cloned()
                    } else {
                        card.fields.get(field_path).cloned()
                    };
                    
                    if let Some(value) = field_value {
                        if !allowed_values.contains(&value) {
                            errors.push(format!(
                                "Field {} has invalid value: {:?} (allowed: {:?})",
                                field_path, value, allowed_values
                            ));
                        }
                    }
                }
                
                // Check frozen fields (warnings only, as they might have been set at creation)
                for frozen in &constraints.frozen_fields {
                    if frozen == "author.id" {
                        // Check if author changed (would need history to be sure)
                        warnings.push(format!("Field {} should not be modified after creation", frozen));
                    }
                }
            }
        }
    }
    
    let valid = errors.is_empty();
    (valid, errors, warnings)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    
    match &cli.command {
        Commands::New {
            name,
            slug,
            required_field,
            enum_field,
            frozen_field,
        } => {
            let uid = uid::generate_uid();
            let card_slug = slug.clone().unwrap_or_else(|| {
                format!("template-{}", name.to_lowercase()
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                    .collect::<String>())
            });
            
            let mut card = Card::new(
                format!("Template: {}", name),
                card_slug.clone(),
                uid.clone(),
            );
            
            // Parse enum fields (format: "field=value1,value2,value3")
            let mut enum_fields_map = std::collections::HashMap::new();
            for enum_spec in enum_field {
                if let Some((field, values_str)) = enum_spec.split_once('=') {
                    let values: Vec<serde_json::Value> = values_str
                        .split(',')
                        .map(|v| serde_json::Value::String(v.trim().to_string()))
                        .collect();
                    enum_fields_map.insert(field.to_string(), values);
                }
            }
            
            let constraints = if !required_field.is_empty() || !enum_fields_map.is_empty() || !frozen_field.is_empty() {
                Some(TemplateConstraints {
                    required_fields: required_field.clone(),
                    enum_fields: enum_fields_map,
                    frozen_fields: frozen_field.clone(),
                })
            } else {
                None
            };
            
            let template_facet = TemplateFacet {
                defaults: std::collections::HashMap::new(), // Can be populated from template body
                constraints,
            };
            
            let facets = Facets {
                content: Some(cardstack_lib::card::ContentFacet {
                    mime: "text/markdown".to_string(),
                    body: format!("# Template: {}\n\nDefault structure goes here...", name),
                }),
                collection: None,
                template: Some(template_facet),
            };
            
            card.facets = Some(facets);
            
            let file_path = save_card(&repo, &mut card)?;
            
            if cli.json || cli.jsonl {
                let envelope = cardstack_lib::card::CardEnvelope::from(card);
                println!("{}", serde_json::to_string(&envelope)?);
            } else {
                println!("Created template: {} ({})", name, uid);
                println!("Path: {}", file_path.display());
            }
        }
        Commands::List => {
            let all_cards = load_all_cards(&repo)?;
            let templates: Vec<_> = all_cards.iter()
                .filter(|c| c.has_template())
                .collect();
            
            if cli.jsonl {
                for template in templates {
                    let envelope = cardstack_lib::card::CardEnvelope::from(template.clone());
                    println!("{}", serde_json::to_string(&envelope)?);
                }
            } else {
                println!("Found {} template(s):", templates.len());
                for template in templates {
                    println!("  {} - {} ({})", template.slug, template.title, template.uid);
                }
            }
        }
        Commands::Show { slug } => {
            let template = load_card(&repo, slug)?;
            
            if !template.has_template() {
                anyhow::bail!("Card '{}' is not a template", slug);
            }
            
            if cli.json || cli.jsonl {
                let envelope = cardstack_lib::card::CardEnvelope::from(template);
                println!("{}", serde_json::to_string(&envelope)?);
            } else {
                println!("Template: {} ({})", template.title, template.uid);
                println!();
                
                if let Some(facets) = &template.facets {
                    if let Some(template_facet) = &facets.template {
                        println!("Defaults:");
                        if template_facet.defaults.is_empty() {
                            println!("  (none)");
                        } else {
                            for (k, v) in &template_facet.defaults {
                                println!("  {}: {:?}", k, v);
                            }
                        }
                        println!();
                        
                        if let Some(constraints) = &template_facet.constraints {
                            println!("Constraints:");
                            if !constraints.required_fields.is_empty() {
                                println!("  Required fields: {}", constraints.required_fields.join(", "));
                            }
                            if !constraints.enum_fields.is_empty() {
                                println!("  Enum fields:");
                                for (field, values) in &constraints.enum_fields {
                                    println!("    {}: {:?}", field, values);
                                }
                            }
                            if !constraints.frozen_fields.is_empty() {
                                println!("  Frozen fields: {}", constraints.frozen_fields.join(", "));
                            }
                        }
                    }
                }
                
                if let Some(body) = template.get_content() {
                    println!();
                    println!("Template body:");
                    println!("{}", body);
                }
            }
        }
        Commands::Validate { uid, query } => {
            let all_cards = load_all_cards(&repo)?;
            
            let cards_to_validate: Vec<_> = if let Some(uid_str) = uid {
                vec![load_card(&repo, uid_str)?]
            } else if let Some(_query_str) = query {
                // For now, just validate all cards (query filtering can be added)
                all_cards.clone()
            } else {
                all_cards.clone()
            };
            
            let mut results = Vec::new();
            
            for card in cards_to_validate {
                // Find template if linked via derived-from
                let template_uid = card.links.iter()
                    .find(|l| l.r#type == "derived-from")
                    .map(|l| &l.to);
                
                if let Some(template_uid) = template_uid {
                    if let Ok(template) = load_card(&repo, template_uid) {
                        if template.has_template() {
                            let (valid, errors, warnings) = validate_card_against_template(&card, &template);
                            results.push(ValidationResult {
                                uid: card.uid.clone(),
                                slug: card.slug.clone(),
                                template: Some(template.slug.clone()),
                                valid,
                                errors,
                                warnings,
                            });
                            continue;
                        }
                    }
                }
                
                // No template found
                results.push(ValidationResult {
                    uid: card.uid.clone(),
                    slug: card.slug.clone(),
                    template: None,
                    valid: true,
                    errors: Vec::new(),
                    warnings: vec!["No template linked".to_string()],
                });
            }
            
            if cli.jsonl {
                for result in results {
                    println!("{}", serde_json::to_string(&result)?);
                }
            } else {
                let valid_count = results.iter().filter(|r| r.valid).count();
                let invalid_count = results.len() - valid_count;
                
                println!("Validation Results:");
                println!("  Valid: {}", valid_count);
                println!("  Invalid: {}", invalid_count);
                println!();
                
                for result in results {
                    if !result.valid || !result.errors.is_empty() || !result.warnings.is_empty() {
                        println!("{} ({})", result.slug, result.uid);
                        if let Some(template) = &result.template {
                            println!("  Template: {}", template);
                        }
                        if !result.errors.is_empty() {
                            println!("  Errors:");
                            for err in &result.errors {
                                println!("    - {}", err);
                            }
                        }
                        if !result.warnings.is_empty() {
                            println!("  Warnings:");
                            for warn in &result.warnings {
                                println!("    - {}", warn);
                            }
                        }
                        println!();
                    }
                }
            }
        }
    }
    
    Ok(())
}
