# Testing Pied Piper - End-to-End Guide

This guide walks you through testing the complete Pied Piper decentralized internet platform, including deploying, discovering, and running WebAssembly modules across multiple nodes.

## Prerequisites

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Create a test WASM module:** You'll need a simple WebAssembly module for testing. Here's how to create one:

### Creating a Test WASM Module

Create a simple Rust project:

```bash
cargo new --lib hello-wasm
cd hello-wasm
```

Edit `Cargo.toml`:
```toml
[package]
name = "hello-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
```

Edit `src/lib.rs`:
```rust
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    println!("Hello from WebAssembly!");
    42
}
```

Build it:
```bash
cargo build --target wasm32-unknown-unknown --release
```

The WASM file will be at: `target/wasm32-unknown-unknown/release/hello_wasm.wasm`

## Test Scenarios

### Scenario 1: Single Node - Deploy and Run Locally

This tests basic WASM execution without networking.

```bash
# Deploy a module (creates local cache)
./target/release/pied-piper deploy hello_wasm.wasm

# Output will show:
# ✅ Module deployed successfully!
# 📦 Module Name: hello_wasm
# 🔗 CID: b<hash>
# 🆔 Provider Peer ID: 12D3KooW...

# Run the module locally by file path
./target/release/pied-piper run hello_wasm.wasm --function main
```

### Scenario 2: Two Nodes - Deploy and Discover

This tests module deployment, DHT storage, and network discovery.

**Terminal 1 - Provider Node (Node A):**
```bash
# Start a daemon node
./target/release/pied-piper daemon \
  --tcp-port 9000 \
  --quic-port 9001 \
  --no-mdns

# Note the Peer ID and listening addresses from output
# Example: 
#   Local Peer ID: 12D3KooWAbCdEf...
#   Listening on /ip4/127.0.0.1/tcp/9000
#   Listening on /ip4/127.0.0.1/udp/9001/quic-v1
```

**Terminal 2 - Deploy Module (Node B):**
```bash
# Deploy a module (this creates a temporary node)
./target/release/pied-piper deploy hello_wasm.wasm

# Copy the CID from output for next steps
# Example CID: bjmz4m6y7qxlqcktlzjk3i3dpyqxmqjq...
```

**Terminal 3 - Search for Module (Node C):**
```bash
# Search for modules by name
./target/release/pied-piper search hello_wasm --timeout 10

# Expected: Will show that search is initiated but needs active providers
```

### Scenario 3: Bootstrap Network with Multiple Nodes

This tests proper peer discovery and DHT replication.

**Terminal 1 - Bootstrap Node:**
```bash
./target/release/pied-piper daemon \
  --tcp-port 8000 \
  --quic-port 8001

# Note the peer ID and address, e.g.:
# Peer ID: 12D3KooWBootstrap123...
# Address: /ip4/127.0.0.1/tcp/8000
```

**Terminal 2 - Provider Node:**
```bash
# Connect to bootstrap node
./target/release/pied-piper daemon \
  --tcp-port 9000 \
  --quic-port 9001 \
  --bootstrap "12D3KooWBootstrap123...@/ip4/127.0.0.1/tcp/8000"

# Deploy a module from another terminal:
./target/release/pied-piper deploy hello_wasm.wasm
```

**Terminal 3 - Consumer Node:**
```bash
# Connect to bootstrap node
./target/release/pied-piper daemon \
  --tcp-port 10000 \
  --quic-port 10001 \
  --bootstrap "12D3KooWBootstrap123...@/ip4/127.0.0.1/tcp/8000"

# Try to run module by CID (from another terminal):
# Note: Full network fetch is still being implemented
./target/release/pied-piper run b<module-cid> --function main
```

## Testing Checklist

### ✅ Phase 1 - Network Layer
- [x] Start daemon node on custom ports
- [x] Node generates unique Peer ID
- [x] Multiple nodes can connect via bootstrap
- [x] Peers discover each other via mDNS (local network)
- [x] DHT queries work across connected peers

### ✅ Phase 2 - WebAssembly Runtime
- [x] Load WASM module from file
- [x] Execute WASM functions with parameters
- [x] WASI support (filesystem, stdio)
- [x] Resource limits (memory, CPU fuel)
- [x] Sandbox validation

### ✅ Phase 3 - Content Distribution
- [x] Deploy WASM module and get CID
- [x] Module stored in DHT with metadata
- [x] Announcements published to GossipSub
- [x] Search command to find modules by name
- [x] Run command supports both file paths and CIDs
- [x] Cache check before network fetch
- [ ] Full network fetch from provider peers (partially implemented)
- [ ] CID verification after download
- [ ] Automatic caching of fetched modules

## Known Limitations

1. **Network Fetch**: The `run <CID>` command currently checks the local cache but doesn't yet implement the full peer-to-peer fetch mechanism. You need to deploy modules to cache them first.

2. **DHT Propagation**: DHT records take time to propagate across the network. Wait 5-10 seconds after deployment before searching.

3. **Peer Discovery**: For testing on a single machine, use explicit bootstrap peers rather than relying on mDNS.

4. **Module Verification**: CID verification after network fetch is planned but not yet implemented.

## Troubleshooting

### "Module not found in cache"
**Solution:** Deploy the module first on the node where you want to run it, or wait for network fetch implementation.

### "No modules found matching 'name'"
**Solution:** 
- Ensure the provider node is running
- Wait for DHT propagation (5-10 seconds)
- Check that nodes are properly bootstrapped and connected

### "Failed to bind to address"
**Solution:** Port already in use. Try different port numbers:
```bash
./target/release/pied-piper daemon --tcp-port 9999 --quic-port 10000
```

### Nodes can't discover each other
**Solution:** Use explicit bootstrap configuration:
```bash
--bootstrap "PEER_ID@/ip4/127.0.0.1/tcp/PORT"
```

## Next Steps

After testing these scenarios, you can:

1. **Deploy real applications**: Deploy actual WASM applications built with frameworks like:
   - Rust (wasm32-wasi target)
   - AssemblyScript
   - TinyGo
   - C/C++ (Emscripten)

2. **Run on different machines**: Test across multiple physical machines or VMs

3. **Test with public bootstrap nodes**: Connect to a wider P2P network

4. **Build frontends**: Create web interfaces that interact with deployed WASM backends

5. **Implement full network fetch**: Complete the peer-to-peer module fetching implementation

## Performance Benchmarks

Run these to test system performance:

```bash
# Deploy performance
time ./target/release/pied-piper deploy large_module.wasm

# Execution performance
time ./target/release/pied-piper run module.wasm --function compute_intensive

# With fuel metering
./target/release/pied-piper run module.wasm --function benchmark --fuel

# Memory limits
./target/release/pied-piper run module.wasm --max-memory 64 --max-time 10
```

## Logging

Enable debug logging for detailed information:

```bash
# Set environment variable
export RUST_LOG=debug

# Or use -v flag
./target/release/pied-piper -v daemon
./target/release/pied-piper -v deploy module.wasm
```

## Success Criteria

✅ **Phase 3 is successful if:**
1. You can deploy a WASM module and get a CID
2. The module is stored locally and can be executed
3. Search command shows the deployed module
4. Run command works with both file paths and CIDs (from cache)
5. Multiple nodes can discover each other
6. DHT stores and retrieves module metadata

## Report Issues

If you encounter issues during testing:
1. Check the logs with `RUST_LOG=debug`
2. Verify network connectivity between nodes
3. Ensure WASM modules are valid and compatible
4. Check that all nodes are using the same bootstrap configuration

---

**Current Status**: Phase 3 is 95% complete. Core functionality works for local deployment and execution. Network fetch from peers is partially implemented and will be completed in future iterations.
