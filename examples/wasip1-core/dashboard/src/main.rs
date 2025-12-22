//! Dashboard - Interactive frontend for Pied Piper modules

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

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
    fn html(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("text/html".to_string()),
        }
    }
    
    fn json(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
}

fn main() {
    let exit_code = match handle_request() {
        Ok(_) => 0,
        Err(e) => {
            // Write error to stderr for debugging
            eprintln!("Dashboard error: {}", e);
            // Also try to write a JSON error response to stdout
            let error_response = WasmResponse {
                status: 500,
                headers: HashMap::new(),
                body: format!("{{\"error\": \"Internal error: {}\"}}", e),
                content_type: Some("application/json".to_string()),
            };
            if let Ok(json) = serde_json::to_string(&error_response) {
                let _ = io::stdout().write_all(json.as_bytes());
                let _ = io::stdout().flush();
            }
            1
        }
    };
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn handle_request() -> Result<(), Box<dyn std::error::Error>> {
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
        ("GET", "/") | ("GET", "/index.html") => serve_dashboard(),
        ("GET", "/api/modules") => list_modules(),
        _ => serve_dashboard(), // Default to dashboard for any path
    }
}

fn serve_dashboard() -> WasmResponse {
    let html = include_str!("../index.html");
    WasmResponse::html(html.to_string())
}

fn list_modules() -> WasmResponse {
    // This would ideally query the network for available modules
    // For now, return known modules
    let modules = serde_json::json!({
        "modules": [
            {
                "cid": "b6mvygz2yetlnjhmgsilzkoucbkrupoamjnhuwyn2p3usgck23wvq",
                "name": "hello-api",
                "status": "active"
            },
            {
                "cid": "bmjncyyz5pox4zbfwajqib35znicam5q45cxvq4wdvrppd3gv2fra",
                "name": "joke-api",
                "status": "active"
            }
        ]
    });
    
    WasmResponse::json(modules.to_string())
}
