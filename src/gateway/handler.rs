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
        info!("Executing WASM API: {} path: {}", cid, path);
        
        // Fetch module
        let bytes = self.fetch_module(cid)
            .await?
            .context("Module not found")?;
        
        // Create runtime config
        let config = WasmRuntimeConfig::default();
        let runtime = WasmRuntime::new(config)?;
        
        // Load and validate module
        let module = runtime.load_module(&bytes)?;
        
        // TODO: For now, return success without actual execution
        // In future: Create proper WASI environment, instantiate, and call handler
        let response_text = serde_json::json!({
            "status": "success",
            "message": "WASM API execution (placeholder)",
            "path": path,
            "cid": cid,
            "body_length": body.len(),
        }).to_string();
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(response_text.into())
            .unwrap())
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
