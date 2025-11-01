use anyhow::{Context, Result};
use cardstack_lib::{card::{Card, Computed}, query, serialize};
use clap::{Parser, Subcommand};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "gauge")]
#[command(about = "Semantic Volume analyzer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    
    #[arg(long, global = true)]
    jsonl: bool,
    
    #[arg(long, default_value = "full")]
    analyzer: String,
    
    #[arg(long)]
    no_embeddings: bool,
    
    #[arg(long, default_value = "100")]
    neighbors: u32,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze cards and compute semantic volume metrics
    Analyze {
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

/// Count tokens (words) in text
fn count_tokens(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

/// Compute Normalized Information Density (bits per token) via compression
fn compute_nid_bpt(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    
    let tokens = count_tokens(text);
    if tokens == 0 {
        return 0.0;
    }
    
    // Compress and measure
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes()).ok();
    let compressed = encoder.finish().unwrap_or_default();
    
    // NID = 8 * compressed_bytes / tokens
    8.0 * compressed.len() as f64 / tokens as f64
}

/// Extract sentences from text (simple heuristic)
fn extract_sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() > 10) // Filter very short fragments
        .collect()
}

/// Compute structure density (headings, bullets, codeblocks per 100 tokens)
fn compute_structure_density(text: &str, tokens: u32) -> f64 {
    if tokens == 0 {
        return 0.0;
    }
    
    let headings = text.matches('#').count();
    let bullets = text.matches('-').count() + text.matches('*').count();
    let codeblocks = text.matches("```").count() / 2;
    
    let structures = headings + bullets + codeblocks;
    (structures as f64 / tokens as f64) * 100.0
}

/// Compute link density (links per 100 tokens)
fn compute_link_density(link_count: usize, tokens: u32) -> f64 {
    if tokens == 0 {
        return 0.0;
    }
    (link_count as f64 / tokens as f64) * 100.0
}

/// Placeholder for cohesion (mean pairwise cosine similarity)
/// In full implementation, this would use sentence embeddings
fn compute_cohesion_placeholder(sentences: &[String]) -> f64 {
    if sentences.len() < 2 {
        return 1.0;
    }
    
    // Placeholder: simple heuristic based on shared words
    let mut similarities = Vec::new();
    for i in 0..sentences.len().min(50) { // Limit to avoid O(n²) explosion
        for j in (i + 1)..sentences.len().min(50) {
            let words_i: std::collections::HashSet<_> = sentences[i]
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();
            let words_j: std::collections::HashSet<_> = sentences[j]
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();
            
            let intersection = words_i.intersection(&words_j).count();
            let union = words_i.union(&words_j).count();
            
            if union > 0 {
                let similarity = intersection as f64 / union as f64;
                similarities.push(similarity);
            }
        }
    }
    
    if similarities.is_empty() {
        0.5
    } else {
        similarities.iter().sum::<f64>() / similarities.len() as f64
    }
}

/// Placeholder for bandwidth (number of topic clusters)
/// In full implementation, this would use k-means clustering on embeddings
fn compute_bandwidth_placeholder(sentences: &[String]) -> u32 {
    if sentences.is_empty() {
        return 0;
    }
    if sentences.len() < 5 {
        return 1;
    }
    
    // Simple heuristic: if sentences are very similar, bandwidth = 1
    // If diverse, estimate 2-3 clusters
    let avg_len = sentences.iter().map(|s| s.len()).sum::<usize>() as f64 / sentences.len() as f64;
    let len_variance = sentences.iter()
        .map(|s| (s.len() as f64 - avg_len).powi(2))
        .sum::<f64>() / sentences.len() as f64;
    
    if len_variance < avg_len * 0.3 {
        1
    } else if len_variance < avg_len * 0.7 {
        2
    } else {
        3.min(sentences.len().min(5) as u32)
    }
}

/// Compute redundancy (max similarity to nearest neighbor)
fn compute_redundancy_placeholder(
    card: &Card,
    neighbors: &[Card],
    _analyzer_full: bool,
) -> f64 {
    if neighbors.is_empty() {
        return 0.0;
    }
    
    // Simple heuristic: compare tag/keyword overlap
    let card_tags: std::collections::HashSet<_> = card.tags.iter().collect();
    let card_keywords: std::collections::HashSet<_> = card.keywords.iter().collect();
    
    let mut max_sim: f64 = 0.0;
    for neighbor in neighbors.iter().take(20) {
        let neighbor_tags: std::collections::HashSet<_> = neighbor.tags.iter().collect();
        let neighbor_keywords: std::collections::HashSet<_> = neighbor.keywords.iter().collect();
        
        let tag_overlap = if card_tags.is_empty() && neighbor_tags.is_empty() {
            1.0
        } else {
            let intersection = card_tags.intersection(&neighbor_tags).count();
            let union = card_tags.union(&neighbor_tags).count();
            if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            }
        };
        
        let keyword_overlap = if card_keywords.is_empty() && neighbor_keywords.is_empty() {
            1.0
        } else {
            let intersection = card_keywords.intersection(&neighbor_keywords).count();
            let union = card_keywords.union(&neighbor_keywords).count();
            if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            }
        };
        
        let sim = (tag_overlap + keyword_overlap) / 2.0;
        max_sim = max_sim.max(sim);
    }
    
    max_sim
}

/// Compute composite Semantic Volume
fn compute_sv(computed: &Computed) -> f64 {
    let tokens_norm = computed.tokens.unwrap_or(0) as f64 / 200.0;
    let nid_bpt = computed.nid_bpt.unwrap_or(0.0);
    let cohesion = computed.cohesion.unwrap_or(0.7);
    let redundancy = computed.redundancy.unwrap_or(0.0);
    
    let nid_factor = (nid_bpt / 5.0).clamp(0.5, 1.5);
    let cohesion_factor = (cohesion / 0.7).clamp(0.6, 1.4);
    let redundancy_factor = (1.0 - redundancy).clamp(0.5, 1.3);
    
    tokens_norm * nid_factor * cohesion_factor * redundancy_factor
}

/// Analyze a single card
fn analyze_card(
    card: Card,
    all_cards: &[Card],
    analyzer_full: bool,
    neighbors_count: u32,
) -> Computed {
    let body = card.get_content().unwrap_or("");
    let tokens = count_tokens(body);
    let nid_bpt = compute_nid_bpt(body);
    let link_density = compute_link_density(card.links.len(), tokens);
    let structure_density = compute_structure_density(body, tokens);
    
    let (cohesion, bandwidth, redundancy) = if analyzer_full && !body.is_empty() {
        let sentences = extract_sentences(body);
        let cohesion_val = compute_cohesion_placeholder(&sentences);
        let bandwidth_val = compute_bandwidth_placeholder(&sentences);
        
        // Find neighbors (same deck or similar tags)
        let neighbors: Vec<Card> = all_cards.iter()
            .filter(|c| c.uid != card.uid)
            .take(neighbors_count as usize)
            .cloned()
            .collect();
        
        let redundancy_val = compute_redundancy_placeholder(&card, &neighbors, true);
        (Some(cohesion_val), Some(bandwidth_val), Some(redundancy_val))
    } else {
        (None, None, None)
    };
    
    let mut computed = Computed {
        tokens: Some(tokens),
        nid_bpt: Some(nid_bpt),
        cohesion,
        bandwidth,
        redundancy,
        link_density: Some(link_density),
        structure_density: Some(structure_density),
        sv: None,
        last_analyzed: Some(chrono::Utc::now()),
    };
    
    // Compute SV
    computed.sv = Some(compute_sv(&computed));
    
    computed
}

#[derive(serde::Serialize)]
struct AnalysisResult {
    #[serde(rename = "type")]
    result_type: String,
    uid: String,
    computed: Computed,
    suggestion: String,
    rationale: String,
    version: String,
}

fn suggest_action(computed: &Computed) -> (String, String) {
    let tokens = computed.tokens.unwrap_or(0);
    let bandwidth = computed.bandwidth.unwrap_or(1);
    let cohesion = computed.cohesion.unwrap_or(0.7);
    let redundancy = computed.redundancy.unwrap_or(0.0);
    let sv = computed.sv.unwrap_or(1.0);
    let structure_density = computed.structure_density.unwrap_or(0.0);
    let nid_bpt = computed.nid_bpt.unwrap_or(5.0);
    
    // Decision rules from spec
    if (tokens > 350 && bandwidth >= 3) || (cohesion < 0.45 && tokens > 250) {
        return (
            "split".to_string(),
            format!("tokens={} bandwidth={} cohesion={:.2}", tokens, bandwidth, cohesion),
        );
    }
    
    if tokens < 80 && redundancy > 0.85 {
        return (
            "merge".to_string(),
            format!("tokens={} redundancy={:.2}", tokens, redundancy),
        );
    }
    
    if (redundancy > 0.9 && tokens > 200) || (nid_bpt < 2.5 && tokens > 200) {
        return (
            "prune".to_string(),
            format!("redundancy={:.2} nid_bpt={:.2} tokens={}", redundancy, nid_bpt, tokens),
        );
    }
    
    if tokens > 300 && structure_density < 0.8 {
        return (
            "refactor".to_string(),
            format!("tokens={} structure_density={:.2}", tokens, structure_density),
        );
    }
    
    if sv > 1.6 {
        return (
            "consider-split".to_string(),
            format!("sv={:.2}", sv),
        );
    }
    
    if sv < 0.5 {
        return (
            "consider-merge".to_string(),
            format!("sv={:.2}", sv),
        );
    }
    
    ("ok".to_string(), format!("sv={:.2}", sv))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    
    let analyzer_full = cli.analyzer == "full" && !cli.no_embeddings;
    
    match &cli.command {
        Commands::Analyze { uid, query: query_str } => {
            let all_cards = load_all_cards(&repo)?;
            
            let cards_to_analyze: Vec<Card> = if let Some(uid_str) = uid {
                // Analyze single card
                all_cards.iter()
                    .find(|c| c.uid == *uid_str || c.slug == *uid_str)
                    .map(|c| vec![c.clone()])
                    .context("Card not found")?
            } else if let Some(q) = query_str {
                // Parse query and filter
                let parsed_query = query::parse_query_shorthand(q)?;
                // Simple filtering (matches scout's logic)
                all_cards.iter()
                    .filter(|card| {
                        if let Some(ref filter) = parsed_query.filter {
                            match filter {
                                query::Filter::All(preds) => {
                                    preds.iter().all(|p| {
                                        if p.contains("tags contains") {
                                            if let Some(tag) = p.split('"').nth(1) {
                                                card.tags.contains(&tag.to_string())
                                            } else {
                                                false
                                            }
                                        } else {
                                            true
                                        }
                                    })
                                }
                                _ => true,
                            }
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect()
            } else {
                // Analyze all cards
                all_cards.clone()
            };
            
            for card in cards_to_analyze {
                let computed = analyze_card(
                    card.clone(),
                    &all_cards,
                    analyzer_full,
                    cli.neighbors,
                );
                
                let (suggestion, rationale) = suggest_action(&computed);
                
                if cli.jsonl {
                    let result = AnalysisResult {
                        result_type: "analysis".to_string(),
                        uid: card.uid.clone(),
                        computed,
                        suggestion,
                        rationale,
                        version: "svspec-1".to_string(),
                    };
                    println!("{}", serde_json::to_string(&result)?);
                } else {
                    println!("Card: {} ({})", card.title, card.uid);
                    println!("  Tokens: {}", computed.tokens.unwrap_or(0));
                    println!("  NID (bits/token): {:.2}", computed.nid_bpt.unwrap_or(0.0));
                    if let Some(c) = computed.cohesion {
                        println!("  Cohesion: {:.2}", c);
                    }
                    if let Some(b) = computed.bandwidth {
                        println!("  Bandwidth: {}", b);
                    }
                    if let Some(r) = computed.redundancy {
                        println!("  Redundancy: {:.2}", r);
                    }
                    println!("  Link density: {:.2}", computed.link_density.unwrap_or(0.0));
                    println!("  Structure density: {:.2}", computed.structure_density.unwrap_or(0.0));
                    println!("  SV: {:.2}", computed.sv.unwrap_or(0.0));
                    println!("  Suggestion: {}", suggestion);
                    println!();
                }
            }
        }
    }
    
    Ok(())
}
