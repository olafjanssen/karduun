# Publisher CLI Tool

Publish and manage card publications in the Karduun suite.

## Overview

The `publisher` tool allows you to publish cards to albums, unpublish them, and list their publications. This tool is used to manage the publication status of cards and track where they have been published.

## Installation

```bash
cargo install --path publisher
# or
cargo build --release --bin publisher
```

## Commands

### `publisher publish`

Publish a card to an album.

**Usage:**
```bash
publisher publish [--album <album_name>] <card_uid_or_slug>
```

**Options:**
- `--album <album_name>` - Name of the album to publish to (optional, defaults to "general")
- `<card_uid_or_slug>` - UID or slug of the card to publish

**Examples:**
```bash
# Publish a card to an album
publisher publish --album "Research Papers" --card "my-card"
```

### `publisher unpublish`

Unpublish a card from an album.

**Usage:**
```bash
publisher unpublish [--album <album_name>] <card_uid_or_slug>
```

**Options:**
- `--album <album_name>` - Name of the album to unpublish from (optional, defaults to "general")
- `<card_uid_or_slug>` - UID or slug of the card to unpublish

**Examples:**
```bash
# Unpublish a card from an album
publisher unpublish --album "Research Papers" --card "my-card"
```

### `publisher list`

List publications for a card.

**Usage:**
```bash
publisher list <card_uid_or_slug>
```

**Options:**
- `<card_uid_or_slug>` - UID or slug of the card

**Examples:**
```bash
# List publications for a card
publisher list --card "my-card"
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root

## Examples

# Publish and Manage Cards

```bash
# Publish a card to the default "general" album
publisher publish "my-card"

# Publish a card to a specific album
publisher publish --album "Research Papers" "my-card"

# List publications for a card
publisher list "my-card"

# Unpublish a card from the default "general" album
publisher unpublish "my-card"

# Unpublish a card from a specific album
publisher unpublish --album "Research Papers" "my-card"
```

## Configuration

Card publications are stored in the `publications` field of each card in the `.cardstack/cards/` directory. This field is automatically managed by the `publisher` tool.

## Future Enhancements

- Publication metadata (date, version, etc.)
- Advanced publication tracking
- Publication history and versioning
- Collaboration and sharing features