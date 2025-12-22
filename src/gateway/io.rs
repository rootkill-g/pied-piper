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

    /// Request body as string (may be base64-encoded for binary data)
    pub body: String,

    /// Content type
    pub content_type: Option<String>,

    /// Flag indicating if body is base64-encoded binary data
    #[serde(default)]
    pub body_is_base64: bool,
}

/// HTTP response returned from WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResponse {
    /// HTTP status code (200, 404, 500, etc.)
    pub status: u16,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body as string (may be base64-encoded for binary data)
    pub body: String,

    /// Content type (defaults to application/json)
    pub content_type: Option<String>,

    /// Flag indicating if body is base64-encoded binary data
    #[serde(default)]
    pub body_is_base64: bool,
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
            body_is_base64: false,
        }
    }

    /// Create a new WASM request with binary body (base64-encoded)
    pub fn new_binary(method: String, path: String, binary_body: Vec<u8>) -> Self {
        use base64::Engine;
        let body = base64::engine::general_purpose::STANDARD.encode(&binary_body);
        Self {
            method,
            path,
            query: HashMap::new(),
            headers: HashMap::new(),
            body,
            content_type: Some("application/octet-stream".to_string()),
            body_is_base64: true,
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

    /// Decode body if it's base64-encoded
    pub fn decode_body(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        if self.body_is_base64 {
            base64::engine::general_purpose::STANDARD.decode(&self.body)
        } else {
            Ok(self.body.as_bytes().to_vec())
        }
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
            body_is_base64: false,
        }
    }

    /// Create a successful response with binary body (base64-encoded)
    pub fn ok_binary(binary_body: Vec<u8>, content_type: Option<String>) -> Self {
        use base64::Engine;
        let body = base64::engine::general_purpose::STANDARD.encode(&binary_body);
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: content_type.or(Some("application/octet-stream".to_string())),
            body_is_base64: true,
        }
    }

    /// Create an error response
    pub fn error(status: u16, message: String) -> Self {
        let body = serde_json::json!({
            "error": true,
            "message": message,
            "status": status,
        })
        .to_string();

        Self {
            status,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
            body_is_base64: false,
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

    /// Decode body if it's base64-encoded
    pub fn decode_body(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        if self.body_is_base64 {
            base64::engine::general_purpose::STANDARD.decode(&self.body)
        } else {
            Ok(self.body.as_bytes().to_vec())
        }
    }

    /// Get body as bytes (decoding base64 if necessary)
    pub fn body_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        self.decode_body()
    }
}

impl Default for WasmResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
            content_type: Some("application/json".to_string()),
            body_is_base64: false,
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

    #[test]
    fn test_binary_request() {
        let binary_data = vec![0u8, 1, 2, 3, 255, 254, 253];
        let req = WasmRequest::new_binary(
            "POST".to_string(),
            "/api/upload".to_string(),
            binary_data.clone(),
        );

        assert_eq!(req.body_is_base64, true);
        assert_eq!(req.content_type, Some("application/octet-stream".to_string()));

        // Should be able to decode back
        let decoded = req.decode_body().unwrap();
        assert_eq!(decoded, binary_data);
    }

    #[test]
    fn test_binary_response() {
        let binary_data = vec![255u8, 254, 253, 0, 1, 2];
        let resp = WasmResponse::ok_binary(binary_data.clone(), None);

        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_is_base64, true);
        assert_eq!(
            resp.content_type,
            Some("application/octet-stream".to_string())
        );

        // Should be able to decode back
        let decoded = resp.body_bytes().unwrap();
        assert_eq!(decoded, binary_data);
    }

    #[test]
    fn test_request_query_and_headers() {
        let req = WasmRequest::new("GET".to_string(), "/test".to_string(), String::new())
            .with_query("param1".to_string(), "value1".to_string())
            .with_query("param2".to_string(), "value2".to_string())
            .with_header("Authorization".to_string(), "Bearer token".to_string())
            .with_content_type("application/json".to_string());

        assert_eq!(req.query.get("param1").unwrap(), "value1");
        assert_eq!(req.query.get("param2").unwrap(), "value2");
        assert_eq!(
            req.headers.get("Authorization").unwrap(),
            "Bearer token"
        );
        assert_eq!(req.content_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_response_with_custom_headers() {
        let resp = WasmResponse::ok("data".to_string())
            .with_header("X-Custom".to_string(), "value".to_string())
            .with_header("Cache-Control".to_string(), "no-cache".to_string());

        assert_eq!(resp.headers.get("X-Custom").unwrap(), "value");
        assert_eq!(resp.headers.get("Cache-Control").unwrap(), "no-cache");
    }
}
