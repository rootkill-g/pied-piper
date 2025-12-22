# Joke API - Advanced WASM Example

A more complex example demonstrating advanced API patterns in WASI P2.

## Features

- **Multiple Endpoints**: Several joke categories and routes
- **Pseudo-Random Selection**: Uses system time for joke selection
- **JSON Processing**: Complex JSON structures and parsing
- **Error Handling**: Proper error responses
- **RESTful Design**: Clean API structure

## Endpoints

- `GET /api/health` - Health check
- `GET /api/joke` - Random joke from any category
- `GET /api/joke/programming` - Programming jokes
- `GET /api/joke/chuck` - Chuck Norris jokes
- `GET /api/joke/dad` - Dad jokes
- `GET /api/categories` - List all categories
- `GET /api/info` - API information

## Build

```bash
cargo build --target wasm32-wasip2 --release
```

## Deploy

```bash
# From the pied-piper root directory
./target/release/pied-piper deploy examples/joke-api/target/wasm32-wasip2/release/joke-api.wasm
```

## Test

```bash
# Health check
curl http://localhost:8080/cid/<CID>/api/health

# Get a random joke
curl http://localhost:8080/cid/<CID>/api/joke

# Get a programming joke
curl http://localhost:8080/cid/<CID>/api/joke/programming

# List categories
curl http://localhost:8080/cid/<CID>/api/categories
```

## Future Enhancements

To make real external HTTP calls, this would need:

1. **WASI HTTP Support**: Use `wasi:http/outgoing-handler` interface
2. **HTTP Client Library**: Like `reqwest` with WASI target support
3. **Async Runtime**: For handling concurrent requests

Currently, jokes are hardcoded to demonstrate the API structure without external dependencies.

## Note on External HTTP Calls

WASI Preview 2 supports HTTP through the `wasi:http` interface, but it requires:

- Component-based HTTP client (not yet stable in Rust ecosystem)
- Or using lower-level WIT bindings
- Or waiting for libraries like `reqwest` to support WASI HTTP

This example focuses on demonstrating complex API logic and routing patterns that work today with WASI P2.
