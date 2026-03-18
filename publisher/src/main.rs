use anyhow::{Context, Result};
use cardstack_lib::{get_repo_root, load_all_cards, save_card};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "publisher")]
#[command(about = "Publish cards to albums", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish a card to an album
    Publish {
        /// Album name
        #[arg(default_value = "general")]
        album: String,
        /// Card UID or slug
        card: String,
    },
    /// Unpublish a card from an album
    Unpublish {
        /// Album name
        #[arg(default_value = "general")]
        album: String,
        /// Card UID or slug
        card: String,
    },
    /// List publications for a card
    List {
        /// Card UID or slug
        card: String,
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
        Commands::Publish { album, card } => {
            let mut all_cards = load_all_cards(&repo)?;
            let card = all_cards
                .iter_mut()
                .find(|(_, c)| c.uid == *card || c.slug == *card)
                .context("Card not found")?;
            let card_uid = card.1.uid.clone();
            if !card.1.publications.contains(&album) {
                card.1.publications.push(album.clone());
                save_card(&repo, &mut card.1)?;

                // Add card to album's card list
                let mut albums = load_albums(&repo)?;
                let album_index = match find_album(&albums, &album) {
                    Some(index) => index,
                    None => {
                        albums.push(Album {
                            name: album.clone(),
                            cards: Vec::new(),
                            archived: false,
                        });
                        albums.len() - 1
                    }
                };
                if !albums[album_index].cards.contains(&card_uid) {
                    albums[album_index].cards.push(card_uid.clone());
                    save_albums(&repo, &albums)?;
                }

                println!("Published card {} to album {}", card_uid, album);
            } else {
                println!("Card {} is already published to album {}", card_uid, album);
            }
        }
        Commands::Unpublish { album, card } => {
            let mut all_cards = load_all_cards(&repo)?;
            let card = all_cards
                .iter_mut()
                .find(|(_, c)| c.uid == *card || c.slug == *card)
                .context("Card not found")?;
            let card_uid = card.1.uid.clone();
            if card.1.publications.contains(&album) {
                card.1.publications.retain(|a| a != album);
                save_card(&repo, &mut card.1)?;

                // Remove card from album's card list
                let mut albums = load_albums(&repo)?;
                if let Some(album_index) = find_album(&albums, &album) {
                    albums[album_index].cards.retain(|uid| uid != &card_uid);
                    save_albums(&repo, &albums)?;
                }

                println!("Unpublished card {} from album {}", card_uid, album);
            } else {
                println!("Card {} is not published to album {}", card_uid, album);
            }
        }
        Commands::List { card } => {
            let all_cards = load_all_cards(&repo)?;
            let card = all_cards
                .iter()
                .find(|(_, c)| c.uid == *card || c.slug == *card)
                .context("Card not found")?;
            let card_uid = card.1.uid.clone();
            if card.1.publications.is_empty() {
                println!("Card {} has no publications", card_uid);
            } else {
                println!("Publications for card {}:", card_uid);
                for album in &card.1.publications {
                    println!("- {}", album);
                }
            }
        }
    }

    Ok(())
}
