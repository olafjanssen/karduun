# Catalog

Build and manage the SQLite index for fast card queries.

## Overview

Catalog creates and maintains a SQLite database index of all cards in your repository. This enables fast queries, full-text search, and efficient link traversal. The index is stored in `.cardstack/index/cards.db` and includes FTS5 for full-text search.

## Installation

```bash
cargo install --path catalog
# or
cargo build --release --bin catalog
```

## Commands

### `catalog rebuild`

Build or rebuild the SQLite index from card files.

**Usage:**
```bash
catalog rebuild
```

**What it does:**
1. Scans `cards/` directory recursively
2. Parses all `.yaml` card files
3. Populates database tables:
   - `cards` - Card metadata (uid, slug, title, tags, fields, etc.)
   - `links` - Typed links between cards
   - `computed` - Computed metrics (from gauge analysis)
   - `fts` - FTS5 virtual table for full-text search
4. Clears existing index data before rebuilding

**Examples:**
```bash
# Rebuild index
catalog rebuild

# First run after init
catalog rebuild
```

**Output:**
```
Loading cards from filesystem...
Found 42 card(s)
Index rebuilt successfully
  - 42 cards indexed
```

**When to rebuild:**
- After creating/modifying cards outside of tools
- After bulk operations
- When index appears stale (check with `catalog status`)
- Periodically for large repositories

### `catalog status`

Show index health and staleness information.

**Usage:**
```bash
catalog status
```

**Output:**
```
Index Status:
  Database: .cardstack/index/cards.db
  Cards: 42
  Links: 87
  FTS entries: 42
  Filesystem cards: 42
  ✓ Index is up to date
```

**Staleness Detection:**
- Compares card count in database vs filesystem
- Warns if counts don't match
- Suggests rebuild if stale

**Examples:**
```bash
# Check index health
catalog status

# Regular health check
catalog status
```

### `catalog vacuum`

Optimize the SQLite database.

**Usage:**
```bash
catalog vacuum
```

**What it does:**
- Reclaims unused disk space
- Defragments database
- Updates statistics for query planner
- Can improve query performance

**When to use:**
- After deleting many cards
- Periodically for large databases
- If database file size is unusually large

**Examples:**
```bash
# Optimize after cleanup
catalog vacuum

# Regular maintenance
catalog vacuum
```

## Database Schema

### `cards` Table
- `uid` (PRIMARY KEY) - Card identifier
- `slug` - Human-readable identifier
- `title` - Card title
- `created`, `updated` - Timestamps (ISO 8601)
- `tags_json` - Tags as JSON array
- `fields_json` - Custom fields as JSON object
- `has_collection` - Boolean flag
- `has_template` - Boolean flag
- `path` - Relative file path

### `links` Table
- `src_uid` - Source card UID
- `type` - Link type (contains, cites, etc.)
- `dst_uid` - Destination card UID
- PRIMARY KEY (src_uid, type, dst_uid)

### `computed` Table
- `uid` (PRIMARY KEY, FK to cards)
- `tokens`, `nid_bpt`, `cohesion`, `bandwidth`, `redundancy`
- `link_density`, `structure_density`, `sv`
- `last_analyzed` - Timestamp

### `fts` Table (FTS5 Virtual Table)
- `uid` - Card UID (UNINDEXED)
- `body` - Card content (indexed for full-text search)

## Index Maintenance

### Regular Workflow

```bash
# After creating cards
scribe new "Card 1"
scribe new "Card 2"
catalog rebuild

# Check status
catalog status

# Optimize periodically
catalog vacuum
```

### Automation

The index is automatically updated when using `scribe` commands, but you may want to rebuild after:
- Manual file edits
- Bulk imports
- Git operations (pull, merge, etc.)

### Performance

**Rebuild times:**
- ~100 cards: <1 second
- ~1,000 cards: <5 seconds
- ~10,000 cards: <30 seconds

**Query performance:**
- Indexed queries: <50ms for most operations
- File-based fallback: slower but always works

## Integration

### With Scout

Scout can use the index for faster queries:
```bash
catalog rebuild
scout list --query "tag:research"  # Uses index
```

### With Gauge

Computed metrics are stored in the index:
```bash
gauge analyze --query "tag:design" --jsonl
catalog rebuild  # Updates computed table
```

### With SQLite Tools

You can inspect the database directly:
```bash
sqlite3 .cardstack/index/cards.db "SELECT * FROM cards LIMIT 10;"
sqlite3 .cardstack/index/cards.db "SELECT COUNT(*) FROM links;"
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root

## Troubleshooting

### Index not found
```
Error: Index does not exist. Run 'catalog rebuild' to create it.
```
Solution: Run `catalog rebuild`

### Stale index warning
```
⚠️  Index may be stale (count mismatch)
```
Solution: Run `catalog rebuild`

### Large database file
Solution: Run `catalog vacuum` to optimize

### Corrupted database
Solution: Delete `.cardstack/index/cards.db` and rebuild

## Storage

The index is stored in:
```
.cardstack/index/cards.db
```

This file is excluded from Git by default (via `.gitignore`). It can be safely deleted and rebuilt at any time - the source of truth is always the card files in `cards/`.

