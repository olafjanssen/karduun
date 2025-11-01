# Karduun CLI Suite

A comprehensive Rust CLI toolkit for managing atomic cards and self-organizing decks. Cards are stored as YAML+Markdown files with a hidden `.cardstack/` workspace for indexes and cache. The suite is split into 8 composable tools that communicate via JSONL streams.

## Vision

**Small, True, and Composable**: Everything worth thinking about can be represented as finite cards and composable decks. A card is the smallest honest unit of meaning; a deck is a lens (static list or query) that arranges cards for a purpose.

## Principles

- **One primitive**: Everything is a Card. "Decks" and "Templates" are facets of some cards.
- **Finite semantic volume**: Cards maintain a target size band and measurable semantic volume (SV). Automation proposes split/merge/prune/refactor based on metrics.
- **Human + machine readability**: YAML front-matter + Markdown body; JSON for programmatic I/O; Git for history.
- **Typed links, loose hierarchy**: Graphs first (contains, parent-of, part-of, cites…), trees only when needed and acyclic.
- **Determinism**: Queries, sorting, metrics, and exports are deterministic and reproducible.
- **CLI-first**: Everything is an idempotent command. Composable shell UX is default.

## Tools

| Tool | Purpose | Commands |
|------|---------|----------|
| **scribe** | Core CRUD operations | `init`, `new`, `show`, `edit`, `archive`, `fork`, `merge`, `link`, `deck:*` |
| **scout** | Query and search | `list`, `grep`, `backlinks`, `tree` |
| **catalog** | Index builder | `rebuild`, `status`, `vacuum` |
| **gauge** | Semantic Volume analyzer | `analyze` |
| **curator** | Organization planner/executor | `plan`, `apply`, `autoclean` |
| **stencil** | Template management | `new`, `list`, `show`, `validate` |
| **porter** | Import/Export | `export`, `import` |
| **notary** | Signing & timestamping | `sign`, `verify`, `timestamp` |

## Quick Start

```bash
# Initialize a new cardstack repository
scribe init

# Create your first card
scribe new "My First Card" --tag example

# Rebuild the index
catalog rebuild

# List all cards
scout list

# Analyze semantic volume
scout list --jsonl | gauge analyze --jsonl

# Self-organize your cards
scout list --query "sv>1.6" --jsonl | gauge analyze --jsonl | curator autoclean --apply
```

## Installation

```bash
# Build from source
cargo build --release

# Install all tools
cargo install --path scribe
cargo install --path scout
# ... etc for each tool
```

## Storage Model

**Visible** (committed to Git):
- `/cards/YYYY/MM/<uid>--<slug>.yaml` - Card files
- `/media/<hash>/<filename>` - Media attachments

**Hidden** (`.cardstack/`, mostly ignored):
- `config.yml` - Repository settings (committed)
- `index/cards.db` - SQLite index with FTS5
- `embeddings/` - Cached sentence embeddings
- `logs/` - Action logs
- `hooks/` - Pre/post hooks

## Card Model

Every entity is a **Card** with optional facets:

- `facets.content`: Markdown body
- `facets.collection`: Deck behavior (static/query/hybrid)
- `facets.template`: Template definitions with constraints

See `/schemas/Card.schema.json` for the complete schema.

## Semantic Volume

Cards maintain measurable metrics:
- **tokens**: Word/tokenizer count
- **nid_bpt**: Normalized Information Density (bits per token)
- **cohesion**: Mean pairwise cosine similarity
- **bandwidth**: Number of topic clusters
- **redundancy**: Similarity to nearest neighbor
- **sv**: Composite Semantic Volume score

Automation uses these metrics to suggest **split**, **merge**, **prune**, or **refactor** actions.

## License

- Code: MIT
- Documentation: CC-BY-4.0

