# Pied Piper Examples

This directory contains example applications demonstrating different features of the Pied Piper platform.

## Directory Structure

Examples are organized by their WASM build target:

```
examples/
├── wasip1-core/           # Standard WASM modules (wasm32-wasip1)
│   ├── hello-api/         # ✅ Simple HTTP API
│   ├── joke-api/          # ✅ Random joke API
│   ├── todo-api/          # ✅ CRUD with storage
│   ├── static-blog/       # ✅ Blog API with storage
│   └── dashboard/         # ⚠️  Interactive dashboard (runtime error)
├── wasip1-component/      # Component model (cargo component build)
│   ├── ws-echo/           # ✅ WebSocket echo server
│   └── api-client/        # ⚠️  HTTP client with WIT (type mismatch)
├── tar-bundles/           # Frontend bundles (HTML/CSS/JS)
│   ├── blog-frontend/     # ✅ Blog UI
│   └── web-app/           # ✅ Demo frontend
├── wasip2-component/      # WASI Preview 2 (future)
├── api-client-core/       # Library for api-client
├── chat-ws/               # Standalone app
└── component-host-demo/   # Component demo (untested)
```

**Legend**: ✅ Working | ⚠️ Partial/Issues | ❌ Not working

---

## WASI Preview 1 Core Modules (`wasip1-core/`)

Standard WASM modules built with `cargo build --target wasm32-wasip1 --release`

### ✅ Working Examples

1. **hello-api** - Simple HTTP API
   - Location: `wasip1-core/hello-api/`
   - Build: `cargo build --target wasm32-wasip1 --release`
   - Binary: `target/wasm32-wasip1/release/hello_api.wasm`
   - CID: `brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka`
   - Features: Basic HTTP handlers (GET, POST)
   - Status: ✅ Production ready

2. **joke-api** - Random joke API
   - Location: `wasip1-core/joke-api/`
   - Build: `cargo build --target wasm32-wasip1 --release`
   - Binary: `target/wasm32-wasip1/release/joke_api.wasm`
   - CID: `b27zyw23x6pwbsyhwa5rhb4vl3xvwmmh6jrdq3b3mwkpbc2fh6gva`
   - Features: JSON responses, multiple endpoints
   - Status: ✅ Production ready

3. **todo-api** - CRUD API with storage
   - Location: `wasip1-core/todo-api/`
   - Build: `cargo build --target wasm32-wasip1 --release`
   - Binary: `target/wasm32-wasip1/release/todo_api.wasm`
   - CID: `belirxknyp4fwqxkiz3iv3ht4aiglyub7movzhuulh6nlrp6xq4da`
   - Features: Persistent storage, full CRUD operations
   - Fixed: Storage API migration (env module, host_storage_*)
   - Status: ✅ Production ready

4. **static-blog** - Blog API with storage
   - Location: `wasip1-core/static-blog/`
   - Build: `cargo build --target wasm32-wasip1 --release`
   - Binary: `target/wasm32-wasip1/release/static_blog.wasm`
   - CID: `b3fartwhbq6i5gzzrj7vwmafopfj3c7kvxqj7i4k7tdrm4izpjtyq`
   - Features: Blog posts, persistent storage
   - Fixed: Storage API migration
   - Status: ✅ Production ready

### ⚠️ Issues

5. **dashboard** - Interactive frontend module
   - Location: `wasip1-core/dashboard/`
   - Build: `cargo build --target wasm32-wasip1 --release`
   - Binary: `target/wasm32-wasip1/release/dashboard.wasm`
   - CID: `b67crvzzho3c7addlyurrctaea4xm6vbg4w74v5njdc6mzjcv2caa`
   - Status: Runtime error (exits with status 1)
   - Issue: JSON parsing or request handling failure

---

## WASI Preview 1 Components (`wasip1-component/`)

Component model modules built with `cargo component build --release`

### ✅ Working Examples

6. **ws-echo** - WebSocket echo server
   - Location: `wasip1-component/ws-echo/`
   - Build: `cargo component build --release`
   - Binary: `target/wasm32-wasip1/release/ws_echo.wasm`
   - CID: `bqamwzd2kpfc32mrvitwd5atb6sjoo6r7blhpyzsb7b7zijo3izka`
   - Access: `ws://localhost:8080/ws/cid/[CID]`
   - Features: WebSocket with echo, uppercase, reverse, count
   - Testing: Use `websocat` client
   - Status: ✅ WebSocket working

### ⚠️ Partial Support

7. **api-client** - HTTP client component with custom WIT
   - Location: `wasip1-component/api-client/`
   - Build: `cargo component build --release`
   - Binary: `target/wasm32-wasip1/release/api_client.wasm`
   - CID: `b426e6qbvo6mn7bkclccq5vr4aik6fqxwjb3hs5uasdewusymqcna`
   - Status: Wasmtime component linker type mismatch
   - Issue: Custom WIT imports (http, storage, crypto) - async tuple return types
   - Features: Demonstrates external API calls, caching, hashing

---

## TAR Bundles (`tar-bundles/`)

Frontend applications bundled as TAR archives (HTML/CSS/JS)

### ✅ Working Examples

8. **blog-frontend** - Blog UI
   - Location: `tar-bundles/blog-frontend/`
   - Build: `./bundle.sh` (creates tar archive)
   - Binary: `blog-frontend.tar`
   - CID: `b6r4dcsqbxotkjqewlzrjkvozknwhv2slb6sgvettqnzs6edyiqsq`
   - Features: Complete blog interface, communicates with static-blog API
   - Access: `http://localhost:8080/cid/[CID]/`
   - Status: ✅ Production ready

9. **web-app** - Demo frontend
   - Location: `tar-bundles/web-app/`
   - Build: `./bundle.sh` (creates tar archive)
   - Binary: `web-app.tar`
   - CID: `bgqqyk6sl4l4v6zzghmtprzejxte2jgbz7al22gswjztzxaies5va`
   - Features: HTML/CSS/JS bundle demonstrating asset serving
   - Fixed: TAR asset serving, trailing slash redirect
   - Status: ✅ Production ready

---

## Other Examples

10. **chat-ws** - Chat application
    - Status: Appears to be standalone binary, not a WASM module
    - Build: Standard cargo build (not for WASM)

11. **api-client-core** - Library crate
    - Status: Supporting library for api-client
    - Build: Not deployable standalone

12. **component-host-demo** - Component model demo
    - Status: Not yet tested
    - Build: TBD

---

## Quick Start

### Build and Deploy wasip1-core Module

```bash
cd wasip1-core/hello-api
cargo build --target wasm32-wasip1 --release
../../target/release/pied-piper deploy --name hello-api \
  target/wasm32-wasip1/release/hello_api.wasm
```

### Build and Deploy wasip1-component

```bash
cd wasip1-component/ws-echo
cargo component build --release
../../target/release/pied-piper deploy --name ws-echo \
  target/wasm32-wasip1/release/ws_echo.wasm
```

### Build and Deploy TAR Bundle

```bash
cd tar-bundles/blog-frontend
./bundle.sh
../../target/release/pied-piper deploy --name blog-frontend \
  blog-frontend.tar
```

---

## Build Commands Quick Reference

```bash
# WASI Preview 1 (Core Modules) - from wasip1-core/<example>/
cargo build --target wasm32-wasip1 --release

# WASI Preview 1 Components - from wasip1-component/<example>/
cargo component build --release

# Frontend Bundles - from tar-bundles/<example>/
./bundle.sh  # Creates TAR archive

# Deploy any module - from examples/
../target/release/pied-piper deploy --name <name> <path-to-wasm-or-tar>
```

---

## Common Patterns

### Storage API (for wasip1 modules)

Modules using persistent storage should use:

```rust
// Import from env module
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_storage_get(key_ptr: *const u8, key_len: u32, 
                        buf_ptr: *mut u8, buf_len: u32) -> i32;
    fn host_storage_set(key_ptr: *const u8, key_len: u32,
                        val_ptr: *const u8, val_len: u32) -> i32;
    fn host_storage_delete(key_ptr: *const u8, key_len: u32) -> i32;
}
```

### Response Format

```rust
use std::collections::HashMap;

#[derive(Serialize)]
struct WasmResponse {
    status: u16,
    headers: HashMap<String, String>,  // Not Vec<(String, String)>
    body: String,
    content_type: Option<String>,
}
```

### Request Format

```rust
#[derive(Deserialize)]
struct WasmRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,    // Not Option<String>
    headers: HashMap<String, String>,
    body: String,
    content_type: Option<String>,
}
```

---

## Testing

### HTTP APIs
```bash
curl http://localhost:8080/cid/[CID]/[path]
curl http://localhost:8080/app/[name]/[path]
```

### WebSocket Components
```bash
# Using websocat
echo '{"type":"echo","id":"1","data":"Hello"}' | \
  websocat ws://localhost:8080/ws/cid/[CID]
```

### Frontend Bundles
Open in browser: `http://localhost:8080/cid/[CID]/`

---

## Fixes Applied

1. **Storage API Migration** (todo-api, static-blog)
   - Changed from `host::storage_*_v2` to `env::host_storage_*`
   - Updated buffer size parameters (u32)
   - Changed Response headers: `Vec<(String, String)>` → `HashMap<String, String>`
   - Changed Request query: `Option<String>` → `HashMap<String, String>`

2. **TAR Asset Serving** (web-app, blog-frontend)
   - Fixed to serve all files from TAR, not just index.html
   - Added proper content-type detection

3. **Trailing Slash Redirect** (web-app)
   - Added 301 redirect from `/cid/xyz` to `/cid/xyz/` for bundles
   - Fixes relative asset path resolution in browsers

---

## Status Legend

- ✅ **Working** - Fully functional, production ready
- ⚠️ **Partial** - Deployed but has limitations
- ❌ **Not Working** - Has blocking issues
- ❓ **Untested** - Not yet tested or not applicable

---

## Target Summary

| Target | Location | Count | Status |
|--------|----------|-------|--------|
| wasm32-wasip1 (core) | `wasip1-core/` | 5 | 4/5 working |
| wasm32-wasip1 (component) | `wasip1-component/` | 2 | 1/2 working |
| TAR Bundle | `tar-bundles/` | 2 | 2/2 working |
| Other/Library | root | 3 | N/A |

**Overall: 8 out of 12 examples fully working** ✅

---

## Need Help?

- See individual target folder READMEs for build instructions
- Check [EXAMPLES_TEST_RESULTS.md](../EXAMPLES_TEST_RESULTS.md) for detailed test results
- Review [main documentation](../docs/) for architecture and API details

---

## Recent Changes

**December 23, 2025** - Examples reorganized by build target:
- Created `wasip1-core/` for standard WASM modules
- Created `wasip1-component/` for component model modules  
- Created `tar-bundles/` for frontend applications
- Created `wasip2-component/` for future WASI Preview 2
- Each folder has its own README with build instructions
- Simplified structure - no nested `_by-target` folder needed!
