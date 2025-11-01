# Scribe

Core CRUD operations on cards and decks.

## Overview

Scribe is the primary tool for creating, reading, updating, and managing cards in your cardstack repository. It handles all basic operations including card creation, editing, linking, and deck management.

## Installation

```bash
cargo install --path scribe
# or
cargo build --release --bin scribe
```

## Commands

### `scribe init`

Bootstrap a new cardstack repository in the current directory.

**Usage:**
```bash
scribe init
```

Creates the following structure:
- `cards/` - Card storage directory
- `media/` - Media attachments
- `.cardstack/` - Hidden workspace (config, index, cache)
- `schemas/` - JSON Schema files

### `scribe new`

Create a new card.

**Usage:**
```bash
scribe new "Title" [OPTIONS]
```

**Options:**
- `--template <slug>` - Create from template card
- `--slug <slug>` - Custom slug (auto-generated from title if omitted)
- `--tag <tag>` - Add tags (can be repeated)
- `--field <key=value>` - Add custom fields (can be repeated)
- `--body <path>` - Load body content from file
- `--json` - Output JSON CardEnvelope instead of file path

**Examples:**
```bash
# Create a simple card
scribe new "My First Card"

# Create with tags and fields
scribe new "Research Note" --tag research --tag important --field status=draft --field priority=high

# Create from template
scribe new "Meeting Notes" --template meeting-template

# Create with body from file
scribe new "Project Plan" --body notes.md
```

### `scribe show`

Display a card by UID or slug.

**Usage:**
```bash
scribe show <uid|slug>
```

**Options:**
- `--json` - Output JSON CardEnvelope format

**Examples:**
```bash
scribe show ulid_01ABC123
scribe show my-card-slug
scribe show my-card --json
```

### `scribe edit`

Modify an existing card.

**Usage:**
```bash
scribe edit <uid|slug> [OPTIONS]
```

**Options:**
- `--title <title>` - Change title
- `--slug <slug>` - Change slug
- `--field <key=value>` - Set or update field (can be repeated)
- `--unset <key>` - Remove field (can be repeated)
- `--set-body <path>` - Replace body content from file
- `--append-body <path>` - Append body content from file

**Examples:**
```bash
# Change title
scribe edit my-card --title "Updated Title"

# Update fields
scribe edit my-card --field status=published --field priority=low

# Replace body
scribe edit my-card --set-body new-content.md

# Append to body
scribe edit my-card --append-body additional-notes.md
```

### `scribe archive`

Soft-delete a card by marking it as archived.

**Usage:**
```bash
scribe archive <uid|slug>
```

**Examples:**
```bash
scribe archive old-card
```

### `scribe fork`

Duplicate a card with provenance tracking.

**Usage:**
```bash
scribe fork <uid|slug> [--with-links]
```

**Options:**
- `--with-links` - Copy all links from source card

**Examples:**
```bash
# Basic fork
scribe fork original-card

# Fork with links preserved
scribe fork original-card --with-links
```

### `scribe merge`

Combine two cards, merging content and metadata.

**Usage:**
```bash
scribe merge <src> <dst> [--strategy <strategy>]
```

**Options:**
- `--strategy <ours|theirs|manual>` - Merge strategy (future use)

**Examples:**
```bash
scribe merge card-a card-b
```

**Behavior:**
- Bodies are merged with separator
- Tags are unioned (duplicates removed)
- Fields are merged (conflicts noted in `_conflicts` field)
- Source card is archived with `merged_into` field
- Provenance link (`derived-from`) is added to destination

### `scribe link`

Create a typed link between cards.

**Usage:**
```bash
scribe link <from> --to <to> --type <type>
```

**Common link types:**
- `contains` - Deck contains card
- `cites` - Card references another
- `parent-of` - Hierarchical parent
- `part-of` - Composition relationship
- `relates-to` - General relationship

**Examples:**
```bash
# Create a citation link
scribe link research-note --to source-card --type cites

# Add card to deck
scribe link my-deck --to my-card --type contains

# Create hierarchical link
scribe link parent-card --to child-card --type parent-of
```

### `scribe unlink`

Remove a link between cards.

**Usage:**
```bash
scribe unlink <from> --to <to> [--type <type>]
```

**Options:**
- `--type <type>` - Remove only links of specific type (otherwise removes all links between cards)

**Examples:**
```bash
# Remove all links
scribe unlink card-a --to card-b

# Remove specific link type
scribe unlink card-a --to card-b --type cites
```

## Deck Operations

### `scribe deck:new`

Create a new deck (card with collection facet).

**Usage:**
```bash
scribe deck:new "Name" [--mode <static|query|hybrid>] [--query "..."]
```

**Options:**
- `--mode` - Deck mode: `static` (explicit members), `query` (dynamic), or `hybrid`
- `--query` - Query DSL for dynamic/hybrid decks

**Examples:**
```bash
# Create static deck
scribe deck:new "Project Alpha"

# Create query-based deck
scribe deck:new "Open Drafts" --mode query --query "status=draft tag:design"

# Create hybrid deck
scribe deck:new "My Collection" --mode hybrid --query "tag:favorite"
```

### `scribe deck:add`

Add cards to a static deck.

**Usage:**
```bash
scribe deck:add <deck> <card1> [card2 ...]
```

**Examples:**
```bash
scribe deck:add my-deck card1 card2 card3
```

### `scribe deck:remove`

Remove cards from a static deck.

**Usage:**
```bash
scribe deck:remove <deck> <card1> [card2 ...]
```

**Examples:**
```bash
scribe deck:remove my-deck card1 card2
```

### `scribe deck:snapshot`

Freeze a dynamic deck into a static snapshot.

**Usage:**
```bash
scribe deck:snapshot <deck> --out <uid|slug>
```

**Examples:**
```bash
scribe deck:snapshot open-drafts --out open-drafts-2025-01-15
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root (default: auto-detect from current directory)
- `--json` - Output JSON format where applicable

## Examples

```bash
# Initialize repository
scribe init

# Create a research note
scribe new "Quantum Computing Basics" \
  --tag research \
  --tag quantum \
  --field status=draft \
  --field topic=computing

# Create a deck for organizing research
scribe deck:new "Research Queue" --mode query --query "status=draft tag:research"

# Link cards
scribe link quantum-basics --to computing-fundamentals --type cites

# Update card status
scribe edit quantum-basics --field status=published
```

## Output Formats

- **Human-readable** (default): File paths, status messages
- **JSON**: Use `--json` flag for machine-readable `CardEnvelope` format

## File Storage

Cards are stored in:
```
cards/YYYY/MM/<uid>--<slug>.yaml
```

The UID is a ULID (time-ordered), making files naturally sortable by creation time.

