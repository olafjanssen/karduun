#!/bin/bash

# Test script for Karduun MCP Server

echo "Building Karduun MCP Server..."
cargo build --bin karduun-mcp

if [ $? -ne 0 ]; then
    echo "Build failed"
    exit 1
fi

echo "Build successful!"
echo "You can now run the MCP server with:"
echo "  cargo run --bin karduun-mcp"
echo ""
echo "The server will listen on 127.0.0.1:8080 and provide MCP access to all Karduun tools:"
echo ""
echo "📝 Scribe (Core CRUD):"
echo "  - scribe.new, scribe.show, scribe.edit, scribe.archive"
echo "  - scribe.fork, scribe.merge, scribe.link, scribe.unlink"
echo "  - scribe.deck.* (new, show, add, remove, snapshot)"
echo ""
echo "🔍 Scout (Query & Search):"
echo "  - scout.list, scout.grep, scout.backlinks, scout.tree"
echo ""
echo "📚 Catalog (Index Management):"
echo "  - catalog.rebuild, catalog.status, catalog.vacuum"
echo ""
echo "📊 Gauge (Analytics):"
echo "  - gauge.analyze (single card or repository)"
echo ""
echo "🧹 Curator (Organization):"
echo "  - curator.plan, curator.apply, curator.autoclean"
echo ""
echo "🎨 Stencil (Templates):"
echo "  - stencil.new, stencil.list, stencil.show, stencil.validate"
echo ""
echo "🚢 Porter (Import/Export):"
echo "  - porter.export, porter.import"
echo ""
echo "🔏 Notary (Signing):"
echo "  - notary.sign, notary.verify, notary.timestamp"
echo ""
echo "🌱 Eco (Ecosystem):"
echo "  - eco.scan, eco.resonance, eco.print, eco.mature, eco.status, eco.evolve"
echo ""
echo "Example MCP request (using curl or MCP client):"
echo '{
  "jsonrpc": "2.0",
  "method": "scribe.new",
  "params": {
    "title": "Test Card",
    "slug": "test-card"
  },
  "id": 1
}'
