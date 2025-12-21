# Hello API Example

A simple WebAssembly module demonstrating API handling in the Pied Piper gateway.

## Building

```bash
# Build the WASM module
cargo build --target wasm32-wasip2 --release

# Or use wasm32-unknown-unknown for non-WASI
cargo build --target wasm32-unknown-unknown --release
```

## Testing with Pied Piper

```bash
# Deploy the module
pied-piper deploy examples/hello-api/target/wasm32-wasip2/release/hello_api.wasm

# Start the gateway
pied-piper gateway --listen 127.0.0.1:8080

# Test the API endpoint
curl -X POST http://localhost:8080/cid/<your-cid>/api/hello -d '{"test":"data"}'
```

## API Handler Convention

The module exports a `handle_request` function that the gateway calls to process API requests.

```rust
#[no_mangle]
pub extern "C" fn handle_request() -> i32 {
    // Process request and return status code
    0 // Success
}
```

Future enhancements will include:
- Reading request body from WASI stdin
- Parsing JSON request data
- Writing JSON response to WASI stdout
- HTTP status code handling
- Header management
