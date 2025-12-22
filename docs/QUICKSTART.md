# Pied Piper Quickstart Guide

Get up and running with Pied Piper in 5 minutes! This guide walks you through building and deploying your first decentralized WebAssembly application.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Your First Module](#your-first-module)
- [Deploy to P2P Network](#deploy-to-p2p-network)
- [Add Asset Bundling](#add-asset-bundling)
- [Use Host Functions](#use-host-functions)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)
- [Next Steps](#next-steps)

## Prerequisites

**Required:**
- Rust 1.75+ ([install](https://rustup.rs))
- Git

**Optional (for specific features):**
- `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`
- Docker (for containerized deployment)

**Check your setup:**
```bash
rustc --version   # Should be 1.75 or higher
cargo --version
```

## Installation

### Option 1: Build from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/rootkill-g/pied-piper
cd pied-piper

# Build in release mode
cargo build --release

# Binary will be at target/release/pied-piper
./target/release/pied-piper --version
```

### Option 2: Install to PATH

```bash
# After building, copy to your PATH
sudo cp target/release/pied-piper /usr/local/bin/

# Now you can run from anywhere
pied-piper --version
```

### Option 3: Pre-built Binary

```bash
# Download latest release (Linux x86_64 example)
curl -L https://github.com/rootkill-g/pied-piper/releases/latest/download/pied-piper-linux-x86_64.tar.gz | tar xz
sudo mv pied-piper /usr/local/bin/
```

## Your First Module

Let's create a simple "Hello World" WASM module.

### Step 1: Create a New Rust Project

```bash
mkdir hello-pied-piper
cd hello-pied-piper
cargo init --lib
```

### Step 2: Configure for WASM

Edit `Cargo.toml`:
```toml
[package]
name = "hello-pied-piper"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Step 3: Write Your Module

Edit `src/lib.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Deserialize)]
struct Request {
    method: String,
    path: String,
}

#[derive(Serialize)]
struct Response {
    status: u16,
    body: String,
}

#[no_mangle]
pub extern "C" fn _start() {
    // Read request from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    
    let request: Request = serde_json::from_str(&input).unwrap();
    
    // Create response
    let response = Response {
        status: 200,
        body: format!("Hello from Pied Piper! You requested: {}", request.path),
    };
    
    // Write response to stdout
    let output = serde_json::to_string(&response).unwrap();
    io::stdout().write_all(output.as_bytes()).unwrap();
}
```

### Step 4: Build WASM Module

```bash
# Add WASM target
rustup target add wasm32-wasip1

# Build for WASM
cargo build --target wasm32-wasip1 --release

# Your module is at:
# target/wasm32-wasip1/release/hello_pied_piper.wasm
```

### Step 5: Test Locally

```bash
# Start Pied Piper gateway
pied-piper gateway

# In another terminal, deploy your module
pied-piper deploy target/wasm32-wasip1/release/hello_pied_piper.wasm \
  --name hello \
  --version 1.0.0

# Test it
curl http://localhost:8080/app/hello
# Output: Hello from Pied Piper! You requested: /
```

**🎉 Congratulations! You just deployed your first decentralized WASM app!**

## Deploy to P2P Network

Now let's deploy to the P2P network for true decentralization.

### Step 1: Start with P2P Discovery

```bash
# Start gateway with P2P enabled
pied-piper gateway \
  --tcp-port 4001 \
  --quic-port 4002 \
  --listen 0.0.0.0:8080
```

### Step 2: Deploy with Metadata

```bash
pied-piper deploy target/wasm32-wasip1/release/hello_pied_piper.wasm \
  --name hello \
  --version 1.0.0 \
  --author "Your Name" \
  --description "My first Pied Piper app"
```

The CLI will output:
```
📦 Deploying module: hello (v1.0.0)
🔐 Module CID: bafkreig...
✅ Published to DHT: hello -> bafkreig...
🌐 Available at: http://localhost:8080/app/hello
```

### Step 3: Access from Any Peer

Your app is now accessible from any Pied Piper node:
```bash
curl http://any-peer-node:8080/app/hello
```

## Add Asset Bundling

Let's create a full web app with HTML, CSS, and JS.

### Step 1: Create Assets

```bash
mkdir assets
```

**assets/index.html:**
```html
<!DOCTYPE html>
<html>
<head>
    <title>My Pied Piper App</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <h1>Hello from Pied Piper!</h1>
    <button onclick="fetchData()">Fetch Data</button>
    <div id="output"></div>
    <script src="app.js"></script>
</body>
</html>
```

**assets/styles.css:**
```css
body {
    font-family: sans-serif;
    max-width: 800px;
    margin: 50px auto;
    padding: 20px;
}

h1 {
    color: #2563eb;
}

button {
    background: #2563eb;
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 4px;
    cursor: pointer;
}

button:hover {
    background: #1d4ed8;
}
```

**assets/app.js:**
```javascript
async function fetchData() {
    const response = await fetch('/api/data');
    const data = await response.json();
    document.getElementById('output').textContent = JSON.stringify(data, null, 2);
}
```

### Step 2: Update WASM Module for API Routes

Edit `src/lib.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Deserialize)]
struct Request {
    method: String,
    path: String,
}

#[derive(Serialize)]
struct Response {
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    body: String,
}

#[no_mangle]
pub extern "C" fn _start() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    
    let request: Request = serde_json::from_str(&input).unwrap();
    
    // Handle different routes
    let response = match request.path.as_str() {
        "/api/data" => {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            
            Response {
                status: 200,
                headers: Some(headers),
                body: r#"{"message": "Hello from API!", "timestamp": 1234567890}"#.to_string(),
            }
        }
        _ => {
            Response {
                status: 404,
                headers: None,
                body: "Not found".to_string(),
            }
        }
    };
    
    let output = serde_json::to_string(&response).unwrap();
    io::stdout().write_all(output.as_bytes()).unwrap();
}
```

### Step 3: Deploy with Assets

```bash
# Rebuild WASM
cargo build --target wasm32-wasip1 --release

# Deploy with asset bundle
pied-piper deploy target/wasm32-wasip1/release/hello_pied_piper.wasm \
  --name my-app \
  --version 1.0.0 \
  --assets ./assets
```

### Step 4: Access Your App

```bash
# Open in browser
open http://localhost:8080/app/my-app

# Or curl the API
curl http://localhost:8080/app/my-app/api/data
```

## Use Host Functions

Pied Piper provides powerful host functions for HTTP, storage, and crypto.

### HTTP Client Example

```rust
// External function declarations
extern "C" {
    fn http_get_v2(url_ptr: i32, url_len: i32) -> i64;
}

// Helper function
fn fetch(url: &str) -> (u16, Vec<u8>) {
    let result = unsafe {
        http_get_v2(url.as_ptr() as i32, url.len() as i32)
    };
    
    let status = (result >> 32) as u16;
    let len = (result & 0xFFFFFFFF) as usize;
    
    // Read from response buffer (implementation specific)
    let body = vec![0u8; len]; // Simplified
    
    (status, body)
}

#[no_mangle]
pub extern "C" fn _start() {
    // Fetch external API
    let (status, body) = fetch("https://api.github.com/users/octocat");
    
    if status == 200 {
        let data = String::from_utf8_lossy(&body);
        println!("GitHub data: {}", data);
    }
}
```

### Storage Example

```rust
extern "C" {
    fn storage_get_v2(key_ptr: i32, key_len: i32) -> i64;
    fn storage_set_v2(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32;
}

// Increment a counter
fn increment_counter() -> u32 {
    // Get current count
    let key = b"counter";
    let result = unsafe {
        storage_get_v2(key.as_ptr() as i32, key.len() as i32)
    };
    
    let found = (result >> 32) != 0;
    let len = (result & 0xFFFFFFFF) as usize;
    
    let mut count: u32 = if found && len > 0 {
        // Parse stored value
        let mut buffer = vec![0u8; len];
        // ... read from buffer ...
        String::from_utf8_lossy(&buffer).parse().unwrap_or(0)
    } else {
        0
    };
    
    // Increment
    count += 1;
    
    // Save back
    let count_str = count.to_string();
    unsafe {
        storage_set_v2(
            key.as_ptr() as i32,
            key.len() as i32,
            count_str.as_ptr() as i32,
            count_str.len() as i32,
        );
    }
    
    count
}
```

### Crypto Example

```rust
extern "C" {
    fn crypto_blake3(data_ptr: i32, data_len: i32, out_ptr: i32);
}

fn hash_content(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    unsafe {
        crypto_blake3(
            data.as_ptr() as i32,
            data.len() as i32,
            hash.as_mut_ptr() as i32,
        );
    }
    hash
}

// Use as content ID
let content = b"Hello, World!";
let hash = hash_content(content);
let hex_hash: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
println!("Content ID: {}", hex_hash);
```

For complete API reference, see [docs/API.md](./API.md).

## Common Patterns

### 1. JSON API with Error Handling

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

fn handle_api_request(path: &str) -> (u16, String) {
    match path {
        "/api/users" => {
            let response = ApiResponse {
                success: true,
                data: Some(serde_json::json!([
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"},
                ])),
                error: None,
            };
            (200, serde_json::to_string(&response).unwrap())
        }
        _ => {
            let response = ApiResponse {
                success: false,
                data: None,
                error: Some("Not found".to_string()),
            };
            (404, serde_json::to_string(&response).unwrap())
        }
    }
}
```

### 2. Request Router

```rust
struct Router {
    // Map of (method, path) -> handler
}

impl Router {
    fn route(&self, method: &str, path: &str) -> Response {
        match (method, path) {
            ("GET", "/") => self.handle_index(),
            ("GET", path) if path.starts_with("/api/") => self.handle_api(path),
            ("POST", "/submit") => self.handle_submit(),
            _ => self.handle_404(),
        }
    }
}
```

### 3. Persistent State

```rust
// Use storage for persistent counters, sessions, etc.
struct AppState {
    visit_count: u32,
    last_visitor: String,
}

impl AppState {
    fn load() -> Self {
        // Load from storage
        let mut state = AppState::default();
        if let Some(data) = storage_get("app_state") {
            state = serde_json::from_slice(&data).unwrap();
        }
        state
    }
    
    fn save(&self) {
        let data = serde_json::to_vec(self).unwrap();
        storage_set("app_state", &data);
    }
}
```

### 4. External API Proxy

```rust
// Fetch data from external API and cache
fn fetch_with_cache(url: &str) -> String {
    let cache_key = format!("cache:{}", url);
    
    // Check cache first
    if let Some(cached) = storage_get(&cache_key) {
        return String::from_utf8_lossy(&cached).to_string();
    }
    
    // Fetch from external API
    let (status, body) = http_get(url);
    
    if status == 200 {
        // Cache for next time
        storage_set(&cache_key, &body);
        return String::from_utf8_lossy(&body).to_string();
    }
    
    "Error fetching data".to_string()
}
```

## Troubleshooting

### Build Errors

**Problem:** `error: linker 'rust-lld' not found`

**Solution:**
```bash
rustup component add rust-lld
rustup target add wasm32-wasip1
```

**Problem:** `cannot find function 'http_get_v2'`

**Solution:** Declare the extern function:
```rust
extern "C" {
    fn http_get_v2(url_ptr: i32, url_len: i32) -> i64;
}
```

### Runtime Errors

**Problem:** Module returns 500 error

**Solution:** Check gateway logs:
```bash
pied-piper gateway  # Shows WASM execution errors
```

**Problem:** Storage not persisting

**Solution:** Storage is in-memory by default. Check config:
```yaml
storage:
  data_dir: /var/lib/pied-piper  # Persistent storage location
```

### Deployment Issues

**Problem:** `Module not found` after deployment

**Solution:** Wait a few seconds for DHT propagation:
```bash
# Give DHT time to propagate
sleep 5
curl http://localhost:8080/app/your-app
```

**Problem:** Assets not loading

**Solution:** Check asset paths are relative:
```html
<!-- Good -->
<link rel="stylesheet" href="styles.css">

<!-- Bad -->
<link rel="stylesheet" href="/styles.css">
```

### P2P Networking

**Problem:** No peers connecting

**Solution:** Check firewall and ports:
```bash
# Allow P2P ports
sudo ufw allow 4001/tcp
sudo ufw allow 4002/udp

# Check if ports are listening
netstat -tuln | grep -E '4001|4002'
```

**Problem:** Module not available on other nodes

**Solution:** Ensure DHT is bootstrapped:
```bash
pied-piper gateway \
  --bootstrap /ip4/203.0.113.1/tcp/4001/p2p/12D3KooW...
```

## Next Steps

### Explore Examples

Check out complete examples in the `examples/` directory:

```bash
cd examples

# Simple API server
cd hello-api && cat README.md

# API with external HTTP calls
cd joke-api && cat README.md

# WebSocket echo server
cd ws-echo && cat README.md

# Full web application
cd web-app && cat README.md
```

### Learn More

- **API Reference:** [docs/API.md](./API.md) - All host functions
- **Deployment:** [docs/DEPLOYMENT.md](./DEPLOYMENT.md) - Production deployment
- **Security:** [docs/SECURITY.md](./SECURITY.md) - Security features
- **Architecture:** [docs/ARCHITECTURE.md](./ARCHITECTURE.md) - System design

### Join the Community

- GitHub: https://github.com/rootkill-g/pied-piper
- Issues: https://github.com/rootkill-g/pied-piper/issues
- Discussions: https://github.com/rootkill-g/pied-piper/discussions

### Build Something Cool

Ideas to try:
- Personal blog with WASM backend
- URL shortener with distributed storage
- Image hosting service
- Chat application with WebSockets
- API gateway/proxy
- Serverless functions platform

**Share what you build!** Open a discussion or PR to add your project to the examples.

---

**Happy Hacking! 🚀**

*Last Updated: December 22, 2025*
