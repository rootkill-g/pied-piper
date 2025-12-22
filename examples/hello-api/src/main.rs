//! Hello API - Example WASM backend for Pied Piper
//! 
//! This example demonstrates how to build a WASM API handler that:
//! - Reads WasmRequest JSON from stdin
//! - Processes the request
//! - Writes WasmResponse JSON to stdout
//!
//! The handler responds to different endpoints:
//! - GET /hello - Returns a greeting
//! - POST /echo - Echoes back the request body
//! - GET /info - Returns API information

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

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

impl WasmResponse {
    fn ok(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    fn error(status: u16, message: String) -> Self {
        let body = serde_json::json!({
            "error": message,
            "status": status
        }).to_string();
        
        Self {
            status,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    fn not_found(path: String) -> Self {
        Self::error(404, format!("Endpoint not found: {}", path))
    }
}

/// Main request handler - called by the main function
fn handle_request() -> i32 {
    match process_request() {
        Ok(_) => 0,  // Success
        Err(e) => {
            eprintln!("Error processing request: {}", e);
            1  // Error
        }
    }
}

fn process_request() -> Result<(), Box<dyn std::error::Error>> {
    // Read request JSON from stdin
    let mut stdin_buffer = String::new();
    io::stdin().read_to_string(&mut stdin_buffer)?;
    
    // Parse the request
    let request: WasmRequest = serde_json::from_str(&stdin_buffer)?;
    
    // Route the request
    let response = route_request(&request);
    
    // Write response JSON to stdout
    let response_json = serde_json::to_string(&response)?;
    io::stdout().write_all(response_json.as_bytes())?;
    io::stdout().flush()?;
    
    Ok(())
}

fn route_request(req: &WasmRequest) -> WasmResponse {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/hello") | ("GET", "/api/hello") => handle_hello(req),
        ("POST", "/echo") | ("POST", "/api/echo") => handle_echo(req),
        ("GET", "/info") | ("GET", "/api/info") => handle_info(req),
        ("GET", "/health") | ("GET", "/api/health") => handle_health(),
        _ => WasmResponse::not_found(req.path.clone()),
    }
}

fn handle_hello(req: &WasmRequest) -> WasmResponse {
    // Get name from query parameter or use default
    let name = req.query.get("name")
        .map(|s| s.as_str())
        .unwrap_or("World");
    
    let response_data = serde_json::json!({
        "message": format!("Hello, {}! 👋", name),
        "path": req.path,
        "method": req.method
    });
    
    WasmResponse::ok(response_data.to_string())
}

fn handle_echo(req: &WasmRequest) -> WasmResponse {
    let response_data = serde_json::json!({
        "echo": req.body,
        "method": req.method,
        "path": req.path,
        "content_type": req.content_type,
        "body_length": req.body.len()
    });
    
    WasmResponse::ok(response_data.to_string())
}

fn handle_info(_req: &WasmRequest) -> WasmResponse {
    let response_data = serde_json::json!({
        "name": "hello-api",
        "version": "1.0.0",
        "description": "Example WASM API handler for Pied Piper",
        "endpoints": [
            {
                "method": "GET",
                "path": "/api/hello",
                "query_params": ["name"],
                "description": "Returns a greeting message"
            },
            {
                "method": "POST",
                "path": "/api/echo",
                "description": "Echoes back the request body"
            },
            {
                "method": "GET",
                "path": "/api/info",
                "description": "Returns API information"
            },
            {
                "method": "GET",
                "path": "/api/health",
                "description": "Health check endpoint"
            }
        ],
        "powered_by": "Pied Piper - Decentralized Internet Platform"
    });
    
    WasmResponse::ok(response_data.to_string())
}

fn handle_health() -> WasmResponse {
    let response_data = serde_json::json!({
        "status": "healthy"
    });
    
    WasmResponse::ok(response_data.to_string())
}

// Entry point for WASI P2 Command components
// This is automatically called when the component is executed
fn main() {
    // Process the request by calling handle_request
    let exit_code = handle_request();
    
    // Exit with the appropriate code
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
