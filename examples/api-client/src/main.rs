use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

// Import host functions - use pre-generated bindings
mod bindings;
use bindings::component::api_client::{crypto, host, http, storage};

/// Request structure matching the gateway's WasmRequest format
#[derive(Debug, Deserialize)]
struct WasmRequest {
    method: String,
    path: String,
    #[serde(default)]
    query: HashMap<String, String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

/// Response structure matching the gateway's WasmResponse format
#[derive(Debug, Serialize)]
struct WasmResponse {
    status: u16,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    status: String,
    data: Option<serde_json::Value>,
    error: Option<String>,
    timestamp: i64,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    data: Vec<u8>,
    created_at: i64,
    ttl_seconds: i64,
}

impl WasmResponse {
    fn ok(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }

    fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.to_string(),
            content_type: Some("application/json".to_string()),
        }
    }
}

fn main() {
    // Read request JSON from stdin
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    // Parse the request
    let request: WasmRequest = match serde_json::from_str(&input) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to parse request: {}", e);
            let response = WasmResponse::error(400, &format!("Invalid request: {}", e));
            let _ = writeln!(io::stdout(), "{}", serde_json::to_string(&response).unwrap());
            return;
        }
    };

    // Route the request
    let response = handle_request(&request.path, &request.body);

    // Write response JSON to stdout
    match serde_json::to_string(&response) {
        Ok(json) => {
            let _ = writeln!(io::stdout(), "{}", json);
        }
        Err(e) => {
            eprintln!("Failed to serialize response: {}", e);
        }
    }
}

/// Main request handler that routes to different endpoints
fn handle_request(path: &str, body: &str) -> WasmResponse {
    host::log(&format!("Request: {} - {}", path, body));

    let response_body = match path {
        "/health" => handle_health(),
        "/external" => handle_external_api(body),
        "/cache" => handle_cache(body),
        "/stats" => handle_stats(),
        "/hash" => handle_hash(body),
        "/counter" => handle_counter(),
        _ => error_response("Not found", 404),
    };

    WasmResponse::ok(response_body)
}

// ============================================================================
// ENDPOINT HANDLERS
// ============================================================================

/// Health check endpoint
fn handle_health() -> String {
    success_response(serde_json::json!({
        "status": "healthy",
        "uptime_ms": host::now_millis(),
        "storage_keys": storage::list_count(),
    }))
}

/// Call an external API and cache the response
fn handle_external_api(body: &str) -> String {
    #[derive(Deserialize)]
    struct Request {
        url: String,
        #[serde(default)]
        use_cache: bool,
    }

    let req: Request = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e), 400),
    };

    // Check cache first
    if req.use_cache {
        if let Some(cached) = get_cached(&req.url, 300) {
            // 5 minute TTL
            host::log(&format!("Cache hit: {}", req.url));
            return match String::from_utf8(cached) {
                Ok(s) => s,
                Err(_) => error_response("Invalid cached data", 500),
            };
        }
    }

    // Make HTTP request
    host::log(&format!("Fetching: {}", req.url));
    let (status, body): (u32, Vec<u8>) = http::get(&req.url);

    if status == 0 {
        return error_response("Network error", 500);
    }

    if status != 200 {
        return error_response(&format!("HTTP {}", status), status as i32);
    }

    // Parse response
    let response_text = match String::from_utf8(body) {
        Ok(s) => s,
        Err(_) => return error_response("Invalid UTF-8 response", 500),
    };

    // Try to parse as JSON
    let data = serde_json::from_str::<serde_json::Value>(&response_text)
        .unwrap_or(serde_json::Value::String(response_text.clone()));

    // Cache the result
    if req.use_cache {
        let response = success_response(data.clone());
        set_cached(&req.url, response.as_bytes(), 300);
    }

    success_response(data)
}

/// Cache management endpoint
fn handle_cache(body: &str) -> String {
    #[derive(Deserialize)]
    struct Request {
        action: String,
        key: Option<String>,
        value: Option<String>,
    }

    let req: Request = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e), 400),
    };

    match req.action.as_str() {
        "get" => {
            let key = match req.key {
                Some(k) => k,
                None => return error_response("Missing key", 400),
            };

            let (found, value) = storage::get(&key);
            if found {
                let data = String::from_utf8_lossy(&value);
                success_response(serde_json::json!({
                    "key": key,
                    "value": data.to_string(),
                    "found": true,
                }))
            } else {
                success_response(serde_json::json!({
                    "key": key,
                    "found": false,
                }))
            }
        }
        "set" => {
            let key = match req.key {
                Some(k) => k,
                None => return error_response("Missing key", 400),
            };
            let value = match req.value {
                Some(v) => v,
                None => return error_response("Missing value", 400),
            };

            let success = storage::set(&key, value.as_bytes());
            success_response(serde_json::json!({
                "key": key,
                "success": success,
            }))
        }
        "delete" => {
            let key = match req.key {
                Some(k) => k,
                None => return error_response("Missing key", 400),
            };

            let existed = storage::delete(&key);
            success_response(serde_json::json!({
                "key": key,
                "deleted": existed,
            }))
        }
        "list" => success_response(serde_json::json!({
            "count": storage::list_count(),
        })),
        _ => error_response("Unknown action", 400),
    }
}

/// Statistics endpoint
fn handle_stats() -> String {
    let count = storage::list_count();
    let timestamp = host::now_millis();
    let random = host::random_u32();

    success_response(serde_json::json!({
        "storage": {
            "total_keys": count,
        },
        "system": {
            "timestamp_ms": timestamp,
            "random_sample": random,
        }
    }))
}

/// Hash data using BLAKE3
fn handle_hash(body: &str) -> String {
    #[derive(Deserialize)]
    struct Request {
        data: String,
    }

    let req: Request = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e), 400),
    };

    let hash = crypto::blake3_hash(req.data.as_bytes());
    let hex = hex_encode(&hash);

    success_response(serde_json::json!({
        "data": req.data,
        "hash": hex,
        "algorithm": "blake3",
    }))
}

/// Counter with automatic increment
fn handle_counter() -> String {
    let key = "global_counter";
    let mut buffer = [0u8; 8];

    // Get current value
    let current = {
        let (found, value): (bool, Vec<u8>) = storage::get(key);
        if found && value.len() >= 8 {
            u64::from_le_bytes([
                value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
            ])
        } else {
            0
        }
    };

    // Increment
    let new_value = current + 1;
    buffer.copy_from_slice(&new_value.to_le_bytes());

    // Save
    storage::set(key, &buffer);

    success_response(serde_json::json!({
        "counter": new_value,
        "previous": current,
    }))
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Get cached data if it exists and hasn't expired
fn get_cached(key: &str, ttl_seconds: i64) -> Option<Vec<u8>> {
    let cache_key = format!("cache:{}", key);

    let (found, value) = storage::get(&cache_key);
    if !found {
        return None;
    }

    // Deserialize cache entry
    let entry: CacheEntry = match serde_json::from_slice(&value) {
        Ok(e) => e,
        Err(_) => return None,
    };

    // Check if expired
    let now = host::now_millis() / 1000;
    let age = now - entry.created_at;

    if age >= entry.ttl_seconds {
        host::log(&format!("Cache expired: {} (age: {}s)", key, age));
        return None;
    }

    Some(entry.data)
}

/// Set cached data with TTL
fn set_cached(key: &str, data: &[u8], ttl_seconds: i64) {
    let cache_key = format!("cache:{}", key);
    let now = host::now_millis() / 1000;

    let entry = CacheEntry {
        data: data.to_vec(),
        created_at: now,
        ttl_seconds,
    };

    if let Ok(json) = serde_json::to_vec(&entry) {
        storage::set(&cache_key, &json);
        host::log(&format!("Cached: {} (TTL: {}s)", key, ttl_seconds));
    }
}

/// Create a success response
fn success_response(data: serde_json::Value) -> String {
    let response = ApiResponse {
        status: "success".to_string(),
        data: Some(data),
        error: None,
        timestamp: host::now_millis(),
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"status":"error","error":"Failed to serialize response"}"#.to_string()
    })
}

/// Create an error response
fn error_response(message: &str, _code: i32) -> String {
    let response = ApiResponse {
        status: "error".to_string(),
        data: None,
        error: Some(message.to_string()),
        timestamp: host::now_millis(),
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {
        format!(r#"{{"status":"error","error":"{}"}}"#, message)
    })
}

/// Convert bytes to hex string
fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut hex = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        hex.push(HEX_CHARS[(byte >> 4) as usize] as char);
        hex.push(HEX_CHARS[(byte & 0xf) as usize] as char);
    }

    hex
}
