#!/bin/bash

echo "Building Karduun Chat Client..."
cargo build --bin karduun-chat

if [ $? -ne 0 ]; then
    echo "Build failed"
    exit 1
fi

echo "Build successful!"
echo ""
echo "You can now run the Karduun Chat client with:"
echo "  cargo run --bin karduun-chat"
echo ""
echo "Or connect to a specific server:"
echo "  cargo run --bin karduun-chat -- --server http://localhost:8080"
echo ""
echo "Features:"
echo "  ✅ TUI interface with ratatui"
echo "  ✅ Connects to Karduun MCP Server"
echo "  ✅ Simple MCP request parsing"
echo "  ✅ Color-coded messages (user: green, server: blue)"
echo "  ✅ Real-time interaction"
echo ""
echo "Example usage:"
echo "  Type: scribe.new {\"title\": \"My Card\", \"slug\": \"my-card\"}"
echo "  Press Enter to send"
echo "  Press ESC to exit"
