#!/usr/bin/env bash
# Script to test multi-node peer discovery
# Run this to see nodes discover and connect to each other

set -e

echo "🚀 Pied Piper Multi-Node Test"
echo "=============================="
echo ""
echo "Starting 3 nodes that will discover each other via mDNS..."
echo "Press Ctrl+C to stop all nodes"
echo ""

# Build if needed
if [ ! -f "./target/release/pied-piper" ]; then
    echo "Building project..."
    cargo build --release
    echo ""
fi

# Create log directory
mkdir -p logs

# Start nodes in background
echo "Starting Node 1..."
./target/release/pied-piper daemon --verbose --topic test --tcp-port 4001 --quic-port 5001 > logs/node1.log 2>&1 &
NODE1_PID=$!

echo "Starting Node 2..."
./target/release/pied-piper daemon --verbose --topic test --tcp-port 4002 --quic-port 5002 > logs/node2.log 2>&1 &
NODE2_PID=$!

echo "Starting Node 3..."
./target/release/pied-piper daemon --verbose --topic test --tcp-port 4003 --quic-port 5003 > logs/node3.log 2>&1 &
NODE3_PID=$!

echo ""
echo "✅ All nodes started!"
echo ""
echo "Node 1: PID $NODE1_PID (TCP: 4001, QUIC: 5001)"
echo "Node 2: PID $NODE2_PID (TCP: 4002, QUIC: 5002)"  
echo "Node 3: PID $NODE3_PID (TCP: 4003, QUIC: 5003)"
echo ""
echo "Logs are being written to:"
echo "  - logs/node1.log"
echo "  - logs/node2.log"
echo "  - logs/node3.log"
echo ""
echo "Waiting for nodes to discover each other..."
sleep 3

# Check if nodes discovered each other
echo ""
echo "Checking node logs for peer discovery..."
if grep -q "mDNS discovered peer" logs/node1.log logs/node2.log logs/node3.log; then
    echo "✅ SUCCESS: Nodes discovered each other via mDNS!"
    echo ""
    echo "Sample discovery events:"
    grep "mDNS discovered peer" logs/node*.log | head -n 5
else
    echo "⏳ Waiting for peer discovery... (check logs for details)"
fi

echo ""
echo "To view live logs, run in another terminal:"
echo "  tail -f logs/node1.log"
echo "  tail -f logs/node2.log"
echo "  tail -f logs/node3.log"
echo ""
echo "Press Ctrl+C to stop all nodes..."
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "🛑 Stopping all nodes..."
    kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true
    echo "✅ All nodes stopped"
    echo ""
    echo "Check logs/ directory for complete logs"
    exit 0
}

trap cleanup INT TERM

# Keep script running
wait
