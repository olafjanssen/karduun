#!/bin/bash

echo "🚀 Testing Karduun MCP Server and Chat Client Connection"
echo ""

# Build both components
echo "🔨 Building components..."
cargo build --bin karduun-mcp 2>/dev/null || { echo "❌ Failed to build MCP server"; exit 1; }
cargo build --bin karduun-chat 2>/dev/null || { echo "❌ Failed to build chat client"; exit 1; }

echo "✅ Build successful!"
echo ""

echo "📋 Test Instructions:"
echo "1. Open a new terminal and run the MCP server:"
echo "   cargo run --bin karduun-mcp"
echo ""
echo "2. In another terminal, run the chat client:"
echo "   cargo run --bin karduun-chat"
echo ""
echo "3. Try these commands in the chat:"
echo "   scribe.new {\"title\": \"Test Card\", \"slug\": \"test-card\"}"
echo "   scout.list {}"
echo "   catalog.status {}"
echo ""
echo "💡 The server should respond with JSON results!"
echo ""

echo "🔧 Troubleshooting:"
echo "- Make sure the server is running before starting the client"
echo "- Check that both are using the same address (127.0.0.1:8080)"
echo "- The server uses raw TCP, not HTTP"
echo "- Use ESC to exit the chat client"
