mod tui;

use anyhow::Result;
use cardstack_lib::{
    card::{Card, Computed},
    query,
    repository::{get_repo_root, load_all_cards},
};
use clap::{Parser, Subcommand};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use std::path::PathBuf;

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
    /// Launch interactive TUI for browsing card metrics
    Tui {
        #[arg(long)]
        query: Option<String>,
    },
}

fn count_tokens(text: &str, model: &TextEmbedding) -> u32 {
    if text.is_empty() {
        return 0;
    }
    match model.tokenizer.encode(text, false) {
        Ok(encoding) => encoding.get_ids().len() as u32,
        Err(_) => text.split_whitespace().count() as u32,
    }
}

fn init_embedding_model() -> Result<TextEmbedding> {
    let model = match TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    ) {
        Ok(m) => m,
        Err(_) => TextEmbedding::try_new(Default::default())?,
    };
    Ok(model)
}

fn compute_nid_bpt(text: &str, tokens: u32) -> f64 {
    if text.is_empty() || tokens == 0 {
        return 0.0;
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes()).ok();
    let compressed = encoder.finish().unwrap_or_default();
    8.0 * compressed.len() as f64 / tokens as f64
}

fn extract_sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() > 10)
        .collect()
}

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

fn compute_link_density(link_count: usize, tokens: u32) -> f64 {
    if tokens == 0 {
        return 0.0;
    }
    (link_count as f64 / tokens as f64) * 100.0
}

fn compute_cohesion(sentences: &[String], model: &mut TextEmbedding) -> Result<f64> {
    if sentences.len() < 2 {
        return Ok(1.0);
    }
    let sentences_limited: Vec<&String> = sentences.iter().take(50).collect();
    let texts: Vec<&str> = sentences_limited.iter().map(|s| s.as_str()).collect();
    let embeddings = model.embed(texts, None)?;
    if embeddings.len() < 2 {
        return Ok(1.0);
    }
    let mut similarities = Vec::new();
    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
            similarities.push(sim);
        }
    }
    if similarities.is_empty() {
        Ok(0.5)
    } else {
        Ok(similarities.iter().sum::<f64>() / similarities.len() as f64)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot_product / (norm_a * norm_b)) as f64
}

fn compute_bandwidth(sentences: &[String], model: &mut TextEmbedding) -> Result<u32> {
    if sentences.is_empty() {
        return Ok(1);
    }
    let sentences_limited: Vec<&String> = sentences.iter().take(50).collect();
    let texts: Vec<&str> = sentences_limited.iter().map(|s| s.as_str()).collect();
    let embeddings = model.embed(texts, None)?;
    if embeddings.len() < 2 {
        return Ok(1);
    }
    let mut distances = Vec::new();
    for i in 0..embeddings.len().min(20) {
        for j in (i + 1)..embeddings.len().min(20) {
            distances.push(cosine_distance(&embeddings[i], &embeddings[j]));
        }
    }
    if distances.is_empty() {
        return Ok(1);
    }
    let mean_dist = distances.iter().sum::<f64>() / distances.len() as f64;
    let variance = distances
        .iter()
        .map(|d| (d - mean_dist).powi(2))
        .sum::<f64>()
        / distances.len() as f64;
    let clusters: u32 = if mean_dist < 0.3 {
        if variance < 0.01 {
            1
        } else if variance < 0.05 {
            2
        } else {
            3
        }
    } else if mean_dist < 0.5 {
        if variance < 0.02 {
            2
        } else if variance < 0.08 {
            3
        } else {
            4
        }
    } else {
        if variance < 0.05 {
            3
        } else if variance < 0.15 {
            4
        } else {
            5
        }
    };
    Ok(clusters.max(1).min(5))
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    1.0 - cosine_similarity(a, b)
}

fn compute_redundancy(card: &Card, neighbors: &[Card], model: &mut TextEmbedding) -> Result<f64> {
    if neighbors.is_empty() {
        return Ok(0.0);
    }
    let card_text = format!("{} {}", card.title, card.get_content().unwrap_or(""));
    if card_text.trim().is_empty() {
        return Ok(0.0);
    }
    let card_embedding = model.embed(vec![card_text.as_str()], None)?;
    if card_embedding.is_empty() {
        return Ok(0.0);
    }
    let mut max_similarity: f64 = 0.0;
    for neighbor in neighbors.iter().take(10) {
        let neighbor_text = format!(
            "{} {}",
            neighbor.title,
            neighbor.get_content().unwrap_or("")
        );
        if neighbor_text.trim().is_empty() {
            continue;
        }
        if let Ok(ne) = model.embed(vec![neighbor_text.as_str()], None) {
            if !ne.is_empty() {
                let sim = cosine_similarity(&card_embedding[0], &ne[0]);
                max_similarity = max_similarity.max(sim);
            }
        }
    }
    Ok(max_similarity.clamp(0.0, 1.0))
}

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

fn analyze_card(
    card: Card,
    all_cards: &[Card],
    analyzer_full: bool,
    neighbors_count: u32,
    mut model: Option<&mut TextEmbedding>,
) -> Result<Computed> {
    // Construct a comprehensive textual representation of the card
    let mut body_parts: Vec<String> = Vec::new();

    // Add title
    body_parts.push(card.title.clone());

    // Add keywords
    if !card.keywords.is_empty() {
        body_parts.push(card.keywords.join(" "));
    }

    // Add fields
    for (key, value) in &card.fields {
        if let Some(value_str) = value.as_str() {
            body_parts.push(format!("{} {}", key, value_str));
        } else if let Some(value_str) = value.as_str() {
            body_parts.push(format!("{} {}", key, value_str));
        }
    }

    // Add content body if available
    if let Some(content) = card.get_content() {
        body_parts.push(content.to_string());
    }

    let body = body_parts.join(" ");
    let tokens = if let Some(ref m) = model {
        count_tokens(&body, m)
    } else {
        body.split_whitespace().count() as u32
    };
    let nid_bpt = compute_nid_bpt(&body, tokens);
    let link_density = compute_link_density(card.links.len(), tokens);
    let structure_density = compute_structure_density(&body, tokens);
    let (cohesion, bandwidth, redundancy) = if analyzer_full && !body.is_empty() {
        if let Some(ref mut m) = model {
            let sentences = extract_sentences(&body);
            let cohesion_val = compute_cohesion(&sentences, m)?;
            let bandwidth_val = compute_bandwidth(&sentences, m)?;
            let neighbors: Vec<Card> = all_cards
                .iter()
                .filter(|c| c.uid != card.uid)
                .take(neighbors_count as usize)
                .cloned()
                .collect();
            let redundancy_val = compute_redundancy(&card, &neighbors, m)?;
            (
                Some(cohesion_val),
                Some(bandwidth_val),
                Some(redundancy_val),
            )
        } else {
            (None, None, None)
        }
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
    computed.sv = Some(compute_sv(&computed));
    Ok(computed)
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

    if (tokens > 350 && bandwidth >= 3) || (cohesion < 0.45 && tokens > 250) {
        return (
            "split".to_string(),
            format!(
                "tokens={} bandwidth={} cohesion={:.2}",
                tokens, bandwidth, cohesion
            ),
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
            format!(
                "redundancy={:.2} nid_bpt={:.2} tokens={}",
                redundancy, nid_bpt, tokens
            ),
        );
    }
    if tokens > 300 && structure_density < 0.8 {
        return (
            "refactor".to_string(),
            format!(
                "tokens={} structure_density={:.2}",
                tokens, structure_density
            ),
        );
    }
    if sv > 1.6 {
        return ("consider-split".to_string(), format!("sv={:.2}", sv));
    }
    if sv < 0.5 {
        return ("consider-merge".to_string(), format!("sv={:.2}", sv));
    }
    ("ok".to_string(), format!("sv={:.2}", sv))
}

fn filter_cards<'a>(all_cards: &'a [Card], query_str: &str) -> Vec<Card> {
    match query::parse_query_shorthand(query_str) {
        Ok(parsed_query) => all_cards
            .iter()
            .filter(|card| {
                if let Some(ref filter) = parsed_query.filter {
                    match filter {
                        query::Filter::All(preds) => preds.iter().all(|p| {
                            if p.contains("tags contains") {
                                if let Some(tag) = p.split('"').nth(1) {
                                    card.tags.contains(&tag.to_string())
                                } else {
                                    false
                                }
                            } else {
                                true
                            }
                        }),
                        _ => true,
                    }
                } else {
                    true
                }
            })
            .cloned()
            .collect(),
        Err(_) => all_cards.to_vec(),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    let analyzer_full = cli.analyzer == "full" && !cli.no_embeddings;

    match &cli.command {
        Commands::Tui { query } => {
            let all_cards_with_paths = load_all_cards(&repo)?;
            let all_cards: Vec<Card> = all_cards_with_paths
                .into_iter()
                .map(|(_, card)| card)
                .collect();
            let cards = if let Some(q) = query {
                filter_cards(&all_cards, q)
            } else {
                all_cards.clone()
            };

            let mut model_opt = if analyzer_full {
                match init_embedding_model() {
                    Ok(m) => {
                        eprintln!("Initialized embedding model");
                        Some(m)
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Could not load embedding model ({}), using fast analysis",
                            e
                        );
                        None
                    }
                }
            } else {
                None
            };

            eprintln!(
                "Analyzing {} cards{}...",
                cards.len(),
                if model_opt.is_some() {
                    " with embeddings"
                } else {
                    ""
                }
            );

            let analyses: Vec<tui::CardAnalysis> = cards
                .into_iter()
                .map(|card| {
                    let computed = analyze_card(
                        card.clone(),
                        &all_cards,
                        model_opt.is_some(),
                        cli.neighbors,
                        model_opt.as_mut(),
                    )?;
                    let (suggestion, rationale) = suggest_action(&computed);
                    Ok(tui::CardAnalysis {
                        card,
                        computed,
                        suggestion,
                        rationale,
                    })
                })
                .collect::<Result<_>>()?;

            tui::run_tui(analyses)?;
        }

        Commands::Analyze {
            uid,
            query: query_str,
        } => {
            let mut model_opt = if analyzer_full {
                match init_embedding_model() {
                    Ok(m) => {
                        eprintln!("Initialized embedding model");
                        Some(m)
                    }
                    Err(e) => {
                        eprintln!("Error: Could not initialize embedding model: {}", e);
                        eprintln!("Embedding model is required for full analysis. Exiting.");
                        return Err(e);
                    }
                }
            } else {
                None
            };

            let all_cards_with_paths = load_all_cards(&repo)?;
            let all_cards: Vec<Card> = all_cards_with_paths
                .into_iter()
                .map(|(_, card)| card)
                .collect();

            let cards_to_analyze: Vec<Card> = if let Some(uid_str) = uid {
                all_cards
                    .iter()
                    .find(|c| c.uid == *uid_str || c.slug == *uid_str)
                    .map(|c| vec![c.clone()])
                    .ok_or_else(|| anyhow::anyhow!("Card not found: {}", uid_str))?
            } else if let Some(q) = query_str {
                filter_cards(&all_cards, q)
            } else {
                all_cards.clone()
            };

            for card in cards_to_analyze {
                let computed = analyze_card(
                    card.clone(),
                    &all_cards,
                    analyzer_full,
                    cli.neighbors,
                    model_opt.as_mut(),
                )?;
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
                    if let Err(e) =
                        writeln!(std::io::stdout(), "{}", serde_json::to_string(&result)?)
                    {
                        if e.kind() != std::io::ErrorKind::BrokenPipe {
                            return Err(e.into());
                        }
                        break;
                    }
                } else {
                    macro_rules! println_or_break {
                        ($fmt:expr $(, $arg:expr)*) => {
                            if let Err(e) = writeln!(std::io::stdout(), $fmt $(, $arg)*) {
                                if e.kind() != std::io::ErrorKind::BrokenPipe {
                                    return Err(e.into());
                                }
                                break;
                            }
                        };
                    }
                    println_or_break!("Card: {} ({})", card.title, card.uid);
                    println_or_break!("  Tokens: {}", computed.tokens.unwrap_or(0));
                    println_or_break!("  NID (bits/token): {:.2}", computed.nid_bpt.unwrap_or(0.0));
                    if let Some(c) = computed.cohesion {
                        println_or_break!("  Cohesion: {:.2}", c);
                    }
                    if let Some(b) = computed.bandwidth {
                        println_or_break!("  Bandwidth: {}", b);
                    }
                    if let Some(r) = computed.redundancy {
                        println_or_break!("  Redundancy: {:.2}", r);
                    }
                    println_or_break!(
                        "  Link density: {:.2}",
                        computed.link_density.unwrap_or(0.0)
                    );
                    println_or_break!(
                        "  Structure density: {:.2}",
                        computed.structure_density.unwrap_or(0.0)
                    );
                    println_or_break!("  SV: {:.2}", computed.sv.unwrap_or(0.0));
                    println_or_break!("  Suggestion: {}", suggestion);
                    println_or_break!("");
                }
            }
        }
    }

    Ok(())
}
