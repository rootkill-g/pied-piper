# Examples Reorganization - Migration Guide

## What Changed (December 23, 2025)

Examples are now organized by build target for easier navigation and understanding.

## Old Structure → New Structure

```
examples/                      examples/
├── hello-api/          →     ├── wasip1-core/
├── joke-api/                 │   ├── hello-api/
├── todo-api/                 │   ├── joke-api/
├── static-blog/              │   ├── todo-api/
├── dashboard/                │   ├── static-blog/
├── ws-echo/            →     │   └── dashboard/
├── api-client/               ├── wasip1-component/
├── blog-frontend/      →     │   ├── ws-echo/
├── web-app/                  │   └── api-client/
                              ├── tar-bundles/
                              │   ├── blog-frontend/
                              │   └── web-app/
                              └── wasip2-component/ (future)
```

## Path Changes

### Build Paths

| Old | New |
|-----|-----|
| `examples/hello-api/` | `examples/wasip1-core/hello-api/` |
| `examples/ws-echo/` | `examples/wasip1-component/ws-echo/` |
| `examples/blog-frontend/` | `examples/tar-bundles/blog-frontend/` |

### Deploy Commands

Old:
```bash
cd examples/hello-api
cargo build --target wasm32-wasip1 --release
../target/release/pied-piper deploy --name hello-api target/wasm32-wasip1/release/hello_api.wasm
```

New:
```bash
cd examples/wasip1-core/hello-api
cargo build --target wasm32-wasip1 --release
../../target/release/pied-piper deploy --name hello-api target/wasm32-wasip1/release/hello_api.wasm
```

**Note**: Need to go up two directories now (`../../`) instead of one (`../`)

## Updating Scripts

If you have automation scripts, update paths:

```bash
# Old
EXAMPLES_DIR="examples"
cd "$EXAMPLES_DIR/hello-api"

# New
EXAMPLES_DIR="examples/wasip1-core"
cd "$EXAMPLES_DIR/hello-api"
```

## Benefits

✅ Clear separation by build target  
✅ Easy to find examples for specific use cases  
✅ Target-specific README files  
✅ Simpler to understand which build command to use  
✅ Room for future targets (wasip2-component)

## Questions?

See the main [examples/README.md](README.md) for full documentation.
