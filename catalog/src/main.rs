use anyhow::{Context, Result};
use cardstack_lib::{card::Card, serialize};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "catalog")]
#[command(about = "Build and manage card index", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Rebuild the SQLite index from card files
    Rebuild,
    /// Show index status and health
    Status,
    /// Optimize the database
    Vacuum,
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

fn get_index_path(repo: &Path) -> PathBuf {
    repo.join(".cardstack").join("index").join("cards.db")
}

fn init_database(db_path: &Path) -> Result<Connection> {
    // Create parent directory if needed
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let conn = Connection::open(db_path)?;
    
    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cards (
            uid TEXT PRIMARY KEY,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            created TEXT NOT NULL,
            updated TEXT NOT NULL,
            tags_json TEXT,
            fields_json TEXT,
            has_collection INTEGER DEFAULT 0,
            has_template INTEGER DEFAULT 0,
            path TEXT NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS links (
            src_uid TEXT NOT NULL,
            type TEXT NOT NULL,
            dst_uid TEXT NOT NULL,
            PRIMARY KEY (src_uid, type, dst_uid),
            FOREIGN KEY (src_uid) REFERENCES cards(uid),
            FOREIGN KEY (dst_uid) REFERENCES cards(uid)
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS computed (
            uid TEXT PRIMARY KEY,
            tokens INTEGER,
            nid_bpt REAL,
            cohesion REAL,
            bandwidth INTEGER,
            redundancy REAL,
            link_density REAL,
            structure_density REAL,
            sv REAL,
            last_analyzed TEXT,
            FOREIGN KEY (uid) REFERENCES cards(uid)
        )",
        [],
    )?;
    
    // Create FTS5 virtual table for full-text search
    // Try to drop existing table first (FTS5 tables need special handling)
    // Note: DROP on FTS5 virtual tables automatically removes shadow tables
    let _ = conn.execute("DROP TABLE IF EXISTS fts", []);
    // Create FTS table with correct schema (no IF NOT EXISTS since we just dropped it)
    conn.execute(
        "CREATE VIRTUAL TABLE fts USING fts5(
            uid UNINDEXED,
            body
        )",
        [],
    )?;
    
    Ok(conn)
}

fn load_all_cards(repo: &Path) -> Result<Vec<(PathBuf, Card)>> {
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
                    cards.push((path.to_path_buf(), card));
                }
            }
        }
    }
    
    Ok(cards)
}

fn rebuild_index(repo: &Path) -> Result<()> {
    let db_path = get_index_path(repo);
    
    // If database exists, drop FTS table explicitly before initializing
    // FTS5 virtual tables need special handling
    if db_path.exists() {
        let temp_conn = Connection::open(&db_path)?;
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts", []);
        // Also try to drop any shadow tables that FTS5 might create
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts_data", []);
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts_idx", []);
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts_config", []);
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts_docsize", []);
        let _ = temp_conn.execute("DROP TABLE IF EXISTS fts_content", []);
    }
    
    let conn = init_database(&db_path)?;
    
    println!("Loading cards from filesystem...");
    let cards = load_all_cards(repo)?;
    println!("Found {} card(s)", cards.len());
    
    // Clear existing data
    conn.execute("DELETE FROM links", [])?;
    conn.execute("DELETE FROM computed", [])?;
    conn.execute("DELETE FROM cards", [])?;
    
    // Insert cards
    let mut stmt_card = conn.prepare(
        "INSERT OR REPLACE INTO cards (uid, slug, title, created, updated, tags_json, fields_json, has_collection, has_template, path) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )?;
    
    let mut stmt_link = conn.prepare(
        "INSERT OR REPLACE INTO links (src_uid, type, dst_uid) VALUES (?1, ?2, ?3)"
    )?;
    
    let mut stmt_fts = conn.prepare(
        "INSERT INTO fts (uid, body) VALUES (?1, ?2)"
    )?;
    
    for (path, card) in &cards {
        let tags_json = serde_json::to_string(&card.tags)?;
        let fields_json = serde_json::to_string(&card.fields)?;
        let path_str = path.to_string_lossy().to_string();
        let relative_path = path_str.strip_prefix(repo.to_string_lossy().as_ref())
            .map(|s| s.trim_start_matches(std::path::MAIN_SEPARATOR))
            .unwrap_or(&path_str);
        
        stmt_card.execute(params![
            card.uid,
            card.slug,
            card.title,
            card.created.to_rfc3339(),
            card.updated.to_rfc3339(),
            tags_json,
            fields_json,
            if card.has_collection() { 1 } else { 0 },
            if card.has_template() { 1 } else { 0 },
            relative_path,
        ])?;
        
        // Insert links
        for link in &card.links {
            stmt_link.execute(params![
                card.uid,
                link.r#type,
                link.to,
            ])?;
        }
        
        // Insert FTS content
        if let Some(body) = card.get_content() {
            stmt_fts.execute(params![
                card.uid,
                body,
            ])?;
        }
    }
    
    println!("Index rebuilt successfully");
    println!("  - {} cards indexed", cards.len());
    
    Ok(())
}

fn show_status(repo: &Path) -> Result<()> {
    let db_path = get_index_path(repo);
    
    if !db_path.exists() {
        println!("Index does not exist. Run 'catalog rebuild' to create it.");
        return Ok(());
    }
    
    let conn = Connection::open(&db_path)?;
    
    let card_count: i64 = conn.query_row("SELECT COUNT(*) FROM cards", [], |row| row.get(0))?;
    let link_count: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))?;
    let fts_count: i64 = conn.query_row("SELECT COUNT(*) FROM fts", [], |row| row.get(0))?;
    
    println!("Index Status:");
    println!("  Database: {}", db_path.display());
    println!("  Cards: {}", card_count);
    println!("  Links: {}", link_count);
    println!("  FTS entries: {}", fts_count);
    
    // Check staleness (compare file count)
    let cards = load_all_cards(repo)?;
    println!("  Filesystem cards: {}", cards.len());
    
    if card_count != cards.len() as i64 {
        println!("  ⚠️  Index may be stale (count mismatch)");
    } else {
        println!("  ✓ Index is up to date");
    }
    
    Ok(())
}

fn vacuum_database(repo: &Path) -> Result<()> {
    let db_path = get_index_path(repo);
    
    if !db_path.exists() {
        anyhow::bail!("Index does not exist. Run 'catalog rebuild' first.");
    }
    
    let conn = Connection::open(&db_path)?;
    conn.execute("VACUUM", [])?;
    
    println!("Database optimized");
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    
    match &cli.command {
        Commands::Rebuild => {
            rebuild_index(&repo)?;
        }
        Commands::Status => {
            show_status(&repo)?;
        }
        Commands::Vacuum => {
            vacuum_database(&repo)?;
        }
    }
    
    Ok(())
}
