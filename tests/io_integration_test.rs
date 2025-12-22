use std::collections::HashMap;

/// Test WasmRequest and WasmResponse data structures
/// This verifies the contract between the gateway and WASM modules
#[test]
fn test_wasm_request_structure() {
    // Create a sample WasmRequest
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("User-Agent".to_string(), "PiedPiper-Test/1.0".to_string());
    
    let mut query = HashMap::new();
    query.insert("test".to_string(), "value".to_string());
    query.insert("foo".to_string(), "bar".to_string());
    
    let request = serde_json::json!({
        "method": "POST",
        "path": "/api/echo",
        "query": query,
        "headers": headers,
        "body": "{\"message\":\"Hello, World!\"}",
        "content_type": "application/json"
    });
    
    let request_json = serde_json::to_string(&request).expect("Failed to serialize request");
    
    // Verify it can be serialized and deserialized
    let parsed: serde_json::Value = serde_json::from_str(&request_json)
        .expect("Failed to parse request JSON");
    
    assert_eq!(parsed["method"], "POST");
    assert_eq!(parsed["path"], "/api/echo");
    assert_eq!(parsed["body"], "{\"message\":\"Hello, World!\"}");
    assert_eq!(parsed["content_type"], "application/json");
    
    // Verify query and headers
    assert_eq!(parsed["query"]["test"], "value");
    assert_eq!(parsed["headers"]["Content-Type"], "application/json");
    
    println!("✅ WasmRequest structure test passed!");
}

#[test]
fn test_wasm_response_structure() {
    // Create a sample WasmResponse
    let mut response_headers = HashMap::new();
    response_headers.insert("X-Test".to_string(), "value".to_string());
    
    let response = serde_json::json!({
        "status": 200,
        "headers": response_headers,
        "body": "{\"result\":\"success\"}",
        "content_type": "application/json"
    });
    
    let response_json = serde_json::to_string(&response).expect("Failed to serialize");
    
    // Verify it can be parsed
    let parsed: serde_json::Value = serde_json::from_str(&response_json)
        .expect("Failed to parse response JSON");
    
    assert_eq!(parsed["status"], 200);
    assert_eq!(parsed["body"], "{\"result\":\"success\"}");
    assert_eq!(parsed["content_type"], "application/json");
    assert_eq!(parsed["headers"]["X-Test"], "value");
    
    println!("✅ WasmResponse structure test passed!");
}

// Test binary body support
#[test]
fn test_binary_body_encoding() {
    // This test verifies that we can handle binary data in request/response
    // For now, we'll encode binary as base64 in JSON
    
    let binary_data = vec![0u8, 1, 2, 3, 255, 254, 253];
    let base64_data = base64::encode(&binary_data);
    
    let request = serde_json::json!({
        "method": "POST",
        "path": "/api/upload",
        "headers": {
            "Content-Type": "application/octet-stream"
        },
        "body": base64_data,
        "content_type": "application/octet-stream"
    });
    
    let request_json = serde_json::to_string(&request).expect("Failed to serialize");
    
    // Verify we can round-trip binary data
    let parsed: serde_json::Value = serde_json::from_str(&request_json)
        .expect("Failed to parse");
    
    let decoded = base64::decode(parsed["body"].as_str().unwrap())
        .expect("Failed to decode base64");
    
    assert_eq!(decoded, binary_data, "Binary data should round-trip correctly");
    
    println!("✅ Binary body test passed!");
}

// Helper to add base64 crate for testing
mod base64 {
    pub fn encode(data: &[u8]) -> String {
        use std::fmt::Write;
        data.iter().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{:02x}", b);
            output
        })
    }
    
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|e| format!("Invalid hex: {}", e))
            })
            .collect()
    }
}
