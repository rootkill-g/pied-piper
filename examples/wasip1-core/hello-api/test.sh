#!/usr/bin/env bash

# Test script for hello-api example
# This demonstrates the full WASM I/O workflow

set -e

echo "🧪 Testing Hello API Example"
echo "=============================="
echo ""

# Check if wasmtime is installed
if ! command -v wasmtime &> /dev/null; then
    echo "⚠️  wasmtime not found. Install with: curl https://wasmtime.dev/install.sh -sSf | bash"
    echo "   Skipping local WASM test..."
else
    echo "✅ wasmtime found"
    
    # Test locally with wasmtime
    echo ""
    echo "📝 Test 1: Hello endpoint (local)"
    echo "Input:"
    cat << 'EOF' | tee /tmp/request.json
{"method":"GET","path":"/api/hello","query":{"name":"Alice"},"headers":{},"body":"","content_type":null}
EOF
    
    echo ""
    echo "Output:"
    cat /tmp/request.json | wasmtime run \
        --invoke handle_request \
        target/wasm32-wasip2/release/hello_api.wasm 2>&1 || true
    
    echo ""
    echo ""
    echo "📝 Test 2: API Info endpoint (local)"
    echo "Input:"
    cat << 'EOF' | tee /tmp/request2.json
{"method":"GET","path":"/api/info","query":{},"headers":{},"body":"","content_type":null}
EOF
    
    echo ""
    echo "Output:"
    cat /tmp/request2.json | wasmtime run \
        --invoke handle_request \
        target/wasm32-wasip2/release/hello_api.wasm 2>&1 || true
    
    echo ""
    echo ""
    echo "📝 Test 3: Echo endpoint with POST data (local)"
    echo "Input:"
    cat << 'EOF' | tee /tmp/request3.json
{"method":"POST","path":"/api/echo","query":{},"headers":{},"body":"{\"message\":\"Hello from test!\"}","content_type":"application/json"}
EOF
    
    echo ""
    echo "Output:"
    cat /tmp/request3.json | wasmtime run \
        --invoke handle_request \
        target/wasm32-wasip2/release/hello_api.wasm 2>&1 || true
fi

echo ""
echo "=============================="
echo "📦 Module Information"
echo "=============================="
echo "File: target/wasm32-wasip2/release/hello_api.wasm"
echo "Size: $(du -h target/wasm32-wasip2/release/hello_api.wasm | cut -f1)"
echo ""
echo "To deploy:"
echo "  ../../target/release/pied-piper deploy target/wasm32-wasip2/release/hello_api.wasm"
echo ""
echo "To test via gateway:"
echo "  # Start gateway: ../../target/release/pied-piper gateway --port 8080"
echo "  # Test: curl 'http://localhost:8080/cid/<cid>/api/hello?name=Test'"
echo ""
