use anyhow::{Context, Result};
use cardstack_lib::{get_repo_root, load_all_cards};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "album")]
#[command(about = "Manage albums and card publications", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new album
    Create {
        /// Name of the album
        name: String,
    },
    /// List all albums
    List,
    /// Add a card to an album
    Add {
        /// Album name
        album: String,
        /// Card UID or slug
        card: String,
    },
    /// Transfer a card to another album
    Transfer {
        /// Source album name
        from: String,
        /// Destination album name
        to: String,
        /// Card UID or slug
        card: String,
    },
    /// Archive an album
    Archive {
        /// Album name
        name: String,
    },
    /// List cards in an album
    Show {
        /// Album name
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Album {
    name: String,
    cards: Vec<String>,
    archived: bool,
}

fn get_albums_file_path(repo: &Path) -> PathBuf {
    repo.join(".cardstack").join("albums.json")
}

fn load_albums(repo: &Path) -> Result<Vec<Album>> {
    let albums_file = get_albums_file_path(repo);
    if !albums_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(albums_file)?;
    let albums: Vec<Album> = serde_json::from_str(&content)?;
    Ok(albums)
}

fn save_albums(repo: &Path, albums: &[Album]) -> Result<()> {
    let albums_file = get_albums_file_path(repo);
    fs::create_dir_all(albums_file.parent().unwrap())?;
    let content = serde_json::to_string_pretty(albums)?;
    fs::write(albums_file, content)?;
    Ok(())
}

fn find_album(albums: &[Album], name: &str) -> Option<usize> {
    albums.iter().position(|a| a.name == name)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;

    match &cli.command {
        Commands::Create { name } => {
            let mut albums = load_albums(&repo)?;
            if find_album(&albums, name).is_some() {
                anyhow::bail!("Album '{}' already exists", name);
            }
            albums.push(Album {
                name: name.clone(),
                cards: Vec::new(),
                archived: false,
            });
            save_albums(&repo, &albums)?;
            println!("Created album: {}", name);
        }
        Commands::List => {
            let albums = load_albums(&repo)?;
            if albums.is_empty() {
                println!("No albums found");
                return Ok(());
            }
            println!("Albums:");
            for album in albums {
                let status = if album.archived { " (archived)" } else { "" };
                println!("- {}{}", album.name, status);
            }
        }
        Commands::Add { album, card } => {
            let mut albums = load_albums(&repo)?;
            let album_index =
                find_album(&albums, album).context(format!("Album '{}' not found", album))?;
            let all_cards = load_all_cards(&repo)?;
            let card = all_cards
                .into_iter()
                .find(|(_, c)| c.uid == *card || c.slug == *card)
                .context("Card not found")?;
            let card_uid = card.1.uid.clone();
            if !albums[album_index].cards.contains(&card_uid) {
                albums[album_index].cards.push(card_uid.clone());
                save_albums(&repo, &albums)?;
                println!("Added card {} to album {}", card_uid, album);
            } else {
                println!("Card {} is already in album {}", card_uid, album);
            }
        }
        Commands::Transfer { from, to, card } => {
            let mut albums = load_albums(&repo)?;
            let from_index =
                find_album(&albums, from).context(format!("Album '{}' not found", from))?;
            let to_index = find_album(&albums, to).context(format!("Album '{}' not found", to))?;
            let all_cards = load_all_cards(&repo)?;
            let card = all_cards
                .into_iter()
                .find(|(_, c)| c.uid == *card || c.slug == *card)
                .context("Card not found")?;
            let card_uid = card.1.uid.clone();
            if !albums[from_index].cards.contains(&card_uid) {
                anyhow::bail!("Card {} is not in album {}", card_uid, from);
            }
            if albums[to_index].cards.contains(&card_uid) {
                anyhow::bail!("Card {} is already in album {}", card_uid, to);
            }
            albums[from_index].cards.retain(|uid| uid != &card_uid);
            albums[to_index].cards.push(card_uid.clone());
            save_albums(&repo, &albums)?;
            println!("Transferred card {} from {} to {}", card_uid, from, to);
        }
        Commands::Archive { name } => {
            let mut albums = load_albums(&repo)?;
            let album_index =
                find_album(&albums, name).context(format!("Album '{}' not found", name))?;
            albums[album_index].archived = true;
            save_albums(&repo, &albums)?;
            println!("Archived album: {}", name);
        }
        Commands::Show { name } => {
            let albums = load_albums(&repo)?;
            let album = albums
                .iter()
                .find(|a| a.name == *name)
                .context(format!("Album '{}' not found", name))?;
            if album.cards.is_empty() {
                println!("Album '{}' has no cards", name);
                return Ok(());
            }
            println!("Cards in album '{}':", name);
            let all_cards = load_all_cards(&repo)?;
            for card_uid in &album.cards {
                if let Some((_, card)) = all_cards.iter().find(|(_, c)| c.uid == *card_uid) {
                    println!("- {} ({})", card.title, card_uid);
                }
            }
        }
    }

    Ok(())
}
