# Pied Piper Architecture

This document provides a comprehensive overview of Pied Piper's architecture, design decisions, and internal workings.

## Table of Contents

- [System Overview](#system-overview)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [P2P Networking](#p2p-networking)
- [WASM Runtime](#wasm-runtime)
- [Content Addressing](#content-addressing)
- [Gateway & HTTP](#gateway--http)
- [Security Model](#security-model)
- [Performance & Scalability](#performance--scalability)
- [Design Decisions](#design-decisions)

## System Overview

Pied Piper is a decentralized platform for running WebAssembly applications over a P2P network. It combines:

1. **libp2p networking** for decentralized communication
2. **WebAssembly runtime** (Wasmtime) for safe code execution
3. **Content-addressed storage** (Blake3) for deterministic module identification
4. **HTTP gateway** (Axum) for web access to P2P content
5. **Kademlia DHT** for distributed service discovery

```
┌─────────────────────────────────────────────────────────────┐
│                      Pied Piper Node                         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐     ┌────────────┐ │
│  │   HTTP/WS    │─────▶│    Gateway   │────▶│   WASM     │ │
│  │   Gateway    │      │   Handler    │     │  Runtime   │ │
│  └──────────────┘      └──────────────┘     └────────────┘ │
│         │                      │                     │       │
│         │                      ▼                     │       │
│         │              ┌──────────────┐             │       │
│         │              │   Module     │             │       │
│         │              │   Loader     │◀────────────┘       │
│         │              └──────────────┘                     │
│         │                      │                             │
│         │                      ▼                             │
│         │              ┌──────────────┐                     │
│         └─────────────▶│   Content    │                     │
│                        │   Provider   │                     │
│                        └──────────────┘                     │
│                                │                             │
│                                ▼                             │
│              ┌───────────────────────────────┐             │
│              │        P2P Network Layer       │             │
│              │  (libp2p: DHT, GossipSub,     │             │
│              │   Circuit Relay, QUIC/TCP)    │             │
│              └───────────────────────────────┘             │
│                                                               │
└─────────────────────────────────────────────────────────────┘
                          │         ▲
                          │         │
                          ▼         │
              ┌───────────────────────────────┐
              │      Other Pied Piper Nodes    │
              │    (Peer Discovery, Module     │
              │     Sharing, DHT Records)      │
              └───────────────────────────────┘
```

### Key Characteristics

- **Decentralized**: No single point of failure or central authority
- **Content-Addressed**: Modules identified by cryptographic hash (CID)
- **Sandboxed**: WASM provides memory safety and isolation
- **Portable**: WASM modules run anywhere without recompilation
- **Scalable**: P2P architecture scales horizontally

## Core Components

### 1. Network Layer (`src/network/`)

**Responsibilities:**
- P2P connectivity and peer discovery
- DHT-based content routing
- Pub/sub messaging (GossipSub)
- NAT traversal (Circuit Relay, DCUtR)

**Key Files:**
- `node.rs` - NetworkNode manages swarm and handles commands
- `behaviour.rs` - libp2p behavior combining DHT, GossipSub, etc.
- `transport.rs` - QUIC and TCP transport configuration
- `command.rs` - Command pattern for async network operations

**Technologies:**
- **libp2p 0.56**: Modular P2P networking framework
- **QUIC**: Fast, encrypted UDP-based transport
- **Kademlia DHT**: Distributed hash table for content routing
- **GossipSub**: Publish-subscribe messaging
- **mDNS**: Local peer discovery

### 2. WASM Runtime (`src/wasm/`)

**Responsibilities:**
- Module loading and caching
- WASM execution with WASI support
- Host function implementation
- Resource limits and sandboxing

**Key Files:**
- `runtime.rs` - WasmRuntime executes modules
- `loader.rs` - ModuleLoader with LRU cache
- `host.rs` - Host functions (HTTP, storage, crypto)
- `sandbox.rs` - Execution limits and fuel management

**Technologies:**
- **Wasmtime 39.0.1**: Fast WASM runtime from Bytecode Alliance
- **WASI P1/P2**: WebAssembly System Interface
- **Component Model**: Advanced module composition

### 3. Gateway (`src/gateway/`)

**Responsibilities:**
- HTTP/HTTPS server
- WebSocket connections
- Request routing and handling
- Asset serving (HTML, CSS, JS)

**Key Files:**
- `server.rs` - Axum HTTP server
- `handler.rs` - Request processing and WASM execution
- `io.rs` - Request/response serialization
- `websocket.rs` - WebSocket protocol handler
- `tls.rs` - TLS/HTTPS configuration

**Technologies:**
- **Axum 0.7**: Modern async web framework
- **Tower**: Middleware and service abstraction
- **Rustls**: Pure-Rust TLS implementation

### 4. Content System (`src/content/`)

**Responsibilities:**
- Content-addressed storage
- Module publishing and discovery
- Provider advertisement
- Content retrieval

**Key Files:**
- `provider.rs` - Advertises available modules
- `publisher.rs` - Publishes modules to DHT
- `discovery.rs` - Discovers modules by name/CID
- `protocol.rs` - Content exchange protocol

### 5. Security (`src/security/`)

**Responsibilities:**
- Rate limiting (token bucket)
- Request validation
- DDoS protection
- Connection tracking

**Key Files:**
- `mod.rs` - SecurityMiddleware and validators

### 6. Configuration (`src/config.rs`)

**Responsibilities:**
- YAML/TOML/JSON config loading
- Environment variable overrides
- Default values and validation

## Data Flow

### HTTP Request to WASM Execution

```
1. HTTP Request arrives
   ↓
2. Gateway receives request
   ├─ Security middleware (rate limit, validation)
   ├─ Route matching (/app/name or /cid/xyz)
   └─ Extract identifier
   ↓
3. Content Resolution
   ├─ Check local cache (ModuleLoader)
   ├─ If not cached: query DHT for providers
   ├─ Fetch from peer(s)
   └─ Verify CID (Blake3 hash)
   ↓
4. WASM Execution
   ├─ Load module into Wasmtime
   ├─ Create WasiState (stdin/stdout/stderr)
   ├─ Serialize request → JSON
   ├─ Write to WASM stdin
   ├─ Execute _start() function
   ├─ Read from WASM stdout
   └─ Deserialize JSON → response
   ↓
5. HTTP Response
   ├─ Apply security headers
   ├─ Set caching headers (ETag)
   └─ Send to client
```

### Module Publication Flow

```
1. User runs: pied-piper deploy module.wasm --name myapp
   ↓
2. CLI reads WASM file
   ↓
3. Compute CID (Blake3 hash of contents)
   ↓
4. If assets provided:
   ├─ Create AppBundle (WASM + assets)
   ├─ Serialize to bytes
   └─ Compute bundle CID
   ↓
5. Store in local cache
   ↓
6. Publish to DHT:
   ├─ Put record: /pied-piper/name/myapp → CID
   ├─ Advertise as provider for CID
   └─ Wait for DHT confirmation
   ↓
7. Module now discoverable by name or CID
```

### Peer Discovery Flow

```
1. Node starts
   ↓
2. Listen on TCP and QUIC ports
   ↓
3. mDNS discovery (local network)
   ├─ Broadcast presence
   └─ Receive peer announcements
   ↓
4. DHT bootstrap
   ├─ Connect to bootstrap peers
   ├─ Join Kademlia DHT
   └─ Populate routing table
   ↓
5. Ongoing discovery
   ├─ Random DHT walks
   ├─ Provider lookups
   └─ Gossip subscriptions
```

## P2P Networking

### libp2p Stack

Pied Piper uses a carefully configured libp2p stack:

**Transport Layer:**
```rust
QUIC (primary)
  └─ Encrypted by default (TLS 1.3)
  └─ Multiplexed streams
  └─ 0-RTT connection resumption

TCP (fallback)
  └─ Noise protocol encryption
  └─ Yamux stream multiplexing
```

**Network Behavior:**
```rust
PiedPiperBehaviour = 
  Kademlia (DHT)
    + GossipSub (Pub/Sub)
    + mDNS (Local discovery)
    + Identify (Peer info exchange)
    + Ping (Keep-alive)
    + RelayClient (NAT traversal)
    + Dcutr (Direct connection upgrade)
```

### Kademlia DHT

**Purpose**: Distributed key-value store for module discovery

**Key Space**: `/pied-piper/name/<app-name>` → CID

**Operations:**
- `PUT_VALUE`: Publish module by name
- `GET_VALUE`: Resolve name to CID
- `ADD_PROVIDER`: Advertise module availability
- `GET_PROVIDERS`: Find who has a module

**Persistence**: DHT state saved to disk for fast restarts

### GossipSub

**Purpose**: Efficient pub/sub messaging for:
- Module announcements
- Peer updates
- Future: CRDT synchronization

**Topics:**
- `/pied-piper/announcements` - Module publications
- `/pied-piper/crdt-sync` - State synchronization

### Circuit Relay & DCUtR

**Problem**: NAT/firewall prevents direct connections

**Solution**:
1. **Circuit Relay**: Relay traffic through a public peer
2. **DCUtR** (Direct Connection Upgrade through Relay):
   - Use relay to coordinate
   - Establish direct connection via hole-punching
   - Upgrade from relayed to direct

## WASM Runtime

### Wasmtime Configuration

```rust
Config:
  - Async support: Yes (Tokio integration)
  - Component model: Yes (WASI P2)
  - Fuel: Yes (execution limits)
  - Memory: Max 64MB per module (configurable)
  - Epoch interruption: Yes (timeout protection)
```

### Execution Model

**1. Module Loading:**
```
WASM bytes → Wasmtime::Module
  ├─ Validate (structure, types)
  ├─ Compile (Cranelift JIT)
  └─ Cache (on disk and in-memory LRU)
```

**2. Instance Creation:**
```
Module + Store<WasiState> → Instance
  ├─ Link WASI functions
  ├─ Link host functions
  └─ Initialize memory
```

**3. Execution:**
```
Call _start():
  ├─ Set fuel limit
  ├─ Capture stdin/stdout
  ├─ Execute with timeout
  ├─ Consume fuel
  └─ Return or trap
```

### WASI Support

**WASI P1 (Preview 1):**
- File system (virtualized, sandboxed)
- Environment variables
- Command-line arguments
- Standard I/O (stdin, stdout, stderr)
- Clock functions
- Random number generation

**WASI P2 (Component Model):**
- Structured interfaces (WIT)
- Type-safe imports/exports
- Async operations
- Resource handles

### Host Functions

Host functions bridge WASM sandbox to real world:

```rust
Logging:
  - host_log(message) → Gateway logs

Time:
  - host_now_millis() → Unix timestamp

Random:
  - host_random_u32() → Crypto-secure RNG

HTTP:
  - http_get(url) → (status, body)
  - http_post(url, body) → (status, response)

Storage:
  - storage_get(key) → value
  - storage_set(key, value)
  - storage_delete(key)

Crypto:
  - crypto_blake3(data) → hash
  - crypto_sha256(data) → hash
```

## Content Addressing

### CID Generation

```rust
fn compute_cid(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    let multihash = Multihash::wrap(
        BLAKE3_CODE,
        hash.as_bytes()
    ).unwrap();
    
    let cid = Cid::new_v1(
        RAW_CODEC,
        multihash
    );
    
    cid.to_string()
}
```

**Result**: `bafkreiabcdef...` (base32-encoded CID v1)

### Module Cache

**In-Memory (LRU):**
```rust
LruCache<ModuleCid, Arc<Vec<u8>>>
  - Capacity: 256 modules (configurable)
  - Eviction: Least recently used
  - Thread-safe: Arc<RwLock<...>>
```

**On-Disk:**
```
.pied-piper/modules/
  ├─ <cid1>.wasm
  ├─ <cid2>.wasm
  └─ <cid3>.bundle
```

**Lookup Order:**
1. In-memory cache (fast)
2. Disk cache (medium)
3. P2P network (slow)

## Gateway & HTTP

### Request Routing

```
Incoming request → Match pattern:

/health            → Health check
/ready             → Readiness probe
/metrics           → Prometheus metrics
/info              → Node information

/app/<name>        → Resolve name → Execute WASM
/app/<name>/*path  → Route to WASM with path

/cid/<cid>         → Fetch by CID → Execute WASM
/cid/<cid>/*path   → Route to WASM with path

/ws/app/<name>     → WebSocket connection
/ws/cid/<cid>      → WebSocket by CID

/*                 → 404 Not Found
```

### Asset Serving

**SPA Fallback:**
```
Request: /my-app/dashboard
  ├─ Check if file exists → No
  ├─ Path has no extension → Yes
  └─ Serve index.html (SPA route)
```

**Caching Strategy:**
```
HTML files:
  - Cache-Control: max-age=3600 (1 hour)
  - ETag: Blake3 hash of content

Static assets (.js, .css, .wasm):
  - Cache-Control: max-age=31536000, immutable
  - ETag: Blake3 hash
```

### WebSocket Protocol

```
1. Client connects: ws://host/ws/app/chat
   ↓
2. Upgrade to WebSocket
   ↓
3. For each message:
   ├─ Client → JSON message
   ├─ Execute WASM handler
   ├─ WASM → JSON response
   └─ Response → Client
   ↓
4. Connection persists for bidirectional communication
```

## Security Model

### Defense Layers

**1. Network Layer:**
- Encrypted transports (QUIC TLS 1.3, Noise protocol)
- Peer authentication (libp2p PeerID)
- Connection limits per peer

**2. Gateway Layer:**
- Rate limiting (token bucket algorithm)
- Request validation (path traversal, injection)
- DDoS protection (connection limits, timeouts)
- Security headers (CSP, HSTS, X-Frame-Options)

**3. WASM Layer:**
- Memory isolation (linear memory sandbox)
- No direct system access
- Fuel limits (execution timeout)
- WASI capabilities (limited syscalls)

### Threat Mitigation

| Threat | Mitigation |
|--------|------------|
| **Code Injection** | WASM sandboxing, no eval() |
| **Path Traversal** | Path validation, no `../` allowed |
| **DDoS** | Rate limiting, connection limits, timeouts |
| **XSS** | Content-Security-Policy header |
| **CSRF** | SameSite cookies, CORS |
| **Module Tampering** | Content addressing, CID verification |
| **Resource Exhaustion** | Fuel limits, memory limits, timeouts |
| **Network Sniffing** | Encrypted transports (QUIC/Noise) |

## Performance & Scalability

### Optimization Techniques

**1. Connection Pooling:**
```rust
HTTP Client:
  - Pool size: 10 per host
  - Keep-alive: 90 seconds
  - TCP keep-alive: 60 seconds
```

**2. Caching:**
```
Module Cache:
  - In-memory LRU: 256 modules
  - Disk cache: Unlimited
  - Asset caching: 1 year for immutable files
```

**3. Compression:**
```
HTTP Responses:
  - Brotli (preferred)
  - Gzip (fallback)
  - Applied to: HTML, CSS, JS, JSON
```

**4. Async Everywhere:**
```
Tokio Runtime:
  - Multi-threaded scheduler
  - Work-stealing task executor
  - Non-blocking I/O
```

### Scalability Characteristics

**Horizontal Scaling:**
- Add more nodes → More capacity
- DHT distributes load automatically
- No central bottleneck

**Vertical Scaling:**
- More CPU → More concurrent WASM executions
- More RAM → Larger module cache
- Faster disk → Better disk cache performance

**Limitations:**
- Single node: ~10K req/sec (depends on WASM complexity)
- Network: DHT queries add ~100-500ms latency
- Storage: In-memory only (for now)

## Design Decisions

### Why libp2p?

**Pros:**
- ✅ Battle-tested (IPFS, Filecoin, Polkadot)
- ✅ Modular (pick protocols you need)
- ✅ NAT traversal built-in
- ✅ Multiple transports (QUIC, TCP, WebRTC)
- ✅ Rust implementation

**Alternatives Considered:**
- Custom P2P stack → Too complex, reinventing wheel
- Tor/I2P → Too slow for web apps
- WebRTC only → Browser-centric, limited ecosystem

### Why Wasmtime?

**Pros:**
- ✅ Fast (Cranelift JIT)
- ✅ Secure (sandbox by design)
- ✅ WASI support
- ✅ Component model
- ✅ Production-ready (Cloudflare, Fermyon)

**Alternatives Considered:**
- wasmer → Less mature component model support
- wasm3 interpreter → Slower execution
- Native code → No sandboxing, security risk

### Why Blake3?

**Pros:**
- ✅ Extremely fast (parallelizable)
- ✅ Cryptographically secure
- ✅ 32-byte output (good for CIDs)
- ✅ Tree structure (enables streaming)

**Alternatives Considered:**
- SHA-256 → Slower, less parallelizable
- xxHash → Fast but not cryptographic
- SHA-3 → Slower than Blake3

### Why Axum?

**Pros:**
- ✅ Type-safe extractors
- ✅ Tower middleware ecosystem
- ✅ Excellent async performance
- ✅ Great ergonomics

**Alternatives Considered:**
- actix-web → Older, less type-safe
- warp → More complex error handling
- hyper directly → Too low-level

### Why JSON for I/O?

**Pros:**
- ✅ Human-readable (easier debugging)
- ✅ Universal (every language has JSON)
- ✅ Self-describing structure
- ✅ Extensible (add fields without breaking)

**Alternatives Considered:**
- Protobuf → Requires schema distribution
- MessagePack → Binary but less common
- CBOR → Good but less tooling support

## Future Architecture

### Planned Enhancements

**1. Persistent Storage:**
- SQLite backend
- Distributed storage (DHT-based)
- Replication and backup

**2. CRDT Synchronization:**
- Distributed state management
- Conflict-free replicated data types
- Real-time collaboration

**3. Advanced Networking:**
- WebRTC support (browser nodes)
- DHT improvements (performance)
- Bootstrap node network

**4. WASM Enhancements:**
- Streaming compilation
- Module linking (WASM imports)
- Shared-nothing parallelism

**5. Observability:**
- Distributed tracing
- Better metrics granularity
- Performance profiling

### Research Areas

- Zero-knowledge proofs for privacy
- Verifiable computation
- Federated learning
- Smart contract integration
- Cross-chain bridges

## References

- **libp2p**: https://libp2p.io
- **Wasmtime**: https://wasmtime.dev
- **WASI**: https://wasi.dev
- **Blake3**: https://github.com/BLAKE3-team/BLAKE3
- **Kademlia**: https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf
- **IPFS**: https://ipfs.tech

---

**Last Updated:** December 22, 2025  
**Version:** 0.5.0
