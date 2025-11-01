# Scout

Query and search cards in your cardstack repository.

## Overview

Scout provides read-only query and search capabilities for cards. It can list cards matching filters, perform full-text search, find backlinks, and display hierarchical relationships.

## Installation

```bash
cargo install --path scout
# or
cargo build --release --bin scout
```

## Commands

### `scout list`

List cards matching a query.

**Usage:**
```bash
scout list [--query "..."] [--sort "..."] [--limit N] [--jsonl]
```

**Options:**
- `--query <dsl>` - Query DSL filter
- `--sort <fields>` - Comma-separated sort fields (prefix with `-` for descending)
- `--limit <N>` - Maximum results
- `--jsonl` - Output JSONL CardEnvelope format

**Query DSL Examples:**
```bash
# Simple field filter
scout list --query "status=draft"

# Tag filter
scout list --query "tag:research"

# Multiple conditions
scout list --query "status=draft tag:design"

# Sort results
scout list --query "tag:research" --sort "-updated,title"

# Limit results
scout list --query "tag:important" --limit 10
```

**Sort Fields:**
- `updated` - Last update time (default descending)
- `created` - Creation time
- `title` - Alphabetical
- `uid` - Tiebreaker (always last)

**Examples:**
```bash
# List all cards
scout list

# Find draft cards
scout list --query "status=draft"

# Find cards with specific tag
scout list --query "tag:research"

# Sort by update time (newest first)
scout list --sort "-updated"

# Output JSONL for piping
scout list --query "tag:design" --jsonl
```

### `scout grep`

Full-text search in card content.

**Usage:**
```bash
scout grep <pattern> [--query "..."] [--paths] [--jsonl]
```

**Options:**
- `--query <dsl>` - Additional filter query
- `--paths` - Output file paths only
- `--jsonl` - Output JSONL CardEnvelope format

**Examples:**
```bash
# Search for text in all cards
scout grep "quantum computing"

# Search within filtered cards
scout grep "algorithm" --query "tag:research"

# Get file paths only
scout grep "TODO" --paths

# JSONL output for processing
scout grep "bug" --jsonl
```

### `scout backlinks`

Show all cards that link to a target card.

**Usage:**
```bash
scout backlinks <uid|slug> [--jsonl]
```

**Options:**
- `--jsonl` - Output JSONL CardEnvelope format

**Examples:**
```bash
# Find backlinks to a card
scout backlinks my-card

# JSONL output
scout backlinks ulid_01ABC123 --jsonl
```

**Use Cases:**
- Find which cards reference a particular card
- Discover related content
- Track citation networks

### `scout tree`

Display hierarchical tree structure via parent-of and contains links.

**Usage:**
```bash
scout tree <uid|slug> [--depth N] [--jsonl]
```

**Options:**
- `--depth <N>` - Maximum tree depth (default: 10)
- `--jsonl` - Output JSONL format (flattened)

**Examples:**
```bash
# Show tree structure
scout tree root-card

# Limit depth
scout tree root-card --depth 3

# Visual output
scout tree project-root
# Output:
# ulid_01ABC... - Project Root
#   ├─ ulid_01DEF... - Section 1
#   │   ├─ ulid_01GHI... - Subsection 1.1
#   │   └─ ulid_01JKL... - Subsection 1.2
#   └─ ulid_01MNO... - Section 2
```

**Link Types Traversed:**
- `parent-of` - Parent-child hierarchy
- `contains` - Deck membership (when parent is deck)

## Query DSL

Scout uses a simple query DSL that can be parsed into canonical JSON.

### Field Filters

```bash
# Equality
status=draft
fields.priority=high

# Field paths use dot notation
fields.author.name="John Doe"
```

### Tag Filters

```bash
# Tag presence
tag:research

# Multiple tags (AND)
tag:research tag:important
```

### Link Filters

```bash
# Has link type
link:contains

# Link to specific card
link:contains>ulid_01ABC123
```

### Boolean Logic

- Multiple conditions are AND by default
- Queries are parsed into `all`/`any`/`none` filters

### Sort Specification

```bash
# Single field
--sort "-updated"

# Multiple fields (comma-separated)
--sort "-updated,title,uid"

# Descending (prefix with -)
--sort "-updated"  # newest first
--sort "created"   # oldest first
```

## Output Formats

### Human-Readable (default)

Lists card UIDs and titles:
```
Found 5 card(s)
  ulid_01ABC... - Research Note 1
  ulid_01DEF... - Research Note 2
  ...
```

### JSONL Format

Each line is a `CardEnvelope` JSON object:
```json
{"type":"card","uid":"ulid_01ABC...","slug":"research-note","title":"Research Note",...}
```

Perfect for piping to other tools:
```bash
scout list --query "tag:research" --jsonl | gauge analyze --jsonl
```

## Integration Examples

### Find cards needing analysis
```bash
scout list --query "status=draft" --jsonl | gauge analyze --jsonl
```

### Search and analyze
```bash
scout grep "TODO" --jsonl | gauge analyze --jsonl
```

### Find orphaned cards (no backlinks)
```bash
scout list --jsonl | jq -r '.uid' | while read uid; do
  if [ -z "$(scout backlinks "$uid")" ]; then
    echo "$uid has no backlinks"
  fi
done
```

### Pipeline to organization
```bash
scout list --query "status=draft" --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --yes
```

## Performance Notes

- Uses file-based search by default
- Can integrate with `catalog` index when available for faster queries
- For large repositories (>1000 cards), use `catalog rebuild` first

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--jsonl` - Machine-readable JSONL output

