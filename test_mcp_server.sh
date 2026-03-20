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
echo "The server will listen on 127.0.0.1:8080 and provide MCP access to:"
echo "  - scribe.new: Create new cards"
echo "  - scribe.show: Show card details"
echo "  - Other scribe operations (edit, archive, fork, merge, link, unlink)"
echo "  - Deck operations (new, show, add, remove, snapshot)"

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
