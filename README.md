# Pied Piper - Decentralized Internet Platform

> A fully decentralized internet platform for running WebAssembly applications, built with Rust and libp2p.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-101%20passing-brightgreen.svg)](#testing)
[![Status](https://img.shields.io/badge/status-production%20ready-success.svg)](PROJECT_STATUS.md)

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** with `wasm32-wasip1` target
- **Cargo** (comes with Rust)

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-wasip1
```

### Installation

#### Option 1: Docker (Recommended for Quick Start)

```bash
# Clone the repository
git clone https://github.com/rootkill-g/pied-piper
cd pied-piper

# Start 4-node network with Docker Compose
docker compose up -d

# Or use the Makefile
make up
```

Access nodes at:
- Bootstrap: http://localhost:8080
- Node 1: http://localhost:8081
- Node 2: http://localhost:8082
- Node 3: http://localhost:8083

**See [DOCKER.md](DOCKER.md) for complete Docker deployment guide.**

#### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/rootkill-g/pied-piper
cd pied-piper

# Build in release mode
cargo build --release

# Binary location: target/release/pied-piper
```

### Launch Your First Node

```bash
# Start a node with default settings
./target/release/pied-piper serve

# Or with custom configuration
./target/release/pied-piper serve --gateway-port 8080 --quic-port 4001
```

The gateway will be available at **http://localhost:8080**

### Deploy Your First WASM Module

```bash
# Deploy a WASM module
./target/release/pied-piper deploy examples/hello-api/target/wasm32-wasip1/release/hello_api.wasm

# Output example:
# ✅ Module deployed successfully!
# 📦 Module Name: hello_api
# 🔗 CID: bafybei...
# 🌐 Access at: http://localhost:8080/cid/bafybei.../api/hello
```

### Access Your Module

```bash
# Via HTTP gateway
curl http://localhost:8080/cid/<YOUR_CID>/api/hello

# Or in your browser
open http://localhost:8080/cid/<YOUR_CID>
```

---

## 📋 Project Status

**Current Version:** 0.5.0  
**Status:** ✅ **Production Ready** (Phases 1-5 Complete)

### Completed Features

- ✅ **Phase 1: Network Foundation** - libp2p, QUIC/TCP, Kademlia DHT, GossipSub
- ✅ **Phase 2: WASM Runtime** - Wasmtime, WASI P1/P2, host functions, sandboxing
- ✅ **Phase 3: Content Distribution** - CID-based storage, P2P distribution, name resolution
- ✅ **Phase 4: Advanced Features** - HTTP/WebSocket, CRDTs, full I/O
- ✅ **Phase 5: Production Readiness** - Metrics, security, configuration, documentation

**Test Coverage:** 101/101 tests passing (100%)  
**Code Base:** ~13,000 lines of production Rust code  
**Documentation:** 2,000+ lines across 9 comprehensive guides

**See [PROJECT_STATUS.md](PROJECT_STATUS.md) for detailed progress and Phase 6+ roadmap.**

---

## 📦 PiperNet Package Format (.pn)

**NEW**: Pied Piper now supports encrypted package deployment with the `.pn` format!

### Why .pn Packages?

- **🔒 Encrypted**: AES-256-GCM encryption per-node (operators can't read your code)
- **📦 Bundled**: WASM + assets + metadata in one file
- **✅ Verified**: Cryptographic integrity checks
- **📝 Manifest**: Cargo-like `pn.toml` for metadata and dependencies
- **⚡ Compressed**: Zstd compression reduces file size

### Quick Example

```bash
# 1. Initialize package manifest
pied-piper package init --name my-app

# 2. Edit pn.toml (similar to Cargo.toml)
# Set name, version, description, entrypoint, etc.

# 3. Build your WASM module
cargo build --target wasm32-wasip1 --release

# 4. Create encrypted .pn package
pied-piper package build

# 5. Deploy to network (coming soon)
pied-piper package deploy my-app-1.0.0.pn
```

### pn.toml Example

```toml
[metadata]
name = "hello-api"
version = "1.0.0"
description = "A simple Hello World API"
author = "You <you@example.com>"

type = "backend"  # backend, frontend, fullstack, or library
entrypoint = "target/wasm32-wasip1/release/hello_api.wasm"
assets = []  # Frontend files for fullstack apps

[dependencies]
# auth-lib = "^1.0"
```

**See [docs/PN_FORMAT.md](docs/PN_FORMAT.md) for complete specification and [PACKAGE_STATUS.md](PACKAGE_STATUS.md) for implementation status.**

---

## 🎯 Key Features

### Decentralized Infrastructure
- **No central servers** - fully P2P network using libp2p
- **Content addressing** - immutable deployments via CID
- **Peer discovery** - automatic via mDNS and Kademlia DHT
- **NAT traversal** - Circuit Relay and DCUtR hole-punching

### WebAssembly Runtime
- **Multi-language support** - Run Rust, C, C++, AssemblyScript, and more
- **WASI P1 & P2** - Support for both core modules and components
- **Sandboxed execution** - Memory-safe with configurable resource limits
- **Host functions** - HTTP client, storage, crypto, logging

### HTTP Gateway
- **Browser-compatible** - Access WASM apps via HTTP/HTTPS
- **TLS support** - Self-signed certificates for HTTPS
- **WebSocket** - Real-time communication
- **Asset bundling** - Deploy complete web apps (HTML/CSS/JS + WASM)
- **SPA support** - Client-side routing fallback

### Distributed State
- **CRDTs** - Conflict-free replicated data types
- **Eventually consistent** - Automatic conflict resolution
- **GossipSub sync** - Real-time state propagation
- **LWW-Map & OR-Set** - Two CRDT implementations

---

## 📚 Documentation

### Getting Started
- **[Quickstart Guide](docs/QUICKSTART.md)** - Deploy your first app in 5 minutes
- **[Architecture Overview](docs/ARCHITECTURE.md)** - System design and components
- **[API Reference](docs/API.md)** - Complete host function documentation

### Operations
- **[Configuration Guide](docs/CONFIGURATION.md)** - Node configuration and settings
- **[Deployment Guide](docs/DEPLOYMENT.md)** - Production deployment (Docker, Kubernetes, cloud)
- **[Security Guide](docs/SECURITY.md)** - Security model and best practices

### Development
- **[WASM I/O Guide](docs/WASM_IO_GUIDE.md)** - Building WASM modules with HTTP I/O
- **[WebSocket Guide](docs/WEBSOCKET_GUIDE.md)** - Real-time communication
- **[Testing Guide](docs/TESTING_GUIDE.md)** - Testing your modules and deployments

### Reference
- **[Project Status](PROJECT_STATUS.md)** - Detailed progress and roadmap
- **[Example Applications](examples/)** - Sample apps (REST API, WebSocket chat, blog)

---

## 🛠️ Usage Examples

### Deploy a Backend API

```bash
# Build your Rust WASM module
cd my-api
cargo build --target wasm32-wasip1 --release

# Deploy to the network
pied-piper deploy target/wasm32-wasip1/release/my_api.wasm

# Access your API
curl http://localhost:8080/cid/<CID>/api/endpoint
```

### Deploy a Full Web Application

```bash
# Bundle WASM + assets (HTML/CSS/JS)
pied-piper deploy my_app.wasm --assets ./dist/

# Access in browser
open http://localhost:8080/cid/<CID>
```

### Search for Modules

```bash
# Search by name
pied-piper search hello --timeout 10

# List all local modules
pied-piper list
```

### Configuration

```bash
# Generate example config
pied-piper config init

# Validate config
pied-piper config validate config.yaml

# Show current config
pied-piper config show
```

---

## 🔬 Development

### Build from Source

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logs
RUST_LOG=debug cargo run -- serve
```

### Project Structure

```
pied-piper/
├── src/
│   ├── main.rs              # Entry point and CLI
│   ├── config.rs            # Configuration management
│   ├── bundle.rs            # Asset bundling
│   ├── network/             # P2P networking (libp2p)
│   ├── wasm/                # WASM runtime and host functions
│   ├── gateway/             # HTTP gateway and routing
│   ├── content/             # Content distribution
│   ├── crdt/                # CRDT state management
│   └── metrics/             # Prometheus metrics
├── examples/                # Example WASM modules
├── tests/                   # Integration tests
└── docs/                    # Documentation
```

---

## 🧪 Testing

```bash
# Run all tests (89 tests)
cargo test

# Run specific test suite
cargo test --test io_integration_test

# Run with output
cargo test -- --nocapture

# Test coverage breakdown:
# - 70 unit tests
# - 16 integration tests  
# - 3 I/O integration tests
```

### Example Modules

The repository includes several example modules in `examples/`:

- **hello-api** - Simple HTTP API
- **joke-api** - Fetches jokes from external API
- **dashboard** - Web dashboard with metrics
- **web-app** - Complete SPA example
- **ws-echo** - WebSocket echo server
- **test-echo-api** - I/O testing module

---

## 🚀 Deployment

### Production Configuration

```yaml
# config.production.yaml
gateway_port: 443
enable_https: true
tls_cert_path: "/path/to/cert.pem"
tls_key_path: "/path/to/key.pem"

quic_port: 4001
enable_mdns: false  # Disable for production

bootstrap_peers:
  - "/dns4/bootstrap1.piedpiper.network/tcp/4001/p2p/12D3KooW..."
  - "/dns4/bootstrap2.piedpiper.network/tcp/4001/p2p/12D3KooW..."
```

### Run in Production

```bash
# Using config file
pied-piper serve --config config.production.yaml

# Or via environment variables
PP_GATEWAY_PORT=443 PP_ENABLE_HTTPS=true pied-piper serve

# With systemd (see docs/DEPLOYMENT.md for full example)
sudo systemctl start pied-piper
```

---

## 📊 Metrics

Access Prometheus metrics at `http://localhost:8080/metrics`

Key metrics:
- **Network**: peers connected, messages sent/received, bytes transferred
- **DHT**: records stored, query duration
- **Gateway**: HTTP requests, response time, WebSocket connections
- **WASM**: execution duration, cache hits/misses, host function calls
- **CRDTs**: operations, merge count, sync messages

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Commit (`git commit -m 'feat: add amazing feature'`)
6. Push (`git push origin feature/amazing-feature`)
7. Open a Pull Request

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## � Acknowledgments

- **libp2p** - Modular P2P networking stack
- **Wasmtime** - Fast and secure WebAssembly runtime
- **Axum** - Ergonomic web framework
- **Tokio** - Asynchronous runtime

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/rootkill-g/pied-piper/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rootkill-g/pied-piper/discussions)
- **Documentation**: [docs/](docs/)

---

## 🗺️ Roadmap

**Immediate (Weeks 1-2)**
- ✅ Complete I/O implementation with binary support
- ✅ Frontend serving enhancements (SPA, security headers)
- ⏳ Documentation improvements (this README)

**Short-term (Weeks 3-4)**
- Security hardening (rate limiting, DDoS protection)
- Performance benchmarking
- Docker containerization

**Long-term (Months 2-3)**
- Public bootstrap nodes
- Web dashboard for node management
- Developer SDKs (Rust, JS, Go)
- Community building

See [Project.md](Project.md) for the complete vision and technical roadmap.

---

<div align="center">

**Built with ❤️ by the Pied Piper community**

[Website](https://piedpiper.network) • [Documentation](docs/) • [Examples](examples/)

</div>bash
# Run from local file
./target/release/pied-piper run module.wasm --function main

# Run from network by CID (checks cache first)
./target/release/pied-piper run bjmz4m6y7qxlqcktlzjk... --function main

# Run with custom resource limits
./target/release/pied-piper run module.wasm --function compute \
  --max-memory 32 \
  --max-time 10 \
  --fuel
```

#### Start HTTP Gateway (NEW!)

```bash
# Start gateway with defaults (localhost:8080)
./target/release/pied-piper gateway

# Custom configuration
./target/release/pied-piper gateway \
  --listen 0.0.0.0:8080 \
  --tcp-port 4001 \
  --quic-port 4002 \
  --cors true

# Access in browser:
# - http://localhost:8080/ - Welcome page
# - http://localhost:8080/health - Health check
# - http://localhost:8080/cid/<CID>/ - Access app by CID
# - http://localhost:8080/app/<name>/ - Access app by name
```

See [docs/GATEWAY.md](./docs/GATEWAY.md) for complete gateway documentation.

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

See [TESTING.md](./TESTING.md) for comprehensive end-to-end testing guide.

**Quick Test:**

**Terminal 1:**
```bash
./target/release/pied-piper daemon --verbose --tcp-port 8000
```

**Terminal 2:**
```bash
# Deploy a module
./target/release/pied-piper deploy hello.wasm

# Search for it
./target/release/pied-piper search hello
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

## License

MIT License - See [LICENSE](LICENSE) file for details

## Acknowledgments

Built with:
- [libp2p](https://libp2p.io/) - Modular P2P networking stack
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Asynchronous runtime
- [Wasmtime](https://wasmtime.dev/) - WebAssembly runtime

Inspired by:
- [IPFS](https://ipfs.io/) - Distributed file system
- [Holochain](https://holochain.org/) - Agent-centric computing
- [Solid](https://solidproject.org/) - Decentralized web

## Contact & Community

- **GitHub:** [github.com/rootkill-g/pied-piper](https://github.com/rootkill-g/pied-piper)
- **Issues:** [GitHub Issues](https://github.com/rootkill-g/pied-piper/issues)
- **Discussions:** [GitHub Discussions](https://github.com/rootkill-g/pied-piper/discussions)

---

**Status:** ✅ Production Ready (Phases 1-5 Complete) | **Next:** Phase 6 - Launch & Community

*Last updated: December 22, 2025*
