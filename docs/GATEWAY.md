# HTTP Gateway Implementation Guide

The Pied Piper HTTP Gateway provides web browser access to applications running on the decentralized P2P network.

## Architecture

```
Browser Request → HTTP Gateway → Content Resolver → Module Loader/P2P Network
                                     ↓
                        WASM Runtime ← Module Bytes
                                     ↓
                        HTTP Response ← Execution Result
```

### Components

1. **GatewayServer** (`src/gateway/server.rs`)
   - Axum HTTP server handling incoming requests
   - Route configuration and middleware
   - State management (network node, loader, config)

2. **RequestHandler** (`src/gateway/handler.rs`)
   - Routes requests to appropriate handlers
   - Fetches modules from cache/network
   - Executes WASM for API endpoints
   - Serves frontend HTML/assets

3. **ContentResolver** (`src/gateway/resolver.rs`)
   - Resolves human-readable names to CIDs
   - DHT lookups for module discovery
   - Checks module availability

4. **Router** (`src/gateway/router.rs`)
   - URL pattern matching
   - Path parameter extraction
   - Query string parsing

## URL Structure

### Content Access

- **By CID**: `/cid/<content-id>/[path]`
  - Direct content-addressed access
  - Example: `/cid/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/`

- **By Name**: `/app/<app-name>/[path]`
  - Human-readable application names
  - Resolves to CID via DHT
  - Example: `/app/my-awesome-app/`

### API Endpoints

- **CID-based API**: `/cid/<content-id>/api/<path>`
  - POST/PUT/DELETE requests routed to WASM handler
  - Example: `POST /cid/bafyb.../api/users`

- **Name-based API**: `/app/<app-name>/api/<path>`
  - Same as CID-based but with name resolution
  - Example: `POST /app/my-app/api/hello`

### System Endpoints

- `/health` - Health check (returns "OK")
- `/info` - Gateway information (peer ID, version, status)
- `/` - Welcome page with usage documentation

## Configuration

```rust
pub struct GatewayConfig {
    listen_addr: SocketAddr,     // e.g., 127.0.0.1:8080
    enable_cors: bool,            // CORS for browsers
    max_body_size: usize,         // Max request size (default: 10MB)
    request_timeout: u64,         // Timeout in seconds
    index_file: String,           // Default: "index.html"
    verbose: bool,                // Logging level
}
```

## CLI Usage

```bash
# Start gateway with defaults
pied-piper gateway

# Custom configuration
pied-piper gateway \
  --listen 0.0.0.0:8080 \
  --tcp-port 4001 \
  --quic-port 4002 \
  --bootstrap /ip4/10.0.0.1/tcp/4001/p2p/12D3Koo... \
  --cors true \
  --timeout 30
```

## Request Flow

### 1. Browser Requests Application

```http
GET /app/my-app/ HTTP/1.1
Host: gateway.piedpiper.local
```

### 2. Gateway Resolves Name to CID

```rust
resolver.resolve("my-app")
  → DHT lookup for "my-app"
  → Returns CID: "bafybeig..."
```

### 3. Module Fetched from Cache/Network

```rust
loader.get_from_cache(cid)
  → Cache hit? Return bytes
  → Cache miss? Fetch from P2P network
```

### 4. Content Served

- **Frontend**: HTML/CSS/JS served directly
- **API**: WASM executed, result returned as JSON

## WASM API Handler Convention

WASM modules export handler functions that the gateway calls:

```rust
#[no_mangle]
pub extern "C" fn handle_request() -> i32 {
    // Read request from stdin/memory
    // Process request
    // Write response to stdout/memory
    0 // Return status code
}
```

### Handler Discovery

Gateway looks for functions in this order:
1. `_handle_request` (WASI convention)
2. `handle_request` (standard name)

If not found, returns 501 Not Implemented error.

## Example: Deploying an API

### 1. Create WASM Module

```rust
// api.rs
#[no_mangle]
pub extern "C" fn handle_request() -> i32 {
    println!("{{\"message\": \"Hello from WASM!\"}}");
    0
}
```

### 2. Compile to WASM

```bash
cargo build --target wasm32-wasip2 --release
```

### 3. Deploy to Network

```bash
pied-piper deploy target/wasm32-wasip2/release/my_api.wasm
# Returns CID: bafybeig...
```

### 4. Access via Gateway

```bash
# By CID
curl -X POST http://localhost:8080/cid/bafybeig.../api/hello

# By name (after DHT registration)
curl -X POST http://localhost:8080/app/my-api/api/hello
```

## Response Types

### Success (200 OK)

```json
{
  "status": "success",
  "message": "API handler executed successfully",
  "path": "/hello",
  "cid": "bafybeig...",
  "note": "Full I/O integration coming soon"
}
```

### Not Found (404)

```html
<!DOCTYPE html>
<html>
<head><title>404 - Not Found</title></head>
<body>
  <h1>404 - Not Found</h1>
  <p>The requested application or resource was not found on the network.</p>
  <p><a href="/">Return to Gateway Home</a></p>
</body>
</html>
```

### Not Implemented (501)

```json
{
  "error": "No API handler found",
  "message": "Module must export 'handle_request' or '_handle_request' function",
  "path": "/api/hello",
  "cid": "bafybeig..."
}
```

### Internal Error (500)

```json
{
  "error": "Execution failed",
  "message": "WASM trap: out of bounds memory access",
  "path": "/api/hello",
  "cid": "bafybeig..."
}
```

## Security & Sandboxing

WASM modules run with resource limits:

- **Memory**: 64MB for API handlers
- **Execution Time**: 10 seconds max
- **Fuel**: 1,000,000 instructions (for metering)
- **WASI**: Sandboxed file/network access

## Performance Considerations

### Caching Strategy

1. **Memory Cache**: Recently accessed modules stay in RAM
2. **Disk Cache**: Downloaded modules persist across restarts
3. **Network Fetch**: Only when not in cache

### Module Reuse

- Compiled WASM modules are cached
- Multiple requests reuse same instance
- Store pooling for concurrent requests (future)

## Current Limitations & Roadmap

### ✅ Implemented

- HTTP server with routing
- CID and name-based access
- DHT name resolution
- Cache-first module fetching
- WASM module loading and execution
- Error handling and beautiful responses

### 🔨 In Progress

- Full WASI I/O for request/response
- Request body parsing
- Response formatting
- Header management

### ❌ Not Yet Implemented

- Frontend asset bundling (HTML/CSS/JS)
- Static file serving from bundles
- Multi-module applications
- HTTPS/TLS support
- Authentication/authorization
- Rate limiting
- WebSocket support
- Streaming responses

## Testing

### Manual Testing

```bash
# Terminal 1: Start gateway
pied-piper gateway --listen 127.0.0.1:8080 --verbose

# Terminal 2: Test endpoints
curl http://localhost:8080/health
curl http://localhost:8080/info
curl http://localhost:8080/
```

### With a Deployed Module

```bash
# Deploy test module
pied-piper deploy examples/hello-api/target/wasm32-wasip2/release/hello_api.wasm

# Test via gateway (replace with actual CID)
curl -X POST http://localhost:8080/cid/<YOUR-CID>/api/test -d '{"test":"data"}'
```

## Architecture Decisions

### Why Mutex Instead of RwLock?

```rust
// ❌ Doesn't work - Swarm is !Sync
Arc<RwLock<NetworkNode>>

// ✅ Works - Mutex is Send + Sync even if T is !Sync
Arc<Mutex<NetworkNode>>
```

libp2p's `Swarm` is `!Sync`, so we can't use `RwLock` with axum's state system (which requires `Send + Sync`). `Mutex` works because it only requires `T: Send`, not `T: Sync`.

Trade-off: No concurrent reads, but necessary for axum compatibility.

### Module Structure

```
src/gateway/
├── mod.rs       # Module exports
├── server.rs    # HTTP server (axum)
├── resolver.rs  # Name → CID resolution
├── handler.rs   # Request handling & WASM execution
└── router.rs    # URL routing utilities
```

Clean separation of concerns for maintainability.

## Troubleshooting

### Gateway Won't Start

```
Error: Failed to bind HTTP server
```

**Solution**: Port already in use. Try different port:
```bash
pied-piper gateway --listen 127.0.0.1:8081
```

### Module Not Found

```
404 - Module <CID> not found
```

**Solution**: Ensure module is deployed and cached:
```bash
# Deploy first
pied-piper deploy my-module.wasm
```

### Name Resolution Fails

```
404 - Application 'my-app' not found
```

**Solution**: 
1. Ensure module is deployed with name
2. Wait for DHT propagation (can take seconds)
3. Check bootstrap peers are connected

### WASM Execution Fails

```
500 - Execution failed: WASM trap
```

**Solution**:
1. Check WASM module is valid
2. Ensure handler function exists
3. Review fuel/memory limits
4. Check wasmtime logs with `--verbose`

## Contributing

When adding new features:

1. Update route handlers in `server.rs`
2. Add business logic in `handler.rs`
3. Update tests
4. Document in this guide
5. Add examples

## Related Documentation

- [Project.md](../Project.md) - Overall project vision
- [TESTING.md](../TESTING.md) - Test strategy
- [examples/hello-api/](../examples/hello-api/) - Example WASM module
