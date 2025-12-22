# Test WASM Module for I/O Integration Tests

This directory contains test WASM modules for integration testing.

## test-echo-api

A simple API module that:
1. Reads WasmRequest JSON from stdin
2. Echoes back a WasmResponse with the request details
3. Tests full request/response cycle

Build with:
```bash
cd test-echo-api
cargo build --target wasm32-wasip1 --release
```

The compiled WASM will be at: `target/wasm32-wasip1/release/test_echo_api.wasm`
