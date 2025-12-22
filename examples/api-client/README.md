# API Client Example

This example demonstrates all host functions available to WASM modules:

- **HTTP Client**: Make GET/POST requests to external APIs
- **Storage**: Key-value storage with caching
- **Crypto**: BLAKE3 hashing
- **System**: Logging, timestamps, random numbers

## Features

### 1. External API Calls (`/external`)
Call any external API with optional caching:

```bash
curl -X POST http://localhost:8080/app/api-client/external \
  -H "Content-Type: application/json" \
  -d '{"url":"https://api.github.com/zen","use_cache":true}'
```

Response:
```json
{
  "status": "success",
  "data": "Design for failure.",
  "timestamp": 1640000000000
}
```

### 2. Cache Management (`/cache`)

**Get cached value:**
```bash
curl -X POST http://localhost:8080/app/api-client/cache \
  -H "Content-Type: application/json" \
  -d '{"action":"get","key":"my-key"}'
```

**Set value:**
```bash
curl -X POST http://localhost:8080/app/api-client/cache \
  -H "Content-Type: application/json" \
  -d '{"action":"set","key":"my-key","value":"my-value"}'
```

**Delete value:**
```bash
curl -X POST http://localhost:8080/app/api-client/cache \
  -H "Content-Type: application/json" \
  -d '{"action":"delete","key":"my-key"}'
```

**List keys:**
```bash
curl -X POST http://localhost:8080/app/api-client/cache \
  -H "Content-Type: application/json" \
  -d '{"action":"list"}'
```

### 3. Hashing (`/hash`)
Generate BLAKE3 hashes:

```bash
curl -X POST http://localhost:8080/app/api-client/hash \
  -H "Content-Type: application/json" \
  -d '{"data":"Hello, Pied Piper!"}'
```

Response:
```json
{
  "status": "success",
  "data": {
    "data": "Hello, Pied Piper!",
    "hash": "1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8",
    "algorithm": "blake3"
  }
}
```

### 4. Counter (`/counter`)
Atomic counter with automatic increment:

```bash
curl -X POST http://localhost:8080/app/api-client/counter
```

Response:
```json
{
  "status": "success",
  "data": {
    "counter": 42,
    "previous": 41
  }
}
```

### 5. Statistics (`/stats`)
View system statistics:

```bash
curl http://localhost:8080/app/api-client/stats
```

Response:
```json
{
  "status": "success",
  "data": {
    "storage": {
      "total_keys": 5
    },
    "system": {
      "timestamp_ms": 1640000000000,
      "random_sample": 3456789012
    }
  }
}
```

### 6. Health Check (`/health`)
Check if the service is running:

```bash
curl http://localhost:8080/app/api-client/health
```

Response:
```json
{
  "status": "success",
  "data": {
    "status": "healthy",
    "uptime_ms": 1640000000000,
    "storage_keys": 5
  }
}
```

## Building

```bash
cargo component build --release
```

Output: `target/wasm32-wasip2/release/api_client.wasm`

## Deploying

```bash
# Bundle the module
pied-piper bundle \
  --wasm target/wasm32-wasip2/release/api_client.wasm \
  --name api-client

# Publish to network
pied-piper publish \
  --bundle api-client.ppc \
  --name api-client

# Start gateway
pied-piper gateway --listen 0.0.0.0:8080
```

## Architecture

```
┌─────────────────┐
│   HTTP Client   │
│   (curl/fetch)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Pied Piper     │
│   Gateway       │
│  (Port 8080)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  WASM Module    │◄──┐
│  (api-client)   │   │
└────────┬────────┘   │
         │            │
         ▼            │
┌─────────────────┐   │
│  Host Functions │   │
│                 │   │
│  • HTTP Client  │───┤ External APIs
│  • Storage      │   │ (GitHub, etc.)
│  • Crypto       │   │
│  • System       │   │
└─────────────────┘   │
                      │
                      └─ (Request/Response)
```

## Code Examples

### Making HTTP Requests

```rust
// Simple GET request
let (status, body) = http::get("https://api.example.com/data".to_string());

if status == 200 {
    let data = String::from_utf8_lossy(&body);
    host::log(format!("Success: {}", data));
}

// POST request
let request_body = br#"{"action":"deploy"}"#;
let (status, response) = http::post(
    "https://api.example.com/deploy".to_string(),
    request_body.to_vec()
);
```

### Using Storage

```rust
// Store data
let key = "user:123".to_string();
let value = b"Alice".to_vec();
storage::set(key, value);

// Retrieve data
let (found, value) = storage::get("user:123".to_string());
if found {
    let name = String::from_utf8_lossy(&value);
    host::log(format!("User: {}", name));
}

// Delete data
let deleted = storage::delete("user:123".to_string());

// Count keys
let count = storage::list_count();
```

### Hashing

```rust
// Hash some data
let data = b"Hello, World!".to_vec();
let hash = crypto::blake3_hash(data);

// Convert to hex
let hex = hex_encode(&hash);
host::log(format!("Hash: {}", hex));
```

### Logging and Time

```rust
// Log messages
host::log("Application started".to_string());

// Get current timestamp
let now = host::now_millis();
host::log(format!("Current time: {} ms", now));

// Generate random number
let random = host::random_u32();
let session_id = format!("session_{}", random);
```

## Testing

```bash
# Build first
cargo component build --release

# Test health endpoint
curl http://localhost:8080/app/api-client/health

# Test external API with caching
curl -X POST http://localhost:8080/app/api-client/external \
  -H "Content-Type: application/json" \
  -d '{"url":"https://api.github.com/zen","use_cache":true}'

# Test again (should be cached)
curl -X POST http://localhost:8080/app/api-client/external \
  -H "Content-Type: application/json" \
  -d '{"url":"https://api.github.com/zen","use_cache":true}'

# Test counter (call multiple times)
for i in {1..5}; do
  curl -X POST http://localhost:8080/app/api-client/counter
  echo ""
done

# Test hashing
curl -X POST http://localhost:8080/app/api-client/hash \
  -H "Content-Type: application/json" \
  -d '{"data":"Pied Piper Compression Algorithm"}'
```

## Implementation Details

### Caching Strategy

The example implements a TTL-based cache:

1. **Cache Key**: `cache:{original_key}`
2. **Cache Entry**: JSON with `{data, created_at, ttl_seconds}`
3. **Expiration**: Checked on every read
4. **Invalidation**: Automatic on expiry

### Error Handling

All endpoints return consistent JSON:

```json
{
  "status": "success|error",
  "data": {...},
  "error": "error message",
  "timestamp": 1640000000000
}
```

### Memory Management

- Efficient buffer usage with appropriate sizing
- Minimal allocations with string reuse
- JSON parsing with error recovery

## Troubleshooting

**HTTP requests fail:**
- Check that the gateway has internet access
- Verify the URL is correct and accessible
- Check for CORS or SSL issues

**Cache not working:**
- Verify storage is available: `/stats` endpoint
- Check TTL settings (default: 300 seconds)
- Ensure keys are consistent

**Build errors:**
- Update `cargo-component`: `cargo install cargo-component`
- Check Rust version: `rustc --version` (need 1.75+)
- Clean build: `cargo clean && cargo component build --release`

## Performance Tips

1. **Use caching** for frequently accessed external APIs
2. **Batch operations** when possible
3. **Limit buffer sizes** to avoid memory waste
4. **Log selectively** to reduce overhead

## See Also

- [Host Functions Guide](../../docs/HOST_FUNCTIONS.md)
- [WebSocket Guide](../../docs/WEBSOCKET_GUIDE.md)
- [WASM I/O Guide](../../docs/WASM_IO_GUIDE.md)
