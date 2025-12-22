# PiperNet Package Format (.pn)

## Overview

The `.pn` (PiperNet Package) format is a secure, encrypted package format for deploying applications to the PiperNet decentralized network. It provides:

- **Encryption**: AES-256-GCM encryption ensures deployed apps are unreadable even with filesystem access
- **Compression**: Zstd compression reduces package size
- **Bundling**: Combines WASM modules, assets, and metadata in a single file
- **Integrity**: SHA-256 checksums verify package authenticity
- **Metadata**: Manifest-based system similar to Cargo.toml

## Why .pn Format?

### Security
- **Node-level encryption**: Each node encrypts packages with keys derived from its peer ID
- **Content obfuscation**: Even if someone gains filesystem access to a node, they cannot read deployed applications
- **Tamper detection**: Cryptographic signatures ensure integrity

### Convenience
- **Single file deployment**: Upload one `.pn` file instead of multiple WASM/asset files
- **Dependency management**: Automatically bundles and resolves dependencies
- **Metadata**: Built-in versioning, author, license information

### Network Standard
- **Mandatory format**: All PiperNet deployments must use `.pn` packages
- **Interoperability**: Standard format ensures compatibility across nodes
- **Discovery**: Metadata enables efficient package search and indexing

## Package Structure

A `.pn` file is a binary format with the following structure:

```
┌─────────────────────────────────┐
│   Magic Bytes (4 bytes)         │  "PN\x01\x00"
├─────────────────────────────────┤
│   Encrypted + Compressed Data   │
│   ┌──────────────────────────┐  │
│   │ pn.toml (manifest)       │  │
│   │ module.wasm (encrypted)  │  │
│   │ assets/* (encrypted)     │  │
│   │ dependencies/* (encr.)   │  │
│   │ signature                │  │
│   └──────────────────────────┘  │
└─────────────────────────────────┘
```

## Manifest File (pn.toml)

Every `.pn` package contains a `pn.toml` manifest file that describes the package:

```toml
# PiperNet Package Manifest

[metadata]
name = "hello-api"
version = "1.0.0"
description = "A simple Hello World API"
author = "Your Name <your.email@example.com>"
license = "MIT"
homepage = "https://example.com/hello-api"
repository = "https://github.com/user/hello-api"

# Package type: backend, frontend, fullstack, or library
type = "backend"

# Main WASM module to execute
entrypoint = "target/wasm32-wasip1/release/hello-api.wasm"

# Frontend assets (for fullstack apps)
assets = [
    "static/index.html",
    "static/style.css",
    "static/app.js",
]

# Dependencies (other PiperNet packages)
[dependencies]
# package-name = "version-requirement"
# auth-lib = "^1.0"
# database = "0.5.2"
```

### Metadata Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Package name (alphanumeric, hyphens, underscores) |
| `version` | Yes | Semantic version (e.g., "1.0.0") |
| `description` | No | Short description of the package |
| `author` | No | Author name and email |
| `license` | No | License identifier (MIT, Apache-2.0, etc.) |
| `homepage` | No | Project homepage URL |
| `repository` | No | Source code repository URL |

### Package Types

- **`backend`**: WASM API module only
- **`frontend`**: Static web assets (HTML/CSS/JS) with optional WASM
- **`fullstack`**: Both backend WASM and frontend assets
- **`library`**: Reusable WASM component for other packages

## Creating a Package

### Step 1: Create pn.toml

Create a `pn.toml` file in your project root:

```bash
cd my-project
cat > pn.toml << 'EOF'
[metadata]
name = "my-app"
version = "1.0.0"
description = "My awesome app"
author = "Me <me@example.com>"

type = "backend"
entrypoint = "target/wasm32-wasip1/release/my_app.wasm"
assets = []
[dependencies]
EOF
```

### Step 2: Build Your WASM Module

```bash
# For WASI P1 backend
cargo build --target wasm32-wasip1 --release

# For WASI P2 component
cargo component build --release
```

### Step 3: Package It

```bash
# Build .pn package (future command)
pied-piper package build

# This will create: my-app-1.0.0.pn
```

### Step 4: Deploy

```bash
# Deploy to network
pied-piper deploy my-app-1.0.0.pn

# Output:
# ✅ Package deployed successfully!
# 📦 Package: my-app v1.0.0
# 🔗 CID: bafybei...
# 🌐 Access at: http://localhost:8080/app/my-app
```

## Security Model

### Encryption

Each PiperNet node derives a unique encryption key from its peer ID:

```rust
fn derive_key(peer_id: &str) -> [u8; 32] {
    SHA256("pipernet-encryption-v1:" + peer_id)
}
```

This means:
- **Per-node encryption**: Each node encrypts packages with its own key
- **Filesystem security**: Stolen disk/files are unreadable without the node's identity
- **No shared keys**: Nodes don't share encryption keys

### Decryption Flow

1. Node receives `.pn` package
2. Verifies magic bytes and format
3. Decrypts using node's derived key
4. Decompresses with Zstd
5. Validates signature
6. Executes WASM module

### What's Encrypted?

- ✅ WASM module bytecode
- ✅ Frontend assets (HTML, CSS, JS)
- ✅ Dependency modules
- ❌ Manifest metadata (needed for discovery)
- ❌ Package CID (content identifier)

## Examples

### Backend API

```toml
[metadata]
name = "todo-api"
version = "2.1.0"
type = "backend"
entrypoint = "target/wasm32-wasip1/release/todo_api.wasm"
```

### Frontend Web App

```toml
[metadata]
name = "todo-ui"
version = "1.5.0"
type = "frontend"
entrypoint = "build/index.html"
assets = [
    "build/index.html",
    "build/styles.css",
    "build/app.js",
    "build/assets/*",
]
```

### Full-Stack App

```toml
[metadata]
name = "blog"
version = "3.0.0"
type = "fullstack"
entrypoint = "target/wasm32-wasip1/release/blog_api.wasm"
assets = [
    "frontend/dist/index.html",
    "frontend/dist/**/*.css",
    "frontend/dist/**/*.js",
]

[dependencies]
markdown-parser = "^1.0"
auth-lib = "0.5.2"
```

### Library Component

```toml
[metadata]
name = "auth-lib"
version = "0.5.2"
type = "library"
entrypoint = "target/wasm32-wasip2/release/auth.wasm"
```

## CLI Commands (Coming Soon)

```bash
# Initialize new package
pied-piper package init

# Build .pn package
pied-piper package build

# Validate package
pied-piper package verify my-app.pn

# Extract package contents (with node key)
pied-piper package extract my-app.pn --output ./extracted

# Deploy package
pied-piper deploy my-app.pn

# Search for packages
pied-piper search auth

# Install package as dependency
pied-piper package add auth-lib@^1.0
```

## Migration from Current Format

Current deployment workflow:
```bash
# Old way
pied-piper deploy \
  --name hello-api \
  --version 1.0.0 \
  --file hello-api.wasm
```

New `.pn` workflow:
```bash
# New way
pied-piper package build    # Creates hello-api-1.0.0.pn
pied-piper deploy hello-api-1.0.0.pn
```

## Benefits

### For Developers
- **Simpler deployment**: One file instead of multiple parameters
- **Version management**: Built-in semantic versioning
- **Dependency resolution**: Automatic handling of dependencies
- **Reproducible builds**: Locked versions and hashes

### For Node Operators
- **Security**: Encrypted storage prevents content theft
- **Efficiency**: Compressed packages save bandwidth and storage
- **Integrity**: Signatures prevent tampering
- **Privacy**: Operators can't read deployed content

### For Users
- **Reliability**: Cryptographic verification ensures authenticity
- **Performance**: Optimized compression reduces load times
- **Discovery**: Rich metadata enables search and filtering
- **Trust**: Signed packages from verified authors

## Technical Specifications

### Encryption
- **Algorithm**: AES-256-GCM (Galois/Counter Mode)
- **Key Derivation**: SHA-256 of peer ID
- **Nonce**: 96-bit random (12 bytes)
- **Tag**: 128-bit authentication tag (16 bytes)

### Compression
- **Algorithm**: Zstd (Zstandard)
- **Level**: 3 (balanced speed/compression)
- **Typical Ratio**: 3-5x for WASM, 10-15x for text assets

### Integrity
- **Hash**: SHA-256
- **Signature**: Ed25519 (future)
- **Verification**: On package load and execution

### File Format
```
Offset   Size   Description
------   ----   -----------
0        4      Magic bytes: 'P' 'N' 0x01 0x00
4        12     Nonce for AES-GCM
16       N      Encrypted + compressed payload
N+16     16     Authentication tag
```

## Future Enhancements

- [ ] Digital signatures with Ed25519
- [ ] Multi-node replication awareness
- [ ] Differential updates (delta packages)
- [ ] Package registry/index
- [ ] Dependency caching and CDN
- [ ] Package mirroring
- [ ] Access control and permissions
- [ ] Package marketplace

## See Also

- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
- [SECURITY.md](SECURITY.md) - Security architecture
- [API.md](API.md) - HTTP/WebSocket API reference
- [EXAMPLES.md](examples/README.md) - Example applications
