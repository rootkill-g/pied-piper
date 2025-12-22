use anyhow::{Context, Result};
use axum::{
    response::{Response},
    http::{StatusCode, header},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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
    pub async fn handle_cid_request(&self, cid: &str, path: Option<&str>, method: &str, query: Option<&str>) -> Response {
        debug!("Handling CID request: {} path: {:?} query: {:?} method: {}", cid, path, query, method);
        
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
        // Treat empty string path same as None (normalize trailing slash)
        let path_normalized = path.filter(|p| !p.is_empty()).unwrap_or("");
        debug!("Path after normalization: '{}' (was {:?})", path_normalized, path);
        let path = path_normalized;
        
        // Check if it's a WASM component (magic bytes 0x00 0x61 0x73 0x6d 0x0d)
        let is_component = bytes.len() >= 5 
            && bytes[0] == 0x00 
            && bytes[1] == 0x61 
            && bytes[2] == 0x73 
            && bytes[3] == 0x6d 
            && bytes[4] == 0x0d;
        
        // If it's a WASM component, execute it as API (components can serve HTML or JSON)
        if is_component {
            match self.execute_wasm_api(cid, path, "", method, query).await {
                Ok(response) => response,
                Err(e) => {
                    error!("Error executing WASM component: {}", e);
                    self.error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Execution error: {}", e),
                    )
                }
            }
        // If path is empty or ends with .html, serve as frontend
        } else if path.is_empty() || path.ends_with(".html") || path == &self.config.index_file {
            self.serve_frontend(&bytes, path).await
        } else {
            // Otherwise, try to serve as static asset
            self.serve_static_file(&bytes, path).await
        }
    }
    
    /// Handle a request for a named application
    pub async fn handle_app_request(&self, name: &str, path: Option<&str>, method: &str, query: Option<&str>) -> Response {
        debug!("Handling app request: {} path: {:?} query: {:?} method: {}", name, path, query, method);
        
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
        self.handle_cid_request(&cid, path, method, query).await
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
    async fn serve_frontend(&self, bytes: &[u8], path: &str) -> Response {
        // Check if this is a bundled application (TAR archive)
        if Self::is_tar_archive(bytes) {
            // Try to serve index.html from the bundle
            let index_path = if path.is_empty() || path == "/" {
                "index.html"
            } else {
                path
            };
            
            match Self::extract_from_tar(bytes, index_path).await {
                Ok(Some(html_data)) => {
                    info!("Serving frontend: {} ({} bytes)", index_path, html_data.len());
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .body(html_data.into())
                        .unwrap()
                }
                Ok(None) => {
                    // index.html not found in bundle, return app listing
                    self.serve_app_listing(bytes).await
                }
                Err(e) => {
                    error!("Failed to extract HTML from bundle: {}", e);
                    self.serve_app_listing(bytes).await
                }
            }
        } else {
            // Single WASM file - show placeholder page
            self.serve_app_listing(bytes).await
        }
    }
    
    /// Serve application listing (fallback when no index.html found)
    async fn serve_app_listing(&self, bytes: &[u8]) -> Response {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Pied Piper App</title>
    <style>
        body {{ font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }}
        h1 {{ color: #2563eb; }}
        .info {{ background: #f3f4f6; padding: 15px; border-radius: 8px; margin: 20px 0; }}
    </style>
</head>
<body>
    <h1>🚀 WebAssembly Application</h1>
    <div class="info">
        <p><strong>This application is served from the Pied Piper network!</strong></p>
        <p>Module size: {} bytes</p>
        <p>Content type: {}</p>
    </div>
    <p><em>To serve a complete web application, bundle your HTML, CSS, JS, and WASM files into a TAR archive with an index.html file.</em></p>
</body>
</html>"#,
            bytes.len(),
            if Self::is_tar_archive(bytes) { "TAR bundle" } else { "Single WASM file" }
        );
        
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(html.into())
            .unwrap()
    }

    
    /// Serve static files (CSS, JS, images, etc.)
    async fn serve_static_file(&self, bytes: &[u8], path: &str) -> Response {
        // Determine content type from extension
        let content_type = Self::guess_content_type(path);
        
        // Check if this is a bundled asset (TAR format) or single file
        if Self::is_tar_archive(bytes) {
            // Extract the requested file from the TAR archive
            match Self::extract_from_tar(bytes, path).await {
                Ok(Some(file_data)) => {
                    info!("Serving static file '{}' from bundle ({} bytes)", path, file_data.len());
                    
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                        .header(header::ETAG, format!("\"{}\"", blake3::hash(&file_data).to_hex()))
                        .body(file_data.into())
                        .unwrap()
                }
                Ok(None) => {
                    self.error_response(
                        StatusCode::NOT_FOUND,
                        &format!("File '{}' not found in bundle", path),
                    )
                }
                Err(e) => {
                    error!("Failed to extract file from bundle: {}", e);
                    self.error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to extract file from bundle",
                    )
                }
            }
        } else {
            // Single file - serve directly with caching
            info!("Serving single file ({} bytes)", bytes.len());
            
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header(header::ETAG, format!("\"{}\"", blake3::hash(bytes).to_hex()))
                .body(bytes.to_vec().into())
                .unwrap()
        }
    }
    
    /// Execute WASM as backend API
    async fn execute_wasm_api(&self, cid: &str, path: &str, body: &str, method: &str, query: Option<&str>) -> Result<Response> {
        use super::io::{WasmRequest, WasmResponse};
        
        info!("Executing WASM API: {} path: {} method: {} query: {:?}", cid, path, method, query);
        
        // Fetch module
        let bytes = self.fetch_module(cid)
            .await?
            .context("Module not found")?;
        
        // Create WasmRequest with the actual HTTP method
        // Ensure path has leading slash
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        
        let mut wasm_request = WasmRequest::new(
            method.to_string(),
            normalized_path,
            body.to_string(),
        )
        .with_content_type("application/json".to_string());
        
        // Parse and add query parameters if present
        if let Some(query_str) = query {
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    // URL decode the key and value
                    let key = urlencoding::decode(key).unwrap_or_else(|_| key.into()).to_string();
                    let value = urlencoding::decode(value).unwrap_or_else(|_| value.into()).to_string();
                    wasm_request = wasm_request.with_query(key, value);
                }
            }
        }
        
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
        
        // Detect if this is a component (WASI P2) or core module (WASI P1)
        // Components have magic bytes: 0x00 0x61 0x73 0x6d 0x0d 0x00 0x01 0x00
        // Core modules have:          0x00 0x61 0x73 0x6d 0x01 0x00 0x00 0x00
        let is_component = bytes.len() >= 8 
            && bytes[0..4] == [0x00, 0x61, 0x73, 0x6d] // "\0asm"
            && bytes[4] == 0x0d; // Component version marker
        
        if is_component {
            info!("Detected WASI P2 component (size: {} bytes)", bytes.len());
            
            // Load as component
            let component = runtime.load_component(&bytes)
                .context(format!("Failed to load WASI P2 component (size: {} bytes)", bytes.len()))?;
            info!("Loaded WASI P2 component for API execution");
            
            // Create store with stdin containing the request
            let mut store = runtime.create_store_with_stdin(request_json.into_bytes())?;
            
            // Execute the component using the Command pattern
            match runtime.execute_component_command(&mut store, &component).await {
                Ok(_) => {
                    info!("Component executed successfully");
                }
                Err(e) => {
                    warn!("Component execution failed: {}", e);
                    
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(
                            serde_json::json!({
                                "error": "Component execution failed",
                                "message": format!("{}", e),
                                "path": path,
                                "cid": cid,
                            })
                            .to_string()
                            .into(),
                        )
                        .unwrap());
                }
            }
            
            // Get stdout from the store
            let stdout_bytes = runtime.get_stdout(&store);
            
            if stdout_bytes.is_empty() {
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(
                        serde_json::json!({
                            "status": "success",
                            "message": "Component executed but produced no output",
                            "path": path,
                            "cid": cid,
                        })
                        .to_string()
                        .into(),
                    )
                    .unwrap());
            }
            
            // Try to parse the output as JSON (WasmResponse)
            match String::from_utf8(stdout_bytes.clone()) {
                Ok(stdout_str) => {
                    info!("Component output: {} bytes", stdout_str.len());
                    
                    // Try to parse as WasmResponse
                    match serde_json::from_str::<serde_json::Value>(&stdout_str) {
                        Ok(json_response) => {
                            // Check if it's a WasmResponse format
                            if let Some(status) = json_response.get("status").and_then(|v| v.as_u64()) {
                                let body = json_response.get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&stdout_str)
                                    .to_string();
                                
                                let content_type = json_response.get("content_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("application/json");
                                
                                return Ok(Response::builder()
                                    .status(StatusCode::from_u16(status as u16).unwrap_or(StatusCode::OK))
                                    .header(header::CONTENT_TYPE, content_type)
                                    .body(body.into())
                                    .unwrap());
                            }
                            
                            // If not WasmResponse format, return the JSON as-is
                            return Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(stdout_str.into())
                                .unwrap());
                        }
                        Err(_) => {
                            // Not valid JSON, return as plain text
                            return Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "text/plain")
                                .body(stdout_str.into())
                                .unwrap());
                        }
                    }
                }
                Err(e) => {
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(
                            serde_json::json!({
                                "error": "Invalid UTF-8 in component output",
                                "message": format!("{}", e),
                                "path": path,
                                "cid": cid,
                            })
                            .to_string()
                            .into(),
                        )
                        .unwrap());
                }
            }
        }
        
        // Legacy core module path (WASI P1)
        info!("Detected core WASM module (size: {} bytes)", bytes.len());
        let module = runtime.load_module(&bytes)
            .context(format!("Failed to load Wasm module (size: {} bytes)", bytes.len()))?;
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
        // HTML & XML
        if path.ends_with(".html") || path.ends_with(".htm") {
            "text/html"
        } else if path.ends_with(".xml") {
            "application/xml"
        }
        // CSS & JavaScript
        else if path.ends_with(".css") {
            "text/css"
        } else if path.ends_with(".js") || path.ends_with(".mjs") {
            "application/javascript"
        } else if path.ends_with(".json") {
            "application/json"
        }
        // Images
        else if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else if path.ends_with(".gif") {
            "image/gif"
        } else if path.ends_with(".svg") {
            "image/svg+xml"
        } else if path.ends_with(".webp") {
            "image/webp"
        } else if path.ends_with(".ico") {
            "image/x-icon"
        }
        // Fonts
        else if path.ends_with(".woff") {
            "font/woff"
        } else if path.ends_with(".woff2") {
            "font/woff2"
        } else if path.ends_with(".ttf") {
            "font/ttf"
        } else if path.ends_with(".otf") {
            "font/otf"
        }
        // WebAssembly
        else if path.ends_with(".wasm") {
            "application/wasm"
        }
        // Text files
        else if path.ends_with(".txt") {
            "text/plain"
        } else if path.ends_with(".md") {
            "text/markdown"
        }
        // Default
        else {
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
    
    /// Check if bytes represent a TAR archive
    fn is_tar_archive(bytes: &[u8]) -> bool {
        // TAR archives have "ustar" magic bytes at offset 257
        if bytes.len() < 262 {
            return false;
        }
        &bytes[257..262] == b"ustar"
    }
    
    /// Extract a file from a TAR archive
    async fn extract_from_tar(archive_bytes: &[u8], file_path: &str) -> Result<Option<Vec<u8>>> {
        use std::io::Cursor;
        use tokio::task;
        
        let archive_bytes = archive_bytes.to_vec();
        let file_path = file_path.to_string();
        
        // Run TAR extraction in a blocking task to avoid blocking the async runtime
        task::spawn_blocking(move || {
            let cursor = Cursor::new(archive_bytes);
            let mut archive = tar::Archive::new(cursor);
            
            // Normalize path (remove leading slash)
            let normalized_path = file_path.trim_start_matches('/');
            
            // Search for the file in the archive
            for entry in archive.entries()? {
                let mut entry = entry?;
                let entry_path = entry.path()?;
                let entry_path_str = entry_path.to_string_lossy();
                
                // Match the path
                if entry_path_str == normalized_path || entry_path_str == file_path {
                    let mut contents = Vec::new();
                    std::io::Read::read_to_end(&mut entry, &mut contents)?;
                    return Ok(Some(contents));
                }
                
                // Also try with "index.html" appended if path is a directory
                if normalized_path.is_empty() || normalized_path.ends_with('/') {
                    let index_path = format!("{}index.html", normalized_path);
                    if entry_path_str == index_path {
                        let mut contents = Vec::new();
                        std::io::Read::read_to_end(&mut entry, &mut contents)?;
                        return Ok(Some(contents));
                    }
                }
            }
            
            // File not found
            Ok(None)
        })
        .await
        .context("TAR extraction task failed")?
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
