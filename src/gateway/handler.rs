use anyhow::{Context, Result};
use axum::{
    response::{Response},
    http::{StatusCode, header},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::network::NetworkNode;
use crate::wasm::{ModuleLoader, WasmRuntime, WasmRuntimeConfig};
use super::server::GatewayConfig;

/// Handles HTTP requests and routes them to WASM modules
pub struct RequestHandler {
    network: Arc<Mutex<NetworkNode>>,
    loader: Arc<ModuleLoader>,
    config: GatewayConfig,
}

impl RequestHandler {
    pub fn new(
        network: Arc<Mutex<NetworkNode>>,
        loader: Arc<ModuleLoader>,
        config: GatewayConfig,
    ) -> Self {
        Self { network, loader, config }
    }
    
    /// Handle a request for a CID-based resource
    pub async fn handle_cid_request(&self, cid: &str, path: Option<&str>) -> Response {
        debug!("Handling CID request: {} path: {:?}", cid, path);
        
        // Fetch module bytes
        let bytes = match self.fetch_module(cid).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                return self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Module {} not found", cid),
                );
            }
            Err(e) => {
                error!("Error fetching module {}: {}", cid, e);
                return self.error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error fetching module: {}", e),
                );
            }
        };
        
        // Determine request type
        let path = path.unwrap_or("");
        
        // If path is empty or ends with .html, serve as frontend
        if path.is_empty() || path.ends_with(".html") || path == &self.config.index_file {
            self.serve_frontend(&bytes, path).await
        } else {
            // Otherwise, try to serve as static asset
            self.serve_static_file(&bytes, path).await
        }
    }
    
    /// Handle a request for a named application
    pub async fn handle_app_request(&self, name: &str, path: Option<&str>) -> Response {
        debug!("Handling app request: {} path: {:?}", name, path);
        
        // First, resolve name to CID
        let cid = match self.resolve_name(name).await {
            Ok(Some(cid)) => cid,
            Ok(None) => {
                return self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Application '{}' not found", name),
                );
            }
            Err(e) => {
                error!("Error resolving name {}: {}", name, e);
                return self.error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error resolving name: {}", e),
                );
            }
        };
        
        // Now handle as CID request
        self.handle_cid_request(&cid, path).await
    }
    
    /// Handle API requests (POST/PUT/DELETE to WASM backends)
    pub async fn handle_api_request(&self, target: &str, path: &str, body: String) -> Response {
        debug!("Handling API request: {} path: {}", target, path);
        
        // Resolve target (could be CID or name)
        let cid = if Self::looks_like_cid(target) {
            target.to_string()
        } else {
            match self.resolve_name(target).await {
                Ok(Some(cid)) => cid,
                Ok(None) => {
                    return self.error_response(
                        StatusCode::NOT_FOUND,
                        &format!("Application '{}' not found", target),
                    );
                }
                Err(e) => {
                    error!("Error resolving {}: {}", target, e);
                    return self.error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Error: {}", e),
                    );
                }
            }
        };
        
        // Fetch and execute WASM
        match self.execute_wasm_api(&cid, path, &body).await {
            Ok(response) => response,
            Err(e) => {
                error!("Error executing WASM API: {}", e);
                self.error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Execution error: {}", e),
                )
            }
        }
    }
    
    /// Fetch module bytes from cache or network
    async fn fetch_module(&self, cid: &str) -> Result<Option<Vec<u8>>> {
        use crate::wasm::ModuleCid;
        
        let module_cid = ModuleCid::new(cid.to_string());
        
        // Try cache first
        if let Some((_info, bytes)) = self.loader.get_from_cache(&module_cid).await {
            return Ok(Some(bytes.to_vec()));
        }
        
        // TODO: Network fetch when fully implemented
        Ok(None)
    }
    
    /// Resolve name to CID
    async fn resolve_name(&self, name: &str) -> Result<Option<String>> {
        let mut network = self.network.lock().await;
        let results = network.search_modules_by_name(name).await?;
        
        Ok(results.first().map(|info| info.cid.0.clone()))
    }
    
    /// Check if string looks like CID
    fn looks_like_cid(s: &str) -> bool {
        s.starts_with('b') && s.len() > 30
    }
    
    /// Serve frontend HTML (placeholder - will be enhanced)
    async fn serve_frontend(&self, bytes: &[u8], _path: &str) -> Response {
        // For now, return a simple HTML page that loads the WASM
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Pied Piper App</title>
    <style>
        body {{ font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }}
        h1 {{ color: #2563eb; }}
    </style>
</head>
<body>
    <h1>🚀 WebAssembly Application</h1>
    <p>This application is served from the Pied Piper network!</p>
    <p>Module size: {} bytes</p>
    <p><em>Frontend WASM execution in browser coming soon...</em></p>
</body>
</html>"#,
            bytes.len()
        );
        
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(html.into())
            .unwrap()
    }
    
    /// Serve static files (CSS, JS, images, etc.)
    async fn serve_static_file(&self, _bytes: &[u8], path: &str) -> Response {
        // Determine content type from extension
        let content_type = Self::guess_content_type(path);
        
        // For now, return 404 as we don't have asset bundling yet
        self.error_response(
            StatusCode::NOT_FOUND,
            &format!("Static file '{}' not found (asset bundling not implemented)", path),
        )
    }
    
    /// Execute WASM as backend API
    async fn execute_wasm_api(&self, cid: &str, path: &str, body: &str) -> Result<Response> {
        use super::io::{WasmRequest, WasmResponse};
        
        info!("Executing WASM API: {} path: {}", cid, path);
        
        // Fetch module
        let bytes = self.fetch_module(cid)
            .await?
            .context("Module not found")?;
        
        // Create WasmRequest
        let wasm_request = WasmRequest::new(
            "POST".to_string(),
            path.to_string(),
            body.to_string(),
        )
        .with_content_type("application/json".to_string());
        
        // Serialize request to JSON
        let request_json = wasm_request.to_json()
            .context("Failed to serialize request")?;
        
        info!("Request payload: {} bytes", request_json.len());
        
        // Create runtime config with reasonable limits for API execution
        let config = WasmRuntimeConfig {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB for API handlers
            max_execution_time: std::time::Duration::from_secs(10), // 10 second timeout
            enable_async: true,
            enable_wasi: true,
            enable_fuel: true,
            initial_fuel: 1_000_000, // Generous fuel for API calls
        };
        
        let runtime = WasmRuntime::new(config)?;
        
        // Load and validate module
        let module = runtime.load_module(&bytes)?;
        info!("Loaded WASM module for API execution");
        
        // Create store with stdin containing the request
        let mut store = runtime.create_store_with_stdin(request_json.into_bytes())?;
        
        // Instantiate with WASI
        let instance = runtime.instantiate_with_wasi(&mut store, &module).await?;
        info!("Instantiated WASM module");
        
        // Look for API handler function (convention: _handle_request or handle_request)
        let handler_func_name = if instance.get_func(&mut store, "_handle_request").is_some() {
            "_handle_request"
        } else if instance.get_func(&mut store, "handle_request").is_some() {
            "handle_request"
        } else {
            // No standard handler found, return helpful error
            return Ok(Response::builder()
                .status(StatusCode::NOT_IMPLEMENTED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(
                    serde_json::json!({
                        "error": "No API handler found",
                        "message": "Module must export 'handle_request' or '_handle_request' function",
                        "path": path,
                        "cid": cid,
                    })
                    .to_string()
                    .into(),
                )
                .unwrap());
        };
        
        info!("Calling handler function: {}", handler_func_name);
        
        // Execute the handler function
        match runtime.execute_function(&mut store, &instance, handler_func_name, &[]).await {
            Ok(_results) => {
                info!("Handler function executed successfully");
                
                // Get stdout from the store
                let stdout_bytes = runtime.get_stdout(&store);
                
                if stdout_bytes.is_empty() {
                    // No output - module might not have written anything
                    // Return success with note
                    let response_text = serde_json::json!({
                        "status": "success",
                        "message": "Handler executed but produced no output",
                        "path": path,
                        "cid": cid,
                        "note": "Module should write WasmResponse JSON to stdout",
                    }).to_string();
                    
                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(response_text.into())
                        .unwrap());
                }
                
                // Parse the WASM response from stdout
                let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
                info!("WASM output: {} bytes", stdout_bytes.len());
                debug!("WASM response: {}", stdout_str);
                
                match WasmResponse::from_json(&stdout_str) {
                    Ok(wasm_response) => {
                        // Build HTTP response from WasmResponse
                        let mut response_builder = Response::builder()
                            .status(StatusCode::from_u16(wasm_response.status).unwrap_or(StatusCode::OK));
                        
                        // Add content type
                        let content_type = wasm_response.content_type
                            .unwrap_or_else(|| "application/json".to_string());
                        response_builder = response_builder.header(header::CONTENT_TYPE, content_type);
                        
                        // Add custom headers
                        for (key, value) in wasm_response.headers {
                            response_builder = response_builder.header(key, value);
                        }
                        
                        Ok(response_builder
                            .body(wasm_response.body.into())
                            .unwrap())
                    }
                    Err(parse_err) => {
                        // Failed to parse response - return the raw output
                        error!("Failed to parse WASM response: {}", parse_err);
                        
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "text/plain")
                            .body(stdout_str.into())
                            .unwrap())
                    }
                }
            }
            Err(e) => {
                error!("WASM execution error: {}", e);
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(
                        serde_json::json!({
                            "error": "Execution failed",
                            "message": e.to_string(),
                            "path": path,
                            "cid": cid,
                        })
                        .to_string()
                        .into(),
                    )
                    .unwrap())
            }
        }
    }

    
    /// Guess content type from file extension
    fn guess_content_type(path: &str) -> &'static str {
        if path.ends_with(".html") {
            "text/html"
        } else if path.ends_with(".css") {
            "text/css"
        } else if path.ends_with(".js") {
            "application/javascript"
        } else if path.ends_with(".json") {
            "application/json"
        } else if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else if path.ends_with(".wasm") {
            "application/wasm"
        } else {
            "application/octet-stream"
        }
    }
    
    /// Create an error response
    fn error_response(&self, status: StatusCode, message: &str) -> Response {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{} - Error</title>
    <style>
        body {{ font-family: sans-serif; text-align: center; margin-top: 100px; }}
        h1 {{ color: #dc2626; }}
    </style>
</head>
<body>
    <h1>{} - Error</h1>
    <p>{}</p>
    <p><a href="/">Return to Gateway Home</a></p>
</body>
</html>"#,
            status.as_u16(),
            status.as_u16(),
            message
        );
        
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/html")
            .body(html.into())
            .unwrap()
    }
}

/// WASM-specific handler
pub struct WasmHandler;

impl WasmHandler {
    /// Execute WASM function and return result
    pub async fn execute(_module_bytes: &[u8], _function: &str, _args: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement WASM execution with proper I/O
        unimplemented!("WASM handler not fully implemented")
    }
}
