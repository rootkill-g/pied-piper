# Host Functions Guide

This document describes the host functions available to WebAssembly modules running in Pied Piper.

## Available Host Functions

## WASI P2 Component Interfaces

Components can import the same host capabilities via component instances named
`host`, `http`, `storage`, and `crypto`. These use component-friendly types
(strings and byte lists) instead of raw memory pointers.

### Component Signatures

- `host.log(message: string) -> ()`
- `host.now_millis() -> s64`
- `host.random_u32() -> u32`
- `http.get(url: string) -> (status: u32, body: list<u8>)`
- `http.post(url: string, body: list<u8>) -> (status: u32, body: list<u8>)`
- `storage.get(key: string) -> (found: bool, value: list<u8>)`
- `storage.set(key: string, value: list<u8>) -> bool`
- `storage.delete(key: string) -> bool`
- `storage.list_count() -> u32`
- `crypto.blake3_hash(data: list<u8>) -> list<u8>`

Notes:
- `storage.get` returns `found=false` and an empty `value` when the key is missing.
- HTTP errors return `status=0` and an empty body.

### 1. HTTP Client Functions (Network I/O)

#### `http.get(url_ptr: i32, url_len: i32, out_ptr: i32, out_max_len: i32) -> i64`

Performs an HTTP GET request.

**Parameters:**
- `url_ptr`: Pointer to the URL string in WASM memory
- `url_len`: Length of the URL string
- `out_ptr`: Pointer to output buffer for response body
- `out_max_len`: Maximum length to write to output buffer

**Returns:**
- High 32 bits: HTTP status code (0 if request failed)
- Low 32 bits: Actual bytes written to output buffer

**Example (WAT):**
```wat
(module
  (import "http" "get" (func $http_get (param i32 i32 i32 i32) (result i64)))
  (memory (export "memory") 1)
  
  ;; ... call $http_get with URL and buffer pointers
)
```

#### `http.post(url_ptr: i32, url_len: i32, body_ptr: i32, body_len: i32, out_ptr: i32, out_max_len: i32) -> i64`

Performs an HTTP POST request.

**Parameters:**
- `url_ptr`: Pointer to the URL string
- `url_len`: Length of the URL string
- `body_ptr`: Pointer to request body data
- `body_len`: Length of request body
- `out_ptr`: Pointer to output buffer for response
- `out_max_len`: Maximum length to write to output buffer

**Returns:**
- High 32 bits: HTTP status code (0 if request failed)
- Low 32 bits: Actual bytes written to output buffer

---

### 2. Key-Value Storage Functions

#### `storage.get(key_ptr: i32, key_len: i32, out_ptr: i32, out_max_len: i32) -> i32`

Retrieves a value from the key-value store.

**Parameters:**
- `key_ptr`: Pointer to the key string
- `key_len`: Length of the key string
- `out_ptr`: Pointer to output buffer for value
- `out_max_len`: Maximum length to write

**Returns:**
- Length of value written (or -1 if key not found)

**Example (WAT):**
```wat
(module
  (import "storage" "get" (func $storage_get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  
  ;; ... call $storage_get with key and buffer pointers
)
```

#### `storage.set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32`

Stores a key-value pair.

**Parameters:**
- `key_ptr`: Pointer to the key string
- `key_len`: Length of the key string
- `value_ptr`: Pointer to value data
- `value_len`: Length of value data

**Returns:**
- 0 on success

#### `storage.delete(key_ptr: i32, key_len: i32) -> i32`

Deletes a key from storage.

**Parameters:**
- `key_ptr`: Pointer to the key string
- `key_len`: Length of the key string

**Returns:**
- 1 if key existed and was deleted, 0 if key didn't exist

#### `storage.list_count() -> i32`

Returns the number of keys in storage.

**Returns:**
- Count of keys in storage

---

### 3. Cryptography Functions

#### `crypto.blake3_hash(data_ptr: i32, data_len: i32, out_ptr: i32) -> ()`

Computes the Blake3 hash of input data.

**Parameters:**
- `data_ptr`: Pointer to input data
- `data_len`: Length of input data
- `out_ptr`: Pointer to 32-byte output buffer for hash

**Note:** Output buffer must be at least 32 bytes.

---

### 4. Logging and Utilities

#### `host.log(ptr: i32, len: i32) -> ()`

Logs a message from the WASM module.

**Parameters:**
- `ptr`: Pointer to message string
- `len`: Length of message string

#### `host.now_millis() -> i64`

Returns current time in milliseconds since Unix epoch.

**Returns:**
- Current timestamp in milliseconds

#### `host.random_u32() -> u32`

Generates a random 32-bit unsigned integer.

**Returns:**
- Random u32 value

---

## Module Dependency Resolution

Modules can now declare dependencies in their metadata. The loader will automatically resolve and load dependencies in the correct order.

**Example ModuleInfo with dependencies:**
```rust
ModuleInfo {
    cid: ModuleCid::from_bytes(&module_bytes),
    name: Some("my-app".to_string()),
    version: Some("1.0.0".to_string()),
    size: module_bytes.len(),
    dependencies: vec![
        ModuleCid::new("bafybeigdyrzt...".to_string()), // http-client module
        ModuleCid::new("bafybeif5vtzk...".to_string()), // database module
    ],
    author: Some("alice@example.com".to_string()),
    description: Some("My awesome app".to_string()),
}
```

**Loading with dependencies:**
```rust
let loader = ModuleLoader::new(cache_dir).await?;
let modules = loader.load_with_dependencies(&main_cid).await?;

// modules contains all dependencies in load order, followed by main module
for (info, bytes) in modules {
    println!("Loading: {} ({})", info.name.unwrap_or_default(), info.cid);
}
```

---

## Security Notes

1. **HTTP Client**: 
   - 30-second timeout on all requests
   - No automatic redirect following
   - Sandboxed - cannot access local network

2. **Storage**:
   - In-memory only (not persisted to disk yet)
   - Shared across all modules in the same runtime
   - No size limits enforced (future enhancement)

3. **Cryptography**:
   - Uses Blake3 for hashing (fast and secure)
   - Fixed 32-byte output

4. **Resource Limits**:
   - All functions use async/await internally
   - Blocking operations run in Tokio's blocking pool
   - Memory access is bounds-checked

---

## Implementation Details

All host functions are implemented in `src/wasm/host.rs`:

- **HostFunctions**: Main struct containing shared state
- **NetworkHostFunctions**: HTTP client functions
- **StorageHostFunctions**: Key-value storage functions
- **CryptoHostFunctions**: Cryptographic operations

Host functions are automatically added to the WASM linker when creating a runtime.

---

## Future Enhancements

Planned improvements:

1. **Persistent Storage**: Save KV store to disk
2. **HTTP Headers**: Support for custom headers in HTTP requests
3. **WebSockets**: Real-time bidirectional communication
4. **File I/O**: Sandboxed file system access
5. **Networking**: TCP/UDP socket support
6. **More Crypto**: Support for signing, verification, encryption

---

Last updated: December 22, 2025
