<!-- 036772ec-4be3-4e03-8cf1-a777f22be731 879d7a0b-c6e4-4886-8a6a-3d777ca999de -->
# Karduun CLI Suite - Implementation Plan

## Overview

Build a comprehensive Rust CLI toolkit (Karduun) for managing atomic cards and self-organizing decks. Cards are stored as YAML+Markdown files with a hidden `.cardstack/` workspace for indexes and cache. The suite is split into 8 composable tools that communicate via JSONL streams.

## Architecture Decisions

**Storage Model**: Hybrid approach

- Visible: `/cards/YYYY/MM/<uid>--<slug>.yaml`, `/media/<hash>/<filename>`
- Hidden: `/.cardstack/` (config, SQLite index, embeddings cache, logs, hooks)
- Git-friendly: commit `config.yml`, ignore derived data

**Card Model**: Unified - everything is a Card with optional facets:

- `facets.content`: Markdown body
- `facets.collection`: Deck behavior (static/query/hybrid)
- `facets.template`: Template definitions with constraints

**Tool Communication**: JSONL streams (CardEnvelope, AnalysisResult, OrgAction)

**Language**: Rust with standard, minimal dependencies (prefer stdlib or most common crates)

## Phase 1: Foundation & Core Tools (Weeks 1-4)

### 1.1 Project Setup (`/`)

- Initialize Rust workspace with 8 binary crates + 1 shared lib crate
- `Cargo.toml` workspace configuration
- `README.md` with vision, installation, quick start
- `.gitignore` for Rust artifacts + `.cardstack/` exclusions
- `LICENSE` (MIT for code, CC-BY for docs)

### 1.2 Shared Library (`/cardstack-lib/`)

**Purpose**: Common data structures, serialization, utilities

**Key modules**:

- `card.rs`: Card struct with serde YAML/JSON support
- `schema.rs`: Card and Query JSON Schema (Draft 2020-12)
- `query.rs`: Query DSL parser (shorthand → canonical JSON)
- `serialize.rs`: Deterministic YAML serialization (fixed key order, normalized whitespace)
- `uid.rs`: ULID/UUIDv7 generation and validation
- `canonical.rs`: Content hashing (SHA-256/Blake3) for integrity

**Dependencies**:

- `serde`, `serde_yaml`, `serde_json`, `serde_json_path`
- `ulid` or `uuid` with v7 support
- `regex` for query parsing
- `jsonschema` for validation
- `sha2` or `blake3` for hashing

**Deliverables**:

- JSON Schema files in `/schemas/Card.schema.json`, `/schemas/Query.schema.json`
- Round-trip tests for card serialization
- Query parser tests (shorthand ↔ canonical JSON)

### 1.3 Scribe Tool (`/scribe/`)

**Purpose**: Core CRUD operations on cards

**Commands**:

- `scribe init`: Bootstrap repo (create dirs, default config, schemas)
- `scribe new "Title"`: Create card with ULID, optional template
- `scribe show <uid|slug>`: Display card (human-readable or --json)
- `scribe edit <uid|slug>`: Modify metadata/content
- `scribe archive <uid|slug>`: Soft-delete
- `scribe fork <uid|slug>`: Duplicate with provenance
- `scribe merge <src> <dst>`: Combine cards
- `scribe link <from> --to <to> --type <type>`: Create typed edge
- `scribe deck:new|add|remove|snapshot`: Deck facet helpers

**Key implementation**:

- YAML front matter + Markdown body parsing
- Deterministic serialization before write
- Template application (load template card, merge defaults, enforce constraints)
- Deck facet manipulation (static members, query DSL)
- Hook system (pre-save/post-save executables in `.cardstack/hooks/`)

**Output**: Card files in `/cards/YYYY/MM/`, or JSONL CardEnvelope with --json

### 1.4 Scout Tool (`/scout/`)

**Purpose**: Query and search (read-only)

**Commands**:

- `scout list`: Query cards with filter/sort/limit
- `scout grep <pattern>`: Full-text search
- `scout backlinks <uid|slug>`: Show incoming links
- `scout tree <uid|slug>`: Hierarchical view via parent-of links

**Key implementation**:

- Query parser (supports: `status=draft tag:design sort:-updated,title`)
- File-based execution (fallback if index missing)
- Index integration (SQLite queries when catalog exists)
- Output JSONL CardEnvelope format

**Output**: JSONL stream or human-readable table

### 1.5 Catalog Tool (`/catalog/`)

**Purpose**: Index builder and cache manager

**Commands**:

- `catalog rebuild`: Build SQLite index from card files
- `catalog status`: Show index health/staleness
- `catalog vacuum`: Optimize database

**SQLite Schema**:

```
cards(uid PRIMARY KEY, slug, title, created, updated, tags_json, fields_json, 
      has_collection, has_template, path)
links(src_uid, type, dst_uid, PRIMARY KEY(src_uid,type,dst_uid))
computed(uid, tokens, nid_bpt, cohesion, bandwidth, redundancy, 
        link_density, structure_density, sv, last_analyzed)
fts(uid, body) -- FTS5 virtual table
```

**Key implementation**:

- Recursive file scan of `/cards/`
- YAML parsing + Markdown extraction
- Backlink computation (reverse index)
- FTS5 index for grep
- Deterministic tiebreaker (uid) for sort stability

**Dependencies**: `rusqlite` with FTS5, `walkdir`

## Phase 2: Analysis & Organization (Weeks 5-7)

### 2.1 Gauge Tool (`/gauge/`)

**Purpose**: Semantic Volume analyzer

**Commands**:

- `gauge analyze [--uid <uid> | --query "..."]`: Compute metrics
- Options: `--analyzer fast|full`, `--no-embeddings`, `--neighbors N`

**Metrics Implementation**:

- `tokens`: Word/tokenizer count
- `nid_bpt`: 8 * bytes(gzip(text)) / tokens (compression-based density)
- `cohesion`: Mean pairwise cosine similarity of sentence embeddings
- `bandwidth`: K-means clustering (k=1..5 via silhouette), count clusters
- `redundancy`: Max cosine sim to nearest neighbor card
- `link_density`: outbound_links / (tokens/100)
- `structure_density`: (headings + bullets + codeblocks) / (tokens/100)
- `sv`: Composite formula (see spec)

**Fast profile**: Skip embeddings (tokens, NID, structure only)

**Full profile**: Include sentence embeddings (cache in `.cardstack/embeddings/`)

**Output**: JSONL AnalysisResult with computed metrics + suggestion + rationale

**Dependencies**:

- `gzip` or `flate2` for compression
- Sentence transformers model (e.g., `sentence-transformers` via ONNX or `candle`)
- `kmeans` or manual clustering for bandwidth
- Embeddings cache (persist vectors by sentence hash)

### 2.2 Curator Tool (`/curator/`)

**Purpose**: Apply organization plans (split/merge/prune/refactor)

**Commands**:

- `curator plan`: Convert AnalysisResult → OrgAction (with threshold rules)
- `curator apply`: Execute OrgAction stream (mutate cards)
- `curator autoclean`: One-shot analyze → plan → apply

**Actions**:

- **Split**: Partition sentences by cluster; spawn children; parent becomes deck
- **Merge**: Append bodies; union metadata; archive source; add derived-from link
- **Prune**: Archive card; optionally create summary
- **Refactor**: Insert headings; improve structure

**Key implementation**:

- Threshold evaluation from `.cardstack/config.yml`
- Rationale logging to `.cardstack/logs/*.ndjson`
- History tracking (append to card or commit message)
- Hook invocation (pre-apply/post-apply)
- Dry-run mode (show diff, require --yes to write)

## Phase 3: Templates & Integration (Week 8)

### 3.1 Stencil Tool (`/stencil/`)

**Purpose**: Template management and validation

**Commands**:

- `stencil new "Name"`: Create template card
- `stencil list`: List templates
- `stencil show <slug>`: Display template
- `stencil validate <uid|--query "...">`: Check cards against template constraints

**Key implementation**:

- Template facet parsing (defaults, required_fields, enum_fields, frozen_fields)
- Constraint validation (used by scribe when --template)
- JSONL validation reports

### 3.2 Porter Tool (`/porter/`)

**Purpose**: Import/Export

**Commands**:

- `porter export --query "..." --format jsonl|csv|md --out <dir>`
- `porter import --from jsonl|csv|md --in <dir> --template <slug>`

**Formats**:

- JSONL: CardEnvelope stream
- CSV: Flattened metadata (uid, title, tags, fields, body)
- Markdown: Human-readable bundle with front matter

**Key implementation**:

- Round-trip preservation of metadata/links
- Template application on import
- Redaction support (--anonymize: strip signatures, emails)

## Phase 4: Integrity & Polish (Week 9)

### 4.1 Notary Tool (`/notary/`)

**Purpose**: Cryptographic signing and timestamping

**Commands**:

- `notary sign --query "..."`: Ed25519 sign cards
- `notary verify --query "...">`: Verify signatures
- `notary timestamp --query "...">`: OpenTimestamps integration

**Key implementation**:

- Ed25519 signing (add `sign` block to card)
- Signature verification
- Optional OpenTimestamps API integration

**Dependencies**: `ed25519-dalek`, `opentimestamps` (optional)

### 4.2 Documentation & Examples

- User guide: Command reference, query DSL, workflow examples
- Developer guide: Adding new tools, extending query predicates
- Example repo: 25+ cards, 3 decks demonstrating all features
- Golden tests: Fixtures with expected analyzer outputs

## Testing Strategy

**Unit Tests** (each tool):

- Serialization round-trips
- Query parser equivalence
- Metric calculations (golden fixtures)
- Deck operations idempotence

**Property Tests**:

- Split→Merge semantic preservation
- Deterministic query stability across reindex

**Integration Tests**:

- 10k-card repo performance (rebuild <10s, query <200ms)
- End-to-end pipelines (scout → gauge → curator)
- Export/import round-trips

**CI Pipeline**:

- `cargo test` (unit + integration)
- `cargo clippy` (lints)
- `cargo build --release` (binaries)
- Performance benchmarks (criterion)

## File Structure

```
karduun/
├── Cargo.toml                    # Workspace root
├── README.md
├── LICENSE
├── .gitignore
├── cardstack-lib/                # Shared library
│   ├── src/
│   │   ├── lib.rs
│   │   ├── card.rs
│   │   ├── schema.rs
│   │   ├── query.rs
│   │   ├── serialize.rs
│   │   ├── uid.rs
│   │   └── canonical.rs
│   └── Cargo.toml
├── scribe/                       # CRUD tool
│   ├── src/main.rs
│   └── Cargo.toml
├── scout/                        # Query tool
│   ├── src/main.rs
│   └── Cargo.toml
├── catalog/                      # Indexer
│   ├── src/main.rs
│   └── Cargo.toml
├── gauge/                        # Analyzer
│   ├── src/main.rs
│   └── Cargo.toml
├── curator/                     # Organizer
│   ├── src/main.rs
│   └── Cargo.toml
├── stencil/                      # Templates
│   ├── src/main.rs
│   └── Cargo.toml
├── porter/                       # Import/Export
│   ├── src/main.rs
│   └── Cargo.toml
├── notary/                       # Signing
│   ├── src/main.rs
│   └── Cargo.toml
├── schemas/                      # JSON Schemas
│   ├── Card.schema.json
│   └── Query.schema.json
└── tests/                        # Integration tests
    ├── fixtures/                 # Golden test data
    └── integration/
```

## Success Criteria

1. All 8 tools build and run independently
2. Core pipeline works: `scribe new` → `catalog rebuild` → `scout list` → `gauge analyze` → `curator apply`
3. Performance targets met (10k cards: index <10s, query <200ms)
4. Deterministic outputs (same query = same results after reindex)
5. Round-trip integrity (export → import preserves metadata/links)
6. All tests pass (unit, property, integration)

## Open Questions / Decisions Needed

- Embedding model choice: Local Rust crate (candle/onnx) vs external service?
- ULID vs UUIDv7 library (confirm availability)
- CLI argument parsing: `clap` (feature-rich) vs `argh` (minimal)?
- Should we add an umbrella `stack` binary that dispatches to subcommands?