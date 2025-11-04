#!/bin/bash
# Karduun CLI Tutorial: Dead Philosophers Deck
#
# This script demonstrates how to use the Karduun CLI suite to create cards
# and a dynamic deck for "Dead Philosophers" using a REPL-friendly, Bash-first workflow.
#
# The tutorial covers:
#   - Core tools: scribe, scout, catalog
#   - Extended features: stencil (templates), curator (organization), porter (export)
#
# Usage:
#   ./dead_philosophers_tutorial.sh
#
# Or run with a custom workspace directory:
#   WORKDIR=my-philosophers ./dead_philosophers_tutorial.sh
#
# Note: Extended features (stencil, curator, porter) are optional and will be
# skipped if the tools are not installed.

set -euo pipefail

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
    echo ""
}

# ============================================================================
# STEP 1: Verify Installation
# ============================================================================
echo_section "Step 1: Verifying Karduun Tools Installation"

echo_info "Checking if Karduun tools are installed..."

# Core tools (required for basic tutorial)
CORE_TOOLS=("scribe" "scout" "catalog")
# Optional tools (for extended tutorial)
OPTIONAL_TOOLS=("gauge" "curator" "porter" "stencil")

MISSING_CORE=()
MISSING_OPTIONAL=()

for tool in "${CORE_TOOLS[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo_success "$tool is installed: $($tool --version 2>/dev/null || $tool --help | head -1)"
    else
        echo_warn "$tool is not found in PATH"
        MISSING_CORE+=("$tool")
    fi
done

for tool in "${OPTIONAL_TOOLS[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo_success "$tool is installed: $($tool --version 2>/dev/null || $tool --help | head -1)"
    else
        echo_warn "$tool (optional) is not found in PATH"
        MISSING_OPTIONAL+=("$tool")
    fi
done

if [ ${#MISSING_CORE[@]} -gt 0 ]; then
    echo_error "Core tools are missing. Please install them first:"
    echo "  cargo install --path ${MISSING_CORE[*]}"
    echo ""
    echo "Or build from the repo root:"
    echo "  cargo build --release"
    echo ""
    read -p "Press Enter to continue anyway, or Ctrl+C to exit..."
fi

if [ ${#MISSING_OPTIONAL[@]} -gt 0 ]; then
    echo_warn "Optional tools missing: ${MISSING_OPTIONAL[*]}"
    echo "  These will be skipped in the tutorial. Install with:"
    echo "  cargo install --path ${MISSING_OPTIONAL[*]}"
    echo ""
fi

# ============================================================================
# STEP 2: Initialize Workspace
# ============================================================================
echo_section "Step 2: Initializing Workspace"

WORKDIR=${WORKDIR:-dead-philosophers}

if [ -d "$WORKDIR" ]; then
    echo_warn "Directory '$WORKDIR' already exists."
    read -p "Continue and use existing directory? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo_info "Exiting. Set WORKDIR to use a different directory."
        exit 0
    fi
else
    mkdir -p "$WORKDIR"
    echo_success "Created workspace directory: $WORKDIR"
fi

cd "$WORKDIR"

# Check if already initialized
if [ -d ".cardstack" ]; then
    echo_warn "This directory appears to already be a cardstack repository."
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
else
    echo_info "Initializing cardstack repository..."
    scribe init
    echo_success "Repository initialized"
fi

echo ""
echo_info "Current directory structure:"
ls -la | head -10

# ============================================================================
# STEP 3: Create Philosopher Cards
# ============================================================================
echo_section "Step 3: Creating Philosopher Cards"

echo_info "Creating cards for deceased philosophers..."

# Socrates
scribe new "Socrates" \
  --tag philosopher \
  --field status=deceased \
  --field birth=-470 \
  --field death=-399 \
  --field nationality=Greek \
  --field school="Classical Greek" \
  --field notes="Classical Athenian philosopher; known via Plato/Xenophon" \
  --body <(printf "Socrates is credited as a founder of Western philosophy.\n")
echo_success "Created card: Socrates"

# Plato
scribe new "Plato" \
  --tag philosopher \
  --field status=deceased \
  --field birth=-428 \
  --field death=-348 \
  --field nationality=Greek \
  --field school=Platonism \
  --body <(printf "Plato was a student of Socrates and teacher of Aristotle.\n")
echo_success "Created card: Plato"

# Aristotle
scribe new "Aristotle" \
  --tag philosopher \
  --field status=deceased \
  --field birth=-384 \
  --field death=-322 \
  --field nationality=Greek \
  --field school=Peripatetic \
  --body <(printf "Aristotle studied under Plato and tutored Alexander the Great.\n")
echo_success "Created card: Aristotle"

# Confucius
scribe new "Confucius" \
  --tag philosopher \
  --field status=deceased \
  --field birth=-551 \
  --field death=-479 \
  --field nationality=Chinese \
  --field school=Confucianism \
  --body <(printf "Chinese teacher, editor, politician, and philosopher.\n")
echo_success "Created card: Confucius"

# Friedrich Nietzsche
scribe new "Friedrich Nietzsche" \
  --tag philosopher \
  --field status=deceased \
  --field birth=1844 \
  --field death=1900 \
  --field nationality=German \
  --field school=Existentialism \
  --body <(printf "German philosopher known for critiques of morality and religion.\n")
echo_success "Created card: Friedrich Nietzsche"

# Ludwig Wittgenstein
scribe new "Ludwig Wittgenstein" \
  --tag philosopher \
  --field status=deceased \
  --field birth=1889 \
  --field death=1951 \
  --field nationality=Austrian-British \
  --field school=Analytic \
  --body <(printf "Analytic philosopher; major works on language and logic.\n")
echo_success "Created card: Ludwig Wittgenstein"

# Simone de Beauvoir
scribe new "Simone de Beauvoir" \
  --tag philosopher \
  --field status=deceased \
  --field birth=1908 \
  --field death=1986 \
  --field nationality=French \
  --field school=Existentialism \
  --body <(printf "French existentialist philosopher and feminist.\n")
echo_success "Created card: Simone de Beauvoir"

# Hannah Arendt
scribe new "Hannah Arendt" \
  --tag philosopher \
  --field status=deceased \
  --field birth=1906 \
  --field death=1975 \
  --field nationality=German-American \
  --field school=Political \
  --body <(printf "Political theorist known for work on totalitarianism and power.\n")
echo_success "Created card: Hannah Arendt"

echo ""
echo_success "Created 8 philosopher cards"

# ============================================================================
# STEP 4: Build Index and Query
# ============================================================================
echo_section "Step 4: Building Index and Querying Cards"

echo_info "Rebuilding catalog index for faster queries..."
catalog rebuild
echo_success "Index rebuilt"

echo ""
echo_info "Listing all cards:"
scout list

echo ""
echo_info "Querying deceased philosophers (sorted by title):"
scout list --query "tag:philosopher status=deceased" --sort "title"

# ============================================================================
# STEP 5: Create Dynamic Deck
# ============================================================================
echo_section "Step 5: Creating Dynamic Deck 'Dead Philosophers'"

echo_info "Creating a query-based deck that automatically includes all deceased philosophers..."
scribe deck new "Dead Philosophers" --mode query --query "tag:philosopher status=deceased"
echo_success "Deck 'Dead Philosophers' created"

echo ""
echo_info "Showing deck contents and metadata:"
scribe deck show dead-philosophers || scribe deck show "Dead Philosophers" || echo_warn "Could not show deck"

# ============================================================================
# STEP 6: Explore Deck Structure
# ============================================================================
echo_section "Step 6: Exploring Deck Structure"

echo_info "Displaying deck tree structure:"
if scout tree dead-philosophers &> /dev/null; then
    scout tree dead-philosophers
elif scout tree "Dead Philosophers" &> /dev/null; then
    scout tree "Dead Philosophers"
else
    echo_warn "Could not display tree. This may be normal for query-based decks."
fi

echo ""
echo_info "Showing backlinks to verify deck membership (cards that link to the deck):"
if scout backlinks dead-philosophers &> /dev/null; then
    scout backlinks dead-philosophers
elif scout backlinks "Dead Philosophers" &> /dev/null; then
    scout backlinks "Dead Philosophers"
else
    echo_info "Note: For query-based decks, membership is determined by the query, not explicit links."
    echo_info "The cards shown above are the current members matching the query."
fi

# ============================================================================
# STEP 7: Optional Analysis Pipeline
# ============================================================================
echo_section "Step 7: Optional - Semantic Analysis Pipeline"

if command -v gauge &> /dev/null; then
    echo_info "Running semantic volume analysis on philosophers..."
    echo_info "(Output limited to first 5 results)"
    scout list --query "tag:philosopher status=deceased" --jsonl | gauge analyze --jsonl | head -n 5 || echo_warn "Analysis pipeline may need more setup"
else
    echo_warn "gauge tool not available, skipping analysis"
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo_section "Tutorial Complete!"

echo_success "Created a dynamic deck of dead philosophers!"
echo ""
echo_info "Summary:"
echo "  - Workspace: $WORKDIR"
echo "  - Cards created: 8 philosophers"
echo "  - Deck: 'Dead Philosophers' (query-based)"
echo ""
echo_info "Next steps you can try:"
echo "  - Add more philosophers: scribe new \"Philosopher Name\" --tag philosopher --field status=deceased ..."
echo "  - Search content: scout grep \"your search term\""
echo "  - Show specific card: scribe show socrates"
echo "  - Create links: scribe link card1 --to card2 --type cites"
echo "  - Query by school: scout list --query \"tag:philosopher fields.school=Existentialism\""
echo ""
echo_info "Repository location: $(pwd)"
echo ""

# ============================================================================
# STEP 8: Create Template with Stencil
# ============================================================================
echo_section "Step 8: Creating Template with Stencil"

if command -v stencil &> /dev/null; then
    echo_info "Creating a template for philosopher cards..."
    
    # Create a philosopher template with constraints
    stencil new "Philosopher Template" \
      --slug template-philosopher \
      --required-field "fields.status" \
      --required-field "fields.birth" \
      --required-field "fields.death" \
      --required-field "fields.nationality" \
      --required-field "fields.school" \
      --enum-field "fields.status=deceased,alive" \
      --frozen-field "fields.birth" \
      --frozen-field "fields.death" || echo_warn "Template creation failed (may already exist)"
    
    echo_success "Template created (if not already present)"
    
    echo ""
    echo_info "Listing all templates:"
    stencil list || echo_warn "stencil list failed"
    
    echo ""
    echo_info "Showing template details:"
    stencil show template-philosopher || echo_warn "Could not show template"
else
    echo_warn "stencil tool not available, skipping template creation"
fi

# ============================================================================
# STEP 9: Organization with Curator
# ============================================================================
echo_section "Step 9: Organization Analysis with Curator"

if command -v curator &> /dev/null && command -v gauge &> /dev/null; then
    echo_info "Analyzing philosophers and creating organization plan..."
    echo_info "(This will show what curator would suggest for improvements)"
    
    # Create a plan (dry-run by default)
    echo_info "Creating organization plan from analysis..."
    scout list --query "tag:philosopher status=deceased" --jsonl | \
      gauge analyze --jsonl | \
      curator plan 2>/dev/null | head -n 3 || echo_warn "curator plan failed or no actions needed"
    
    echo ""
    echo_info "Note: You can apply changes with:"
    echo "  scout list --query 'tag:philosopher' --jsonl | gauge analyze --jsonl | curator plan | curator apply --yes"
else
    echo_warn "curator or gauge tools not available, skipping organization analysis"
fi

# ============================================================================
# STEP 10: Export with Porter
# ============================================================================
echo_section "Step 10: Exporting Cards with Porter"

if command -v porter &> /dev/null; then
    EXPORT_DIR="philosophers-export"
    mkdir -p "$EXPORT_DIR"
    
    echo_info "Exporting philosopher cards to JSONL format..."
    porter export --format jsonl --out "$EXPORT_DIR" --query "tag:philosopher" 2>/dev/null || \
      echo_warn "porter export failed (may need different query format)"
    
    echo ""
    echo_info "Exporting to Markdown format..."
    porter export --format md --out "$EXPORT_DIR/markdown" --query "tag:philosopher" 2>/dev/null || \
      echo_warn "porter export to markdown failed"
    
    if [ -d "$EXPORT_DIR" ]; then
        echo ""
        echo_success "Exports created in: $EXPORT_DIR"
        echo_info "Files:"
        ls -lh "$EXPORT_DIR" | head -5 || true
    fi
else
    echo_warn "porter tool not available, skipping export"
fi

# ============================================================================
# FINAL SUMMARY
# ============================================================================
echo_section "Tutorial Complete - Extended Features!"

echo_success "Completed all extended tutorial steps!"
echo ""
echo_info "Summary of what we did:"
echo "  ✓ Created 8 philosopher cards"
echo "  ✓ Built catalog index"
echo "  ✓ Created dynamic deck 'Dead Philosophers'"
if command -v stencil &> /dev/null; then
    echo "  ✓ Created philosopher template"
fi
if command -v curator &> /dev/null; then
    echo "  ✓ Analyzed organization opportunities"
fi
if command -v porter &> /dev/null; then
    echo "  ✓ Exported cards to multiple formats"
fi
echo ""
echo_info "Next steps you can try:"
echo "  - Create cards from template: scribe new 'Philosopher Name' --template template-philosopher"
echo "  - Validate template compliance: stencil validate --query 'tag:philosopher'"
echo "  - Import exported cards: porter import --from jsonl --in philosophers-export"
echo "  - Apply organization suggestions: scout list --jsonl | gauge analyze --jsonl | curator plan | curator apply --yes"
echo "  - Create static snapshot: scribe deck snapshot dead-philosophers --out dead-philosophers-2025"
echo ""
echo_info "Repository location: $(pwd)"
echo ""

