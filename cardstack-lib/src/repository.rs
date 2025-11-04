use crate::card::Card;
use crate::serialize;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find the repository root by walking up from the given path
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
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

/// Get repository root, with optional override
pub fn get_repo_root(repo_override: Option<PathBuf>) -> Result<PathBuf> {
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

/// Load all cards from the repository
pub fn load_all_cards(repo: &Path) -> Result<Vec<(PathBuf, Card)>> {
    let cards_dir = repo.join("cards");
    if !cards_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut cards = Vec::new();
    for entry in WalkDir::new(&cards_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok((card, _)) = serialize::parse_card_file(&content) {
                    cards.push((path.to_path_buf(), card));
                }
            }
        }
    }
    
    Ok(cards)
}

/// Find a card file by UID or slug
pub fn find_card_file(repo: &Path, identifier: &str) -> Result<PathBuf> {
    let cards_dir = repo.join("cards");
    
    if !cards_dir.exists() {
        anyhow::bail!("Cards directory not found");
    }
    
    for entry in WalkDir::new(&cards_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok((card, _)) = serialize::parse_card_file(&content) {
                    if card.uid == identifier || card.slug == identifier {
                        return Ok(path.to_path_buf());
                    }
                }
            }
        }
    }
    
    anyhow::bail!("Card not found: {}", identifier)
}

/// Load a single card by UID or slug
pub fn load_card(repo: &Path, identifier: &str) -> Result<Card> {
    let card_file = find_card_file(repo, identifier)?;
    let content = std::fs::read_to_string(&card_file)?;
    let (card, _) = serialize::parse_card_file(&content)?;
    Ok(card)
}

/// Get the directory path where a card should be stored
pub fn card_directory(repo: &Path, card: &Card) -> PathBuf {
    let year = card.created.format("%Y").to_string();
    let month = card.created.format("%m").to_string();
    repo.join("cards").join(&year).join(&month)
}

/// Save a card to the repository
/// Updates timestamp and version, handles file moves if needed
pub fn save_card(repo: &Path, card: &mut Card) -> Result<PathBuf> {
    // Update timestamps
    card.updated = chrono::Utc::now();
    card.version += 1;
    
    // Determine path
    let dir = card_directory(repo, card);
    std::fs::create_dir_all(&dir)?;
    
    let filename = format!("{}--{}.yaml", card.uid, card.slug);
    let file_path = dir.join(&filename);
    
    // If card file exists elsewhere (e.g., after edit that moved it), remove old file
    let old_file = find_card_file(repo, &card.uid).ok();
    if let Some(old) = &old_file {
        if old != &file_path && old.exists() {
            std::fs::remove_file(old)?;
        }
    }
    
    // Serialize and write
    let content = serialize::write_card_file(card)?;
    std::fs::write(&file_path, content)?;
    
    Ok(file_path)
}

/// Save a card without updating timestamp/version (for operations that preserve metadata)
pub fn save_card_preserve_metadata(repo: &Path, card: &Card) -> Result<PathBuf> {
    let dir = card_directory(repo, card);
    std::fs::create_dir_all(&dir)?;
    
    let filename = format!("{}--{}.yaml", card.uid, card.slug);
    let file_path = dir.join(&filename);
    
    let content = serialize::write_card_file(card)?;
    std::fs::write(&file_path, content)?;
    
    Ok(file_path)
}

