# Curator

Apply organization plans to automatically split, merge, prune, and refactor cards.

## Overview

Curator executes organization actions based on semantic analysis. It reads analysis results and applies transformations to improve card structure and quality. Always preview with `--dry-run` before applying changes.

## Installation

```bash
cargo install --path curator
# or
cargo build --release --bin curator
```

## Commands

### `curator plan`

Convert AnalysisResult stream to OrgAction plan.

**Usage:**
```bash
curator plan [--rules <rules>]
```

**Input:** JSONL stream of `AnalysisResult` from `gauge analyze`

**Output:** JSONL stream of `OrgAction`

**Options:**
- `--rules <rules>` - Rule set name (future: custom threshold rules)

**Examples:**
```bash
# Plan from analysis
scout list --jsonl | gauge analyze --jsonl | curator plan > plan.jsonl

# Review plan
cat plan.jsonl | jq .
```

**Output Format:**
```json
{
  "type": "org_action",
  "uid": "ulid_01ABC...",
  "action": "split",
  "params": {"strategy": "clusters"},
  "why": "tokens=612 bandwidth=4 cohesion=0.38"
}
```

### `curator apply`

Execute OrgAction stream to mutate cards.

**Usage:**
```bash
curator apply [--yes] [--dry-run]
```

**Input:** JSONL stream of `OrgAction` from `curator plan` or manual creation

**Options:**
- `--yes` - Required to actually apply changes (safety check)
- `--dry-run` - Show what would be done without making changes

**Safety:**
- Always runs in dry-run mode unless `--yes` is provided
- Logs all actions to `.cardstack/logs/*.ndjson`
- Creates provenance links for all changes

**Examples:**
```bash
# Preview changes
cat plan.jsonl | curator apply --dry-run

# Apply changes
cat plan.jsonl | curator apply --yes

# Pipeline from analysis to application
scout list --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --yes
```

### `curator autoclean`

One-shot analyze → plan → apply workflow.

**Usage:**
```bash
curator autoclean [--apply] [OPTIONS]
```

**Options:**
- `--apply` - Actually apply changes (otherwise dry-run)
- `--split-thresh <expr>` - Custom split threshold expression
- `--merge-thresh <expr>` - Custom merge threshold expression
- `--prune-thresh <expr>` - Custom prune threshold expression

**Examples:**
```bash
# Preview cleanup
curator autoclean

# Execute cleanup
curator autoclean --apply
```

**Note:** Currently a stub - use the pipeline approach instead.

## Actions

### Split

**When:** Card is too large or contains multiple topics

**What it does:**
1. Partitions card content into chunks
2. Creates child cards from chunks
3. Converts parent to deck (collection facet)
4. Adds `contains` links to children
5. Adds `part-of` links from children to parent

**Example:**
```json
{
  "type": "org_action",
  "uid": "ulid_01ABC...",
  "action": "split",
  "params": {"strategy": "clusters"},
  "why": "tokens=612 bandwidth=4"
}
```

**Result:**
- Original card becomes deck with overview
- 3-4 child cards created
- Provenance maintained

### Merge

**When:** Small cards with high redundancy

**What it does:**
1. Combines content from source into destination
2. Unions tags (removes duplicates)
3. Merges fields (notes conflicts in `_conflicts`)
4. Archives source card with `merged_into` field
5. Adds `derived-from` link for provenance

**Example:**
```json
{
  "type": "org_action",
  "uid": "ulid_01ABC...",
  "action": "merge",
  "params": {"into": "ulid_01DEF..."},
  "why": "tokens=54 redundancy=0.91"
}
```

**Result:**
- Source card archived
- Destination card contains merged content
- Provenance link created

### Prune

**When:** Redundant or low-information cards

**What it does:**
1. Marks card as archived
2. Sets `pruned_at` timestamp
3. Preserves card for historical reference

**Example:**
```json
{
  "type": "org_action",
  "uid": "ulid_01ABC...",
  "action": "prune",
  "why": "redundancy=0.92 nid_bpt=2.1"
}
```

**Result:**
- Card archived but not deleted
- Can be recovered if needed

### Refactor

**When:** Cards need better structure

**What it does:**
1. Ensures first-level heading exists
2. Improves heading structure
3. Adds section markers where appropriate
4. Preserves all content

**Example:**
```json
{
  "type": "org_action",
  "uid": "ulid_01ABC...",
  "action": "refactor",
  "why": "tokens=350 structure_density=0.5"
}
```

**Result:**
- Card structure improved
- Content preserved
- Better readability

## Workflow Examples

### Safe Organization Workflow

```bash
# 1. Analyze cards
scout list --jsonl | gauge analyze --jsonl > analysis.jsonl

# 2. Review suggestions
cat analysis.jsonl | jq -r 'select(.suggestion != "ok") | "\(.uid): \(.suggestion) - \(.rationale)"'

# 3. Create plan
cat analysis.jsonl | curator plan > plan.jsonl

# 4. Review plan
cat plan.jsonl | jq .

# 5. Preview changes
cat plan.jsonl | curator apply --dry-run

# 6. Apply changes
cat plan.jsonl | curator apply --yes
```

### Automated Cleanup

```bash
# One command pipeline
scout list --query "status=draft" --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --dry-run  # Preview first!

# Then apply if looks good
scout list --query "status=draft" --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --yes
```

### Selective Actions

```bash
# Only split actions
scout list --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  jq 'select(.action == "split")' | \
  curator apply --yes

# Only merge small cards
scout list --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  jq 'select(.action == "merge" and (.why | contains("tokens<80")))' | \
  curator apply --yes
```

### Manual Actions

```json
# Create manual action file
cat > manual-actions.jsonl << EOF
{"type":"org_action","uid":"ulid_01ABC...","action":"refactor","why":"manual"}
EOF

# Apply
cat manual-actions.jsonl | curator apply --yes
```

## Action Logging

All applied actions are logged to `.cardstack/logs/actions_YYYYMMDD.ndjson`:

```json
{"type":"org_action","uid":"ulid_01ABC...","action":"split","why":"..."}
{"type":"org_action","uid":"ulid_01DEF...","action":"merge","why":"..."}
```

## Safety Features

1. **Dry-run by default** - Always preview before applying
2. **Explicit confirmation** - Requires `--yes` flag
3. **Provenance tracking** - All changes have links back to origin
4. **No data loss** - Pruned cards are archived, not deleted
5. **Action logging** - All changes logged for audit trail

## Best Practices

1. **Always dry-run first** - Review changes before applying
2. **Start small** - Test on a few cards before bulk operations
3. **Commit first** - Use version control to track changes
4. **Review logs** - Check action logs in `.cardstack/logs/`
5. **Rebuild index** - Run `catalog rebuild` after bulk changes

## Integration with Other Tools

### With Gauge
```bash
gauge analyze --jsonl | curator plan
```

### With Scout
```bash
scout list --query "tag:research" --jsonl | \
  gauge analyze --jsonl | \
  curator plan | \
  curator apply --yes
```

### With Catalog
```bash
# After applying changes, rebuild index
curator apply --yes < plan.jsonl
catalog rebuild
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--yes` - Required for actual mutations
- `--dry-run` - Preview mode (default for apply)

## Troubleshooting

### No changes applied
- Ensure `--yes` flag is used
- Check dry-run mode is not active
- Verify action format is correct

### Unexpected splits
- Review `bandwidth` metric - high values trigger splits
- Adjust thresholds in config if needed

### Merge conflicts
- Check `_conflicts` field in merged card
- Manual resolution may be needed for complex cases

