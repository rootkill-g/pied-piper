# WASM Request/Response I/O ABI Specification

## Overview

This document defines the Application Binary Interface (ABI) for request/response communication between the Pied Piper gateway and WASM modules. This ABI is designed to support both WASI Preview 1 (core modules) and WASI Preview 2 (component model) modules.

---

## Protocol Version

**Current Version:** 1.0  
**Last Updated:** December 22, 2025

---

## Communication Model

### Input (Request) - Gateway → WASM Module

The gateway sends HTTP request data to the WASM module via **stdin** (file descriptor 0) as a JSON-serialized `WasmRequest`.

### Output (Response) - WASM Module → Gateway

The WASM module writes HTTP response data to **stdout** (file descriptor 1) as a JSON-serialized `WasmResponse`.

### Error Reporting

Modules can write error messages to **stderr** (file descriptor 2). These will be logged by the gateway but not sent to the client.

---

## Data Structures

### WasmRequest

```json
{
  "method": "string",           // HTTP method: GET, POST, PUT, DELETE, etc.
  "path": "string",             // Request path with leading slash, e.g., "/api/users"
  "query": {                     // Query parameters as key-value pairs (optional)
    "key": "value"
  },
  "headers": {                   // HTTP headers as key-value pairs (optional)
    "Header-Name": "value"
  },
  "body": "string",             // Request body as UTF-8 string (optional, "" for no body)
  "content_type": "string|null" // Content-Type header value (optional)
}
```

#### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `method` | String | Yes | HTTP method (uppercase, e.g., "GET", "POST") |
| `path` | String | Yes | URL path starting with "/" |
| `query` | Object | No | Query string parameters (default: empty object) |
| `headers` | Object | No | HTTP headers (default: empty object) |
| `body` | String | No | Request body as UTF-8 string (default: empty string) |
| `content_type` | String/null | No | Content-Type from request headers |

#### Example WasmRequest

```json
{
  "method": "POST",
  "path": "/api/echo",
  "query": {
    "format": "json",
    "debug": "true"
  },
  "headers": {
    "Content-Type": "application/json",
    "User-Agent": "PiedPiper/1.0",
    "Authorization": "Bearer token123"
  },
  "body": "{\"message\":\"Hello, World!\"}",
  "content_type": "application/json"
}
```

---

### WasmResponse

```json
{
  "status": number,              // HTTP status code (200, 404, 500, etc.)
  "headers": {                   // Response headers as key-value pairs (optional)
    "Header-Name": "value"
  },
  "body": "string",             // Response body as UTF-8 string
  "content_type": "string|null" // Content-Type for response (optional)
}
```

#### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `status` | Number | Yes | HTTP status code (200, 201, 400, 404, 500, etc.) |
| `headers` | Object | No | Custom HTTP headers to include in response |
| `body` | String | Yes | Response body as UTF-8 string |
| `content_type` | String/null | No | Content-Type header (default: "application/json") |

#### Example WasmResponse

```json
{
  "status": 200,
  "headers": {
    "X-Custom-Header": "value",
    "Cache-Control": "no-cache"
  },
  "body": "{\"result\":\"success\",\"data\":{\"id\":123}}",
  "content_type": "application/json"
}
```

---

## Implementation Guide

### For WASI Preview 1 (Core Modules)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Debug, Serialize, Deserialize)]
struct WasmRequest {
    method: String,
    path: String,
    #[serde(default)]
    query: HashMap<String, String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WasmResponse {
    status: u16,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

fn main() {
    // 1. Read request from stdin
    let mut request_json = String::new();
    io::stdin().read_to_string(&mut request_json)
        .expect("Failed to read request from stdin");

    // 2. Parse the request
    let request: WasmRequest = serde_json::from_str(&request_json)
        .expect("Failed to parse WasmRequest JSON");

    // 3. Process the request
    let result = match request.method.as_str() {
        "GET" => handle_get(&request),
        "POST" => handle_post(&request),
        _ => format!("Method {} not supported", request.method),
    };

    // 4. Create response
    let response = WasmResponse {
        status: 200,
        headers: HashMap::new(),
        body: result,
        content_type: Some("application/json".to_string()),
    };

    // 5. Write response to stdout
    let response_json = serde_json::to_string(&response)
        .expect("Failed to serialize response");
    
    io::stdout().write_all(response_json.as_bytes())
        .expect("Failed to write response to stdout");
}

fn handle_get(req: &WasmRequest) -> String {
    // Your GET handler logic
    serde_json::json!({
        "message": "GET request received",
        "path": req.path
    }).to_string()
}

fn handle_post(req: &WasmRequest) -> String {
    // Your POST handler logic
    serde_json::json!({
        "message": "POST request received",
        "body": req.body
    }).to_string()
}
```

### For WASI Preview 2 (Component Model)

Components follow the same stdin/stdout protocol:

```rust
// Component with command interface
wit_bindgen::generate!({
    world: "command",
    exports: {
        "wasi:cli/run@0.2.0": Component
    }
});

struct Component;

impl exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        // Read from stdin (same as P1)
        let mut request_json = String::new();
        std::io::stdin().read_to_string(&mut request_json)
            .map_err(|_| ())?;

        let request: WasmRequest = serde_json::from_str(&request_json)
            .map_err(|_| ())?;

        // Process and write to stdout (same as P1)
        let response = process_request(&request);
        let response_json = serde_json::to_string(&response)
            .map_err(|_| ())?;
        
        print!("{}", response_json);
        
        Ok(())
    }
}
```

---

## Binary Data Support

### Current Approach (v1.0)

Binary data must be encoded as Base64 or hex strings within the JSON body:

```json
{
  "method": "POST",
  "path": "/api/upload",
  "headers": {
    "Content-Type": "application/octet-stream"
  },
  "body": "AQIDBP7+/Q==",  // Base64-encoded binary
  "content_type": "application/octet-stream"
}
```

### Future Enhancement (v2.0 - Planned)

For efficient binary support, a future version may use:
- Multipart encoding with separate binary sections
- Shared memory regions for large payloads
- Streaming I/O via host functions

---

## Error Handling

### Module Errors

If your module encounters an error, return an appropriate HTTP status:

```json
{
  "status": 400,
  "headers": {},
  "body": "{\"error\":\"Invalid request\",\"message\":\"Missing required field: name\"}",
  "content_type": "application/json"
}
```

### Gateway Error Responses

If the module fails to return valid JSON or crashes:

```json
{
  "error": "Execution failed",
  "message": "Module panicked: ...",
  "path": "/api/endpoint",
  "cid": "bafybeig..."
}
```

---

## Performance Considerations

### Request Size Limits

- **Max stdin size:** 16 MB (configurable)
- **Max stdout size:** 16 MB (configurable)
- **Execution timeout:** 10 seconds (configurable)

### Memory Usage

- Modules should parse JSON incrementally for large requests
- Use streaming APIs when possible
- Avoid loading entire body into memory if not needed

### Fuel Limits

WASM modules run with fuel metering enabled:
- **Default fuel:** 1,000,000 instructions
- **Adjustable per deployment**

---

## Headers

### Standard HTTP Headers

Common headers your module might receive:

```json
{
  "Content-Type": "application/json",
  "User-Agent": "PiedPiper/1.0",
  "Authorization": "Bearer <token>",
  "Accept": "application/json",
  "Accept-Encoding": "gzip, deflate, br",
  "Host": "example.pp"
}
```

### Custom Headers

You can read and write custom headers:

```rust
// Reading custom header
if let Some(api_key) = request.headers.get("X-API-Key") {
    // Validate API key
}

// Writing custom header
response.headers.insert(
    "X-Request-ID".to_string(),
    "12345".to_string()
);
```

---

## Query Parameters

Query parameters are pre-parsed and URL-decoded:

```
Request: /api/search?q=hello%20world&limit=10

WasmRequest.query:
{
  "q": "hello world",    // Already URL-decoded
  "limit": "10"
}
```

---

## Content Types

### Supported Content Types

#### JSON (Recommended)
```json
{
  "content_type": "application/json",
  "body": "{\"key\":\"value\"}"
}
```

#### Plain Text
```json
{
  "content_type": "text/plain",
  "body": "Hello, World!"
}
```

#### HTML
```json
{
  "content_type": "text/html",
  "body": "<html><body>Hello</body></html>"
}
```

#### Binary (Base64-encoded)
```json
{
  "content_type": "application/octet-stream",
  "body": "AQIDBA=="
}
```

---

## Testing Your Module

### Manual Test

```bash
# Create test request
echo '{"method":"GET","path":"/test","query":{},"headers":{},"body":"","content_type":null}' | \
  wasmtime run --wasi-modules=experimental-wasi-preview1 your-module.wasm
```

### Integration Test

See `/tests/fixtures/test-echo-api/` for a complete example module that demonstrates the I/O protocol.

Build and test:
```bash
cd tests/fixtures/test-echo-api
cargo build --target wasm32-wasip1 --release
echo '{"method":"POST","path":"/echo","query":{},"headers":{},"body":"test","content_type":"text/plain"}' | \
  wasmtime run target/wasm32-wasip1/release/test_echo_api.wasm
```

Expected output:
```json
{"status":200,"headers":{"X-Test-Module":"test-echo-api","X-Echo-Method":"POST"},"body":"{\"message\":\"Echo API Test\",\"received\":{\"method\":\"POST\",\"path\":\"/echo\",\"query\":{},\"headers\":{},\"body\":\"test\",\"content_type\":\"text/plain\"}}","content_type":"application/json"}
```

---

## Best Practices

### 1. Always Validate Input

```rust
fn validate_request(req: &WasmRequest) -> Result<(), String> {
    if req.path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    
    if req.method != "GET" && req.method != "POST" {
        return Err(format!("Unsupported method: {}", req.method));
    }
    
    Ok(())
}
```

### 2. Set Appropriate Status Codes

```rust
match result {
    Ok(data) => WasmResponse {
        status: 200,  // Success
        body: data,
        ..
    },
    Err(NotFound) => WasmResponse {
        status: 404,  // Not Found
        body: error_json,
        ..
    },
    Err(InvalidInput) => WasmResponse {
        status: 400,  // Bad Request
        body: error_json,
        ..
    },
    Err(_) => WasmResponse {
        status: 500,  // Internal Server Error
        body: error_json,
        ..
    },
}
```

### 3. Include Error Details

```rust
let error_response = WasmResponse {
    status: 400,
    headers: HashMap::new(),
    body: serde_json::json!({
        "error": "validation_failed",
        "message": "Invalid email format",
        "field": "email",
        "value": req.body
    }).to_string(),
    content_type: Some("application/json".to_string()),
};
```

### 4. Log to stderr for Debugging

```rust
eprintln!("Processing request: {} {}", req.method, req.path);
eprintln!("Query params: {:?}", req.query);
```

### 5. Set Content-Type Correctly

```rust
let response = if is_html {
    WasmResponse {
        content_type: Some("text/html".to_string()),
        body: html_content,
        ..
    }
} else {
    WasmResponse {
        content_type: Some("application/json".to_string()),
        body: json_content,
        ..
    }
};
```

---

## Versioning

### Current Version: 1.0

**Breaking changes will increment the major version.**

### Version Negotiation (Future)

```json
{
  "abi_version": "2.0",
  "method": "GET",
  ...
}
```

---

## See Also

- [WASM I/O Guide](WASM_IO_GUIDE.md) - Practical examples and tutorials
- [Host Functions](HOST_FUNCTIONS.md) - Additional capabilities beyond stdin/stdout
- [Gateway Documentation](GATEWAY.md) - How the gateway processes requests
- [Testing Guide](TESTING_GUIDE.md) - Testing strategies for WASM modules

---

## Changelog

### v1.0 (December 22, 2025)
- Initial ABI specification
- Support for WASI P1 and P2
- JSON-based stdin/stdout protocol
- Base64 encoding for binary data

---

*For questions or issues, please file an issue on GitHub or consult the community documentation.*
