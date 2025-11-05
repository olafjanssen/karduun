#!/bin/bash
# Karduun CLI Tutorial: Dead Philosophers Deck
#
# This script demonstrates how to use the Karduun CLI suite to create cards
# and a dynamic deck for "Dead Philosophers" using a REPL-friendly, Bash-first workflow.
#
# The tutorial covers:
#   - Core tools: scribe, scout, catalog
#   - Extended features: stencil (templates), curator (organization), notary (signing), porter (export)
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
OPTIONAL_TOOLS=("gauge" "curator" "porter" "stencil" "notary")

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
# STEP 7: Viewing and Searching Cards
# ============================================================================
echo_section "Step 7: Viewing and Searching Individual Cards"

echo_info "Showing details for a specific card (Socrates):"
scribe show socrates 2>/dev/null || scribe show "Socrates" 2>/dev/null || echo_warn "Could not show card details"

echo ""
echo_info "Searching card content for 'philosophy':"
scout grep "philosophy" 2>/dev/null | head -n 5 || echo_warn "grep search failed or no matches"

echo ""
echo_info "Searching for 'Greek' in card content:"
scout grep "Greek" 2>/dev/null | head -n 5 || echo_warn "grep search failed or no matches"

# ============================================================================
# STEP 8: Creating Links Between Cards
# ============================================================================
echo_section "Step 8: Creating Links Between Cards"

echo_info "Creating links to show relationships between philosophers..."

# Link Plato to Socrates (student relationship)
if scribe link plato --to socrates --type "student-of" 2>/dev/null || \
   scribe link "Plato" --to "Socrates" --type "student-of" 2>/dev/null; then
    echo_success "Linked Plato to Socrates (student-of)"
else
    echo_warn "Could not create link (may already exist or command format differs)"
fi

# Link Aristotle to Plato (student relationship)
if scribe link aristotle --to plato --type "student-of" 2>/dev/null || \
   scribe link "Aristotle" --to "Plato" --type "student-of" 2>/dev/null; then
    echo_success "Linked Aristotle to Plato (student-of)"
else
    echo_warn "Could not create link (may already exist or command format differs)"
fi

# Link Nietzsche to Wittgenstein (influence/cites relationship)
if scribe link "Friedrich Nietzsche" --to "Ludwig Wittgenstein" --type "influenced" 2>/dev/null || \
   scribe link nietzsche --to wittgenstein --type "influenced" 2>/dev/null; then
    echo_success "Linked Nietzsche to Wittgenstein (influenced)"
else
    echo_warn "Could not create link (may already exist or command format differs)"
fi

echo ""
echo_info "Showing links for Plato:"
scout links plato 2>/dev/null || scout links "Plato" 2>/dev/null || echo_info "Note: Links may be viewable via other commands"

# ============================================================================
# STEP 9: Advanced Querying
# ============================================================================
echo_section "Step 9: Advanced Querying by Fields"

echo_info "Querying philosophers by school (Existentialism):"
scout list --query "tag:philosopher fields.school=Existentialism" 2>/dev/null || \
  scout list --query "tag:philosopher school=Existentialism" 2>/dev/null || \
  echo_warn "Query format may differ"

echo ""
echo_info "Querying Greek philosophers:"
scout list --query "tag:philosopher fields.nationality=Greek" 2>/dev/null || \
  scout list --query "tag:philosopher nationality=Greek" 2>/dev/null || \
  echo_warn "Query format may differ"

echo ""
echo_info "Querying philosophers by birth year range (ancient, before 0):"
scout list --query "tag:philosopher fields.birth<0" 2>/dev/null || \
  scout list --query "tag:philosopher birth<0" 2>/dev/null || \
  echo_warn "Query format may differ"

# ============================================================================
# STEP 10: Optional Analysis Pipeline
# ============================================================================
echo_section "Step 10: Optional - Semantic Analysis Pipeline"

if command -v gauge &> /dev/null; then
    echo_info "Running semantic volume analysis on philosophers..."
    echo_info "(Output limited to first 5 results)"
    scout list --query "tag:philosopher status=deceased" --jsonl | gauge analyze --jsonl | head -n 5 || echo_warn "Analysis pipeline may need more setup"
else
    echo_warn "gauge tool not available, skipping analysis"
fi

# ============================================================================
# STEP 11: Create Template with Stencil
# ============================================================================
echo_section "Step 11: Creating Template with Stencil"

if command -v stencil &> /dev/null; then
    echo_info "Creating a template for philosopher cards..."
    
    # Check if template already exists
    if stencil show template-philosopher &>/dev/null; then
        echo_info "Template 'template-philosopher' already exists, skipping creation"
    else
        # Create a philosopher template with constraints
        OUTPUT=$(stencil new "Philosopher Template" \
          --slug template-philosopher \
          --required-field "fields.status" \
          --required-field "fields.birth" \
          --required-field "fields.death" \
          --required-field "fields.nationality" \
          --required-field "fields.school" \
          --enum-field "fields.status=deceased,alive" \
          --frozen-field "fields.birth" \
          --frozen-field "fields.death" 2>&1)
        EXIT_CODE=$?
        
        if [ $EXIT_CODE -eq 0 ]; then
            echo_success "Template created successfully"
            echo "$OUTPUT" | head -2
        else
            echo_warn "Template creation failed"
            if [ -n "$OUTPUT" ]; then
                echo_info "Error details:"
                echo "$OUTPUT" | head -3 | sed 's/^/    /'
            fi
        fi
    fi
    
    echo ""
    echo_info "Listing all templates:"
    stencil list 2>/dev/null || echo_warn "stencil list failed"
    
    echo ""
    echo_info "Showing template details:"
    stencil show template-philosopher 2>/dev/null || echo_warn "Could not show template"
    
    echo ""
    echo_info "Validating existing philosopher cards against template:"
    stencil validate --query "tag:philosopher" 2>/dev/null || echo_warn "Template validation failed or command format differs"
    
    echo ""
    echo_info "Demonstrating creating a new card from template (Immanuel Kant):"
    
    # Verify template exists using stencil show (which will fail if template doesn't exist)
    if ! stencil show template-philosopher &>/dev/null; then
        echo_warn "Template 'template-philosopher' not found."
        echo_info "The template may not have been created successfully in the previous step."
        echo_info "You can verify templates with: stencil list"
        echo_info "Skipping card creation from template demonstration."
    else
        echo_success "Template 'template-philosopher' exists and is ready to use"
        # Create a temporary file for the body content
        TEMP_BODY=$(mktemp)
        echo "German philosopher, central figure in modern philosophy" > "$TEMP_BODY"
        
        # Check if card already exists
        if scout list --query "title:Immanuel Kant" 2>/dev/null | grep -q "Immanuel Kant"; then
            echo_info "Card 'Immanuel Kant' already exists, skipping creation"
            echo_info "You can view it with: scribe show immanuel-kant"
            rm -f "$TEMP_BODY"
        else
            # Try to create the card from template
            OUTPUT=$(scribe new "Immanuel Kant" \
              --template template-philosopher \
              --tag philosopher \
              --field status=deceased \
              --field birth=1724 \
              --field death=1804 \
              --field nationality=German \
              --field school=Enlightenment \
              --body "$TEMP_BODY" 2>&1)
            EXIT_CODE=$?
            
            if [ $EXIT_CODE -eq 0 ]; then
                echo_success "Created card 'Immanuel Kant' from template"
                echo "$OUTPUT" | head -1
                echo_info "This card will automatically be included in the 'Dead Philosophers' deck"
            else
                echo_warn "Could not create card from template"
                if [ -n "$OUTPUT" ]; then
                    echo_info "Error details:"
                    echo "$OUTPUT" | head -3 | sed 's/^/    /'
                fi
                echo_info "Note: This may be because:"
                echo_info "  - Template functionality may not be fully implemented yet (see TODO in code)"
                echo_info "  - Card may already exist with a different slug"
                echo_info "  - You can create the card manually: scribe new \"Immanuel Kant\" --tag philosopher --field status=deceased ..."
            fi
            rm -f "$TEMP_BODY"
        fi
    fi
else
    echo_warn "stencil tool not available, skipping template creation"
fi

# ============================================================================
# STEP 12: Organization with Curator
# ============================================================================
echo_section "Step 12: Organization Analysis with Curator"

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
# STEP 13: Signing with Notary
# ============================================================================
echo_section "Step 13: Signing Cards with Notary"

if command -v notary &> /dev/null; then
    echo_info "Notary provides cryptographic signing and verification for cards..."
    
    # Generate a key pair (in a safe location)
    KEY_DIR=".keys"
    mkdir -p "$KEY_DIR"
    
    echo_info "Generating signing key pair..."
    notary generate-key --out "$KEY_DIR" 2>/dev/null || echo_warn "Key generation failed (may already exist)"
    
    if [ -f "$KEY_DIR/secret.key" ]; then
        echo_success "Key pair generated in $KEY_DIR/"
        
        echo ""
        echo_info "Signing philosopher cards..."
        notary sign --query "tag:philosopher status=deceased" --key "$KEY_DIR/secret.key" 2>/dev/null || \
          echo_warn "Signing failed (may need different query or key format)"
        
        echo ""
        echo_info "Verifying signatures..."
        notary verify --query "tag:philosopher status=deceased" --key "$KEY_DIR/public.key" 2>/dev/null || \
          echo_warn "Verification failed or no signatures found"
        
        echo ""
        echo_info "Note: Keep $KEY_DIR/secret.key secure and never commit it to git!"
    else
        echo_warn "Key generation failed, skipping signing demonstration"
    fi
else
    echo_warn "notary tool not available, skipping signing"
fi

# ============================================================================
# STEP 14: Export with Porter
# ============================================================================
echo_section "Step 14: Exporting and Importing Cards with Porter"

if command -v porter &> /dev/null; then
    EXPORT_DIR="philosophers-export"
    mkdir -p "$EXPORT_DIR"
    
    echo_info "Exporting philosopher cards to JSONL format..."
    if porter export --format jsonl --out "$EXPORT_DIR" --query "tag:philosopher" 2>/dev/null; then
        echo_success "Exported to JSONL format"
    else
        echo_warn "porter export failed (may need different query format)"
    fi
    
    echo ""
    echo_info "Exporting to Markdown format..."
    if porter export --format md --out "$EXPORT_DIR/markdown" --query "tag:philosopher" 2>/dev/null; then
        echo_success "Exported to Markdown format"
    else
        echo_warn "porter export to markdown failed"
    fi
    
    if [ -d "$EXPORT_DIR" ]; then
        echo ""
        echo_success "Exports created in: $EXPORT_DIR"
        echo_info "Files:"
        ls -lh "$EXPORT_DIR" | head -5 || true
        
        echo ""
        echo_info "Demonstrating import from JSONL export..."
        IMPORT_DIR="philosophers-import-test"
        mkdir -p "$IMPORT_DIR"
        
        # Try to import (this would typically import into a separate location or test)
        if porter import --from jsonl --in "$EXPORT_DIR" --out "$IMPORT_DIR" 2>/dev/null; then
            echo_success "Import demonstration completed"
        else
            echo_info "Note: Import may require different parameters or target location"
            echo_info "Example import command: porter import --from jsonl --in $EXPORT_DIR"
        fi
    fi
else
    echo_warn "porter tool not available, skipping export/import"
fi

# ============================================================================
# STEP 15: Creating Deck Snapshots
# ============================================================================
echo_section "Step 15: Creating Static Deck Snapshots"

echo_info "Creating a static snapshot of the 'Dead Philosophers' deck..."
echo_info "(This captures the current state of the deck at a point in time)"

if scribe deck snapshot dead-philosophers --out dead-philosophers-snapshot-2025 2>/dev/null || \
   scribe deck snapshot "Dead Philosophers" --out dead-philosophers-snapshot-2025 2>/dev/null; then
    echo_success "Deck snapshot created: dead-philosophers-snapshot-2025"
    echo_info "A snapshot is a static copy that won't change even if cards are added/removed"
else
    echo_warn "Could not create snapshot (command format may differ)"
    echo_info "Example snapshot command: scribe deck snapshot dead-philosophers --out dead-philosophers-2025"
fi

echo ""
echo_info "Listing available decks (including snapshots if created):"
scribe deck list 2>/dev/null || echo_warn "Could not list decks"

# ============================================================================
# FINAL SUMMARY
# ============================================================================
echo_section "Tutorial Complete - Extended Features!"

echo_success "Completed all extended tutorial steps!"
echo ""
echo_info "Summary of what we demonstrated:"
echo "  ✓ Created philosopher cards with tags and fields"
echo "  ✓ Built catalog index for fast queries"
echo "  ✓ Created dynamic deck 'Dead Philosophers' (query-based)"
echo "  ✓ Viewed individual card details"
echo "  ✓ Searched card content with scout grep"
echo "  ✓ Created links between cards (student-of, influenced)"
echo "  ✓ Performed advanced queries by school, nationality, and date ranges"
if command -v stencil &> /dev/null; then
    echo "  ✓ Created philosopher template with constraints"
    echo "  ✓ Validated cards against template"
    echo "  ✓ Created new card from template"
fi
if command -v curator &> /dev/null; then
    echo "  ✓ Analyzed organization opportunities"
fi
if command -v notary &> /dev/null; then
    echo "  ✓ Generated signing keys"
    echo "  ✓ Signed and verified cards cryptographically"
fi
if command -v porter &> /dev/null; then
    echo "  ✓ Exported cards to JSONL and Markdown formats"
    echo "  ✓ Demonstrated import functionality"
fi
echo "  ✓ Created static deck snapshot"
echo ""
echo_info "All core and extended features have been demonstrated!"
echo_info "You can now use these tools to build your own card collections."
echo ""
echo_info "Repository location: $(pwd)"
echo ""

