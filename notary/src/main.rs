use anyhow::{Context, Result};
use cardstack_lib::{card::Card, query, serialize};
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

fn matches_filter(card: &Card, predicate: &str) -> bool {
    // Simple predicate matching (same as scout)
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

fn execute_query(cards: Vec<Card>, q: Option<&query::Query>) -> Vec<Card> {
    let mut results = cards;
    
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
        
        // Apply sort (not critical for signing, but good for consistency)
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
                a.uid.cmp(&b.uid)
            });
        }
        
        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }
    }
    
    results
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
    // IMPORTANT: Exclude signature, computed, updated, and version from hash
    // These are metadata fields that change without changing card content
    // Signature covers the actual content (title, tags, fields, links, facets)
    let mut card_for_hash = card.clone();
    card_for_hash.sign = None;
    card_for_hash.computed = None;
    // Use fixed timestamp for deterministic hashing
    // The signature covers content, not when it was last modified
    card_for_hash.updated = card.created; // Use created time as stable reference
    card_for_hash.version = 1; // Use base version for deterministic hash
    
    // Use JSON for deterministic serialization
    // YAML serialization of HashMap is not deterministic due to hash ordering
    // JSON with sorted collections ensures identical output for identical content
    let json_value = serde_json::to_value(&card_for_hash)?;
    
    // Create a canonical JSON representation with sorted keys
    // serde_json serializes objects deterministically, but we need to sort arrays
    // and ensure HashMap keys are sorted by converting to sorted Vec
    let canonical_json = canonicalize_json(&json_value)?;
    let json_string = serde_json::to_string(&canonical_json)?;
    let hash = cardstack_lib::canonical::blake3_hash(json_string.as_bytes());
    Ok(hash)
}

fn canonicalize_json(value: &serde_json::Value) -> Result<serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => {
            // Build sorted map - serde_json::Map preserves insertion order
            let mut sorted_pairs: Vec<_> = map.iter().collect();
            sorted_pairs.sort_by_key(|(k, _)| *k);
            
            let mut sorted_map = serde_json::Map::new();
            for (k, v) in sorted_pairs {
                sorted_map.insert(k.clone(), canonicalize_json(v)?);
            }
            Ok(serde_json::Value::Object(sorted_map))
        }
        serde_json::Value::Array(arr) => {
            // Canonicalize each element first
            let mut canonicalized: Vec<_> = arr.iter()
                .map(|v| canonicalize_json(v))
                .collect::<Result<Vec<_>>>()?;
            
            // Try to sort if all elements are comparable
            // For arrays of objects, sort by a canonical representation
            // For primitive arrays, sort directly
            if canonicalized.iter().all(|v| v.is_string() || v.is_number() || v.is_boolean()) {
                canonicalized.sort_by(|a, b| {
                    // Compare string representations for deterministic ordering
                    let a_str = serde_json::to_string(a).unwrap_or_default();
                    let b_str = serde_json::to_string(b).unwrap_or_default();
                    a_str.cmp(&b_str)
                });
            } else if canonicalized.iter().all(|v| v.is_object()) {
                // For arrays of objects, sort by canonical string representation
                canonicalized.sort_by(|a, b| {
                    let a_str = serde_json::to_string(a).unwrap_or_default();
                    let b_str = serde_json::to_string(b).unwrap_or_default();
                    a_str.cmp(&b_str)
                });
            }
            
            Ok(serde_json::Value::Array(canonicalized))
        }
        _ => Ok(value.clone())
    }
}

fn generate_key_pair() -> Result<(String, String)> {
    // Generate Ed25519 key pair using ed25519-dalek
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use base64::{Engine as _, engine::general_purpose};
    
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    
    // Serialize to bytes and encode as base64
    let secret_bytes = signing_key.to_bytes();
    let public_bytes = verifying_key.to_bytes();
    
    let secret_b64 = general_purpose::STANDARD.encode(secret_bytes);
    let public_b64 = general_purpose::STANDARD.encode(public_bytes);
    
    Ok((secret_b64, public_b64))
}

fn sign_card(card: &Card, secret_key: &str) -> Result<(String, String)> {
    // Compute hash of canonical card
    let hash_hex = compute_card_hash(card)?;
    // Decode hex string to get actual hash bytes
    let hash_bytes = hex::decode(&hash_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode hash: {}", e))?;
    
    // Load signing key from base64
    use ed25519_dalek::{SigningKey, Signer};
    use base64::{Engine as _, engine::general_purpose};
    
    let key_bytes = general_purpose::STANDARD.decode(secret_key.trim())?;
    if key_bytes.len() != 32 {
        anyhow::bail!("Invalid signing key length: expected 32 bytes, got {}", key_bytes.len());
    }
    let key_array: [u8; 32] = key_bytes[..32].try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signing key length"))?;
    let signing_key = SigningKey::from_bytes(&key_array);
    
    // Sign the hash
    let signature = signing_key.sign(&hash_bytes);
    let verifying_key = signing_key.verifying_key();
    
    // Encode signature and public key
    let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());
    let public_key_b64 = general_purpose::STANDARD.encode(verifying_key.to_bytes());
    
    Ok((signature_b64, public_key_b64))
}

fn verify_signature(card: &Card, public_key: &str, signature: &str) -> Result<bool> {
    // Recompute hash of canonical card
    let hash_hex = compute_card_hash(card)?;
    // Decode hex string to get actual hash bytes
    let hash_bytes = hex::decode(&hash_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode hash: {}", e))?;
    
    // Load verifying key and signature from base64
    use ed25519_dalek::{VerifyingKey, Verifier, Signature};
    use base64::{Engine as _, engine::general_purpose};
    
    let public_key_bytes = general_purpose::STANDARD.decode(public_key.trim())?;
    if public_key_bytes.len() != 32 {
        anyhow::bail!("Invalid verifying key length: expected 32 bytes, got {}", public_key_bytes.len());
    }
    let public_key_array: [u8; 32] = public_key_bytes[..32].try_into()
        .map_err(|_| anyhow::anyhow!("Invalid verifying key length"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|e| anyhow::anyhow!("Invalid verifying key: {}", e))?;
    
    let signature_bytes = general_purpose::STANDARD.decode(signature.trim())?;
    if signature_bytes.len() != 64 {
        anyhow::bail!("Invalid signature length: expected 64 bytes, got {}", signature_bytes.len());
    }
    let signature_array: [u8; 64] = signature_bytes[..64].try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
    let signature = Signature::from_bytes(&signature_array);
    
    // Verify signature
    verifying_key.verify(&hash_bytes, &signature)
        .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))?;
    
    Ok(true)
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
                // Parse and execute full query DSL
                let all_cards = load_all_cards(&repo)?;
                let parsed_query = query::parse_query_shorthand(q)?;
                execute_query(all_cards, Some(&parsed_query))
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
                
                // Compute hash and sign BEFORE modifying the card
                let (signature, public_key_b64) = sign_card(&card, &secret_key)?;
                
                // Create key identifier from public key (first 16 bytes as hex)
                use base64::{Engine as _, engine::general_purpose};
                let public_key_bytes = general_purpose::STANDARD.decode(&public_key_b64)?;
                let key_id = hex::encode(&public_key_bytes[..16.min(public_key_bytes.len())]);
                
                // Add signature to card
                card.sign = Some(cardstack_lib::card::Signature {
                    algo: "ed25519".to_string(),
                    by: format!("key:{}", key_id),
                    sig: signature,
                });
                
                // Save card (updates timestamp/version, but signature remains valid
                // because hash excludes these fields)
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
                // Parse and execute full query DSL
                let all_cards = load_all_cards(&repo)?;
                let parsed_query = query::parse_query_shorthand(q)?;
                execute_query(all_cards, Some(&parsed_query))
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
