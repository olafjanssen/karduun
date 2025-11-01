# Gauge

Semantic Volume analyzer for measuring and optimizing card quality.

## Overview

Gauge computes semantic metrics for cards to help identify when cards should be split, merged, pruned, or refactored. It analyzes information density, cohesion, redundancy, and structure to suggest organization actions.

## Installation

```bash
cargo install --path gauge
# or
cargo build --release --bin gauge
```

## Commands

### `gauge analyze`

Analyze cards and compute semantic volume metrics.

**Usage:**
```bash
gauge analyze [--uid <uid>] [--query "..."] [OPTIONS]
```

**Options:**
- `--uid <uid>` - Analyze specific card by UID or slug
- `--query <dsl>` - Analyze cards matching query DSL
- `--analyzer <fast|full>` - Analysis profile (default: full)
- `--no-embeddings` - Skip embedding-based metrics (cohesion, bandwidth, redundancy)
- `--neighbors <N>` - Number of neighbors for redundancy calculation (default: 100)
- `--jsonl` - Output JSONL AnalysisResult format

**Examples:**
```bash
# Analyze specific card
gauge analyze --uid my-card

# Analyze all cards
gauge analyze

# Analyze with query filter
gauge analyze --query "tag:research"

# Fast analysis (no embeddings)
gauge analyze --analyzer fast

# JSONL output for piping
gauge analyze --jsonl
```

## Metrics

### Core Metrics

- **tokens** - Word/token count in card content
- **nid_bpt** - Normalized Information Density (bits per token) via gzip compression
- **link_density** - Number of links per 100 tokens
- **structure_density** - Headings/bullets/codeblocks per 100 tokens

### Advanced Metrics (Full Analyzer)

- **cohesion** - Mean pairwise similarity of sentences (0-1, higher = more cohesive)
- **bandwidth** - Estimated number of topic clusters (1-5)
- **redundancy** - Maximum similarity to nearest neighbor card (0-1, higher = more redundant)
- **sv** - Composite Semantic Volume score

### Semantic Volume (SV)

The composite SV score combines:
- Size normalization (tokens / 200)
- Information density (NID factor)
- Cohesion (how unified the content is)
- Redundancy (uniqueness relative to neighbors)

**SV Interpretation:**
- `sv ~ 1.0` - Well-sized card
- `sv > 1.6` - Too large/packed → consider split
- `sv < 0.5` - Too small/empty → consider merge

## Analysis Profiles

### Fast Profile
```bash
gauge analyze --analyzer fast
```
- Computes: tokens, NID, link density, structure density
- No embeddings required
- Very fast (<50ms per card)
- Useful for quick assessments

### Full Profile
```bash
gauge analyze --analyzer full
```
- All metrics including cohesion, bandwidth, redundancy
- Uses heuristic approximations (no actual embeddings yet)
- Slightly slower but more accurate
- Recommended for detailed analysis

## Action Suggestions

Gauge suggests actions based on metrics:

### `split`
**Trigger:** Cards too large or low cohesion
- `tokens > 350 && bandwidth >= 3`
- `cohesion < 0.45 && tokens > 250`

**Rationale:** Card contains multiple topics or is too long

### `merge`
**Trigger:** Small cards with high redundancy
- `tokens < 80 && redundancy > 0.85`

**Rationale:** Card duplicates content already in another card

### `prune`
**Trigger:** Redundant or low-information cards
- `redundancy > 0.9 && tokens > 200`
- `nid_bpt < 2.5 && tokens > 200`

**Rationale:** Card has low value or high redundancy

### `refactor`
**Trigger:** Cards needing structure improvements
- `tokens > 300 && structure_density < 0.8`

**Rationale:** Card content would benefit from better organization

### `ok`
**Trigger:** Cards within acceptable ranges
- All metrics within target bands

**Rationale:** Card is well-structured and sized

## Output Formats

### Human-Readable (default)

```
Card: Research Note (ulid_01ABC...)
  Tokens: 218
  NID (bits/token): 4.7
  Cohesion: 0.72
  Bandwidth: 2
  Redundancy: 0.31
  Link density: 0.9
  Structure density: 1.3
  SV: 0.98
  Suggestion: ok
```

### JSONL Format

Each line is an `AnalysisResult`:
```json
{
  "type": "analysis",
  "uid": "ulid_01ABC...",
  "computed": {
    "tokens": 218,
    "nid_bpt": 4.7,
    "cohesion": 0.72,
    "bandwidth": 2,
    "redundancy": 0.31,
    "link_density": 0.9,
    "structure_density": 1.3,
    "sv": 0.98,
    "last_analyzed": "2025-01-15T10:30:00Z"
  },
  "suggestion": "ok",
  "rationale": "sv=0.98",
  "version": "svspec-1"
}
```

## Integration Examples

### Analyze and Organize
```bash
# Find cards needing attention
scout list --jsonl | gauge analyze --jsonl | jq 'select(.suggestion != "ok")'

# Full pipeline: analyze → plan → apply
scout list --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --yes
```

### Batch Analysis
```bash
# Analyze all draft cards
scout list --query "status=draft" --jsonl | gauge analyze --jsonl > analysis.jsonl

# Review suggestions
cat analysis.jsonl | jq -r 'select(.suggestion != "ok") | "\(.uid): \(.suggestion) - \(.rationale)"'
```

### Find Overfull Cards
```bash
# Cards that likely need splitting
scout list --jsonl | gauge analyze --jsonl | \
  jq -r 'select(.computed.sv > 1.6) | "\(.uid) - \(.computed.tokens) tokens, sv=\(.computed.sv)"'
```

### Find Duplicates
```bash
# Highly redundant cards
scout list --jsonl | gauge analyze --jsonl | \
  jq -r 'select(.computed.redundancy > 0.85) | "\(.uid) - redundancy=\(.computed.redundancy)"'
```

## Configuration

Thresholds can be configured in `.cardstack/config.yml`:

```yaml
thresholds:
  split: "(tokens>350 && bandwidth>=3) || (cohesion<0.45 && tokens>250)"
  merge: "tokens<80 && redundancy>0.85"
  prune: "redundancy>0.9 && backlinks=0 && age>120"
  refactor: "tokens>300 && structure_density<0.8"
```

## Algorithm Details

### Tokenization
Simple whitespace-based tokenization. For production, consider integrating a proper tokenizer.

### NID Calculation
1. Compress text using gzip
2. Compute: `8 * compressed_bytes / tokens`
3. Higher values indicate denser information

### Cohesion (Heuristic)
- Splits text into sentences
- Compares word overlap between sentence pairs
- Averages Jaccard similarity scores
- Future: Use sentence embeddings for accuracy

### Bandwidth (Heuristic)
- Estimates number of distinct topics
- Based on sentence length variance
- Future: K-means clustering on embeddings

### Redundancy
- Compares tags/keywords with neighboring cards
- Computes maximum overlap similarity
- Future: Semantic similarity via embeddings

## Performance

- **Fast analyzer**: ~50ms per 200-token card
- **Full analyzer**: ~150ms per 200-token card
- **Batch processing**: Suitable for 1000+ cards

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--jsonl` - Machine-readable JSONL output

## Future Enhancements

- True sentence embeddings (via ONNX or Candle)
- Embeddings cache (`.cardstack/embeddings/`)
- K-means clustering for bandwidth
- Semantic similarity for redundancy
- Configurable threshold expressions

