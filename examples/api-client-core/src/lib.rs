use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

// External imports from the host
extern "C" {
    fn host_log(ptr: *const u8, len: usize);
    fn host_now_millis() -> i64;
    fn host_random_u32() -> u32;
    
    // HTTP functions - return status via first param pointer
    fn host_http_get(url_ptr: *const u8, url_len: usize, body_ptr: *mut u8, body_len: *mut usize) -> u32;
    fn host_http_post(url_ptr: *const u8, url_len: usize, body_in_ptr: *const u8, body_in_len: usize, body_out_ptr: *mut u8, body_out_len: *mut usize) -> u32;
    
    // Storage functions
    fn host_storage_get(key_ptr: *const u8, key_len: usize, val_ptr: *mut u8, val_len: *mut usize) -> u32; // returns 1 if found, 0 if not
    fn host_storage_set(key_ptr: *const u8, key_len: usize, val_ptr: *const u8, val_len: usize) -> u32;
    fn host_storage_delete(key_ptr: *const u8, key_len: usize) -> u32;
    fn host_storage_count() -> u32;
    
    // Crypto functions
    fn host_blake3_hash(data_ptr: *const u8, data_len: usize, hash_ptr: *mut u8); // hash is always 32 bytes
}

// Safe wrappers
fn log(message: &str) {
    unsafe {
        host_log(message.as_ptr(), message.len());
    }
}

fn now_millis() -> i64 {
    unsafe { host_now_millis() }
}

fn random_u32() -> u32 {
    unsafe { host_random_u32() }
}

fn http_get(url: &str) -> (u32, Vec<u8>) {
    let mut body = vec![0u8; 65536]; // 64KB buffer
    let mut body_len = body.len();
    
    unsafe {
        let status = host_http_get(url.as_ptr(), url.len(), body.as_mut_ptr(), &mut body_len);
        body.truncate(body_len);
        (status, body)
    }
}

fn http_post(url: &str, body_in: &[u8]) -> (u32, Vec<u8>) {
    let mut body_out = vec![0u8; 65536]; // 64KB buffer
    let mut body_out_len = body_out.len();
    
    unsafe {
        let status = host_http_post(
            url.as_ptr(), url.len(),
            body_in.as_ptr(), body_in.len(),
            body_out.as_mut_ptr(), &mut body_out_len
        );
        body_out.truncate(body_out_len);
        (status, body_out)
    }
}

fn storage_get(key: &str) -> Option<Vec<u8>> {
    let mut val = vec![0u8; 65536]; // 64KB buffer
    let mut val_len = val.len();
    
    unsafe {
        let found = host_storage_get(key.as_ptr(), key.len(), val.as_mut_ptr(), &mut val_len);
        if found == 1 {
            val.truncate(val_len);
            Some(val)
        } else {
            None
        }
    }
}

fn storage_set(key: &str, value: &[u8]) -> bool {
    unsafe {
        host_storage_set(key.as_ptr(), key.len(), value.as_ptr(), value.len()) == 1
    }
}

fn storage_delete(key: &str) -> bool {
    unsafe {
        host_storage_delete(key.as_ptr(), key.len()) == 1
    }
}

fn storage_count() -> u32 {
    unsafe { host_storage_count() }
}

fn blake3_hash(data: &[u8]) -> Vec<u8> {
    let mut hash = vec![0u8; 32];
    unsafe {
        host_blake3_hash(data.as_ptr(), data.len(), hash.as_mut_ptr());
    }
    hash
}

// Request/Response types matching gateway's WasmRequest structure
#[derive(Debug, Deserialize)]
struct Request {
    method: String,
    path: String,
    #[serde(default)]
    query: std::collections::HashMap<String, String>,
    #[serde(default)]
    headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    body: String,
}

#[derive(Debug, Serialize)]
struct Response {
    status: u16,
    #[serde(default)]
    headers: std::collections::HashMap<String, String>,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
}

impl Response {
    fn ok(body: impl Into<String>) -> Self {
        Response {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: body.into(),
            content_type: Some("application/json".to_string()),
        }
    }
    
    fn error(status: u16, error: impl Into<String>) -> Self {
        let error_body = serde_json::json!({
            "error": error.into()
        }).to_string();
        
        Response {
            status,
            headers: std::collections::HashMap::new(),
            body: error_body,
            content_type: Some("application/json".to_string()),
        }
    }
}

// Main handler
#[no_mangle]
pub extern "C" fn _start() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Read request from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    let request: Request = serde_json::from_str(&input)?;
    
    log(&format!("Core module handling: {} {}", request.method, request.path));
    
    // Route the request
    let response = match request.path.as_str() {
        "health" | "/health" => handle_health(),
        "external" | "/external" => handle_external(),
        "cache" | "/cache" => handle_cache(),
        "stats" | "/stats" => handle_stats(),
        "hash" | "/hash" => handle_hash(&request),
        "counter" | "/counter" => handle_counter(),
        _ => Response::error(404, "Not found"),
    };
    
    // Write response to stdout
    let output = serde_json::to_string(&response)?;
    io::stdout().write_all(output.as_bytes())?;
    io::stdout().flush()?;
    
    Ok(())
}

fn handle_health() -> Response {
    let timestamp = now_millis();
    let random = random_u32();
    
    Response::ok(serde_json::json!({
        "status": "healthy",
        "module": "api-client-core",
        "timestamp": timestamp,
        "random": random,
        "type": "core-wasm"
    }).to_string())
}

fn handle_external() -> Response {
    log("Fetching from external API with caching");
    
    // Check cache first
    if let Some(cached) = storage_get("external_cache") {
        if let Ok(data) = String::from_utf8(cached) {
            log("Cache hit!");
            return Response::ok(serde_json::json!({
                "source": "cache",
                "data": data
            }).to_string());
        }
    }
    
    // Fetch from external API
    log("Cache miss, fetching from API");
    let (status, body) = http_get("https://meowfacts.herokuapp.com/");
    
    if status == 200 {
        let data = String::from_utf8_lossy(&body).to_string();
        
        // Cache the result
        storage_set("external_cache", data.as_bytes());
        log("Cached the result");
        
        Response::ok(serde_json::json!({
            "source": "api",
            "data": data,
            "status": status
        }).to_string())
    } else {
        Response::error(502, format!("External API error: {}", status))
    }
}

fn handle_cache() -> Response {
    // Demonstrate KV operations
    storage_set("test_key", b"test_value");
    
    let value = storage_get("test_key")
        .map(|v| String::from_utf8_lossy(&v).to_string())
        .unwrap_or_else(|| "not found".to_string());
    
    let count = storage_count();
    
    Response::ok(serde_json::json!({
        "operations": "set and get",
        "key": "test_key",
        "value": value,
        "total_keys": count
    }).to_string())
}

fn handle_stats() -> Response {
    let count = storage_count();
    let timestamp = now_millis();
    let random = random_u32();
    
    Response::ok(serde_json::json!({
        "storage_keys": count,
        "timestamp": timestamp,
        "random": random
    }).to_string())
}

fn handle_hash(request: &Request) -> Response {
    let data = if request.body.is_empty() {
        "Hello, BLAKE3!"
    } else {
        &request.body
    };
    
    let hash = blake3_hash(data.as_bytes());
    let hash_hex = hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    
    log(&format!("Computed BLAKE3 hash: {}", hash_hex));
    
    Response::ok(serde_json::json!({
        "input": data,
        "hash": hash_hex,
        "algorithm": "blake3"
    }).to_string())
}

fn handle_counter() -> Response {
    // Get current counter
    let current = storage_get("counter")
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    
    let next = current + 1;
    
    // Store incremented value
    storage_set("counter", next.to_string().as_bytes());
    
    log(&format!("Counter incremented: {} -> {}", current, next));
    
    Response::ok(serde_json::json!({
        "previous": current,
        "current": next
    }).to_string())
}
