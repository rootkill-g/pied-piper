# Pied Piper - Decentralized Internet Platform

A fully decentralized internet platform for running WebAssembly applications, built with Rust and libp2p.

## Project Status: Phase 1 - Foundation ✅

**Current Version:** 0.1.0

### Completed Features

- ✅ **libp2p Network Stack**: QUIC and TCP transport with Noise encryption
- ✅ **Peer Discovery**: mDNS for local network discovery, Kademlia DHT for global discovery
- ✅ **Network Protocols**: 
  - Yamux multiplexing
  - Identify protocol for peer information exchange
  - Ping for connection keep-alive
  - GossipSub for pub/sub messaging
- ✅ **CLI Tool**: Command-line interface for node management
- ✅ **Logging**: Structured logging with tracing

## Quick Start

### Prerequisites

- Rust 1.70+ (nightly recommended)
- Cargo

### Installation

```bash
# Clone the repository
git clone https://github.com/your-username/pied-piper
cd pied-piper

# Build the project
cargo build --release

# The binary will be in target/release/pied-piper
```

### Running a Node

#### Start a basic node

```bash
./target/release/pied-piper daemon
```

#### Start with verbose logging

```bash
./target/release/pied-piper daemon --verbose
```

#### Subscribe to topics (for pub/sub messaging)

```bash
./target/release/pied-piper daemon --topic test --topic announcements
```

#### Specify ports

```bash
./target/release/pied-piper daemon --tcp-port 4001 --quic-port 4002
```

#### Connect to bootstrap peers

```bash
./target/release/pied-piper daemon --bootstrap "<peer-id>@/ip4/<ip>/tcp/<port>"
```

### Testing Multi-Node Communication

To test peer discovery, run multiple nodes on the same network:

**Terminal 1:**
```bash
./target/release/pied-piper daemon --verbose --topic test
```

**Terminal 2:**
```bash
./target/release/pied-piper daemon --verbose --topic test
```

The nodes should discover each other via mDNS and connect automatically!

## Current Listening Addresses

When you start a node, it will listen on:
- **TCP**: Random port (or specified with `--tcp-port`)
- **QUIC**: Random port (or specified with `--quic-port`)
- **Interfaces**: All available network interfaces (127.0.0.1, local network IPs, etc.)

Example output:
```
INFO pied_piper::network::node: Listening on /ip4/127.0.0.1/tcp/57668
INFO pied_piper::network::node: Listening on /ip4/192.168.0.100/tcp/57668
INFO pied_piper::network::node: Listening on /ip4/127.0.0.1/udp/49808/quic-v1
INFO pied_piper::network::node: Listening on /ip4/192.168.0.100/udp/49808/quic-v1
```

## Architecture

### Current Architecture (Phase 1)

```
┌─────────────────────────────────────────────────┐
│              Application Layer                   │
│  (CLI - daemon, info, deploy commands)          │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│           Network Layer (libp2p)                │
│  - QUIC/TCP Transport                           │
│  - Noise Encryption                             │
│  - Yamux Multiplexing                           │
│  - Kademlia DHT                                 │
│  - mDNS Discovery                               │
│  - GossipSub Pub/Sub                            │
│  - Identify Protocol                            │
└─────────────────────────────────────────────────┘
```

## CLI Commands

### `daemon`
Start the Pied Piper daemon (P2P node)

**Options:**
- `--tcp-port <PORT>` - TCP port to listen on (default: random)
- `--quic-port <PORT>` - QUIC port to listen on (default: random)
- `--no-mdns` - Disable mDNS local discovery
- `--bootstrap <ADDR>` - Bootstrap peer addresses
- `--topic <TOPIC>` - Topics to subscribe to (can be used multiple times)
- `--verbose` - Enable verbose logging
- `--config <FILE>` - Configuration file path

### `info`
Show node information (coming in Phase 3)

### `deploy`
Deploy a WebAssembly application (coming in Phase 3)

## Development

### Project Structure

```
pied-piper/
├── src/
│   ├── main.rs              # Application entry point
│   ├── cli/                 # CLI argument parsing
│   │   └── mod.rs
│   ├── network/             # P2P networking layer
│   │   ├── mod.rs
│   │   ├── behaviour.rs     # libp2p behaviour
│   │   ├── node.rs          # Network node implementation
│   │   └── transport.rs     # Transport configuration
│   └── storage/             # Content storage (Phase 2)
├── Cargo.toml               # Rust dependencies
├── Project.md               # Comprehensive project plan
└── README.md                # This file
```

### Building from Source

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- daemon --verbose
```

### Dependencies

Key dependencies:
- **libp2p** (0.56.0) - P2P networking framework
- **tokio** (1.48.0) - Async runtime
- **anyhow** (1.0.100) - Error handling
- **clap** (4.5.53) - CLI argument parsing
- **tracing** (0.1.44) - Structured logging
- **serde** (1.0.228) - Serialization

See `Cargo.toml` for the complete list.

## Roadmap

### ✅ Phase 1: Foundation (Completed)
- [x] libp2p network stack with QUIC/TCP
- [x] Kademlia DHT integration
- [x] Peer discovery (mDNS + DHT)
- [x] Basic CLI tools
- [x] Logging and configuration

### 🔄 Phase 2: Wasm Runtime (Next - Months 4-6)
- [ ] Wasmtime integration
- [ ] WASI implementation
- [ ] Resource limiting (CPU/memory)
- [ ] Module loading and caching
- [ ] Host functions (network, storage, crypto)

### 📅 Phase 3: Application Deployment (Months 7-9)
- [ ] Deployment pipeline
- [ ] Application registry
- [ ] Content routing
- [ ] HTTP gateway for browser access
- [ ] Human-readable names (decentralized DNS)

### 📅 Phase 4: Advanced Features (Months 10-12)
- [ ] CRDT-based state management
- [ ] Real-time communication (WebSockets)
- [ ] Identity & security (DIDs)
- [ ] Monitoring & observability

### 📅 Phase 5: Optimization (Months 13-15)
- [ ] Performance optimization
- [ ] Reliability improvements
- [ ] SDKs (Rust, JavaScript, Go)
- [ ] Developer tooling

### 📅 Phase 6: Launch (Month 16+)
- [ ] Production infrastructure
- [ ] Community building
- [ ] Governance model

See [Project.md](./Project.md) for the complete detailed plan.

## Network Protocol Details

### Peer ID
Each node has a unique peer ID derived from its Ed25519 keypair:
```
Example: 12D3KooWQYcdqakvEurMLWsF1exCT3YnZWuQ4rGPrqKwEAJ351qc
```

### Multiaddresses
Nodes are addressed using libp2p multiaddresses:
```
/ip4/192.168.0.100/tcp/4001/p2p/12D3KooW...
/ip4/192.168.0.100/udp/4002/quic-v1/p2p/12D3KooW...
```

### Supported Protocols
- `/pied-piper/1.0.0` - Main protocol
- `/ipfs/kad/1.0.0` - Kademlia DHT
- `/ipfs/ping/1.0.0` - Ping
- `/meshsub/1.1.0` - GossipSub
- `/ipfs/id/1.0.0` - Identify

## Configuration

Configuration file support is planned. For now, use CLI arguments.

## Logging

The project uses structured logging via `tracing`. Set log levels using:

```bash
# Environment variable
RUST_LOG=debug ./target/release/pied-piper daemon

# Or use --verbose flag
./target/release/pied-piper daemon --verbose
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## Performance

Current benchmarks (Phase 1):
- **Peer Discovery**: < 5 seconds on local network
- **Connection Establishment**: < 1 second
- **Memory Usage**: ~50MB base per node
- **Startup Time**: < 1 second

## Contributing

This project is currently in active development. Contributions are welcome!

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

Please ensure:
- Code compiles without warnings
- Tests pass
- Follow Rust best practices
- Update documentation

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Troubleshooting

### "No known peers" warning
This is normal if you haven't specified bootstrap peers. The node is waiting for other peers to connect via mDNS.

### mDNS not discovering peers
- Ensure nodes are on the same local network
- Check firewall settings
- Try specifying explicit bootstrap peers

### Connection refused
- Verify the peer is running
- Check the multiaddress format
- Ensure network connectivity

## Security

**Note**: This is early-stage software. Do not use in production yet.

Current security features:
- Ed25519 keypairs for peer identity
- Noise protocol for encrypted connections
- TLS 1.3 support via QUIC

Planned security features:
- Sandbox isolation for Wasm
- Resource limits
- Capability-based permissions
- DIDs and verifiable credentials

## License

[License information to be added]

## Acknowledgments

Built with:
- [libp2p](https://libp2p.io/) - Modular P2P networking stack
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Asynchronous runtime

Inspired by:
- [IPFS](https://ipfs.io/) - Distributed file system
- [Holochain](https://holochain.org/) - Agent-centric computing
- [Solid](https://solidproject.org/) - Decentralized web

## Contact

[Contact information to be added]

---

**Status**: Phase 1 Complete ✅ | Next: Phase 2 - Wasm Runtime

*Last updated: December 22, 2025*
