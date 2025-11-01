use anyhow::{Context, Result};
use cardstack_lib::{card::Card, serialize, uid};
use clap::{Parser, Subcommand};
use std::fs;
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

fn card_path(repo: &Path, card: &Card) -> PathBuf {
    let year = card.created.format("%Y").to_string();
    let month = card.created.format("%m").to_string();
    repo.join("cards").join(&year).join(&month)
}

fn find_card_file(repo: &Path, identifier: &str) -> Result<PathBuf> {
    // Try to find by uid or slug
    let cards_dir = repo.join("cards");
    
    if !cards_dir.exists() {
        anyhow::bail!("Cards directory not found");
    }
    
    for entry in walkdir::WalkDir::new(&cards_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                // Check if identifier matches uid or slug in filename
                if name.starts_with(identifier) || name.contains(&format!("--{}", identifier)) {
                    return Ok(path.to_path_buf());
                }
            }
        }
    }
    
    anyhow::bail!("Card not found: {}", identifier)
}

fn load_card(repo: &Path, identifier: &str) -> Result<Card> {
    let card_file = find_card_file(repo, identifier)?;
    let content = fs::read_to_string(&card_file)?;
    let (card, _) = serialize::parse_card_file(&content)?;
    Ok(card)
}

fn save_card(repo: &Path, card: &mut Card) -> Result<PathBuf> {
    // Update timestamps
    card.updated = chrono::Utc::now();
    if card.version == 0 {
        card.version = 1;
    }
    
    // Determine path
    let dir = card_path(repo, card);
    fs::create_dir_all(&dir)?;
    
    let filename = format!("{}--{}.yaml", card.uid, card.slug);
    let file_path = dir.join(&filename);
    
    // Serialize and write
    let content = serialize::write_card_file(card)?;
    fs::write(&file_path, content)?;
    
    Ok(file_path)
}

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
        _ => {
            eprintln!("Command not yet implemented");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

