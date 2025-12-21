# Getting Started with Pied Piper

This guide will help you get up and running with the Pied Piper decentralized internet platform.

## What You've Built (Phase 1)

A fully functional peer-to-peer network node that can:
- ✅ Discover other nodes on the local network automatically (mDNS)
- ✅ Connect to remote nodes via bootstrap addresses
- ✅ Communicate securely using Noise encryption
- ✅ Route content using Kademlia DHT
- ✅ Publish/subscribe to topics using GossipSub
- ✅ Use both QUIC and TCP transports

## Installation

### 1. Build the Project

```bash
cd /Users/rootkill/pied-piper
cargo build --release
```

The binary will be at: `./target/release/pied-piper`

### 2. Verify Installation

```bash
./target/release/pied-piper --help
```

You should see the help text with available commands.

## Basic Usage

### Starting a Single Node

```bash
./target/release/pied-piper daemon --verbose
```

This will:
- Generate a new peer ID
- Listen on random TCP and QUIC ports
- Enable mDNS for local peer discovery
- Start the event loop

Expected output:
```
INFO pied_piper: Pied Piper v0.1.0
INFO pied_piper: Local Peer ID: 12D3KooW...
INFO pied_piper::network::node: Listening on /ip4/127.0.0.1/tcp/57668
INFO pied_piper::network::node: Listening on /ip4/192.168.0.100/tcp/57668
```

### Running Multiple Nodes (Local Testing)

#### Option 1: Manual (Separate Terminals)

**Terminal 1:**
```bash
./target/release/pied-piper daemon --verbose --tcp-port 4001 --quic-port 5001 --topic test
```

**Terminal 2:**
```bash
./target/release/pied-piper daemon --verbose --tcp-port 4002 --quic-port 5002 --topic test
```

**Terminal 3:**
```bash
./target/release/pied-piper daemon --verbose --tcp-port 4003 --quic-port 5003 --topic test
```

Within a few seconds, you should see logs indicating the nodes discovered each other:
```
INFO pied_piper::network::node: mDNS discovered peer: 12D3KooW... at /ip4/192.168.0.100/tcp/4002
INFO pied_piper::network::node: Connection established with 12D3KooW...
```

#### Option 2: Automated Test Script

```bash
./test-network.sh
```

This script will:
- Start 3 nodes automatically
- Log output to `logs/node1.log`, `logs/node2.log`, `logs/node3.log`
- Show peer discovery status
- Allow you to stop all nodes with Ctrl+C

## Understanding the Logs

### Key Log Messages

**Node Startup:**
```
INFO pied_piper: Pied Piper v0.1.0
INFO pied_piper::network::node: Local peer ID: 12D3KooW...
```
- Your unique peer identifier

**Listening Addresses:**
```
INFO pied_piper::network::node: Listening on /ip4/192.168.0.100/tcp/4001
INFO pied_piper::network::node: Listening on /ip4/192.168.0.100/udp/5001/quic-v1
```
- Addresses where your node accepts connections

**Peer Discovery:**
```
INFO pied_piper::network::node: mDNS discovered peer: 12D3KooW... at /ip4/192.168.0.100/tcp/4002
```
- Another node was discovered on the local network

**Connection Established:**
```
INFO pied_piper::network::node: Connection established with 12D3KooW... at /ip4/192.168.0.100/tcp/4002
```
- Successfully connected to a peer

**Peer Identified:**
```
INFO pied_piper::network::node: Identified peer: 12D3KooW... - Agent: pied-piper/0.1.0
```
- Exchanged identity information with peer

## Advanced Usage

### Connecting to Bootstrap Peers

If you have a node running at a known address:

```bash
# Node 1 (bootstrap node)
./target/release/pied-piper daemon --tcp-port 4001

# Note the peer ID from the logs, e.g., 12D3KooWABC...

# Node 2 (connecting to Node 1)
./target/release/pied-piper daemon \
  --bootstrap "12D3KooWABC...@/ip4/192.168.0.100/tcp/4001"
```

### Subscribing to Topics

Topics enable pub/sub messaging between nodes:

```bash
./target/release/pied-piper daemon \
  --topic announcements \
  --topic chat \
  --topic data-sync
```

All nodes subscribed to the same topic will form a mesh network for message propagation.

### Disabling mDNS

For testing DHT-only discovery:

```bash
./target/release/pied-piper daemon --no-mdns --bootstrap "<peer-address>"
```

### Custom Ports

Specify exact ports to use:

```bash
./target/release/pied-piper daemon \
  --tcp-port 4001 \
  --quic-port 5001
```

## Checking Node Status

### View Peer ID

Look for this line in the startup logs:
```
INFO pied_piper: Local Peer ID: 12D3KooW...
```

### View Listening Addresses

Check logs for lines like:
```
INFO pied_piper::network::node: Listening on /ip4/...
```

### Monitor Connections

Watch for these log events:
- `Connection established` - New peer connected
- `Connection closed` - Peer disconnected
- `mDNS discovered peer` - Peer found via local discovery

## Common Scenarios

### Scenario 1: Private Local Network

Perfect for development and testing:

```bash
# Start multiple nodes on your machine
./target/release/pied-piper daemon --verbose --topic dev
```

Nodes will find each other via mDNS automatically.

### Scenario 2: Public Bootstrap Network

For connecting to a wider network:

```bash
# Use known bootstrap nodes
./target/release/pied-piper daemon \
  --bootstrap "<peer1-address>" \
  --bootstrap "<peer2-address>"
```

### Scenario 3: Server Deployment

Running a public node:

```bash
# Use fixed ports and specific interface
./target/release/pied-piper daemon \
  --tcp-port 4001 \
  --quic-port 5001 \
  --no-mdns
```

## Troubleshooting

### Problem: "No known peers" Warning

**Cause:** Node hasn't discovered any peers yet

**Solutions:**
- Wait a few seconds for mDNS discovery
- Ensure other nodes are running on the same network
- Specify bootstrap peers manually
- Check firewall settings

### Problem: Nodes Don't Discover Each Other

**Causes & Solutions:**

1. **Different networks:** Ensure nodes are on same local network for mDNS
2. **Firewall blocking:** Allow UDP/TCP traffic on configured ports
3. **mDNS disabled:** Some networks block multicast - use bootstrap peers instead
4. **Wrong addresses:** Verify IP addresses in bootstrap peer addresses

### Problem: Connection Timeouts

**Solutions:**
- Verify peer is still running
- Check network connectivity
- Ensure ports are accessible (not firewalled)
- Try using IP address instead of hostname

### Problem: High Memory Usage

**Current State:** Expected memory usage is ~50MB per node in Phase 1

**Future:** Will implement resource limits in Phase 2

## What's Next?

### Immediate Experimentation

1. **Test Discovery:** Run multiple nodes and watch them discover each other
2. **Test Topics:** Subscribe different nodes to different topics
3. **Test Reconnection:** Stop/start nodes and observe reconnection
4. **Monitor Logs:** Watch the DHT build its routing table

### Phase 2: Wasm Runtime (Coming Soon)

Next, you'll add:
- WebAssembly module loading
- WASI support for system access
- Host functions for networking and storage
- Module execution with resource limits

### Phase 3: Application Deployment

Then you'll implement:
- Deployment tools
- Application registry
- Content distribution
- HTTP gateway for web browsers

## Resources

- **Project Plan:** See [Project.md](./Project.md) for the complete roadmap
- **README:** See [README.md](./README.md) for project overview
- **Code:** Explore `src/network/` for implementation details

## Questions?

The project structure:
```
src/
├── main.rs           # Entry point & CLI handling
├── cli/mod.rs        # Command line interface
└── network/
    ├── mod.rs        # Module exports
    ├── behaviour.rs  # libp2p behaviour combining protocols
    ├── node.rs       # Main network node implementation
    └── transport.rs  # Transport configuration
```

Key files to understand:
- **node.rs:** Main event loop and protocol handling
- **behaviour.rs:** Protocol composition (DHT, mDNS, GossipSub, etc.)
- **main.rs:** CLI commands and application startup

---

**Congratulations!** You've successfully completed Phase 1 of building a decentralized internet platform. You now have a working P2P network foundation that can discover peers and establish secure connections.

Next step: Add WebAssembly runtime support! 🚀
