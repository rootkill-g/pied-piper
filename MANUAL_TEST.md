# Manual Testing Instructions

The HTTP method fix has been implemented and compiled successfully! However, due to nushell's background process handling, you'll need to run the gateway in a separate terminal.

## Terminal Setup

### Terminal 1 - Start Gateway
```bash
cd /Users/rootkill/pied-piper
./target/release/pied-piper gateway --listen 127.0.0.1:3000
```

Leave this running. You should see:
```
✅ Gateway listening on http://127.0.0.1:3000
📡 Ready to serve decentralized applications
```

### Terminal 2 - Run Tests

#### Quick Test Script
```bash
cd /Users/rootkill/pied-piper
./test_api.sh
```

#### Individual Tests

**1. Test GET /api/hello with name parameter:**
```bash
curl "http://localhost:3000/cid/boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q/api/hello?name=Alice"
```

Expected output:
```json
{
  "status": "success",
  "message": "Hello, Alice!",
  "method": "GET",
  "path": "/api/hello"
}
```

**2. Test GET /api/hello without name:**
```bash
curl "http://localhost:3000/cid/boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q/api/hello"
```

Expected output:
```json
{
  "status": "success",
  "message": "Hello, World!",
  "method": "GET",
  "path": "/api/hello"
}
```

**3. Test POST /api/echo:**
```bash
curl -X POST "http://localhost:3000/cid/boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q/api/echo" \
  -H "Content-Type: application/json" \
  -d '{"message": "Test from curl!"}'
```

Expected output:
```json
{
  "status": "success",
  "method": "POST",
  "path": "/api/echo",
  "received": "{\"message\": \"Test from curl!\"}"
}
```

**4. Test GET /api/info:**
```bash
curl "http://localhost:3000/cid/boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q/api/info"
```

Expected output with API documentation.

**5. Test GET /api/health:**
```bash
curl "http://localhost:3000/cid/boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q/api/health"
```

Expected output:
```json
{
  "status": "healthy",
  "service": "hello-api"
}
```

## What Was Fixed

The gateway was hardcoding all requests to WASM modules as "POST". Now it:
1. Accepts all HTTP methods (GET, POST, PUT, DELETE, etc.)
2. Extracts the actual method from incoming requests
3. Passes the correct method to the WASM module
4. Detects `/api/*` paths and routes them to WASM execution

## Key Code Changes

- `src/gateway/server.rs`: Changed routes from `get()` to `any()`
- `src/gateway/handler.rs`: Added method parameter to handlers
- `src/gateway/handler.rs`: Changed from hardcoded "POST" to `method.to_string()`

The hello-api WASM module now correctly receives GET requests and can route them properly!
