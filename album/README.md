# Album CLI Tool

Manage albums and card publications in the Karduun suite.

## Overview

The `album` tool allows you to create, manage, and archive albums, as well as add or transfer cards between albums. Albums are used to organize and group cards for specific purposes, such as publications, collections, or themes.

## Installation

```bash
cargo install --path album
# or
cargo build --release --bin album
```

## Commands

### `album create`

Create a new album.

**Usage:**
```bash
album create --name <album_name>
```

**Options:**
- `--name <album_name>` - Name of the album to create

**Examples:**
```bash
# Create a new album
album create --name "Research Papers"
```

### `album list`

List all albums.

**Usage:**
```bash
album list
```

**Examples:**
```bash
# List all albums
album list
```

### `album add`

Add a card to an album.

**Usage:**
```bash
album add --album <album_name> --card <card_uid_or_slug>
```

**Options:**
- `--album <album_name>` - Name of the album
- `--card <card_uid_or_slug>` - UID or slug of the card to add

**Examples:**
```bash
# Add a card to an album
album add --album "Research Papers" --card "my-card"
```

### `album transfer`

Transfer a card to another album.

**Usage:**
```bash
album transfer --from <source_album> --to <destination_album> --card <card_uid_or_slug>
```

**Options:**
- `--from <source_album>` - Name of the source album
- `--to <destination_album>` - Name of the destination album
- `--card <card_uid_or_slug>` - UID or slug of the card to transfer

**Examples:**
```bash
# Transfer a card to another album
album transfer --from "Drafts" --to "Research Papers" --card "my-card"
```

### `album archive`

Archive an album.

**Usage:**
```bash
album archive --name <album_name>
```

**Options:**
- `--name <album_name>` - Name of the album to archive

**Examples:**
```bash
# Archive an album
album archive --name "Old Research"
```

### `album show`

List cards in an album.

**Usage:**
```bash
album show --name <album_name>
```

**Options:**
- `--name <album_name>` - Name of the album

**Examples:**
```bash
# List cards in an album
album show --name "Research Papers"
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root

## Examples

### Create and Manage Albums
```bash
# Create a new album
album create --name "Research Papers"

# Add a card to the album
album add --album "Research Papers" --card "my-card"

# List all albums
album list

# Show cards in an album
album show --name "Research Papers"
```

### Transfer Cards Between Albums
```bash
# Transfer a card from one album to another
album transfer --from "Drafts" --to "Research Papers" --card "my-card"
```

### Archive Albums
```bash
# Archive an old album
album archive --name "Old Research"
```

## Configuration

Album metadata and card publications are stored in the `.cardstack/albums.json` file. This file is automatically managed by the `album` tool.

## Future Enhancements

- Album metadata (description, tags, etc.)
- Card versioning in albums
- Advanced querying and filtering
- Album sharing and collaboration
```

<file_path>
karduun/publisher/README.md
</file_path>

<edit_description>
Create README.md for publisher tool
</edit_description>