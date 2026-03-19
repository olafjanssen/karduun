use anyhow::Result;
use cardstack_lib::repository::{get_repo_root, load_all_cards};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;

mod tui;

#[derive(Parser)]
#[command(name = "eco")]
#[command(about = "Card ecosystem dynamics simulator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Simulate scanning a card (increases resonance)
    Scan {
        card_id: String,
        #[arg(long, default_value = "1.0")]
        resonance_increase: f64,
    },

    /// Check a card's resonance score
    Resonance { card_id: String },

    /// Print a card (subject to ecosystem quotas)
    Print {
        card_id: String,
        #[arg(long, default_value = "1")]
        copies: u32,
    },

    /// Check if cards can mature into new concepts
    Mature {
        #[arg(long, default_value = "0.8")]
        similarity_threshold: f64,
        #[arg(long, default_value = "3")]
        min_cluster_size: usize,
    },

    /// Show overall ecosystem status
    Status,

    /// Run one evolution cycle
    Evolve,
    /// Launch interactive TUI for ecosystem management
    Tui,
}

// Ecosystem configuration constants
const DAILY_PRINT_QUOTA: u32 = 50;
const WEEKLY_PRINT_QUOTA: u32 = 200;
const RESONANCE_DECAY_RATE: f64 = 0.01; // 1% decay per day
const MAX_RESONANCE: f64 = 1.0;
const MIN_RESONANCE: f64 = 0.0;

#[derive(Debug, Clone)]
struct EcosystemState {
    daily_prints: u32,
    weekly_prints: u32,
    last_reset: DateTime<Utc>,
    card_resonance: HashMap<String, f64>,
    card_print_counts: HashMap<String, u32>,
}

impl EcosystemState {
    fn new() -> Self {
        Self {
            daily_prints: 0,
            weekly_prints: 0,
            last_reset: Utc::now(),
            card_resonance: HashMap::new(),
            card_print_counts: HashMap::new(),
        }
    }

    fn load_or_new() -> Result<Self> {
        // TODO: Implement proper persistence
        Ok(Self::new())
    }

    fn save(&self) -> Result<()> {
        // TODO: Implement proper persistence
        Ok(())
    }

    fn check_quotas(&self) -> bool {
        self.daily_prints < DAILY_PRINT_QUOTA && self.weekly_prints < WEEKLY_PRINT_QUOTA
    }

    fn record_print(&mut self, copies: u32) -> Result<()> {
        self.daily_prints += copies;
        self.weekly_prints += copies;
        Ok(())
    }

    fn update_resonance(&mut self, card_id: &str, amount: f64) {
        let entry = self
            .card_resonance
            .entry(card_id.to_string())
            .or_insert(0.0);
        *entry = (*entry + amount).clamp(MIN_RESONANCE, MAX_RESONANCE);
    }

    fn get_resonance(&self, card_id: &str) -> f64 {
        self.card_resonance.get(card_id).copied().unwrap_or(0.0)
    }

    fn record_card_print(&mut self, card_id: &str, copies: u32) {
        let entry = self
            .card_print_counts
            .entry(card_id.to_string())
            .or_insert(0);
        *entry += copies;
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = get_repo_root(cli.repo)?;

    match cli.command {
        Commands::Scan {
            card_id,
            resonance_increase,
        } => {
            scan_card(&repo_root, &card_id, resonance_increase)?;
        }
        Commands::Resonance { card_id } => {
            check_resonance(&repo_root, &card_id)?;
        }
        Commands::Print { card_id, copies } => {
            print_card(&repo_root, &card_id, copies)?;
        }
        Commands::Mature {
            similarity_threshold,
            min_cluster_size,
        } => {
            check_maturation(&repo_root, similarity_threshold, min_cluster_size)?;
        }
        Commands::Status => {
            show_status(&repo_root)?;
        }
        Commands::Evolve => {
            evolve_ecosystem(&repo_root)?;
        }
        Commands::Tui => {
            tui::run_tui(&repo_root)?;
        }
    }

    Ok(())
}

fn scan_card(repo_root: &PathBuf, card_id: &str, resonance_increase: f64) -> Result<()> {
    let mut state = EcosystemState::load_or_new()?;

    // Load the card to verify it exists
    let cards = load_all_cards(repo_root)?;
    let card = cards
        .iter()
        .find(|(_, c)| c.uid == card_id || c.slug == card_id)
        .map(|(_, c)| c)
        .ok_or_else(|| anyhow::anyhow!("Card not found: {}", card_id))?;

    println!("Scanning card: {} ({})", card.title, card.uid);

    // Update resonance
    state.update_resonance(&card.uid, resonance_increase);
    state.save()?;

    println!(
        "Resonance increased by {}. New resonance: {}",
        resonance_increase,
        state.get_resonance(&card.uid)
    );

    Ok(())
}

fn check_resonance(repo_root: &PathBuf, card_id: &str) -> Result<()> {
    let state = EcosystemState::load_or_new()?;

    // Load the card to verify it exists
    let cards = load_all_cards(repo_root)?;
    let card = cards
        .iter()
        .find(|(_, c)| c.uid == card_id || c.slug == card_id)
        .map(|(_, c)| c)
        .ok_or_else(|| anyhow::anyhow!("Card not found: {}", card_id))?;

    let resonance = state.get_resonance(&card.uid);
    println!("Card: {} ({})", card.title, card.uid);
    println!("Current resonance: {}", resonance);

    // Show interpretation
    if resonance > 0.8 {
        println!("Status: High resonance - this concept is very active");
    } else if resonance > 0.5 {
        println!("Status: Medium resonance - this concept is moderately active");
    } else if resonance > 0.2 {
        println!("Status: Low resonance - this concept needs more attention");
    } else {
        println!("Status: Very low resonance - this concept is dormant");
    }

    Ok(())
}

fn print_card(repo_root: &PathBuf, card_id: &str, copies: u32) -> Result<()> {
    let mut state = EcosystemState::load_or_new()?;

    // Check quotas
    if !state.check_quotas() {
        return Err(anyhow::anyhow!("Print quota exceeded"));
    }

    // Load the card
    let cards = load_all_cards(repo_root)?;
    let card = cards
        .iter()
        .find(|(_, c)| c.uid == card_id || c.slug == card_id)
        .map(|(_, c)| c)
        .ok_or_else(|| anyhow::anyhow!("Card not found: {}", card_id))?;

    println!("Printing card: {} ({})", card.title, card.uid);
    println!("Copies: {}", copies);

    // Record the print
    state.record_print(copies)?;
    state.record_card_print(&card.uid, copies);
    state.save()?;

    println!("Print successful!");
    println!(
        "Daily quota remaining: {}",
        DAILY_PRINT_QUOTA - state.daily_prints
    );
    println!(
        "Weekly quota remaining: {}",
        WEEKLY_PRINT_QUOTA - state.weekly_prints
    );

    Ok(())
}

fn check_maturation(
    repo_root: &PathBuf,
    similarity_threshold: f64,
    min_cluster_size: usize,
) -> Result<()> {
    let state = EcosystemState::load_or_new()?;
    let cards = load_all_cards(repo_root)?;

    println!("Checking for card maturation opportunities...");
    println!("Similarity threshold: {}", similarity_threshold);
    println!("Minimum cluster size: {}", min_cluster_size);

    // Find cards with high resonance
    let high_resonance_cards: Vec<_> = cards
        .iter()
        .filter(|(_, card)| state.get_resonance(&card.uid) > 0.7)
        .map(|(_, card)| card)
        .collect();

    println!(
        "Found {} cards with high resonance",
        high_resonance_cards.len()
    );

    for card in &high_resonance_cards {
        println!(
            "- {} ({}): resonance {}",
            card.title,
            card.uid,
            state.get_resonance(&card.uid)
        );
    }

    // TODO: Implement actual semantic similarity analysis
    // For now, just suggest potential combinations
    if high_resonance_cards.len() >= min_cluster_size {
        println!("\nPotential maturation opportunities:");
        println!("These cards could combine to form new concepts:");

        for (i, card) in high_resonance_cards.iter().take(3).enumerate() {
            println!(
                "{}. {} - resonance: {}",
                i + 1,
                card.title,
                state.get_resonance(&card.uid)
            );
        }

        println!("\nRun 'eco evolve' to attempt creating new cards from these clusters.");
    } else {
        println!("Not enough high-resonance cards for maturation.");
    }

    Ok(())
}

fn show_status(repo_root: &PathBuf) -> Result<()> {
    let state = EcosystemState::load_or_new()?;
    let cards = load_all_cards(repo_root)?;

    println!("=== Ecosystem Status ===");
    println!("Total cards: {}", cards.len());
    println!(
        "Daily print quota: {}/{} used",
        state.daily_prints, DAILY_PRINT_QUOTA
    );
    println!(
        "Weekly print quota: {}/{} used",
        state.weekly_prints, WEEKLY_PRINT_QUOTA
    );

    // Analyze resonance distribution
    let mut high_resonance = 0;
    let mut medium_resonance = 0;
    let mut low_resonance = 0;

    for (_, card) in &cards {
        let resonance = state.get_resonance(&card.uid);
        if resonance > 0.7 {
            high_resonance += 1;
        } else if resonance > 0.4 {
            medium_resonance += 1;
        } else {
            low_resonance += 1;
        }
    }

    println!("\nResonance Distribution:");
    println!("High (>0.7): {} cards", high_resonance);
    println!("Medium (0.4-0.7): {} cards", medium_resonance);
    println!("Low (<0.4): {} cards", low_resonance);

    // Show most resonant cards
    println!("\nTop 5 Most Resonant Cards:");
    let mut cards_with_resonance: Vec<_> = cards
        .iter()
        .map(|(_, card)| (card, state.get_resonance(&card.uid)))
        .collect();

    cards_with_resonance.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (i, (card, resonance)) in cards_with_resonance.iter().take(5).enumerate() {
        println!("{}. {} - resonance: {:.3}", i + 1, card.title, resonance);
    }

    Ok(())
}

fn evolve_ecosystem(repo_root: &PathBuf) -> Result<()> {
    let mut state = EcosystemState::load_or_new()?;
    let cards = load_all_cards(repo_root)?;

    println!("=== Running Ecosystem Evolution ===");

    // Step 1: Apply resonance decay
    println!("Applying resonance decay...");
    for (_, card) in &cards {
        let current_resonance = state.get_resonance(&card.uid);
        let decayed = (current_resonance * (1.0 - RESONANCE_DECAY_RATE)).max(MIN_RESONANCE);
        state.card_resonance.insert(card.uid.clone(), decayed);
    }

    // Step 2: Check for maturation opportunities
    println!("Checking for maturation opportunities...");
    check_maturation(repo_root, 0.8, 3)?;

    // Step 3: Reset quotas if needed
    let now = Utc::now();
    let days_since_reset = (now - state.last_reset).num_days();

    if days_since_reset >= 1 {
        println!("Resetting daily quota...");
        state.daily_prints = 0;
    }

    if days_since_reset >= 7 {
        println!("Resetting weekly quota...");
        state.weekly_prints = 0;
        state.last_reset = now;
    }

    state.save()?;

    println!("Evolution cycle complete!");

    Ok(())
}
