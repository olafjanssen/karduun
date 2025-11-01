use anyhow::{Context, Result};
use cardstack_lib::{card::{Card, Facets, CollectionFacet, CollectionMode}, serialize, uid};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "curator")]
#[command(about = "Apply organization plans", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    
    #[arg(long, global = true)]
    yes: bool,
    
    #[arg(long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert AnalysisResult stream to OrgAction plan
    Plan {
        #[arg(long)]
        rules: Option<String>,
    },
    /// Execute OrgAction stream (mutate cards)
    Apply,
    /// One-shot: analyze → plan → apply
    Autoclean {
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        split_thresh: Option<String>,
        #[arg(long)]
        merge_thresh: Option<String>,
        #[arg(long)]
        prune_thresh: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct AnalysisResult {
    #[serde(rename = "type")]
    result_type: String,
    uid: String,
    computed: cardstack_lib::card::Computed,
    suggestion: String,
    rationale: String,
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OrgAction {
    #[serde(rename = "type")]
    action_type: String,
    uid: String,
    action: String,
    params: Option<serde_json::Value>,
    why: String,
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

fn load_card(repo: &Path, uid: &str) -> Result<Card> {
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
                    if card.uid == uid || card.slug == uid {
                        return Ok(card);
                    }
                }
            }
        }
    }
    
    anyhow::bail!("Card not found: {}", uid)
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

fn log_action(repo: &Path, action: &OrgAction) -> Result<()> {
    let logs_dir = repo.join(".cardstack").join("logs");
    fs::create_dir_all(&logs_dir)?;
    
    let log_file = logs_dir.join(format!("actions_{}.ndjson", 
        chrono::Utc::now().format("%Y%m%d")));
    
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    
    let line = serde_json::to_string(action)?;
    writeln!(file, "{}", line)?;
    
    Ok(())
}

fn split_card(repo: &Path, card: Card, _params: &serde_json::Value) -> Result<()> {
    let body = card.get_content().unwrap_or("");
    let sentences: Vec<&str> = body.split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty() && s.len() > 20)
        .collect();
    
    if sentences.is_empty() {
        println!("  ⚠️  Card has no content to split");
        return Ok(());
    }
    
    // Simple split: divide into chunks
    let chunks = sentences.len().div_ceil(3).max(1);
    let chunk_size = sentences.len().div_ceil(chunks);
    
    let mut children = Vec::new();
    for (i, chunk) in sentences.chunks(chunk_size).enumerate() {
        let child_uid = uid::generate_uid();
        let child_slug = format!("{}-part{}", card.slug, i + 1);
        let child_title = format!("{} (Part {})", card.title, i + 1);
        
        let mut child = Card::new(child_title.clone(), child_slug.clone(), child_uid.clone());
        child = child.with_content(chunk.join(". "));
        child.tags = card.tags.clone();
        child.keywords = card.keywords.clone();
        child.fields = card.fields.clone();
        child.links.push(cardstack_lib::card::Link {
            r#type: "part-of".to_string(),
            to: card.uid.clone(),
        });
        
        save_card(repo, &mut child)?;
        children.push(child_uid);
    }
    
    // Convert parent to deck
    let mut parent = card;
    let collection = CollectionFacet {
        mode: CollectionMode::Static,
        members: children.clone(),
        query: None,
        view: None,
    };
    
    parent.facets = Some(Facets {
        content: Some(cardstack_lib::card::ContentFacet {
            mime: "text/markdown".to_string(),
            body: format!("# {}\n\nThis card was split into {} parts:\n\n{}", 
                parent.title,
                children.len(),
                children.iter().map(|uid| format!("- [Card {}]({})", uid, uid)).collect::<Vec<_>>().join("\n")
            ),
        }),
        collection: Some(collection),
        template: None,
    });
    
    // Add contains links
    for child_uid in &children {
        parent.links.push(cardstack_lib::card::Link {
            r#type: "contains".to_string(),
            to: child_uid.clone(),
        });
    }
    
    save_card(repo, &mut parent)?;
    println!("  ✓ Split into {} child cards", children.len());
    
    Ok(())
}

fn merge_card(repo: &Path, src_uid: &str, dst_uid: Option<&str>) -> Result<()> {
    let src_card = load_card(repo, src_uid)?;
    let dst_uid = dst_uid.unwrap_or_else(|| {
        // Find nearest neighbor by tags
        // For now, just use first available
        src_uid
    });
    
    if dst_uid == src_uid {
        println!("  ⚠️  Cannot merge card into itself");
        return Ok(());
    }
    
    let mut dst_card = load_card(repo, dst_uid)?;
    
    // Merge bodies
    let src_body = src_card.get_content().unwrap_or("");
    let dst_body = dst_card.get_content().unwrap_or("");
    let merged_body = if dst_body.is_empty() {
        src_body.to_string()
    } else if src_body.is_empty() {
        dst_body.to_string()
    } else {
        format!("{}\n\n---\n\n{}", dst_body, src_body)
    };
    dst_card = dst_card.with_content(merged_body);
    
    // Merge tags (union)
    for tag in &src_card.tags {
        if !dst_card.tags.contains(tag) {
            dst_card.tags.push(tag.clone());
        }
    }
    
    // Merge fields
    for (k, v) in &src_card.fields {
        if !dst_card.fields.contains_key(k) {
            dst_card.fields.insert(k.clone(), v.clone());
        }
    }
    
    // Add provenance
    dst_card.links.push(cardstack_lib::card::Link {
        r#type: "derived-from".to_string(),
        to: src_card.uid.clone(),
    });
    
    // Archive source
    let mut archived = src_card;
    archived.fields.insert("archived".to_string(), serde_json::Value::Bool(true));
    archived.fields.insert("merged_into".to_string(), serde_json::Value::String(dst_card.uid.clone()));
    save_card(repo, &mut archived)?;
    
    save_card(repo, &mut dst_card)?;
    println!("  ✓ Merged {} into {}", src_uid, dst_uid);
    
    Ok(())
}

fn prune_card(repo: &Path, card: Card) -> Result<()> {
    let mut pruned = card;
    pruned.fields.insert("archived".to_string(), serde_json::Value::Bool(true));
    pruned.fields.insert("pruned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    
    save_card(repo, &mut pruned)?;
    println!("  ✓ Pruned card {}", pruned.uid);
    
    Ok(())
}

fn refactor_card(repo: &Path, mut card: Card) -> Result<()> {
    let body = card.get_content().unwrap_or("");
    
    // Simple refactoring: ensure first-level headings
    let mut refactored = if !body.trim_start().starts_with('#') {
        format!("# {}\n\n{}", card.title, body)
    } else {
        body.to_string()
    };
    
    // Add section markers if missing
    if !refactored.contains("\n\n## ") && refactored.lines().count() > 10 {
        // Try to infer sections from line breaks
        let lines: Vec<&str> = refactored.lines().collect();
        let mut new_lines = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            new_lines.push(*line);
            if i > 0 && i < lines.len() - 1 && line.is_empty() && !lines[i-1].is_empty() {
                // Potential section break - could add heading here
            }
        }
        
        refactored = new_lines.join("\n");
    }
    
    card = card.with_content(refactored);
    save_card(repo, &mut card)?;
    println!("  ✓ Refactored card {}", card.uid);
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    
    match &cli.command {
        Commands::Plan { rules: _ } => {
            // Read AnalysisResult from stdin
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                
                let analysis: AnalysisResult = serde_json::from_str(&line)?;
                
                // Convert to OrgAction
                let action = OrgAction {
                    action_type: "org_action".to_string(),
                    uid: analysis.uid,
                    action: analysis.suggestion.clone(),
                    params: if analysis.suggestion == "split" {
                        Some(serde_json::json!({"strategy": "clusters"}))
                    } else {
                        None
                    },
                    why: analysis.rationale,
                };
                
                println!("{}", serde_json::to_string(&action)?);
            }
        }
        Commands::Apply => {
            let stdin = io::stdin();
            let mut actions = Vec::new();
            
            for line in stdin.lock().lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                
                let action: OrgAction = serde_json::from_str(&line)?;
                actions.push(action);
            }
            
            if cli.dry_run {
                println!("DRY RUN - Would apply {} action(s):", actions.len());
                for action in &actions {
                    println!("  {}: {} - {}", action.uid, action.action, action.why);
                }
            } else {
                if !cli.yes {
                    eprintln!("⚠️  This will mutate cards. Use --yes to confirm.");
                    return Ok(());
                }
                
                println!("Applying {} action(s)...", actions.len());
                for action in &actions {
                    println!("Processing {} ({})...", action.uid, action.action);
                    
                    let card = load_card(&repo, &action.uid)?;
                    
                    match action.action.as_str() {
                        "split" => {
                            split_card(&repo, card, action.params.as_ref().unwrap_or(&serde_json::Value::Null))?;
                        }
                        "merge" => {
                            let dst_uid = action.params.as_ref()
                                .and_then(|p| p.get("into"))
                                .and_then(|v| v.as_str());
                            merge_card(&repo, &action.uid, dst_uid)?;
                        }
                        "prune" => {
                            prune_card(&repo, card)?;
                        }
                        "refactor" => {
                            refactor_card(&repo, card)?;
                        }
                        _ => {
                            println!("  ⚠️  Unknown action: {}", action.action);
                        }
                    }
                    
                    log_action(&repo, action)?;
                }
                
                println!("✓ Completed");
            }
        }
        Commands::Autoclean { apply, split_thresh: _, merge_thresh: _, prune_thresh: _ } => {
            // One-shot: analyze → plan → apply
            println!("Running autoclean (analyze → plan → apply)...");
            
            // For now, just analyze all and suggest
            println!("Use: scout list --jsonl | gauge analyze --jsonl | curator plan | curator apply --yes");
            
            if *apply {
                println!("  (--apply flag: in future, will automatically execute)");
            }
        }
    }
    
    Ok(())
}
