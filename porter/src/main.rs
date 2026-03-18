use anyhow::{Context, Result};
use cardstack_lib::{
    card::{Card, CardEnvelope},
    get_repo_root, load_all_cards, save_card, uid,
};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, WriterBuilder};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "porter")]
#[command(about = "Import and export cards", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    #[arg(long, global = true)]
    anonymize: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Export cards to various formats
    Export {
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        format: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Import cards from various formats
    Import {
        #[arg(long)]
        from: String,
        #[arg(long)]
        r#in: PathBuf,
        #[arg(long)]
        template: Option<String>,
    },
}

fn filter_cards(cards: Vec<Card>, query: Option<&str>) -> Result<Vec<Card>> {
    if let Some(q) = query {
        // Simple tag filtering for now
        if q.starts_with("tag:") {
            let tag = q.strip_prefix("tag:").unwrap();
            Ok(cards
                .into_iter()
                .filter(|c| c.tags.contains(&tag.to_string()))
                .collect())
        } else {
            // For more complex queries, would parse query DSL
            Ok(cards)
        }
    } else {
        Ok(cards)
    }
}

fn export_jsonl(cards: Vec<Card>, output_dir: &Path, anonymize: bool) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    let file_path = output_dir.join("cards.jsonl");
    let mut file = fs::File::create(&file_path)?;
    let count = cards.len();

    for card in cards {
        let envelope = CardEnvelope::from(card);

        if anonymize {
            // Remove sensitive data
            if let Some(_facets) = &envelope.facets {
                // Would need to remove author info, etc. from facets
            }
        }

        let json = serde_json::to_string(&envelope)?;
        writeln!(file, "{}", json)?;
    }

    println!("Exported {} cards to {}", count, file_path.display());

    Ok(())
}

fn export_csv(cards: Vec<Card>, output_dir: &Path, _anonymize: bool) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    let file_path = output_dir.join("cards.csv");
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_path(&file_path)?;

    let count = cards.len();

    // Write headers
    writer.write_record(&["uid", "slug", "title", "tags", "fields", "body"])?;

    for card in cards {
        let tags = card.tags.join(";");
        let fields = serde_json::to_string(&card.fields)?;
        let body = card.get_content().unwrap_or("").replace('\n', "\\n");

        writer.write_record(&[&card.uid, &card.slug, &card.title, &tags, &fields, &body])?;
    }

    writer.flush()?;
    println!("Exported {} cards to {}", count, file_path.display());

    Ok(())
}

fn export_markdown(cards: Vec<Card>, output_dir: &Path, _anonymize: bool) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    let count = cards.len();

    for card in cards {
        let filename = format!("{}.md", card.slug);
        let file_path = output_dir.join(&filename);
        let mut file = fs::File::create(&file_path)?;

        // Write front matter
        writeln!(file, "---")?;
        writeln!(file, "uid: {}", card.uid)?;
        writeln!(file, "slug: {}", card.slug)?;
        writeln!(file, "title: {}", card.title)?;
        writeln!(file, "created: {}", card.created.to_rfc3339())?;
        writeln!(file, "updated: {}", card.updated.to_rfc3339())?;

        if !card.tags.is_empty() {
            writeln!(file, "tags:")?;
            for tag in &card.tags {
                writeln!(file, "  - {}", tag)?;
            }
        }

        if !card.fields.is_empty() {
            writeln!(file, "fields:")?;
            for (k, v) in &card.fields {
                writeln!(file, "  {}: {:?}", k, v)?;
            }
        }

        writeln!(file, "---")?;
        writeln!(file)?;

        // Write body
        if let Some(body) = card.get_content() {
            writeln!(file, "{}", body)?;
        }
    }

    println!("Exported {} cards to {}", count, output_dir.display());

    Ok(())
}

fn import_jsonl(
    repo: &Path,
    input_dir: &Path,
    template: Option<&str>,
    _anonymize: bool,
) -> Result<()> {
    let file_path = input_dir.join("cards.jsonl");
    if !file_path.exists() {
        anyhow::bail!("cards.jsonl not found in {}", input_dir.display());
    }

    let file = fs::File::open(&file_path)?;
    let reader = io::BufReader::new(file);
    let mut count = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse the JSON line into a serde_json::Value to handle missing fields manually
        let value: serde_json::Value = serde_json::from_str(&line)?;

        // Extract fields with defaults
        let title = value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let uid = value
            .get("uid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uid::generate_uid());

        let slug = value
            .get("slug")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| title.to_lowercase().replace(' ', "-"));

        let created = value
            .get("created")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        let updated = value
            .get("updated")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        let tags = value
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let fields = value
            .get("fields")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<std::collections::HashMap<String, serde_json::Value>>()
            })
            .unwrap_or_default();

        let body = value
            .get("facets")
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("body"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Create the card
        let mut card = Card::new(title.clone(), slug.clone(), uid.clone());
        card.created = created;
        card.updated = updated;
        card.tags = tags;
        card.fields = fields;

        if !body.is_empty() {
            card = card.with_content(body);
        }

        // Apply template if specified
        if let Some(template_slug) = template {
            card.fields.insert(
                "_template".to_string(),
                serde_json::Value::String(template_slug.to_string()),
            );
        }

        save_card(repo, &mut card)?;
        count += 1;
    }

    println!("Imported {} cards from {}", count, file_path.display());
    Ok(())
}

fn import_csv(
    repo: &Path,
    input_dir: &Path,
    template: Option<&str>,
    _anonymize: bool,
) -> Result<()> {
    let file_path = input_dir.join("cards.csv");
    if !file_path.exists() {
        anyhow::bail!("cards.csv not found in {}", input_dir.display());
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&file_path)?;

    let mut count = 0;

    for result in reader.records() {
        let record = result?;

        let uid = record.get(0).context("Missing uid")?.to_string();
        let slug = record.get(1).context("Missing slug")?.to_string();
        let title = record.get(2).context("Missing title")?.to_string();
        let tags_str = record.get(3).unwrap_or("");
        let fields_str = record.get(4).unwrap_or("{}");
        let body = record.get(5).unwrap_or("").replace("\\n", "\n");

        let tags: Vec<String> = tags_str
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let fields: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(fields_str).unwrap_or_default();

        let mut card = Card::new(title, slug, uid);
        card.tags = tags;
        card.fields = fields;
        card = card.with_content(body);

        // Apply template if specified
        if let Some(template_slug) = template {
            card.fields.insert(
                "_template".to_string(),
                serde_json::Value::String(template_slug.to_string()),
            );
        }

        save_card(repo, &mut card)?;
        count += 1;
    }

    println!("Imported {} cards from {}", count, file_path.display());
    Ok(())
}

fn import_markdown(
    repo: &Path,
    input_dir: &Path,
    template: Option<&str>,
    _anonymize: bool,
) -> Result<()> {
    let mut count = 0;

    for entry in WalkDir::new(input_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = fs::read_to_string(path)?;

            // Simple markdown parsing (look for front matter)
            if content.starts_with("---\n") {
                if let Some((front_matter, body)) =
                    content.splitn(3, "---\n").nth(2).and_then(|rest| {
                        let parts: Vec<&str> = rest.splitn(2, "---\n").collect();
                        if parts.len() >= 2 {
                            Some((parts[0], parts[1]))
                        } else {
                            None
                        }
                    })
                {
                    // Parse YAML front matter
                    let front: serde_yaml::Value = serde_yaml::from_str(front_matter)?;

                    let uid = front
                        .get("uid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| uid::generate_uid());
                    let slug = front
                        .get("slug")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("imported")
                                .to_string()
                        });
                    let title = front
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Imported Card")
                        .to_string();

                    let mut card = Card::new(title, slug, uid);

                    // Parse tags
                    if let Some(tags) = front.get("tags").and_then(|v| v.as_sequence()) {
                        card.tags = tags
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }

                    // Parse fields
                    if let Some(fields) = front.get("fields").and_then(|v| v.as_mapping()) {
                        for (k, v) in fields {
                            if let Some(key) = k.as_str() {
                                card.fields
                                    .insert(key.to_string(), serde_json::to_value(v)?);
                            }
                        }
                    }

                    card = card.with_content(body.trim().to_string());

                    // Apply template if specified
                    if let Some(template_slug) = template {
                        card.fields.insert(
                            "_template".to_string(),
                            serde_json::Value::String(template_slug.to_string()),
                        );
                    }

                    save_card(repo, &mut card)?;
                    count += 1;
                }
            }
        }
    }

    println!("Imported {} cards from {}", count, input_dir.display());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = get_repo_root(cli.repo.clone())?;

    match &cli.command {
        Commands::Export { query, format, out } => {
            let all_cards_with_paths = load_all_cards(&repo)?;
            let all_cards: Vec<Card> = all_cards_with_paths
                .into_iter()
                .map(|(_, card)| card)
                .collect();
            let filtered = filter_cards(all_cards, query.as_deref())?;

            match format.as_str() {
                "jsonl" => export_jsonl(filtered, out, cli.anonymize)?,
                "csv" => export_csv(filtered, out, cli.anonymize)?,
                "md" | "markdown" => export_markdown(filtered, out, cli.anonymize)?,
                _ => anyhow::bail!("Unknown format: {}. Use jsonl, csv, or md", format),
            }
        }
        Commands::Import {
            from,
            r#in,
            template,
        } => match from.as_str() {
            "jsonl" => import_jsonl(&repo, r#in, template.as_deref(), cli.anonymize)?,
            "csv" => import_csv(&repo, r#in, template.as_deref(), cli.anonymize)?,
            "md" | "markdown" => import_markdown(&repo, r#in, template.as_deref(), cli.anonymize)?,
            _ => anyhow::bail!("Unknown format: {}. Use jsonl, csv, or md", from),
        },
    }

    Ok(())
}
