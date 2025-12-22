# WASI Preview 1 Components

Component model modules built with `cargo component build --release`

## Examples

- **ws-echo** - WebSocket echo server ✅
- **api-client** - HTTP client with custom WIT imports ⚠️ (wasmtime type mismatch)

## Build

```bash
cd <example-name>
cargo component build --release
```

Binary output: `target/wasm32-wasip1/release/<module>.wasm`

## Deploy

```bash
../../target/release/pied-piper deploy --name <name> \
  target/wasm32-wasip1/release/<module>.wasm
```

## WebSocket Access

WebSocket components are accessed via:
```
ws://localhost:8080/ws/cid/<CID>
ws://localhost:8080/ws/app/<name>
```

Test with websocat:
```bash
echo '{"type":"echo","id":"1","data":"Hello"}' | \
  websocat ws://localhost:8080/ws/cid/<CID>
```

See [main examples README](../README.md) for detailed documentation.
