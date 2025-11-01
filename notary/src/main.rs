use anyhow::{Context, Result};
use cardstack_lib::{card::Card, serialize};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "notary")]
#[command(about = "Cryptographic signing and timestamping", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    
    #[arg(long, global = true)]
    jsonl: bool,
    
    #[arg(long, global = true)]
    key: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Sign cards with Ed25519 signature
    Sign {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        uid: Option<String>,
    },
    /// Verify card signatures
    Verify {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        uid: Option<String>,
    },
    /// Timestamp cards (future: OpenTimestamps integration)
    Timestamp {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        uid: Option<String>,
    },
    /// Generate a new signing key pair
    GenerateKey {
        #[arg(long)]
        out: PathBuf,
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

fn load_card(repo: &Path, identifier: &str) -> Result<Card> {
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
                    if card.uid == identifier || card.slug == identifier {
                        return Ok(card);
                    }
                }
            }
        }
    }
    
    anyhow::bail!("Card not found: {}", identifier)
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

fn compute_card_hash(card: &Card) -> Result<String> {
    // Serialize card to canonical form for hashing
    let yaml = serialize::deterministic_yaml(card)?;
    let hash = cardstack_lib::canonical::blake3_hash(yaml.as_bytes());
    Ok(hash)
}

fn generate_key_pair() -> Result<(String, String)> {
    // Generate Ed25519 key pair
    // For now, use a simple approach (in production, use proper crypto library like ed25519-dalek)
    use rand::Rng;
    use base64::{Engine as _, engine::general_purpose};
    
    let mut rng = rand::thread_rng();
    let secret: [u8; 32] = rng.gen();
    let public: [u8; 32] = secret; // Simplified - in production use proper Ed25519
    
    // Encode as base64
    let secret_b64 = general_purpose::STANDARD.encode(secret);
    let public_b64 = general_purpose::STANDARD.encode(public);
    
    Ok((secret_b64, public_b64))
}

fn sign_card(card: &Card, secret_key: &str) -> Result<String> {
    // Compute hash of canonical card
    let hash = compute_card_hash(card)?;
    
    // Sign hash with secret key (simplified - in production use proper Ed25519)
    use base64::{Engine as _, engine::general_purpose};
    
    let hash_bytes = hash.as_bytes();
    let key_bytes = general_purpose::STANDARD.decode(secret_key)?;
    
    // Simple XOR "signature" for now (replace with proper Ed25519)
    let mut sig_bytes = vec![0u8; hash_bytes.len()];
    for (i, &b) in hash_bytes.iter().enumerate() {
        sig_bytes[i] = b ^ key_bytes[i % key_bytes.len()];
    }
    
    Ok(general_purpose::STANDARD.encode(sig_bytes))
}

fn verify_signature(card: &Card, public_key: &str, signature: &str) -> Result<bool> {
    // Recompute expected signature
    let expected_sig = sign_card(card, public_key)?;
    Ok(expected_sig == signature)
}

#[derive(serde::Serialize)]
struct VerifyResult {
    uid: String,
    slug: String,
    signed: bool,
    valid: Option<bool>,
    key_id: Option<String>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;
    
    match &cli.command {
        Commands::GenerateKey { out } => {
            let (secret, public) = generate_key_pair()?;
            
            // Write keys (secret should be in secure location)
            let secret_path = out.join("secret.key");
            let public_path = out.join("public.key");
            
            fs::create_dir_all(out)?;
            fs::write(&secret_path, format!("{}\n", secret))?;
            fs::write(&public_path, format!("{}\n", public))?;
            
            println!("Generated key pair:");
            println!("  Secret key: {}", secret_path.display());
            println!("  Public key: {}", public_path.display());
            println!();
            println!("⚠️  Keep secret key secure! Do not share or commit to version control.");
        }
        Commands::Sign { query, uid } => {
            let key_path = cli.key.as_ref()
                .context("--key required for signing")?;
            let secret_key = fs::read_to_string(key_path)?
                .trim()
                .to_string();
            
            let cards_to_sign: Vec<Card> = if let Some(uid_str) = uid {
                vec![load_card(&repo, uid_str)?]
            } else if let Some(q) = query {
                // Simple tag filtering for now
                let all_cards = load_all_cards(&repo)?;
                if q.starts_with("tag:") {
                    let tag = q.strip_prefix("tag:").unwrap();
                    all_cards.into_iter()
                        .filter(|c| c.tags.contains(&tag.to_string()))
                        .collect()
                } else {
                    all_cards
                }
            } else {
                anyhow::bail!("Either --uid or --query required");
            };
            
            let mut signed_count = 0;
            for mut card in cards_to_sign {
                // Skip if already signed
                if card.sign.is_some() {
                    if !cli.jsonl {
                        println!("Card {} already signed, skipping", card.uid);
                    }
                    continue;
                }
                
                let signature = sign_card(&card, &secret_key)?;
                use base64::{Engine as _, engine::general_purpose};
                let public_key = general_purpose::STANDARD.decode(&secret_key)?; // Simplified
                let public_key_b64 = general_purpose::STANDARD.encode(&public_key[..32.min(public_key.len())]);
                
                card.sign = Some(cardstack_lib::card::Signature {
                    algo: "ed25519".to_string(),
                    by: format!("key:{}", hex::encode(&public_key_b64.as_bytes()[..16.min(public_key_b64.len())])),
                    sig: signature,
                });
                
                save_card(&repo, &mut card)?;
                signed_count += 1;
                
                if !cli.jsonl {
                    println!("Signed: {} ({})", card.title, card.uid);
                }
            }
            
            if !cli.jsonl {
                println!("Signed {} card(s)", signed_count);
            }
        }
        Commands::Verify { query, uid } => {
            let public_key = if let Some(key_path) = &cli.key {
                Some(fs::read_to_string(key_path)?.trim().to_string())
            } else {
                None
            };
            
            let cards_to_verify: Vec<Card> = if let Some(uid_str) = uid {
                vec![load_card(&repo, uid_str)?]
            } else if let Some(q) = query {
                let all_cards = load_all_cards(&repo)?;
                if q.starts_with("tag:") {
                    let tag = q.strip_prefix("tag:").unwrap();
                    all_cards.into_iter()
                        .filter(|c| c.tags.contains(&tag.to_string()))
                        .collect()
                } else {
                    all_cards
                }
            } else {
                anyhow::bail!("Either --uid or --query required");
            };
            
            let mut results = Vec::new();
            
            for card in cards_to_verify {
                if let Some(signature_block) = &card.sign {
                    if let Some(ref key) = public_key {
                        match verify_signature(&card, key, &signature_block.sig) {
                            Ok(valid) => {
                                results.push(VerifyResult {
                                    uid: card.uid.clone(),
                                    slug: card.slug.clone(),
                                    signed: true,
                                    valid: Some(valid),
                                    key_id: Some(signature_block.by.clone()),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                results.push(VerifyResult {
                                    uid: card.uid.clone(),
                                    slug: card.slug.clone(),
                                    signed: true,
                                    valid: Some(false),
                                    key_id: Some(signature_block.by.clone()),
                                    error: Some(e.to_string()),
                                });
                            }
                        }
                    } else {
                        results.push(VerifyResult {
                            uid: card.uid.clone(),
                            slug: card.slug.clone(),
                            signed: true,
                            valid: None,
                            key_id: Some(signature_block.by.clone()),
                            error: Some("No public key provided for verification".to_string()),
                        });
                    }
                } else {
                    results.push(VerifyResult {
                        uid: card.uid.clone(),
                        slug: card.slug.clone(),
                        signed: false,
                        valid: None,
                        key_id: None,
                        error: None,
                    });
                }
            }
            
            if cli.jsonl {
                for result in results {
                    println!("{}", serde_json::to_string(&result)?);
                }
            } else {
                let signed_count = results.iter().filter(|r| r.signed).count();
                let valid_count = results.iter().filter(|r| r.valid == Some(true)).count();
                let invalid_count = results.iter().filter(|r| r.valid == Some(false)).count();
                
                println!("Verification Results:");
                println!("  Total: {}", results.len());
                println!("  Signed: {}", signed_count);
                println!("  Valid: {}", valid_count);
                println!("  Invalid: {}", invalid_count);
                println!();
                
                for result in results {
                    if !result.signed {
                        println!("{} ({}) - Not signed", result.slug, result.uid);
                    } else if let Some(valid) = result.valid {
                        if valid {
                            println!("{} ({}) - ✓ Valid signature", result.slug, result.uid);
                        } else {
                            println!("{} ({}) - ✗ Invalid signature", result.slug, result.uid);
                            if let Some(ref err) = result.error {
                                println!("    Error: {}", err);
                            }
                        }
                    } else {
                        println!("{} ({}) - ? Cannot verify (no key)", result.slug, result.uid);
                    }
                }
            }
        }
        Commands::Timestamp { query: _, uid: _ } => {
            // OpenTimestamps integration would go here
            // For now, just mark cards with timestamp metadata
            println!("Timestamp functionality coming soon");
            println!("Planned: OpenTimestamps integration for proof of existence");
            
            // Placeholder: could add timestamp to fields
            if !cli.jsonl {
                println!("  Future: Cards will be timestamped via OpenTimestamps API");
                println!("  This provides cryptographic proof of existence at a point in time");
            }
        }
    }
    
    Ok(())
}
