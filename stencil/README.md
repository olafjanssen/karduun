# Stencil

Template management and validation.

## Overview

Stencil helps you create and manage card templates, which define default structures and constraints for new cards. Templates ensure consistency and enforce rules during card creation.

## Installation

```bash
cargo install --path stencil
# or
cargo build --release --bin stencil
```

## Commands

### `stencil new`

Create a new template card.

**Usage:**
```bash
stencil new "Name" [OPTIONS]
```

**Options:**
- `--slug <slug>` - Custom slug (auto-generated from name if omitted)
- `--required-field <field>` - Add required field (can be repeated)
- `--enum-field <field=val1,val2>` - Add enum field constraint (can be repeated)
- `--frozen-field <field>` - Add frozen field (cannot be modified after creation)

**Examples:**
```bash
# Create simple template
stencil new "Research Note"

# Create template with constraints
stencil new "Meeting Template" \
  --required-field "fields.attendees" \
  --required-field "fields.date" \
  --enum-field "fields.status=draft,scheduled,completed" \
  --frozen-field "author.id"

# Custom slug
stencil new "Project Card" --slug template-project-card
```

**Template Structure:**
Templates are cards with a `template` facet containing:
- `defaults` - Default values for new cards
- `constraints` - Validation rules:
  - `required_fields` - Fields that must be present
  - `enum_fields` - Field value restrictions
  - `frozen_fields` - Fields that shouldn't change after creation

### `stencil list`

List all template cards in the repository.

**Usage:**
```bash
stencil list [--jsonl]
```

**Options:**
- `--jsonl` - Output JSONL CardEnvelope format

**Examples:**
```bash
# List templates
stencil list

# JSONL output
stencil list --jsonl
```

**Output:**
```
Found 3 template(s):
  template-research-note - Template: Research Note (ulid_01ABC...)
  template-meeting - Template: Meeting Template (ulid_01DEF...)
  template-project-card - Template: Project Card (ulid_01GHI...)
```

### `stencil show`

Display a template's details including defaults and constraints.

**Usage:**
```bash
stencil show <slug> [--json]
```

**Options:**
- `--json` - Output JSON format

**Examples:**
```bash
# Show template details
stencil show template-research-note

# JSON output
stencil show template-meeting --json
```

**Output:**
```
Template: Template: Research Note (ulid_01ABC...)

Defaults:
  (none)

Constraints:
  Required fields: fields.status, fields.topic
  Enum fields:
    fields.status: ["draft", "active", "published"]
  Frozen fields: author.id

Template body:
# Template: Research Note

Default structure goes here...
```

### `stencil validate`

Check cards against their template constraints.

**Usage:**
```bash
stencil validate [--uid <uid>] [--query "..."] [--jsonl]
```

**Options:**
- `--uid <uid>` - Validate specific card by UID or slug
- `--query <dsl>` - Validate cards matching query
- `--jsonl` - Output JSONL ValidationResult format

**Examples:**
```bash
# Validate specific card
stencil validate --uid my-card

# Validate all cards from a template
stencil validate --query "tag:research"

# JSONL output
stencil validate --jsonl > validation.jsonl
```

**Validation Checks:**
- **Required fields** - Ensures all required fields are present
- **Enum fields** - Verifies field values are in allowed set
- **Frozen fields** - Warns about fields that shouldn't be modified

**Output:**
```
Validation Results:
  Valid: 15
  Invalid: 2

research-note-1 (ulid_01ABC...)
  Template: template-research-note
  Errors:
    - Missing required field: fields.status
    - Field fields.status has invalid value: "pending" (allowed: ["draft", "active", "published"])
  Warnings:
    - Field author.id should not be modified after creation
```

**JSONL Format:**
```json
{
  "uid": "ulid_01ABC...",
  "slug": "research-note-1",
  "template": "template-research-note",
  "valid": false,
  "errors": [
    "Missing required field: fields.status"
  ],
  "warnings": [
    "Field author.id should not be modified after creation"
  ]
}
```

## Template Usage

### Creating Cards from Templates

Use `scribe new` with `--template` flag:

```bash
# Create card from template
scribe new "My Research Note" --template template-research-note

# Template defaults are automatically applied
# Template constraints are validated
```

### Template Linking

When a card is created from a template, a `derived-from` link is automatically created:

```bash
scribe link my-card --to template-research-note --type derived-from
```

This allows `stencil validate` to find the template for validation.

### Template Constraints

**Required Fields:**
```bash
stencil new "Task Template" \
  --required-field "fields.due_date" \
  --required-field "fields.priority"
```

**Enum Fields:**
```bash
stencil new "Status Template" \
  --enum-field "fields.status=new,in-progress,done" \
  --enum-field "fields.priority=low,medium,high"
```

**Frozen Fields:**
```bash
stencil new "Author Template" \
  --frozen-field "author.id" \
  --frozen-field "created"
```

## Integration Examples

### Validate All Cards
```bash
# Find validation issues
stencil validate --jsonl | jq 'select(.valid == false)'

# Count issues
stencil validate --jsonl | jq '[select(.errors | length > 0)] | length'
```

### Template Workflow
```bash
# 1. Create template
stencil new "Project Template" \
  --required-field "fields.project_id" \
  --enum-field "fields.status=planning,active,completed"

# 2. Create cards from template
scribe new "Project Alpha" --template template-project

# 3. Validate compliance
stencil validate --query "tag:project"
```

### Batch Validation
```bash
# Export validation results
stencil validate --jsonl > validation-report.jsonl

# Find problematic cards
cat validation-report.jsonl | jq -r 'select(.errors | length > 0) | .uid'
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--json` - JSON output (where applicable)
- `--jsonl` - JSONL output (where applicable)

## Best Practices

1. **Define templates early** - Set up templates before creating many cards
2. **Use meaningful names** - Template names should describe their purpose
3. **Enforce constraints** - Use enum fields to prevent invalid values
4. **Regular validation** - Run `stencil validate` periodically
5. **Template documentation** - Include usage notes in template body

## Template Examples

### Research Note Template
```bash
stencil new "Research Note" \
  --required-field "fields.topic" \
  --enum-field "fields.status=draft,active,published" \
  --enum-field "fields.source_type=paper,article,book"
```

### Task Template
```bash
stencil new "Task Template" \
  --required-field "fields.due_date" \
  --required-field "fields.priority" \
  --enum-field "fields.priority=low,medium,high,urgent" \
  --enum-field "fields.status=backlog,todo,in-progress,done"
```

### Meeting Template
```bash
stencil new "Meeting Template" \
  --required-field "fields.date" \
  --required-field "fields.attendees" \
  --frozen-field "author.id"
```
