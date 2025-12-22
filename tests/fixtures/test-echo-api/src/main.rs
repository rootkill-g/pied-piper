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
    // Read request from stdin
    let mut request_json = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut request_json) {
        eprintln!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    // Parse the request
    let request: WasmRequest = match serde_json::from_str(&request_json) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to parse request JSON: {}", e);
            let error_response = WasmResponse {
                status: 400,
                headers: HashMap::new(),
                body: format!("Invalid request: {}", e),
                content_type: Some("text/plain".to_string()),
            };
            let response_json = serde_json::to_string(&error_response).unwrap();
            println!("{}", response_json);
            return;
        }
    };

    // Echo back the request details as the response body
    let echo_body = serde_json::json!({
        "message": "Echo API Test",
        "received": {
            "method": request.method,
            "path": request.path,
            "query": request.query,
            "headers": request.headers,
            "body": request.body,
            "content_type": request.content_type,
        }
    });

    // Create response
    let mut response_headers = HashMap::new();
    response_headers.insert("X-Test-Module".to_string(), "test-echo-api".to_string());
    response_headers.insert("X-Echo-Method".to_string(), request.method.clone());

    let response = WasmResponse {
        status: 200,
        headers: response_headers,
        body: echo_body.to_string(),
        content_type: Some("application/json".to_string()),
    };

    // Write response to stdout
    let response_json = serde_json::to_string(&response).unwrap();
    if let Err(e) = io::stdout().write_all(response_json.as_bytes()) {
        eprintln!("Failed to write to stdout: {}", e);
        std::process::exit(1);
    }
}
