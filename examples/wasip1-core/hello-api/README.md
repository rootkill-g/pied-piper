# Hello API Example

A complete WebAssembly API backend demonstrating request/response handling in Pied Piper.

## Overview

This example shows how to build a WASM module that:
- Reads `WasmRequest` JSON from stdin
- Routes requests to different handlers
- Writes `WasmResponse` JSON to stdout
- Supports multiple endpoints with different HTTP methods

## Building

```bash
# Install the WASM target (if not already installed)
rustup target add wasm32-wasip2

# Build the WASM module
cargo build --target wasm32-wasip2 --release

# The output will be at:
# target/wasm32-wasip2/release/hello_api.wasm
```

## Deploying

```bash
# From the project root
cd ../..

# Deploy the module to the network
./target/release/pied-piper deploy examples/hello-api/target/wasm32-wasip2/release/hello_api.wasm

# Output will show the CID, e.g.:
# ✅ Module deployed successfully!
# 🔗 CID: bafybeig...
```

## Testing the API

### Start the Gateway

```bash
# Start the HTTP gateway
./target/release/pied-piper gateway --port 8080
```

### Test Endpoints

```bash
# 1. Hello endpoint (with query parameter)
curl "http://localhost:8080/cid/<your-cid>/api/hello?name=Alice"
# Response: {"message":"Hello, Alice! 👋","timestamp":"...","path":"/api/hello","method":"GET"}

# 2. Echo endpoint (POST with body)
curl -X POST http://localhost:8080/cid/<your-cid>/api/echo \
  -H "Content-Type: application/json" \
  -d '{"test": "data", "message": "Hello from client"}'
# Response: {"echo":"{\"test\":\"data\",\"message\":\"Hello from client\"}","method":"POST",...}

# 3. API Info endpoint
curl http://localhost:8080/cid/<your-cid>/api/info
# Response: Full API documentation with all available endpoints

# 4. Health check
curl http://localhost:8080/cid/<your-cid>/api/health
# Response: {"status":"healthy","timestamp":"..."}
```

## API Endpoints

| Method | Path | Description | Parameters |
|--------|------|-------------|------------|
| GET | `/api/hello` | Returns a greeting message | `?name=<string>` (optional) |
| POST | `/api/echo` | Echoes back the request body | Body: any JSON |
| GET | `/api/info` | Returns API documentation | None |
| GET | `/api/health` | Health check endpoint | None |

## Request/Response Format

### WasmRequest (stdin)
```json
{
  "method": "POST",
  "path": "/api/hello",
  "query": {"name": "Alice"},
  "headers": {"content-type": "application/json"},
  "body": "{\"data\":\"value\"}",
  "content_type": "application/json"
}
```

### WasmResponse (stdout)
```json
{
  "status": 200,
  "headers": {},
  "body": "{\"message\":\"Hello, Alice!\"}",
  "content_type": "application/json"
}
```

## Code Structure

```rust
// 1. Read request from stdin
let mut stdin_buffer = String::new();
io::stdin().read_to_string(&mut stdin_buffer)?;

// 2. Parse JSON
let request: WasmRequest = serde_json::from_str(&stdin_buffer)?;

// 3. Route and process
let response = route_request(&request);

// 4. Write response to stdout
let response_json = serde_json::to_string(&response)?;
io::stdout().write_all(response_json.as_bytes())?;
```

## Development Tips

### Testing Locally

Before deploying, you can test the WASM module locally:

```bash
# Create a test request JSON
echo '{"method":"GET","path":"/api/hello","query":{"name":"Test"},"headers":{},"body":"","content_type":null}' > request.json

# Run with wasmtime (if installed)
wasmtime run --invoke handle_request target/wasm32-wasip2/release/hello_api.wasm < request.json

# The response JSON will be written to stdout
```

### Debugging

Add debug output using `eprintln!()` - these will go to stderr and won't interfere with the JSON response on stdout:

```rust
eprintln!("Debug: Processing request for path: {}", req.path);
```

### Adding New Endpoints

1. Add a new handler function:
```rust
fn handle_new_endpoint(req: &WasmRequest) -> WasmResponse {
    // Your logic here
    WasmResponse::ok(serde_json::json!({"result": "success"}).to_string())
}
```

2. Add routing:
```rust
fn route_request(req: &WasmRequest) -> WasmResponse {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/new") => handle_new_endpoint(req),
        // ... other routes
    }
}
```

## Architecture

```
Browser/Client
    ↓
HTTP Gateway (Axum)
    ↓
WasmRequest JSON → stdin
    ↓
WASM Module (Wasmtime)
    - handle_request()
    - route_request()
    - handler functions
    ↓
WasmResponse JSON ← stdout
    ↓
HTTP Response
    ↓
Browser/Client
```

## Next Steps

- Add database integration using host functions
- Implement authentication/authorization
- Add request validation
- Support WebSocket connections
- Create multi-module applications

## References

- [Pied Piper Documentation](../../docs/)
- [Gateway Implementation](../../docs/GATEWAY.md)
- [WASM I/O Specification](../../src/gateway/io.rs)
