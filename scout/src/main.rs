use anyhow::{Context, Result};
use cardstack_lib::{card::Card, card::CardEnvelope, query, repository::{get_repo_root, load_all_cards}};
use clap::{Parser, Subcommand};
use std::path::{PathBuf};

mod tui;

#[derive(Parser)]
#[command(name = "scout")]
#[command(about = "Query and search cards", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    #[arg(long, global = true)]
    jsonl: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List cards matching query
    List {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Full-text search in card content
    Grep {
        pattern: String,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        paths: bool,
    },
    /// Show backlinks to a card
    Backlinks {
        identifier: String,
    },
    /// Show hierarchical tree via parent-of links
    Tree {
        identifier: String,
        #[arg(long, default_value = "10")]
        depth: u32,
    },
    /// Interactive TUI for browsing cards
    Tui,
}

fn matches_filter(card: &Card, predicate: &str) -> bool {
    // Simple predicate matching (can be enhanced)
    if predicate.contains("tags contains") {
        if let Some(tag) = predicate.split('"').nth(1) {
            return card.tags.contains(&tag.to_string());
        }
    } else if predicate.starts_with("fields.") && predicate.contains(" = ") {
        let parts: Vec<&str> = predicate.split(" = ").collect();
        if parts.len() == 2 {
            let field = parts[0].strip_prefix("fields.").unwrap_or(parts[0]);
            let value = parts[1].trim_matches('"');
            if let Some(field_val) = card.fields.get(field) {
                return field_val.as_str().map(|s| s == value).unwrap_or(false);
            }
        }
    } else if predicate == "has:collection" {
        return card.has_collection();
    } else if predicate == "has:template" {
        return card.has_template();
    }

    false
}

fn execute_query(cards: Vec<(PathBuf, Card)>, q: Option<&query::Query>) -> Vec<Card> {
    let mut results: Vec<Card> = cards.into_iter().map(|(_, card)| card).collect();

    if let Some(query) = q {
        // Apply filter
        if let Some(ref filter) = query.filter {
            match filter {
                query::Filter::All(preds) => {
                    results.retain(|card| {
                        preds.iter().all(|p| matches_filter(card, p))
                    });
                }
                query::Filter::Any(preds) => {
                    results.retain(|card| {
                        preds.iter().any(|p| matches_filter(card, p))
                    });
                }
                query::Filter::None(preds) => {
                    results.retain(|card| {
                        !preds.iter().any(|p| matches_filter(card, p))
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
                a.uid.cmp(&b.uid) // Tiebreaker
            });
        }

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }
    }

    results
}

fn grep_filter(cards: Vec<Card>, pattern: &str) -> Vec<Card> {
    cards.into_iter()
        .filter(|card| {
            card.get_content()
                .map(|body| body.contains(pattern))
                .unwrap_or(false)
        })
        .collect()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;

    match &cli.command {
        Commands::List { query: query_str, sort, limit } => {
            let all_cards = load_all_cards(&repo)?;

            // Parse query
            let parsed_query = if let Some(q) = query_str {
                let mut q_obj = query::parse_query_shorthand(q)?;
                if let Some(s) = sort {
                    q_obj.sort = s.split(',').map(|s| s.to_string()).collect();
                }
                if let Some(l) = limit {
                    q_obj.limit = Some(*l);
                }
                Some(q_obj)
            } else {
                None
            };

            let results = execute_query(all_cards, parsed_query.as_ref());

            if cli.jsonl {
                for card in results {
                    let envelope = CardEnvelope::from(card);
                    println!("{}", serde_json::to_string(&envelope)?);
                }
            } else {
                println!("Found {} card(s)", results.len());
                for card in results {
                    println!("  {} - {}", card.uid, card.title);
                }
            }
        }
        Commands::Grep { pattern, query: query_str, paths } => {
            let all_cards = load_all_cards(&repo)?;

            // Parse query if provided
            let parsed_query = query_str.as_ref()
                .map(|q| query::parse_query_shorthand(q))
                .transpose()?;

            let mut matching_cards = execute_query(all_cards, parsed_query.as_ref());

            // Filter by pattern in content
            matching_cards = grep_filter(matching_cards, pattern);

            if cli.jsonl || *paths {
                for card in matching_cards {
                    if *paths {
                        let path = format!("cards/{}/{}/{}--{}.yaml",
                            card.created.format("%Y"),
                            card.created.format("%m"),
                            card.uid,
                            card.slug);
                        println!("{}", path);
                    } else {
                        let envelope = CardEnvelope::from(card);
                        println!("{}", serde_json::to_string(&envelope)?);
                    }
                }
            } else {
                println!("Found {} matching card(s)", matching_cards.len());
                for card in matching_cards {
                    println!("  {} - {}", card.uid, card.title);
                }
            }
        }
        Commands::Backlinks { identifier } => {
            let all_cards = load_all_cards(&repo)?;

            // Find target card
            let target = all_cards.iter()
                .find(|(_, card)| card.uid == *identifier || card.slug == *identifier)
                .map(|(_, card)| &card.uid)
                .context("Card not found")?;

            // Find cards that link to target
            let backlinks: Vec<_> = all_cards.iter()
                .filter(|(_, card)| {
                    card.links.iter().any(|l| l.to == *target)
                })
                .collect();

            if cli.jsonl {
                for (_, card) in backlinks {
                    let envelope = CardEnvelope::from(card.clone());
                    println!("{}", serde_json::to_string(&envelope)?);
                }
            } else {
                println!("Backlinks to {}:", identifier);
                for (_, card) in backlinks {
                    println!("  {} - {}", card.uid, card.title);
                }
            }
        }
        Commands::Tree { identifier, depth } => {
            let all_cards = load_all_cards(&repo)?;
            let cards_map: std::collections::HashMap<_, _> = all_cards.iter()
                .map(|(_, card)| (card.uid.clone(), card.clone()))
                .collect();

            // Find root
            let root = all_cards.iter()
                .find(|(_, card)| card.uid == *identifier || card.slug == *identifier)
                .map(|(_, card)| &card.uid)
                .context("Card not found")?;

            fn print_tree(
                uid: &str,
                cards: &std::collections::HashMap<String, Card>,
                depth: u32,
                max_depth: u32,
                prefix: &str,
            ) {
                if depth > max_depth {
                    return;
                }

                if let Some(card) = cards.get(uid) {
                    println!("{}{} - {}", prefix, card.uid, card.title);

                    let child_links: Vec<_> = card.links.iter()
                        .filter(|l| l.r#type == "parent-of" || l.r#type == "contains")
                        .collect();

                    for (i, link) in child_links.iter().enumerate() {
                        let is_last = i == child_links.len() - 1;
                        let new_prefix = format!("{}  {}", prefix, if is_last { "└─" } else { "├─" });
                        print_tree(&link.to, cards, depth + 1, max_depth, &new_prefix);
                    }
                }
            }

            print_tree(root, &cards_map, 0, *depth, "");
        }
        Commands::Tui => {
            let all_cards = load_all_cards(&repo)?;
            let cards: Vec<Card> = all_cards.into_iter().map(|(_, card)| card).collect();
            tui::run_tui(cards)?;
        }
    }

    Ok(())
}
