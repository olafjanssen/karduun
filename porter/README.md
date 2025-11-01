# Porter

Import and export cards in various formats.

## Overview

Porter handles importing cards from external sources and exporting cards to standard formats (JSONL, CSV, Markdown) for backup, sharing, or migration between cardstack repositories.

## Installation

```bash
cargo install --path porter
# or
cargo build --release --bin porter
```

## Commands

### `porter export`

Export cards to various formats.

**Usage:**
```bash
porter export --format <format> --out <dir> [--query "..."] [--anonymize]
```

**Options:**
- `--format <jsonl|csv|md>` - Output format
- `--out <dir>` - Output directory
- `--query <dsl>` - Filter cards to export (simple tag: or status= filters)
- `--anonymize` - Remove sensitive data (signatures, emails)

**Formats:**
- **jsonl** - JSON Lines format (one CardEnvelope per line)
- **csv** - CSV with columns: uid, slug, title, tags, fields, body
- **md** / **markdown** - Markdown files with YAML front matter

**Examples:**
```bash
# Export all cards to JSONL
porter export --format jsonl --out ./backup

# Export filtered cards to CSV
porter export --format csv --out ./export \
  --query "tag:research"

# Export to Markdown
porter export --format md --out ./docs

# Anonymized export
porter export --format jsonl --out ./public --anonymize
```

**JSONL Format:**
Each line is a complete `CardEnvelope`:
```json
{"type":"card","uid":"ulid_01ABC...","slug":"my-card","title":"My Card",...}
{"type":"card","uid":"ulid_01DEF...","slug":"another-card","title":"Another Card",...}
```

**CSV Format:**
```csv
uid,slug,title,tags,fields,body
ulid_01ABC...,my-card,My Card,"tag1;tag2","{\"status\":\"draft\"}","Card content..."
```

**Markdown Format:**
Each card becomes a `.md` file with YAML front matter:
```markdown
---
uid: ulid_01ABC...
slug: my-card
title: My Card
created: 2025-01-15T10:30:00Z
updated: 2025-01-15T10:30:00Z
tags:
  - tag1
  - tag2
fields:
  status: "draft"
---
Card content here...
```

### `porter import`

Import cards from various formats.

**Usage:**
```bash
porter import --from <format> --in <dir> [--template <slug>] [--anonymize]
```

**Options:**
- `--from <jsonl|csv|md>` - Input format
- `--in <dir>` - Input directory
- `--template <slug>` - Apply template to imported cards
- `--anonymize` - Import was anonymized (for metadata)

**Examples:**
```bash
# Import from JSONL
porter import --from jsonl --in ./backup

# Import from CSV
porter import --from csv --in ./import-data

# Import from Markdown
porter import --from md --in ./documents

# Import with template
porter import --from jsonl --in ./import --template template-research-note
```

**Import Behavior:**
- Cards retain their UIDs if present
- New UIDs are generated if missing
- Tags and fields are preserved
- Content is restored
- Template can be applied to enforce constraints

## Format Details

### JSONL Format

**Export:**
```bash
porter export --format jsonl --out ./export
# Creates: ./export/cards.jsonl
```

**Import:**
```bash
porter import --from jsonl --in ./export
# Reads: ./import/cards.jsonl
```

**Advantages:**
- Complete metadata preservation
- Machine-readable
- Streaming-friendly
- Best for backups and migrations

### CSV Format

**Export:**
```bash
porter export --format csv --out ./export
# Creates: ./export/cards.csv
```

**Import:**
```bash
porter import --from csv --in ./import
# Reads: ./import/cards.csv
```

**Limitations:**
- Nested fields flattened to JSON strings
- Body content escaped (newlines as `\n`)
- Lossy for complex structures

**Use Cases:**
- Spreadsheet analysis
- Simple data exchange
- Quick exports for viewing

### Markdown Format

**Export:**
```bash
porter export --format md --out ./docs
# Creates: ./docs/*.md files (one per card)
```

**Import:**
```bash
porter import --from md --in ./documents
# Reads all .md files in directory
```

**Structure:**
Each markdown file contains:
1. YAML front matter (metadata)
2. Markdown body (content)

**Advantages:**
- Human-readable
- Git-friendly
- Easy to edit manually
- Standard format

## Use Cases

### Backup and Restore

```bash
# Full backup
porter export --format jsonl --out ./backup-$(date +%Y%m%d)

# Restore
porter import --from jsonl --in ./backup-20250115
```

### Migration Between Repos

```bash
# Export from source repo
cd /path/to/source-repo
porter export --format jsonl --out ../migration

# Import to destination repo
cd /path/to/dest-repo
porter import --from jsonl --in ../migration
```

### Sharing Subset of Cards

```bash
# Export specific tags
porter export --format md --out ./share \
  --query "tag:public"

# Share ./share directory
```

### Data Analysis

```bash
# Export to CSV for analysis
porter export --format csv --out ./analysis \
  --query "tag:research"

# Open in spreadsheet or process with tools
```

### Documentation Export

```bash
# Export as Markdown for documentation
porter export --format md --out ./docs \
  --query "status=published"

# Can be processed by static site generators
```

## Anonymization

When exporting sensitive data:

```bash
porter export --format jsonl --out ./public --anonymize
```

**Anonymized data:**
- Signatures removed
- Author information sanitized
- Email addresses stripped
- Other sensitive fields removed

**Use Cases:**
- Public datasets
- Sharing without personal info
- Compliance requirements

## Template Application

Import with template to enforce structure:

```bash
porter import --from jsonl --in ./import \
  --template template-research-note
```

**What happens:**
- Template defaults applied
- Template constraints validated
- Cards linked to template via `derived-from`
- Validation errors reported

## Integration Examples

### Backup Script
```bash
#!/bin/bash
DATE=$(date +%Y%m%d)
porter export --format jsonl --out "./backups/backup-$DATE"
echo "Backup created: backups/backup-$DATE"
```

### Export for Git
```bash
# Export published cards for static site
porter export --format md --out ./public-docs \
  --query "status=published"
git add ./public-docs
git commit -m "Update published cards"
```

### Import from Markdown Notes
```bash
# Convert existing markdown notes to cards
porter import --from md --in ./notes \
  --template template-note
```

### Batch Processing
```bash
# Export, transform, re-import
porter export --format jsonl --out ./temp
# ... process with jq, python, etc. ...
porter import --from jsonl --in ./temp --template template-updated
```

## Round-Trip Testing

Verify export/import preserves data:

```bash
# Export
porter export --format jsonl --out ./test

# Import to new location
cd /tmp/test-repo
scribe init
porter import --from jsonl --in /path/to/original-repo/test

# Compare (would need custom tool)
# Cards should be identical
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--anonymize` - Remove sensitive data (export) or mark as anonymized (import)

## Best Practices

1. **Use JSONL for backups** - Complete metadata preservation
2. **Use Markdown for sharing** - Human-readable, git-friendly
3. **Use CSV for analysis** - Easy spreadsheet import
4. **Test imports** - Verify data preservation
5. **Template on import** - Apply templates to enforce structure
6. **Regular backups** - Export periodically for safety

## Limitations

- **CSV** - Nested data flattened, body content escaped
- **Markdown** - Complex fields may lose structure
- **Query filtering** - Currently limited to simple tag/status filters
- **Template validation** - Constraints checked but not enforced during import

## Troubleshooting

### Import fails
- Check file format matches `--from` specification
- Verify files exist in `--in` directory
- Ensure repository is initialized (`scribe init`)

### Data loss on import
- Use JSONL format for best preservation
- Check template constraints don't filter data
- Verify original export was complete

### CSV import issues
- Ensure CSV has correct headers
- Check for escaped characters in body field
- Verify JSON fields are valid JSON strings
