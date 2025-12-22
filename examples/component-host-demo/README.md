# Component Host Demo

This example builds a tiny WASI P2 component that calls:

- `http.get`
- `storage.get`

It logs a line via `host.log` and prints a JSON response to stdout.

## Build

Install `cargo-component` if needed:

```bash
cargo install cargo-component
```

Build the component:

```bash
cargo component build --release
```

Output (example path):

```
target/wasm32-wasip2/release/component_host_demo.wasm
```

## Run via Pied Piper

Start the gateway in another terminal:

```bash
../../target/release/pied-piper gateway --port 3000
```

Deploy the component:

```bash
../../target/release/pied-piper deploy target/wasm32-wasip2/release/component_host_demo.wasm \
  --name component-host-demo \
  --version 0.1.0
```

Then request it:

```bash
curl "http://localhost:3000/cid/<CID>/api"
```

Notes:
- `storage.get("greeting")` returns empty unless another component has stored it.
- If HTTP access is blocked, `http.get` returns `status=0` with an empty body.
