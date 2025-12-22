# Pied Piper Host Functions API Reference

This document describes all host functions available to WebAssembly modules running in Pied Piper. These functions provide capabilities that WASM modules cannot implement themselves, such as network access, persistent storage, and cryptographic operations.

## Table of Contents

- [Overview](#overview)
- [Host Functions (Core Modules)](#host-functions-core-modules)
  - [Logging](#logging)
  - [Time](#time)
  - [Random](#random)
- [HTTP Client](#http-client)
  - [V1 API (Memory-based)](#v1-api-memory-based)
  - [V2 API (Return-based)](#v2-api-return-based)
- [Storage](#storage)
  - [V1 API (Memory-based)](#storage-v1-api)
  - [V2 API (Return-based)](#storage-v2-api)
- [Cryptography](#cryptography)
  - [V1 API (Memory-based)](#crypto-v1-api)
  - [V2 API (Return-based)](#crypto-v2-api)
- [Component Model (WASI P2)](#component-model-wasi-p2)
- [Error Handling](#error-handling)
- [Examples](#examples)

## Overview

Pied Piper provides host functions in two styles:

1. **V1 API (Memory-based)**: Traditional WASM-style where data is passed via linear memory pointers
2. **V2 API (Return-based)**: Modern style where data is returned directly from functions

Both APIs are available for compatibility. New modules should use V2 APIs when possible for simpler code.

### Importing Host Functions

In your WASM module, declare host function imports:

**Rust (with `extern "C"`):**
```rust
extern "C" {
    fn host_log(ptr: i32, len: i32);
    fn host_now_millis() -> i64;
    fn host_random_u32() -> u32;
}
```

**AssemblyScript:**
```typescript
@external("env", "host_log")
declare function host_log(ptr: i32, len: i32): void;

@external("env", "host_now_millis")
declare function host_now_millis(): i64;
```

## Host Functions (Core Modules)

### Logging

#### `host_log`

Write a log message visible in gateway logs and metrics.

**Signature:**
```c
void host_log(i32 ptr, i32 len)
```

**Parameters:**
- `ptr`: Pointer to UTF-8 encoded string in linear memory
- `len`: Length of string in bytes

**Returns:** None

**Example (Rust):**
```rust
extern "C" {
    fn host_log(ptr: i32, len: i32);
}

pub fn log(message: &str) {
    unsafe {
        host_log(message.as_ptr() as i32, message.len() as i32);
    }
}

fn main() {
    log("Hello from WASM!");
}
```

**Example (AssemblyScript):**
```typescript
@external("env", "host_log")
declare function host_log(ptr: i32, len: i32): void;

function log(message: string): void {
    const buf = String.UTF8.encode(message);
    host_log(changetype<i32>(buf), buf.byteLength);
}

log("Hello from WASM!");
```

**Gateway Output:**
```
[INFO] Wasm module log: Hello from WASM!
```

### Time

#### `host_now_millis`

Get current Unix timestamp in milliseconds.

**Signature:**
```c
i64 host_now_millis()
```

**Parameters:** None

**Returns:** `i64` - Milliseconds since Unix epoch (1970-01-01 00:00:00 UTC)

**Example (Rust):**
```rust
extern "C" {
    fn host_now_millis() -> i64;
}

fn get_timestamp() -> i64 {
    unsafe { host_now_millis() }
}

fn main() {
    let now = get_timestamp();
    println!("Current time: {} ms", now);
}
```

**Example (AssemblyScript):**
```typescript
@external("env", "host_now_millis")
declare function host_now_millis(): i64;

const now = host_now_millis();
console.log(`Current time: ${now} ms`);
```

**Use Cases:**
- Request timestamps
- Rate limiting
- Cache expiration
- Performance measurement

### Random

#### `host_random_u32`

Generate a cryptographically-secure random 32-bit unsigned integer.

**Signature:**
```c
u32 host_random_u32()
```

**Parameters:** None

**Returns:** `u32` - Random value between 0 and 4,294,967,295

**Example (Rust):**
```rust
extern "C" {
    fn host_random_u32() -> u32;
}

fn generate_id() -> u32 {
    unsafe { host_random_u32() }
}

fn main() {
    let id = generate_id();
    println!("Generated ID: {}", id);
}
```

**Use Cases:**
- Generating unique IDs
- Randomization
- Session tokens
- Load distribution

## HTTP Client

Make outgoing HTTP requests from WASM modules.

### V1 API (Memory-based)

#### `http_get`

Perform HTTP GET request.

**Signature:**
```c
i64 http_get(i32 url_ptr, i32 url_len, i32 out_ptr, i32 out_max_len)
```

**Parameters:**
- `url_ptr`: Pointer to URL string in memory
- `url_len`: Length of URL in bytes
- `out_ptr`: Pointer to output buffer for response body
- `out_max_len`: Maximum bytes to write to output buffer

**Returns:** `i64` - Packed result:
- Upper 32 bits: HTTP status code (200, 404, 500, etc.)
- Lower 32 bits: Actual response body length written

**Example (Rust):**
```rust
extern "C" {
    fn http_get(url_ptr: i32, url_len: i32, out_ptr: i32, out_max_len: i32) -> i64;
}

pub fn get(url: &str, buffer: &mut [u8]) -> Result<(u16, usize), String> {
    let result = unsafe {
        http_get(
            url.as_ptr() as i32,
            url.len() as i32,
            buffer.as_mut_ptr() as i32,
            buffer.len() as i32,
        )
    };
    
    let status = (result >> 32) as u16;
    let body_len = (result & 0xFFFFFFFF) as usize;
    
    if status == 0 {
        return Err("HTTP request failed".to_string());
    }
    
    Ok((status, body_len))
}

fn main() {
    let mut buffer = vec![0u8; 16384]; // 16KB buffer
    
    match get("https://api.example.com/data", &mut buffer) {
        Ok((status, len)) => {
            println!("Status: {}", status);
            let body = std::str::from_utf8(&buffer[..len]).unwrap();
            println!("Body: {}", body);
        }
        Err(e) => println!("Error: {}", e),
    }
}
```

#### `http_post`

Perform HTTP POST request.

**Signature:**
```c
i64 http_post(i32 url_ptr, i32 url_len, i32 body_ptr, i32 body_len, i32 out_ptr, i32 out_max_len)
```

**Parameters:**
- `url_ptr`: Pointer to URL string
- `url_len`: Length of URL
- `body_ptr`: Pointer to request body
- `body_len`: Length of request body
- `out_ptr`: Pointer to output buffer
- `out_max_len`: Maximum response size

**Returns:** `i64` - Packed (status_code, response_length)

**Example (Rust):**
```rust
extern "C" {
    fn http_post(
        url_ptr: i32, url_len: i32,
        body_ptr: i32, body_len: i32,
        out_ptr: i32, out_max_len: i32
    ) -> i64;
}

pub fn post(url: &str, body: &[u8], out_buffer: &mut [u8]) -> Result<(u16, usize), String> {
    let result = unsafe {
        http_post(
            url.as_ptr() as i32, url.len() as i32,
            body.as_ptr() as i32, body.len() as i32,
            out_buffer.as_mut_ptr() as i32, out_buffer.len() as i32,
        )
    };
    
    let status = (result >> 32) as u16;
    let len = (result & 0xFFFFFFFF) as usize;
    
    if status == 0 {
        return Err("HTTP POST failed".to_string());
    }
    
    Ok((status, len))
}
```

### V2 API (Return-based)

#### `http_get_v2`

Modern HTTP GET with separate status and body returns.

**Signature:**
```c
(i32 status, bytes body) http_get_v2(string url)
```

**Returns:**
- Status code (0 on network error)
- Response body bytes

**Example (Rust with WIT):**
```rust
// Automatically generated from WIT
let (status, body) = http_get_v2("https://api.example.com/data");

if status == 200 {
    let text = String::from_utf8(body)?;
    println!("Success: {}", text);
}
```

#### `http_post_v2`

Modern HTTP POST with JSON support.

**Signature:**
```c
(i32 status, bytes body) http_post_v2(string url, bytes body)
```

**Example (Rust):**
```rust
let payload = serde_json::json!({
    "name": "test",
    "value": 42
}).to_string();

let (status, response) = http_post_v2(
    "https://api.example.com/create",
    payload.as_bytes()
);

if status == 201 {
    println!("Created successfully");
}
```

## Storage

Persistent key-value storage across requests.

### Storage V1 API

#### `storage_get`

Read value from storage.

**Signature:**
```c
i32 storage_get(i32 key_ptr, i32 key_len, i32 out_ptr, i32 out_max_len)
```

**Parameters:**
- `key_ptr`: Pointer to key string
- `key_len`: Length of key
- `out_ptr`: Pointer to output buffer
- `out_max_len`: Maximum value size

**Returns:** `i32` - Number of bytes written, or -1 if key not found

**Example (Rust):**
```rust
extern "C" {
    fn storage_get(key_ptr: i32, key_len: i32, out_ptr: i32, out_max_len: i32) -> i32;
}

pub fn get_value(key: &str, buffer: &mut [u8]) -> Option<usize> {
    let len = unsafe {
        storage_get(
            key.as_ptr() as i32,
            key.len() as i32,
            buffer.as_mut_ptr() as i32,
            buffer.len() as i32,
        )
    };
    
    if len < 0 {
        None
    } else {
        Some(len as usize)
    }
}

fn main() {
    let mut buffer = vec![0u8; 1024];
    
    if let Some(len) = get_value("counter", &mut buffer) {
        let value = std::str::from_utf8(&buffer[..len]).unwrap();
        println!("Counter: {}", value);
    } else {
        println!("Counter not found");
    }
}
```

#### `storage_set`

Write value to storage.

**Signature:**
```c
i32 storage_set(i32 key_ptr, i32 key_len, i32 value_ptr, i32 value_len)
```

**Parameters:**
- `key_ptr`: Pointer to key string
- `key_len`: Length of key
- `value_ptr`: Pointer to value bytes
- `value_len`: Length of value

**Returns:** `i32` - 1 on success, 0 on failure

**Example (Rust):**
```rust
extern "C" {
    fn storage_set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32;
}

pub fn set_value(key: &str, value: &[u8]) -> bool {
    let result = unsafe {
        storage_set(
            key.as_ptr() as i32,
            key.len() as i32,
            value.as_ptr() as i32,
            value.len() as i32,
        )
    };
    
    result == 1
}

fn main() {
    let counter = 42;
    let value = counter.to_string();
    
    if set_value("counter", value.as_bytes()) {
        println!("Counter saved");
    }
}
```

#### `storage_delete`

Remove key from storage.

**Signature:**
```c
i32 storage_delete(i32 key_ptr, i32 key_len)
```

**Returns:** `i32` - 1 if deleted, 0 if key didn't exist

#### `storage_count`

Get number of keys in storage.

**Signature:**
```c
i32 storage_count()
```

**Returns:** `i32` - Total number of keys

### Storage V2 API

#### `storage_get_v2`

**Signature:**
```c
(bool found, bytes value) storage_get_v2(string key)
```

**Example (Rust):**
```rust
let (found, value) = storage_get_v2("user:123");

if found {
    let user = String::from_utf8(value)?;
    println!("User data: {}", user);
}
```

#### `storage_set_v2`

**Signature:**
```c
bool storage_set_v2(string key, bytes value)
```

**Example (Rust):**
```rust
let user_data = serde_json::json!({
    "id": 123,
    "name": "Alice"
}).to_string();

if storage_set_v2("user:123", user_data.as_bytes()) {
    println!("User saved");
}
```

## Cryptography

Cryptographic hashing functions.

### Crypto V1 API

#### `crypto_blake3`

Compute BLAKE3 hash (32 bytes).

**Signature:**
```c
void crypto_blake3(i32 data_ptr, i32 data_len, i32 out_ptr)
```

**Parameters:**
- `data_ptr`: Pointer to input data
- `data_len`: Length of input
- `out_ptr`: Pointer to 32-byte output buffer (must be >= 32 bytes)

**Returns:** None (writes directly to output buffer)

**Example (Rust):**
```rust
extern "C" {
    fn crypto_blake3(data_ptr: i32, data_len: i32, out_ptr: i32);
}

pub fn hash_blake3(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    
    unsafe {
        crypto_blake3(
            data.as_ptr() as i32,
            data.len() as i32,
            hash.as_mut_ptr() as i32,
        );
    }
    
    hash
}

fn main() {
    let data = b"Hello, World!";
    let hash = hash_blake3(data);
    
    // Convert to hex
    let hex: String = hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    
    println!("BLAKE3: {}", hex);
}
```

#### `crypto_sha256`

Compute SHA-256 hash (32 bytes).

**Signature:**
```c
void crypto_sha256(i32 data_ptr, i32 data_len, i32 out_ptr)
```

**Usage:** Same as `crypto_blake3`

### Crypto V2 API

#### `crypto_blake3_v2`

**Signature:**
```c
bytes crypto_blake3_v2(bytes data)
```

**Returns:** 32-byte hash

**Example (Rust):**
```rust
let data = b"Hello, World!";
let hash = crypto_blake3_v2(data);

assert_eq!(hash.len(), 32);
println!("Hash: {:?}", hash);
```

## Component Model (WASI P2)

For WASM components using the component model, functions are organized into interfaces.

### Package: `component:api-client`

#### Interface: `host`

```wit
interface host {
    log: func(message: string)
    now-millis: func() -> s64
    random-u32: func() -> u32
}
```

#### Interface: `http`

```wit
interface http {
    get: func(url: string) -> tuple<u32, list<u8>>
    post: func(url: string, body: list<u8>) -> tuple<u32, list<u8>>
}
```

**Example (Rust Component):**
```rust
wit_bindgen::generate!({
    world: "api-client",
    exports: {
        world: Component,
    },
});

use exports::component::api_client::http;

impl http::Guest for Component {
    fn get(url: String) -> (u32, Vec<u8>) {
        // Call imported host function
        let (status, body) = http::get(&url);
        (status, body)
    }
}
```

#### Interface: `storage`

```wit
interface storage {
    get: func(key: string) -> tuple<bool, list<u8>>
    set: func(key: string, value: list<u8>) -> bool
    delete: func(key: string) -> bool
    list-count: func() -> u32
}
```

#### Interface: `crypto`

```wit
interface crypto {
    blake3-hash: func(data: list<u8>) -> list<u8>
}
```

## Error Handling

### HTTP Errors

| Status | Meaning |
|--------|---------|
| 0 | Network error (connection failed, timeout, DNS failure) |
| 200-299 | Success |
| 400-499 | Client error (bad request, not found, unauthorized) |
| 500-599 | Server error |

**Example:**
```rust
let (status, body) = http_get_v2("https://example.com/api");

match status {
    0 => println!("Network error"),
    200..=299 => println!("Success"),
    404 => println!("Not found"),
    500..=599 => println!("Server error"),
    _ => println!("Unexpected status: {}", status),
}
```

### Storage Errors

Storage functions return success/failure booleans. Keys and values are binary-safe (can contain any bytes).

**Limits:**
- Maximum key size: 256 bytes
- Maximum value size: 1 MB
- No storage quota (in-memory only currently)

### Crypto Errors

Crypto functions never fail (they panic on invalid input). Ensure output buffers are correctly sized:
- BLAKE3: 32 bytes
- SHA-256: 32 bytes

## Examples

### Complete REST API Module

```rust
extern "C" {
    fn host_log(ptr: i32, len: i32);
    fn storage_get_v2(key_ptr: i32, key_len: i32) -> i64;
    fn storage_set_v2(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32;
}

// Helper to log messages
fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr() as i32, msg.len() as i32);
    }
}

// Handle HTTP request
#[no_mangle]
pub extern "C" fn handle_request() -> i32 {
    log("Processing request");
    
    // Read counter from storage
    let mut buffer = vec![0u8; 1024];
    let result = unsafe {
        storage_get_v2(
            b"counter".as_ptr() as i32,
            7, // "counter".len()
        )
    };
    
    let found = (result >> 32) != 0;
    let len = (result & 0xFFFFFFFF) as usize;
    
    let counter: u32 = if found && len > 0 {
        // Parse counter value
        String::from_utf8_lossy(&buffer[..len])
            .parse()
            .unwrap_or(0)
    } else {
        0
    };
    
    // Increment counter
    let new_counter = counter + 1;
    let counter_str = new_counter.to_string();
    
    // Save back to storage
    unsafe {
        storage_set_v2(
            b"counter".as_ptr() as i32,
            7,
            counter_str.as_ptr() as i32,
            counter_str.len() as i32,
        );
    }
    
    log(&format!("Counter incremented to {}", new_counter));
    
    // Return success status
    200
}
```

### Fetching External API

```rust
extern "C" {
    fn http_get_v2(url_ptr: i32, url_len: i32) -> i64;
}

fn fetch_joke() -> Result<String, String> {
    let url = "https://official-joke-api.appspot.com/random_joke";
    
    let result = unsafe {
        http_get_v2(url.as_ptr() as i32, url.len() as i32)
    };
    
    let status = (result >> 32) as u32;
    let len = (result & 0xFFFFFFFF) as usize;
    
    if status != 200 {
        return Err(format!("HTTP error: {}", status));
    }
    
    // Read response from memory (implementation-specific)
    let body = read_response_body(len);
    
    String::from_utf8(body)
        .map_err(|_| "Invalid UTF-8".to_string())
}
```

### Content-Addressed Storage

```rust
extern "C" {
    fn crypto_blake3(data_ptr: i32, data_len: i32, out_ptr: i32);
    fn storage_set_v2(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32;
}

fn store_content(data: &[u8]) -> String {
    // Compute content hash
    let mut hash = [0u8; 32];
    unsafe {
        crypto_blake3(
            data.as_ptr() as i32,
            data.len() as i32,
            hash.as_mut_ptr() as i32,
        );
    }
    
    // Convert to hex string for key
    let key: String = hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    
    // Store data by hash
    unsafe {
        storage_set_v2(
            key.as_ptr() as i32,
            key.len() as i32,
            data.as_ptr() as i32,
            data.len() as i32,
        );
    }
    
    key
}
```

## Best Practices

1. **Buffer Sizing**: Always allocate sufficient buffer space for HTTP responses (16KB+ recommended)

2. **Error Handling**: Check return values and handle errors gracefully

3. **Memory Safety**: Ensure pointers are valid and buffers don't overflow

4. **Use V2 APIs**: Prefer V2 APIs for new code (cleaner, safer)

5. **Logging**: Use `host_log` for debugging and monitoring

6. **Storage Keys**: Use namespaced keys to avoid collisions (e.g., `user:123`, `session:abc`)

7. **Request Timeouts**: HTTP requests timeout after 30 seconds

8. **Binary Data**: Storage and HTTP support binary data (not just UTF-8)

## Roadmap

Future host functions under consideration:

- ⏳ WebSocket client connections
- ⏳ Database queries (SQLite/PostgreSQL)
- ⏳ File system access (sandboxed)
- ⏳ DNS lookups
- ⏳ TCP/UDP sockets
- ⏳ Advanced crypto (signatures, encryption)
- ⏳ Pub/sub messaging

## Support

For questions or issues with host functions:
- Check examples in `examples/` directory
- Review integration tests in `tests/`
- Open an issue on GitHub

---

**Last Updated:** December 22, 2025  
**Version:** 0.5.0
