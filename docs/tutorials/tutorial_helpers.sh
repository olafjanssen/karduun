#!/bin/bash
# Shared helper functions for Karduun CLI tutorials

# Colors for output (optional, for better readability)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

echo_success() {
    echo -e "${GREEN}✓${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

echo_error() {
    echo -e "${RED}✗${NC} $1"
}

echo_section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if eco tool is available
check_eco_available() {
    if ! command_exists eco; then
        echo_warn "eco tool not found. Some features will be skipped."
        echo_info "To install: cargo install --path eco"
        return 1
    fi
    return 0
}

# Check if cardstack tools are available
check_tool_available() {
    local tool_name="$1"
    if ! command_exists "$tool_name"; then
        echo_warn "$tool_name tool not found. Some features will be skipped."
        return 1
    fi
    return 0
}

# Initialize workspace directory
init_workspace() {
    local workspace_dir="${1:-ecosystem}"

    if [ -d "$workspace_dir" ]; then
        echo_warn "Directory '$workspace_dir' already exists."
        read -p "Do you want to remove it and start fresh? (y/n) " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$workspace_dir"
        else
            echo_info "Using existing directory: $workspace_dir"
            return 0
        fi
    fi

    mkdir -p "$workspace_dir"
    echo_success "Created workspace directory: $workspace_dir"
    cd "$workspace_dir"

    # Initialize cardstack repository
    if [ ! -d ".cardstack" ]; then
        echo "Initializing cardstack repository..."
        if command_exists scribe; then
            scribe init
            echo_success "Cardstack repository initialized"
        else
            echo_error "scribe tool not found. Cannot initialize repository."
            return 1
        fi
    fi
}

# Extract card UID from filename
extract_card_uid() {
    local filename="$1"
    basename "$filename" | grep -o '^[0-9A-Z]*'
}

# Find card by partial name and extract UID
find_card_uid() {
    local card_name="$1"
    local card_file=$(find cards -name "*$card_name*" | head -1)
    if [ -n "$card_file" ]; then
        extract_card_uid "$card_file"
    else
        echo ""
    fi
}
