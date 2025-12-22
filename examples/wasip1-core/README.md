# WASI Preview 1 Core Modules

Standard WASM modules built with `cargo build --target wasm32-wasip1 --release`

## Examples

- **hello-api** - Simple HTTP API ✅
- **joke-api** - Random joke API ✅
- **todo-api** - CRUD API with persistent storage ✅
- **static-blog** - Blog API with persistent storage ✅
- **dashboard** - Interactive dashboard ⚠️ (has runtime issues)

## Build

```bash
cd <example-name>
cargo build --target wasm32-wasip1 --release
```

Binary output: `target/wasm32-wasip1/release/<module>.wasm`

## Deploy

```bash
../../target/release/pied-piper deploy --name <name> \
  target/wasm32-wasip1/release/<module>.wasm
```

## Requirements

Must export one of: `_start`, `handle_request`, or `_handle_request`

See [main examples README](../README.md) for detailed documentation.
