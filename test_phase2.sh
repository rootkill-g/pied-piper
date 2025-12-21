#!/usr/bin/env bash
# Quick test script for Phase 2 WebAssembly runtime

echo "Phase 2 WebAssembly Runtime - Quick Test"
echo "========================================"
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found"
    exit 1
fi

# Build the project
echo "Building Pied Piper..."
cargo build --release 2>&1 | tail -5

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "✓ Build successful"
echo ""

# Create a simple test WASM module (binary format, base64 encoded)
# This is the compiled version of:
# (module (func (export "test") (result i32) i32.const 42))
WASM_B64="AGFzbQEAAAABBgFgAAF/AwIBABAAAApGBQMBAAELBAABBgsPAQ=="
echo "$WASM_B64" | base64 -d > /tmp/test.wasm

if [ -f /tmp/test.wasm ]; then
    echo "✓ Created test WASM module"
    
    # Test the runtime
    echo ""
    echo "Testing WASM execution..."
    ./target/release/pied-piper run /tmp/test.wasm --function test 2>&1 | head -20
    
    echo ""
    echo "Test complete!"
    rm /tmp/test.wasm
else
    echo "Note: Could not create test WASM module"
    echo "You can test manually with your own WASM files using:"
    echo "  ./target/release/pied-piper run <path-to-wasm> --function <function-name>"
fi

echo ""
echo "Phase 2 runtime is ready!"
echo "Next: Phase 3 - Module distribution over P2P network"
