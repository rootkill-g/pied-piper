use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP request passed to WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRequest {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    
    /// Request path (e.g., "/api/users/123")
    pub path: String,
    
    /// Query parameters
    pub query: HashMap<String, String>,
    
    /// HTTP headers
    pub headers: HashMap<String, String>,
    
    /// Request body as string
    pub body: String,
    
    /// Content type
    pub content_type: Option<String>,
}

/// HTTP response returned from WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResponse {
    /// HTTP status code (200, 404, 500, etc.)
    pub status: u16,
    
    /// Response headers
    pub headers: HashMap<String, String>,
    
    /// Response body as string
    pub body: String,
    
    /// Content type (defaults to application/json)
    pub content_type: Option<String>,
}

impl WasmRequest {
    /// Create a new WASM request
    pub fn new(method: String, path: String, body: String) -> Self {
        Self {
            method,
            path,
            query: HashMap::new(),
            headers: HashMap::new(),
            body,
            content_type: None,
        }
    }
    
    /// Add a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
    
    /// Add a query parameter
    pub fn with_query(mut self, key: String, value: String) -> Self {
        self.query.insert(key, value);
        self
    }
    
    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }
    
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    /// Serialize to JSON bytes
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl WasmResponse {
    /// Create a successful response
    pub fn ok(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    /// Create an error response
    pub fn error(status: u16, message: String) -> Self {
        let body = serde_json::json!({
            "error": true,
            "message": message,
            "status": status,
        }).to_string();
        
        Self {
            status,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    /// Create a not found response
    pub fn not_found(message: String) -> Self {
        Self::error(404, message)
    }
    
    /// Add a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
    
    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }
    
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    /// Parse from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

impl Default for WasmResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
            content_type: Some("application/json".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_request_serialization() {
        let req = WasmRequest::new(
            "POST".to_string(),
            "/api/test".to_string(),
            r#"{"key":"value"}"#.to_string(),
        );
        
        let json = req.to_json().unwrap();
        assert!(json.contains("POST"));
        assert!(json.contains("/api/test"));
    }
    
    #[test]
    fn test_wasm_response_creation() {
        let resp = WasmResponse::ok("success".to_string());
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "success");
        
        let error = WasmResponse::error(500, "failed".to_string());
        assert_eq!(error.status, 500);
        assert!(error.body.contains("failed"));
    }
    
    #[test]
    fn test_wasm_response_deserialization() {
        let json = r#"{"status":200,"headers":{},"body":"test","content_type":"text/plain"}"#;
        let resp = WasmResponse::from_json(json).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "test");
    }
}
