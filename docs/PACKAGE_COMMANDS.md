# PiperNet Package Commands - Quick Reference

## Overview

The `.pn` package format is PiperNet's standard for deploying encrypted, bundled applications. This guide provides quick command reference.

## Prerequisites

```bash
# Install Rust and wasm32-wasip1 target
rustup target add wasm32-wasip1

# Build Pied Piper
cargo build --release
```

## Command Reference

### 1. Initialize Package

Create a `pn.toml` manifest file:

```bash
pied-piper package init                    # In current directory
pied-piper package init --name my-app      # With custom name
pied-piper package init -t backend         # Specify type
pied-piper package init -t fullstack       # For web apps
pied-piper package init --force            # Overwrite existing
```

**Package Types:**
- `backend` - API/service (WASM only)
- `frontend` - Web app (HTML/CSS/JS + optional WASM)
- `fullstack` - Backend + frontend combined
- `library` - Reusable component

### 2. Edit Manifest

Edit the generated `pn.toml`:

```toml
[metadata]
name = "my-app"
version = "1.0.0"
description = "My awesome app"
author = "Your Name <you@example.com>"
license = "MIT"

type = "backend"
entrypoint = "target/wasm32-wasip1/release/my_app.wasm"

assets = [
    # "static/index.html",
    # "static/**/*.css",
]

[dependencies]
# other-package = "^1.0"
```

### 3. Build WASM Module

```bash
# For WASI P1 (core modules)
cargo build --target wasm32-wasip1 --release

# For WASI P2 (components)
cargo component build --release
```

### 4. Build Package

Create encrypted `.pn` package:

```bash
pied-piper package build                   # Uses pn.toml in current dir
pied-piper package build -m path/pn.toml   # Custom manifest
pied-piper package build -o output.pn      # Custom output name
```

**Default Output:** `<name>-<version>.pn` (e.g., `my-app-1.0.0.pn`)

**Encryption:** Uses node's peer ID derived key by default

**Custom Key:**
```bash
# Generate random 32-byte key (64 hex chars)
openssl rand -hex 32

# Build with custom key
pied-piper package build --key <64-char-hex-string>
```

### 5. Verify Package

Validate `.pn` file format:

```bash
pied-piper package verify my-app-1.0.0.pn     # Basic check
pied-piper package verify my-app-1.0.0.pn -v  # Verbose output
```

**Checks:**
- Magic bytes (`PN\x01\x00`)
- File size and structure
- Format version

### 6. Extract Package

Decrypt and extract contents:

```bash
pied-piper package extract my-app.pn           # Uses node's key
pied-piper package extract my-app.pn -o ./out  # Custom output
pied-piper package extract my-app.pn --key <HEX>  # Custom key
```

**Extracted Files:**
- `pn.toml` - Manifest
- `module.wasm` - Main WASM module
- `<assets>/*` - Frontend files
- `dependencies/*` - Dependency modules

### 7. Deploy Package

**Status:** ⚠️ Not yet fully implemented (requires gateway .pn support)

```bash
pied-piper package deploy my-app.pn           # Coming soon
pied-piper package deploy my-app.pn --name custom-name
```

**Current Workaround:**
```bash
# Extract and deploy manually
pied-piper package extract my-app.pn
pied-piper deploy extracted/module.wasm --name my-app
```

## Complete Workflow Example

### Backend API

```bash
# 1. Create new Rust project
cargo new --lib my-api
cd my-api

# 2. Add dependencies (Cargo.toml)
# [dependencies]
# serde = { version = "1.0", features = ["derive"] }
# serde_json = "1.0"

# 3. Write your API (src/lib.rs)
# ... your code ...

# 4. Initialize package
pied-piper package init --name my-api

# 5. Edit pn.toml
# Set entrypoint: target/wasm32-wasip1/release/my_api.wasm

# 6. Build WASM
cargo build --target wasm32-wasip1 --release

# 7. Build package
pied-piper package build

# 8. Verify
pied-piper package verify my-api-1.0.0.pn -v

# 9. Deploy (when ready)
# pied-piper package deploy my-api-1.0.0.pn
```

### Fullstack Web App

```bash
# 1. Create project structure
mkdir my-webapp && cd my-webapp
mkdir frontend backend

# 2. Build backend WASM
cd backend
cargo new --lib api
# ... develop API ...
cargo build --target wasm32-wasip1 --release

# 3. Build frontend
cd ../frontend
# ... create HTML/CSS/JS ...

# 4. Initialize package (from root)
cd ..
pied-piper package init --name my-webapp -t fullstack

# 5. Edit pn.toml
# [metadata]
# type = "fullstack"
# entrypoint = "backend/target/wasm32-wasip1/release/api.wasm"
# assets = [
#     "frontend/index.html",
#     "frontend/**/*.css",
#     "frontend/**/*.js",
# ]

# 6. Build package
pied-piper package build

# 7. Deploy
# pied-piper package deploy my-webapp-1.0.0.pn
```

## Encryption Details

### Key Derivation

By default, packages are encrypted with a key derived from the node's peer ID:

```
key = SHA256("pipernet-encryption-v1:" + peer_id)
```

**Properties:**
- Deterministic: Same peer ID = same key
- Per-node: Each node has unique key
- Secure: 256-bit key strength
- No storage: Key derived on-demand

### Custom Keys

Provide your own 32-byte (256-bit) encryption key:

```bash
# Generate key
KEY=$(openssl rand -hex 32)
echo "Key: $KEY"

# Build with key
pied-piper package build --key $KEY

# Extract with same key
pied-piper package extract my-app.pn --key $KEY
```

**Use Cases:**
- Sharing packages between specific nodes
- Testing/development with known keys
- External key management systems
- Rotating keys for security

### Security Notes

⚠️ **Important:**
- Keys are never stored in package files
- Each node must have correct key to decrypt
- Packages encrypted with different keys are incompatible
- Lost keys = unrecoverable packages

✅ **Best Practices:**
- Use default node-derived keys for production
- Only use custom keys for specific use cases
- Store custom keys securely (password manager, key vault)
- Never commit keys to version control

## Troubleshooting

### "Manifest file not found"

```bash
# Ensure pn.toml exists
ls -la pn.toml

# Or specify path
pied-piper package build -m path/to/pn.toml
```

### "Module not found" during build

```bash
# Check entrypoint path in pn.toml
cat pn.toml | grep entrypoint

# Verify WASM file exists
ls -la target/wasm32-wasip1/release/*.wasm

# Rebuild WASM
cargo build --target wasm32-wasip1 --release
```

### "Invalid package: incorrect magic bytes"

File is not a valid `.pn` package:
```bash
# Check file type
file my-app.pn

# Should show: data (binary)
# If shows: ASCII text, it's not a .pn file

# Rebuild package
pied-piper package build
```

### "Decryption failed"

Wrong encryption key:
```bash
# If using custom key, verify it matches build key
# If using default, ensure same node/peer ID

# Extract with correct key
pied-piper package extract my-app.pn --key <correct-key>
```

### Build warnings about missing assets

Assets in pn.toml don't exist:
```bash
# Check asset paths
cat pn.toml | grep -A 5 "assets ="

# Verify files exist
ls -la static/  # or your asset directory

# Update pn.toml or create missing files
```

## Advanced Usage

### Building Multiple Packages

```bash
# Use different manifests
pied-piper package build -m backend/pn.toml -o backend.pn
pied-piper package build -m frontend/pn.toml -o frontend.pn
```

### Scripted Builds

```bash
#!/bin/bash
# build-and-verify.sh

set -e

echo "Building WASM..."
cargo build --target wasm32-wasip1 --release

echo "Building package..."
pied-piper package build

PKG=$(ls *.pn | head -1)
echo "Verifying $PKG..."
pied-piper package verify "$PKG" -v

echo "✅ Package ready: $PKG"
```

### CI/CD Integration

```yaml
# .github/workflows/build.yml
name: Build Package

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        run: |
          rustup target add wasm32-wasip1
      
      - name: Build WASM
        run: cargo build --target wasm32-wasip1 --release
      
      - name: Build Package
        run: pied-piper package build
      
      - name: Upload Artifact
        uses: actions/upload-artifact@v3
        with:
          name: package
          path: "*.pn"
```

## See Also

- [PN_FORMAT.md](../docs/PN_FORMAT.md) - Complete format specification
- [PACKAGE_STATUS.md](../PACKAGE_STATUS.md) - Implementation status
- [README.md](../README.md) - Project overview
- [DEPLOYMENT.md](../docs/DEPLOYMENT.md) - Deployment guide

## Quick Links

| Command | Description | Status |
|---------|-------------|--------|
| `package init` | Create pn.toml | ✅ Ready |
| `package build` | Build .pn package | ✅ Ready |
| `package verify` | Validate package | ✅ Ready |
| `package extract` | Extract contents | ✅ Ready |
| `package deploy` | Deploy to network | ⚠️ Coming soon |

---

**Need Help?** Run any command with `--help` for detailed usage:
```bash
pied-piper package --help
pied-piper package build --help
```
