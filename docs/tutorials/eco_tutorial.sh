#!/bin/bash
# Karduun CLI Tutorial: Ecosystem Dynamics
#
# This script demonstrates the eco tool for simulating a living ecosystem
# of knowledge cards with resonance tracking, printing quotas, and maturation.
#
# Usage:
#   ./eco_tutorial.sh
#
# Or run with a custom workspace directory:
#   WORKDIR=my-ecosystem ./eco_tutorial.sh

set -euo pipefail

# Source shared helper functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/tutorial_helpers.sh"

echo_section "Karduun Ecosystem Dynamics Tutorial"
echo_info "This tutorial demonstrates the eco tool for simulating"
echo_info "a living ecosystem of knowledge cards."
echo ""

# ============================================================================
# STEP 1: Initialize Workspace
# ============================================================================
echo_section "Step 1: Initializing Workspace"

WORKDIR=${WORKDIR:-eco}
init_workspace "$WORKDIR" || exit 1

# ============================================================================
# STEP 2: Create Knowledge Cards
# ============================================================================
echo_section "Step 2: Creating Knowledge Cards"

echo_info "Creating cards for our ecosystem..."

# Create the core cards mentioned in the briefing
scribe new "Cloud Deployment" --tag cloud --tag infrastructure
scribe new "APIs" --tag cloud --tag programming
scribe new "Security" --tag cloud --tag infrastructure

echo_success "Created 3 knowledge cards"

# ============================================================================
# STEP 3: Simulate Card Scanning (Resonance)
# ============================================================================
echo_section "Step 3: Simulating Card Scanning"

echo_info "When students scan cards, their resonance increases..."

# Get card UIDs
CLOUD_UID=$(find_card_uid "cloud-deployment")
API_UID=$(find_card_uid "apis")
SECURITY_UID=$(find_card_uid "security")

if [ -z "$CLOUD_UID" ] || [ -z "$API_UID" ] || [ -z "$SECURITY_UID" ]; then
    echo_error "Could not find card UIDs. Please check if cards were created."
    exit 1
fi

echo_info "Cloud Deployment UID: $CLOUD_UID"
echo_info "APIs UID: $API_UID"
echo_info "Security UID: $SECURITY_UID"
echo ""

# Simulate multiple scans (in a real scenario, these would be from different users)
echo_info "Simulating multiple card scans..."

# Scan Cloud Deployment 3 times
eco scan $CLOUD_UID --resonance-increase 0.3
eco scan $CLOUD_UID --resonance-increase 0.3
eco scan $CLOUD_UID --resonance-increase 0.3

# Scan APIs 5 times (high interest)
eco scan $API_UID --resonance-increase 0.3
eco scan $API_UID --resonance-increase 0.3
eco scan $API_UID --resonance-increase 0.3
eco scan $API_UID --resonance-increase 0.3
eco scan $API_UID --resonance-increase 0.3

# Scan Security 2 times
eco scan $SECURITY_UID --resonance-increase 0.3
eco scan $SECURITY_UID --resonance-increase 0.3

echo_success "Simulated card scanning complete"

# ============================================================================
# STEP 4: Check Resonance Levels
# ============================================================================
echo_section "Step 4: Checking Resonance Levels"

echo_info "Checking how much resonance each card has accumulated..."
echo ""

eco resonance $CLOUD_UID
echo ""
eco resonance $API_UID
echo ""
eco resonance $SECURITY_UID
echo ""

# ============================================================================
# STEP 5: Print Cards (Subject to Quotas)
# ============================================================================
echo_section "Step 5: Printing Cards"

echo_info "Printing cards is subject to daily and weekly quotas..."
echo ""

# Try to print the high-resonance APIs card
eco print $API_UID --copies 3
echo ""

# Check ecosystem status
eco status
echo ""

# ============================================================================
# STEP 6: Check for Maturation Opportunities
# ============================================================================
echo_section "Step 6: Checking for Maturation Opportunities"

echo_info "The system checks if card clusters can spawn new concepts..."
echo ""

eco mature --similarity-threshold 0.7 --min-cluster-size 2
echo ""

# ============================================================================
# STEP 7: Run Ecosystem Evolution
# ============================================================================
echo_section "Step 7: Running Ecosystem Evolution"

echo_info "Running a complete evolution cycle..."
echo ""

eco evolve
echo ""

# ============================================================================
# STEP 8: Final Status Check
# ============================================================================
echo_section "Step 8: Final Ecosystem Status"

eco status
echo ""

echo_section "Tutorial Complete!"
echo_success "You've successfully simulated a living knowledge ecosystem!"
echo_info "Key concepts demonstrated:"
echo_info "  • Resonance tracking (community interest)"
echo_info "  • Printing quotas (scarcity management)"
echo_info "  • Maturation system (emergent concepts)"
echo_info "  • Ecosystem balance (automatic decay & quotas)"
echo ""
echo_info "Next steps:"
echo_info "  • Try creating more cards and observing ecosystem behavior"
echo_info "  • Experiment with different resonance patterns"
echo_info "  • Explore how maturation could generate new concept cards"
echo ""
