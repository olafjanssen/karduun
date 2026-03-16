mod tui;

use anyhow::Result;
use cardstack_lib::{
    card::{Card, CardEnvelope, CollectionFacet, CollectionMode, Facets},
    serialize, uid,
};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "scribe")]
#[command(about = "Core CRUD operations on cards", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Bootstrap a new cardstack repository
    Init,
    /// Create a new card
    New {
        title: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long)]
        slug: Option<String>,
        #[arg(short, long)]
        tag: Vec<String>,
        #[arg(long)]
        field: Vec<String>,
        #[arg(long)]
        body: Option<PathBuf>,
    },
    /// Display a card
    Show {
        identifier: String,
    },
    /// Edit a card
    Edit {
        identifier: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        field: Vec<String>,
        #[arg(long)]
        unset: Vec<String>,
        #[arg(long)]
        set_body: Option<PathBuf>,
        #[arg(long)]
        append_body: Option<PathBuf>,
    },
    /// Archive (soft-delete) a card
    Archive {
        identifier: String,
    },
    /// Fork a card (duplicate with provenance)
    Fork {
        identifier: String,
        #[arg(long)]
        with_links: bool,
    },
    /// Merge two cards
    Merge {
        src: String,
        dst: String,
        #[arg(long)]
        strategy: Option<String>,
    },
    /// Create a typed link between cards
    Link {
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        r#type: String,
    },
    /// Remove a link
    Unlink {
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        r#type: Option<String>,
    },
    /// Deck operations
    #[command(subcommand, name = "deck")]
    Deck(DeckCommands),
    /// Import cards from JSONL (CardEnvelope format)
    Import {
        #[arg(long)]
        jsonl: bool,
    },
    /// Launch interactive TUI
    Tui,
}

#[derive(Subcommand)]
enum DeckCommands {
    /// Create a new deck (card with collection facet)
    New {
        name: String,
        #[arg(long)]
        mode: Option<String>,
        #[arg(long)]
        query: Option<String>,
    },
    /// Show deck contents and metadata
    Show {
        deck: String,
    },
    /// Add cards to a deck
    Add {
        deck: String,
        cards: Vec<String>,
    },
    /// Remove cards from a deck
    Remove {
        deck: String,
        cards: Vec<String>,
    },
    /// Snapshot a dynamic deck to static
    Snapshot {
        deck: String,
        #[arg(long)]
        out: String,
    },
}

// Use shared repository functions
use cardstack_lib::repository::{get_repo_root, load_card, save_card};

fn init_repo(repo_path: &Path) -> Result<()> {
    let cardstack_dir = repo_path.join(".cardstack");
    let cards_dir = repo_path.join("cards");
    let media_dir = repo_path.join("media");
    let schemas_dir = repo_path.join("schemas");
    
    fs::create_dir_all(&cardstack_dir)?;
    fs::create_dir_all(&cards_dir)?;
    fs::create_dir_all(&media_dir)?;
    fs::create_dir_all(&schemas_dir)?;
    
    // Create config.yml
    let config = r#"analyzer: full
thresholds:
  split: "(tokens>350 && bandwidth>=3) || (cohesion<0.45 && tokens>250)"
  merge: "tokens<80 && redundancy>0.85"
  prune: "redundancy>0.9 && backlinks=0 && age>120"
  refactor: "tokens>300 && structure_density<0.8"
defaults:
  id: ulid
  content_mime: text/markdown
"#;
    fs::write(cardstack_dir.join("config.yml"), config)?;
    
    // Copy schemas if they exist
    let source_schemas = Path::new("schemas");
    if source_schemas.exists() {
        for entry in fs::read_dir(source_schemas)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let dest = schemas_dir.join(entry.file_name());
                fs::copy(entry.path(), dest)?;
            }
        }
    }
    
    println!("Initialized cardstack repository at: {}", repo_path.display());
    Ok(())
}

// find_card_file, load_card, and save_card are now in cardstack_lib::repository

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Init => {
            let repo = cli.repo.unwrap_or_else(|| std::env::current_dir().unwrap());
            init_repo(&repo)?;
        }
        Commands::New { title, template, slug, tag, field, body } => {
            let repo = get_repo_root(cli.repo.clone())?;
            
            let card_uid = uid::generate_uid();
            let card_slug = slug.clone().unwrap_or_else(|| {
                title.to_lowercase()
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                    .collect::<String>()
            });
            
            let mut card = Card::new(title.clone(), card_slug, card_uid);
            
            // Apply template if specified
            if let Some(template_slug) = template {
                let _template_card = load_card(&repo, &template_slug)?;
                // TODO: Apply template defaults and constraints
            }
            
            // Add tags
            card.tags = tag.clone();
            
            // Add fields
            for f in field {
                if let Some((k, v)) = f.split_once('=') {
                    card.fields.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
            }
            
            // Set body if provided
            if let Some(body_path) = body {
                let body_content = fs::read_to_string(body_path)?;
                card = card.with_content(body_content);
            }
            
            let file_path = save_card(&repo, &mut card)?;
            
            if cli.json {
                let envelope = cardstack_lib::card::CardEnvelope::from(card);
                println!("{}", serde_json::to_string(&envelope)?);
            } else {
                println!("Created card: {}", file_path.display());
            }
        }
        Commands::Show { identifier } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let card = load_card(&repo, identifier)?;
            
            if cli.json {
                let envelope = cardstack_lib::card::CardEnvelope::from(card);
                println!("{}", serde_json::to_string(&envelope)?);
            } else {
                println!("Title: {}", card.title);
                println!("UID: {}", card.uid);
                println!("Slug: {}", card.slug);
                if let Some(body) = card.get_content() {
                    println!("\n---\n{}", body);
                }
            }
        }
        Commands::Edit {
            identifier,
            title,
            slug,
            field,
            unset,
            set_body,
            append_body,
        } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let mut card = load_card(&repo, identifier)?;
            
            if let Some(new_title) = title {
                card.title = new_title.clone();
            }
            
            if let Some(new_slug) = slug {
                card.slug = new_slug.clone();
            }
            
            // Update fields
            for f in field {
                if let Some((k, v)) = f.split_once('=') {
                    card.fields.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
            }
            
            // Unset fields
            for k in unset {
                card.fields.remove(k.as_str());
            }
            
            // Set or append body
            if let Some(body_path) = set_body {
                let body_content = fs::read_to_string(body_path)?;
                card = card.with_content(body_content);
            } else if let Some(body_path) = append_body {
                let body_content = fs::read_to_string(body_path)?;
                let existing = card.get_content().unwrap_or("").to_string();
                card = card.with_content(format!("{}\n\n{}", existing, body_content));
            }
            
            let file_path = save_card(&repo, &mut card)?;
            if !cli.json {
                println!("Updated card: {}", file_path.display());
            }
        }
        Commands::Archive { identifier } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let mut card = load_card(&repo, identifier)?;
            
            // Soft-delete: set status field or move to archive
            card.fields.insert("archived".to_string(), serde_json::Value::Bool(true));
            card.fields.insert("archived_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            
            save_card(&repo, &mut card)?;
            if !cli.json {
                println!("Archived card: {}", identifier);
            }
        }
        Commands::Fork {
            identifier,
            with_links,
        } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let source = load_card(&repo, identifier)?;
            
            let new_uid = uid::generate_uid();
            let new_slug = format!("{}-fork", source.slug);
            let mut forked = Card::new(
                format!("{} (fork)", source.title),
                new_slug.clone(),
                new_uid.clone(),
            );
            
            // Copy content if present
            if let Some(body) = source.get_content() {
                forked = forked.with_content(body.to_string());
            }
            
            // Copy metadata
            forked.tags = source.tags.clone();
            forked.keywords = source.keywords.clone();
            forked.fields = source.fields.clone();
            
            // Copy links if requested
            if *with_links {
                forked.links = source.links.clone();
            }
            
            // Add provenance link
            forked.links.push(cardstack_lib::card::Link {
                r#type: "derived-from".to_string(),
                to: source.uid.clone(),
            });
            
            let file_path = save_card(&repo, &mut forked)?;
            if !cli.json {
                println!("Forked card: {} -> {}", identifier, new_uid);
                println!("New card: {}", file_path.display());
            }
        }
        Commands::Merge { src, dst, strategy: _ } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let mut src_card = load_card(&repo, src)?;
            let mut dst_card = load_card(&repo, dst)?;
            
            // Merge bodies
            let src_body = src_card.get_content().unwrap_or("").to_string();
            let dst_body = dst_card.get_content().unwrap_or("").to_string();
            if !src_body.is_empty() {
                let merged_body = if dst_body.is_empty() {
                    src_body
                } else {
                    format!("{}\n\n---\n\n{}", dst_body, src_body)
                };
                dst_card = dst_card.with_content(merged_body);
            }
            
            // Merge tags (union)
            for tag in &src_card.tags {
                if !dst_card.tags.contains(tag) {
                    dst_card.tags.push(tag.clone());
                }
            }
            
            // Merge fields (keep dst, note conflicts)
            for (k, v) in &src_card.fields {
                if dst_card.fields.contains_key(k) && dst_card.fields[k] != *v {
                    // Conflict - store in _conflicts
                    if !dst_card.fields.contains_key("_conflicts") {
                        dst_card.fields.insert("_conflicts".to_string(), serde_json::Value::Object(serde_json::Map::new()));
                    }
                } else {
                    dst_card.fields.insert(k.clone(), v.clone());
                }
            }
            
            // Add provenance link
            dst_card.links.push(cardstack_lib::card::Link {
                r#type: "derived-from".to_string(),
                to: src_card.uid.clone(),
            });
            
            // Archive source
            src_card.fields.insert("archived".to_string(), serde_json::Value::Bool(true));
            src_card.fields.insert("merged_into".to_string(), serde_json::Value::String(dst_card.uid.clone()));
            save_card(&repo, &mut src_card)?;
            
            let file_path = save_card(&repo, &mut dst_card)?;
            if !cli.json {
                println!("Merged {} into {}", src, dst);
                println!("Updated: {}", file_path.display());
            }
        }
        Commands::Link { from, to, r#type } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let mut from_card = load_card(&repo, from)?;
            
            // Validate target exists
            let _to_card = load_card(&repo, to)?;
            
            // Check if link already exists
            let link_exists = from_card.links.iter()
                .any(|l| l.r#type == *r#type && l.to == *to);
            
            if !link_exists {
                from_card.links.push(cardstack_lib::card::Link {
                    r#type: r#type.clone(),
                    to: to.clone(),
                });
                
                save_card(&repo, &mut from_card)?;
                if !cli.json {
                    println!("Linked {} --[{}]--> {}", from, r#type, to);
                }
            } else if !cli.json {
                println!("Link already exists");
            }
        }
        Commands::Unlink { from, to, r#type } => {
            let repo = get_repo_root(cli.repo.clone())?;
            let mut from_card = load_card(&repo, from)?;
            
            let initial_len = from_card.links.len();
            if let Some(type_filter) = r#type {
                from_card.links.retain(|l| !(l.r#type == *type_filter && l.to == *to));
            } else {
                from_card.links.retain(|l| l.to != *to);
            }
            
            if from_card.links.len() < initial_len {
                save_card(&repo, &mut from_card)?;
                if !cli.json {
                    println!("Unlinked {} from {}", from, to);
                }
            } else if !cli.json {
                println!("Link not found");
            }
        }
        Commands::Import { jsonl: _ } => {
            let repo = get_repo_root(cli.repo.clone())?;
            
            // Read CardEnvelopes from stdin, load full cards, and save them
            let stdin = io::stdin();
            let reader = stdin.lock();
            let mut imported_count = 0;
            
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                
                // Try to parse as CardEnvelope first (for pipeline compatibility)
                // If that fails, try parsing as full Card JSON
                match serde_json::from_str::<CardEnvelope>(&line) {
                    Ok(envelope) => {
                        // Load full card from repository using UID
                        match load_card(&repo, &envelope.uid) {
                            Ok(mut card) => {
                                // Save the card (will update timestamp/version)
                                save_card(&repo, &mut card)?;
                                imported_count += 1;
                                if !cli.json {
                                    println!("Imported: {} ({})", card.title, card.uid);
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Could not load card {}: {}", envelope.uid, e);
                            }
                        }
                    }
                    Err(_) => {
                        // Try parsing as full Card JSON
                        match serde_json::from_str::<Card>(&line) {
                            Ok(mut card) => {
                                save_card(&repo, &mut card)?;
                                imported_count += 1;
                                if !cli.json {
                                    println!("Imported: {} ({})", card.title, card.uid);
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Could not parse card: {}", e);
                            }
                        }
                    }
                }
            }
            
            if !cli.json {
                println!("Imported {} card(s)", imported_count);
            }
        }
        Commands::Deck(cmd) => {
            let repo = get_repo_root(cli.repo.clone())?;
            match cmd {
                DeckCommands::Show { deck } => {
                    let deck_card = load_card(&repo, &deck)?;
                    
                    if !deck_card.has_collection() {
                        anyhow::bail!("Card '{}' is not a deck (no collection facet)", deck);
                    }
                    
                    let collection = deck_card.facets
                        .as_ref()
                        .and_then(|f| f.collection.as_ref())
                        .ok_or_else(|| anyhow::anyhow!("Deck has no collection facet"))?;
                    
                    // Show deck metadata
                    println!("Deck: {} ({})", deck_card.title, deck_card.uid);
                    println!("Mode: {:?}", collection.mode);
                    
                    // Load all cards for query execution
                    let all_cards: Vec<(PathBuf, Card)> = {
                        let cards_dir = repo.join("cards");
                        if !cards_dir.exists() {
                            Vec::new()
                        } else {
                            let mut cards = Vec::new();
                            for entry in walkdir::WalkDir::new(&cards_dir) {
                                let entry = entry?;
                                let path = entry.path();
                                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                                    if let Ok(content) = fs::read_to_string(path) {
                                        if let Ok((card, _)) = serialize::parse_card_file(&content) {
                                            cards.push((path.to_path_buf(), card));
                                        }
                                    }
                                }
                            }
                            cards
                        }
                    };
                    
                    match collection.mode {
                        CollectionMode::Query | CollectionMode::Hybrid => {
                            // For query-based decks, extract and execute the query
                            if let Some(query_json) = &collection.query {
                                // Reconstruct query from manually-constructed JSON
                                // The filter is stored as {"op": "all", "predicates": [...]}
                                let mut query_obj = cardstack_lib::query::Query {
                                    filter: None,
                                    sort: Vec::new(),
                                    limit: None,
                                };
                                
                                if let Some(obj) = query_json.as_object() {
                                    // Parse filter
                                    if let Some(filter_val) = obj.get("filter") {
                                        if let Some(filter_obj) = filter_val.as_object() {
                                            if let Some(op) = filter_obj.get("op").and_then(|v| v.as_str()) {
                                                if let Some(preds_val) = filter_obj.get("predicates").and_then(|v| v.as_array()) {
                                                    let preds: Vec<String> = preds_val.iter()
                                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                                        .collect();
                                                    query_obj.filter = match op {
                                                        "all" => Some(cardstack_lib::query::Filter::All(preds)),
                                                        "any" => Some(cardstack_lib::query::Filter::Any(preds)),
                                                        "none" => Some(cardstack_lib::query::Filter::None(preds)),
                                                        _ => None,
                                                    };
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Parse sort
                                    if let Some(sort_val) = obj.get("sort").and_then(|v| v.as_array()) {
                                        query_obj.sort = sort_val.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect();
                                    }
                                    
                                    // Parse limit
                                    if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                                        query_obj.limit = Some(limit_val as u32);
                                    }
                                }
                                
                                let query = query_obj;
                                
                                // Convert to shorthand for display
                                let mut query_parts = Vec::new();
                                if let Some(ref filter) = query.filter {
                                    match filter {
                                        cardstack_lib::query::Filter::All(preds) => {
                                            for pred in preds {
                                                // Convert predicate back to shorthand
                                                if pred.starts_with("tags contains") {
                                                    if let Some(tag) = pred.split('"').nth(1) {
                                                        query_parts.push(format!("tag:{}", tag));
                                                    }
                                                } else if pred.starts_with("fields.") && pred.contains(" = ") {
                                                    let parts: Vec<&str> = pred.split(" = ").collect();
                                                    if parts.len() == 2 {
                                                        let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                        let value = parts[1].trim_matches('"');
                                                        query_parts.push(format!("{}={}", field, value));
                                                    }
                                                } else {
                                                    query_parts.push(pred.clone());
                                                }
                                            }
                                        }
                                        _ => {
                                            query_parts.push(format!("{:?}", filter));
                                        }
                                    }
                                }
                                
                                let query_str = query_parts.join(" ");
                                println!("Query: {}", query_str);
                                
                                // Execute query - reuse the same matching logic
                                let mut results: Vec<Card> = all_cards.iter()
                                    .map(|(_, card)| card.clone())
                                    .collect();
                                
                                // Apply filter (simplified matching logic)
                                if let Some(ref filter) = query.filter {
                                    match filter {
                                        cardstack_lib::query::Filter::All(preds) => {
                                            results.retain(|card| {
                                                preds.iter().all(|p| {
                                                    // Match tags
                                                    if p.starts_with("tags contains") {
                                                        if let Some(tag) = p.split('"').nth(1) {
                                                            return card.tags.contains(&tag.to_string());
                                                        }
                                                    }
                                                    // Match fields
                                                    if p.starts_with("fields.") && p.contains(" = ") {
                                                        let parts: Vec<&str> = p.split(" = ").collect();
                                                        if parts.len() == 2 {
                                                            let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                            let value = parts[1].trim_matches('"');
                                                            if let Some(field_val) = card.fields.get(field) {
                                                                if let Some(s) = field_val.as_str() {
                                                                    return s == value;
                                                                }
                                                                if let Some(b) = field_val.as_bool() {
                                                                    return value == if b { "true" } else { "false" };
                                                                }
                                                                if let Some(n) = field_val.as_i64() {
                                                                    if let Ok(v) = value.parse::<i64>() {
                                                                        return n == v;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    false
                                                })
                                            });
                                        }
                                        cardstack_lib::query::Filter::Any(preds) => {
                                            results.retain(|card| {
                                                preds.iter().any(|p| {
                                                    if p.starts_with("tags contains") {
                                                        if let Some(tag) = p.split('"').nth(1) {
                                                            return card.tags.contains(&tag.to_string());
                                                        }
                                                    }
                                                    if p.starts_with("fields.") && p.contains(" = ") {
                                                        let parts: Vec<&str> = p.split(" = ").collect();
                                                        if parts.len() == 2 {
                                                            let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                            let value = parts[1].trim_matches('"');
                                                            if let Some(field_val) = card.fields.get(field) {
                                                                if let Some(s) = field_val.as_str() {
                                                                    return s == value;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    false
                                                })
                                            });
                                        }
                                        cardstack_lib::query::Filter::None(preds) => {
                                            results.retain(|card| {
                                                !preds.iter().any(|p| {
                                                    if p.starts_with("tags contains") {
                                                        if let Some(tag) = p.split('"').nth(1) {
                                                            return card.tags.contains(&tag.to_string());
                                                        }
                                                    }
                                                    if p.starts_with("fields.") && p.contains(" = ") {
                                                        let parts: Vec<&str> = p.split(" = ").collect();
                                                        if parts.len() == 2 {
                                                            let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                            let value = parts[1].trim_matches('"');
                                                            if let Some(field_val) = card.fields.get(field) {
                                                                if let Some(s) = field_val.as_str() {
                                                                    return s == value;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    false
                                                })
                                            });
                                        }
                                    }
                                }
                                
                                // Apply sort
                                if !query.sort.is_empty() {
                                    results.sort_by(|a, b| {
                                        for sort_key in &query.sort {
                                            let descending = sort_key.starts_with('-');
                                            let key = if descending {
                                                &sort_key[1..]
                                            } else {
                                                sort_key.as_str()
                                            };
                                            
                                            let cmp = match key {
                                                "updated" => a.updated.cmp(&b.updated),
                                                "created" => a.created.cmp(&b.created),
                                                "title" => a.title.cmp(&b.title),
                                                _ => std::cmp::Ordering::Equal,
                                            };
                                            
                                            if cmp != std::cmp::Ordering::Equal {
                                                return if descending { cmp.reverse() } else { cmp };
                                            }
                                        }
                                        a.uid.cmp(&b.uid)
                                    });
                                }
                                
                                // Apply limit
                                if let Some(limit) = query.limit {
                                    results.truncate(limit as usize);
                                }
                                
                                println!("\nMembers ({}):", results.len());
                                for card in results {
                                    println!("  {} - {}", card.uid, card.title);
                                }
                            } else {
                                println!("Query: (none)");
                                println!("\nMembers: 0");
                            }
                        }
                        CollectionMode::Static => {
                            // For static decks, show members list
                            println!("Members ({}):", collection.members.len());
                            for member_uid in &collection.members {
                                // Find the card
                                if let Some((_, card)) = all_cards.iter().find(|(_, c)| &c.uid == member_uid) {
                                    println!("  {} - {}", card.uid, card.title);
                                } else {
                                    println!("  {} - (not found)", member_uid);
                                }
                            }
                        }
                    }
                }
                DeckCommands::New { name, mode, query } => {
                    let uid = uid::generate_uid();
                    let slug = name.to_lowercase()
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                        .collect::<String>();
                    
                    let mode_str = mode.as_deref().unwrap_or("static");
                    let collection_mode = match mode_str {
                        "query" => CollectionMode::Query,
                        "hybrid" => CollectionMode::Hybrid,
                        _ => CollectionMode::Static,
                    };
                    
                    let mut card = Card::new(name.clone(), slug.clone(), uid.clone());
                    
                    // Create collection facet
                    let mut collection = CollectionFacet {
                        mode: collection_mode,
                        members: Vec::new(),
                        query: None,
                        view: None,
                    };
                    
                    if let Some(query_str) = query {
                        // Parse query DSL to canonical JSON
                        let query_json = cardstack_lib::query::parse_query_shorthand(&query_str)?;
                        // Manually construct JSON Value to avoid tagged enum serialization issues
                        let mut query_obj = serde_json::Map::new();
                        if let Some(ref filter) = query_json.filter {
                            let mut filter_obj = serde_json::Map::new();
                            match filter {
                                cardstack_lib::query::Filter::All(preds) => {
                                    filter_obj.insert("op".to_string(), serde_json::Value::String("all".to_string()));
                                    filter_obj.insert("predicates".to_string(), serde_json::Value::Array(
                                        preds.iter().map(|p| serde_json::Value::String(p.clone())).collect()
                                    ));
                                }
                                cardstack_lib::query::Filter::Any(preds) => {
                                    filter_obj.insert("op".to_string(), serde_json::Value::String("any".to_string()));
                                    filter_obj.insert("predicates".to_string(), serde_json::Value::Array(
                                        preds.iter().map(|p| serde_json::Value::String(p.clone())).collect()
                                    ));
                                }
                                cardstack_lib::query::Filter::None(preds) => {
                                    filter_obj.insert("op".to_string(), serde_json::Value::String("none".to_string()));
                                    filter_obj.insert("predicates".to_string(), serde_json::Value::Array(
                                        preds.iter().map(|p| serde_json::Value::String(p.clone())).collect()
                                    ));
                                }
                            }
                            query_obj.insert("filter".to_string(), serde_json::Value::Object(filter_obj));
                        }
                        if !query_json.sort.is_empty() {
                            query_obj.insert("sort".to_string(), serde_json::Value::Array(
                                query_json.sort.iter().map(|s| serde_json::Value::String(s.clone())).collect()
                            ));
                        }
                        if let Some(limit) = query_json.limit {
                            query_obj.insert("limit".to_string(), serde_json::Value::Number(limit.into()));
                        }
                        collection.query = Some(serde_json::Value::Object(query_obj));
                    }
                    
                    let facets = Facets {
                        content: None,
                        collection: Some(collection),
                        template: None,
                    };
                    card.facets = Some(facets);
                    
                    let file_path = save_card(&repo, &mut card)?;
                    if !cli.json {
                        println!("Created deck: {} ({})", name, uid);
                        println!("Path: {}", file_path.display());
                    }
                }
                DeckCommands::Add { deck, cards } => {
                    let mut deck_card = load_card(&repo, deck)?;
                    
                    let facets = deck_card.facets.get_or_insert_with(|| Facets {
                        content: None,
                        collection: Some(CollectionFacet {
                            mode: CollectionMode::Static,
                            members: Vec::new(),
                            query: None,
                            view: None,
                        }),
                        template: None,
                    });
                    
                    let collection = facets.collection.get_or_insert_with(|| CollectionFacet {
                        mode: CollectionMode::Static,
                        members: Vec::new(),
                        query: None,
                        view: None,
                    });
                    
                    for card_id in cards {
                        let card = load_card(&repo, card_id)?;
                        if !collection.members.contains(&card.uid) {
                            collection.members.push(card.uid.clone());
                            deck_card.links.push(cardstack_lib::card::Link {
                                r#type: "contains".to_string(),
                                to: card.uid,
                            });
                        }
                    }
                    
                    save_card(&repo, &mut deck_card)?;
                    if !cli.json {
                        println!("Added {} card(s) to deck {}", cards.len(), deck);
                    }
                }
                DeckCommands::Remove { deck, cards } => {
                    let mut deck_card = load_card(&repo, deck)?;
                    
                    if let Some(facets) = &mut deck_card.facets {
                        if let Some(collection) = &mut facets.collection {
                            for card_id in cards {
                                let card = load_card(&repo, card_id)?;
                                collection.members.retain(|m| m != &card.uid);
                                deck_card.links.retain(|l| !(l.r#type == "contains" && l.to == card.uid));
                            }
                        }
                    }
                    
                    save_card(&repo, &mut deck_card)?;
                    if !cli.json {
                        println!("Removed {} card(s) from deck {}", cards.len(), deck);
                    }
                }
                DeckCommands::Snapshot { deck, out } => {
                    let deck_card = load_card(&repo, deck)?;
                    
                    let snapshot_uid = uid::generate_uid();
                    let snapshot_slug = out.clone();
                    
                    let mut snapshot = Card::new(
                        format!("{} (snapshot)", deck_card.title),
                        snapshot_slug.clone(),
                        snapshot_uid.clone(),
                    );
                    
                    // Determine snapshot members based on deck mode
                    let mut snapshot_members = Vec::new();
                    
                    if let Some(facets) = &deck_card.facets {
                        if let Some(collection) = &facets.collection {
                            match collection.mode {
                                CollectionMode::Query | CollectionMode::Hybrid => {
                                    // Resolve query deck members by executing the query
                                    let all_cards: Vec<(PathBuf, Card)> = {
                                        let cards_dir = repo.join("cards");
                                        if !cards_dir.exists() {
                                            Vec::new()
                                        } else {
                                            let mut cards = Vec::new();
                                            for entry in walkdir::WalkDir::new(&cards_dir) {
                                                let entry = entry?;
                                                let path = entry.path();
                                                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                                                    if let Ok(content) = fs::read_to_string(path) {
                                                        if let Ok((card, _)) = serialize::parse_card_file(&content) {
                                                            cards.push((path.to_path_buf(), card));
                                                        }
                                                    }
                                                }
                                            }
                                            cards
                                        }
                                    };
                                    
                                    // Parse and execute query
                                    if let Some(query_json) = &collection.query {
                                        let mut query_obj = cardstack_lib::query::Query {
                                            filter: None,
                                            sort: Vec::new(),
                                            limit: None,
                                        };
                                        
                                        if let Some(obj) = query_json.as_object() {
                                            // Parse filter
                                            if let Some(filter_val) = obj.get("filter") {
                                                if let Some(filter_obj) = filter_val.as_object() {
                                                    if let Some(op) = filter_obj.get("op").and_then(|v| v.as_str()) {
                                                        if let Some(preds_val) = filter_obj.get("predicates").and_then(|v| v.as_array()) {
                                                            let preds: Vec<String> = preds_val.iter()
                                                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                                                .collect();
                                                            query_obj.filter = match op {
                                                                "all" => Some(cardstack_lib::query::Filter::All(preds)),
                                                                "any" => Some(cardstack_lib::query::Filter::Any(preds)),
                                                                "none" => Some(cardstack_lib::query::Filter::None(preds)),
                                                                _ => None,
                                                            };
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // Parse sort
                                            if let Some(sort_val) = obj.get("sort").and_then(|v| v.as_array()) {
                                                query_obj.sort = sort_val.iter()
                                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                                    .collect();
                                            }
                                            
                                            // Parse limit
                                            if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                                                query_obj.limit = Some(limit_val as u32);
                                            }
                                        }
                                        
                                        let query = query_obj;
                                        
                                        // Execute query - reuse the same matching logic
                                        let mut results: Vec<Card> = all_cards.iter()
                                            .map(|(_, card)| card.clone())
                                            .collect();
                                        
                                        // Apply filter
                                        if let Some(ref filter) = query.filter {
                                            match filter {
                                                cardstack_lib::query::Filter::All(preds) => {
                                                    results.retain(|card| {
                                                        preds.iter().all(|p| {
                                                            if p.starts_with("tags contains") {
                                                                if let Some(tag) = p.split('"').nth(1) {
                                                                    return card.tags.contains(&tag.to_string());
                                                                }
                                                            }
                                                            if p.starts_with("fields.") && p.contains(" = ") {
                                                                let parts: Vec<&str> = p.split(" = ").collect();
                                                                if parts.len() == 2 {
                                                                    let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                                    let value = parts[1].trim_matches('"');
                                                                    if let Some(field_val) = card.fields.get(field) {
                                                                        if let Some(s) = field_val.as_str() {
                                                                            return s == value;
                                                                        }
                                                                        if let Some(b) = field_val.as_bool() {
                                                                            return value == if b { "true" } else { "false" };
                                                                        }
                                                                        if let Some(n) = field_val.as_i64() {
                                                                            if let Ok(v) = value.parse::<i64>() {
                                                                                return n == v;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            false
                                                        })
                                                    });
                                                }
                                                cardstack_lib::query::Filter::Any(preds) => {
                                                    results.retain(|card| {
                                                        preds.iter().any(|p| {
                                                            if p.starts_with("tags contains") {
                                                                if let Some(tag) = p.split('"').nth(1) {
                                                                    return card.tags.contains(&tag.to_string());
                                                                }
                                                            }
                                                            if p.starts_with("fields.") && p.contains(" = ") {
                                                                let parts: Vec<&str> = p.split(" = ").collect();
                                                                if parts.len() == 2 {
                                                                    let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                                    let value = parts[1].trim_matches('"');
                                                                    if let Some(field_val) = card.fields.get(field) {
                                                                        if let Some(s) = field_val.as_str() {
                                                                            return s == value;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            false
                                                        })
                                                    });
                                                }
                                                cardstack_lib::query::Filter::None(preds) => {
                                                    results.retain(|card| {
                                                        !preds.iter().any(|p| {
                                                            if p.starts_with("tags contains") {
                                                                if let Some(tag) = p.split('"').nth(1) {
                                                                    return card.tags.contains(&tag.to_string());
                                                                }
                                                            }
                                                            if p.starts_with("fields.") && p.contains(" = ") {
                                                                let parts: Vec<&str> = p.split(" = ").collect();
                                                                if parts.len() == 2 {
                                                                    let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
                                                                    let value = parts[1].trim_matches('"');
                                                                    if let Some(field_val) = card.fields.get(field) {
                                                                        if let Some(s) = field_val.as_str() {
                                                                            return s == value;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            false
                                                        })
                                                    });
                                                }
                                            }
                                        }
                                        
                                        // Apply sort
                                        if !query.sort.is_empty() {
                                            results.sort_by(|a, b| {
                                                for sort_key in &query.sort {
                                                    let descending = sort_key.starts_with('-');
                                                    let key = if descending {
                                                        &sort_key[1..]
                                                    } else {
                                                        sort_key.as_str()
                                                    };
                                                    
                                                    let cmp = match key {
                                                        "updated" => a.updated.cmp(&b.updated),
                                                        "created" => a.created.cmp(&b.created),
                                                        "title" => a.title.cmp(&b.title),
                                                        _ => std::cmp::Ordering::Equal,
                                                    };
                                                    
                                                    if cmp != std::cmp::Ordering::Equal {
                                                        return if descending { cmp.reverse() } else { cmp };
                                                    }
                                                }
                                                a.uid.cmp(&b.uid)
                                            });
                                        }
                                        
                                        // Apply limit
                                        if let Some(limit) = query.limit {
                                            results.truncate(limit as usize);
                                        }
                                        
                                        // Extract member UIDs
                                        snapshot_members = results.iter().map(|card| card.uid.clone()).collect();
                                    }
                                }
                                CollectionMode::Static => {
                                    // For static decks, use existing members
                                    snapshot_members = collection.members.clone();
                                }
                            }
                            
                            // Create snapshot collection with resolved members
                            let snapshot_collection = CollectionFacet {
                                mode: CollectionMode::Static,
                                members: snapshot_members,
                                query: None,
                                view: collection.view.clone(),
                            };
                            
                            snapshot.facets = Some(Facets {
                                content: None,
                                collection: Some(snapshot_collection),
                                template: None,
                            });
                        }
                    }
                    
                    snapshot.links.push(cardstack_lib::card::Link {
                        r#type: "snapshot-of".to_string(),
                        to: deck_card.uid.clone(),
                    });
                    
                    let file_path = save_card(&repo, &mut snapshot)?;
                    if !cli.json {
                        println!("Snapshotted deck {} to {}", deck, snapshot_uid);
                        println!("Path: {}", file_path.display());
                    }
                }
            }
        }
        Commands::Tui => {
            let repo = get_repo_root(cli.repo.clone())?;
            tui::run_tui(repo)?;
        }
    }

    Ok(())
}

