use anyhow::{Context, Result};
use axum::{
    http::{StatusCode, header},
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::server::GatewayConfig;
use crate::metrics::Metrics;
use crate::network::NetworkClient;
use crate::wasm::{ModuleLoader, WasmRuntime, WasmRuntimeConfig};

/// Handles HTTP requests and routes them to WASM modules
pub struct RequestHandler {
    network: NetworkClient,
    loader: Arc<ModuleLoader>,
    config: GatewayConfig,
    metrics: Option<Arc<Metrics>>,
}

impl RequestHandler {
    pub fn new(network: NetworkClient, loader: Arc<ModuleLoader>, config: GatewayConfig) -> Self {
        Self {
            network,
            loader,
            config,
            metrics: None,
        }
    }

    /// Set metrics for this handler
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Handle a request for a CID-based resource
    pub async fn handle_cid_request(
        &self,
        cid: &str,
        path: Option<&str>,
        method: &str,
        query: Option<&str>,
        headers: &axum::http::HeaderMap,
        body: &axum::body::Bytes,
    ) -> Response {
        let start = Instant::now();
        
        debug!(
            "Handling CID request: {} path: {:?} query: {:?} method: {}",
            cid, path, query, method
        );

        // Fetch module bytes
        let bytes = match self.fetch_module(cid).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                if let Some(metrics) = &self.metrics {
                    metrics.http_requests_total
                        .with_label_values(&[method, path.unwrap_or("/"), "404"])
                        .inc();
                }
                return self
                    .error_response(StatusCode::NOT_FOUND, &format!("Module {} not found", cid));
            }
            Err(e) => {
                error!("Error fetching module {}: {}", cid, e);
                if let Some(metrics) = &self.metrics {
                    metrics.http_requests_total
                        .with_label_values(&[method, path.unwrap_or("/"), "500"])
                        .inc();
                }
                return self.error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error fetching module: {}", e),
                );
            }
        };

        // Determine request type
        // Treat empty string path same as None (normalize trailing slash)
        let path_normalized = path.filter(|p| !p.is_empty()).unwrap_or("");
        debug!(
            "Path after normalization: '{}' (was {:?})",
            path_normalized, path
        );
        let path = path_normalized;

        // Check if it's a WASM component (magic bytes 0x00 0x61 0x73 0x6d 0x0d)
        // or a core WASM module (magic bytes 0x00 0x61 0x73 0x6d 0x01)
        let is_wasm = bytes.len() >= 5
            && bytes[0] == 0x00
            && bytes[1] == 0x61
            && bytes[2] == 0x73
            && bytes[3] == 0x6d
            && (bytes[4] == 0x0d || bytes[4] == 0x01); // 0x0d=component, 0x01=core module

        // If it's WASM (component or core module), execute it as API
        let response = if is_wasm {
            match self
                .execute_wasm_api(cid, path, body, method, query, headers)
                .await
            {
                Ok(response) => {
                    if let Some(metrics) = &self.metrics {
                        let status = response.status().as_u16().to_string();
                        metrics.http_requests_total
                            .with_label_values(&[method, path, &status])
                            .inc();
                    }
                    response
                }
                Err(e) => {
                    error!("Error executing WASM component: {}", e);
                    if let Some(metrics) = &self.metrics {
                        metrics.http_requests_total
                            .with_label_values(&[method, path, "500"])
                            .inc();
                    }
                    self.error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Execution error: {}", e),
                    )
                }
            }
        // If path is empty or ends with .html, serve as frontend
        } else if path.is_empty() || path.ends_with(".html") || path == &self.config.index_file {
            let response = self.serve_frontend(&bytes, path).await;
            if let Some(metrics) = &self.metrics {
                let status = response.status().as_u16().to_string();
                metrics.http_requests_total
                    .with_label_values(&[method, path, &status])
                    .inc();
            }
            response
        } else {
            // Otherwise, try to serve as static asset
            let response = self.serve_static_file(&bytes, path).await;
            if let Some(metrics) = &self.metrics {
                let status = response.status().as_u16().to_string();
                metrics.http_requests_total
                    .with_label_values(&[method, path, &status])
                    .inc();
            }
            response
        };
        
        // Track request duration
        if let Some(metrics) = &self.metrics {
            let duration = start.elapsed().as_secs_f64();
            metrics.http_request_duration
                .with_label_values(&[method, path])
                .observe(duration);
        }
        
        response
    }

    /// Handle a request for a named application
    pub async fn handle_app_request(
        &self,
        name: &str,
        path: Option<&str>,
        method: &str,
        query: Option<&str>,
        headers: &axum::http::HeaderMap,
        body: &axum::body::Bytes,
    ) -> Response {
        debug!(
            "Handling app request: {} path: {:?} query: {:?} method: {}",
            name, path, query, method
        );

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
        self.handle_cid_request(&cid, path, method, query, headers, body)
            .await
    }

    /// Fetch module bytes from cache or network
    async fn fetch_module(&self, cid: &str) -> Result<Option<Vec<u8>>> {
        use crate::wasm::ModuleCid;

        let module_cid = ModuleCid::new(cid.to_string());

        // Try cache first
        if let Some((_info, bytes)) = self.loader.get_from_cache(&module_cid).await {
            return Ok(Some(bytes.to_vec()));
        }

        // Network fetch
        if let Some(metadata) = self.network.find_module_by_cid(&module_cid).await? {
            let dependencies = metadata
                .dependencies
                .iter()
                .map(|dep| ModuleCid::new(dep.clone()))
                .collect::<Vec<_>>();

            // We have metadata, try to fetch from one of the providers
            for provider_str in metadata.providers {
                if let Ok(peer_id) = provider_str.parse() {
                    if let Ok(Some(bytes)) = self.network.fetch_module(&module_cid, peer_id).await {
                        // Cache it before returning?
                        // Note: fetch_module implies fetching bytes.
                        // NetworkNode::handle_command calls provider.handle_request which returns bytes.
                        // But we should cache it locally.
                        // The current `NetworkNode` logic for `FetchModule` command does NOT cache automatically.
                        // It just returns bytes via channel.
                        // So we should cache it here.

                        // Reconstruct info for cache (minimal info)
                        let info = crate::wasm::loader::ModuleInfo {
                            cid: module_cid.clone(),
                            name: metadata.name.clone(),
                            version: metadata.version.clone(),
                            size: bytes.len(),
                            dependencies: dependencies.clone(),
                            author: metadata.author.clone(),
                            description: metadata.description.clone(),
                        };
                        let bytes_arc = Arc::new(bytes.clone());
                        self.loader.add_to_cache(&module_cid, info, bytes_arc).await;

                        if let Err(err) = self.fetch_dependencies(&dependencies).await {
                            warn!("Failed to fetch dependencies for {}: {}", cid, err);
                        }

                        return Ok(Some(bytes));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn fetch_dependencies(&self, dependencies: &[crate::wasm::ModuleCid]) -> Result<()> {
        use crate::wasm::ModuleCid;
        use std::collections::{HashSet, VecDeque};

        let mut queue: VecDeque<ModuleCid> = dependencies.iter().cloned().collect();
        let mut seen = HashSet::new();

        while let Some(dep_cid) = queue.pop_front() {
            if !seen.insert(dep_cid.clone()) {
                continue;
            }

            if self.loader.get_from_cache(&dep_cid).await.is_some() {
                continue;
            }

            let metadata = match self.network.find_module_by_cid(&dep_cid).await? {
                Some(metadata) => metadata,
                None => {
                    warn!("Dependency {} not found in network metadata", dep_cid);
                    continue;
                }
            };

            let dep_dependencies = metadata
                .dependencies
                .iter()
                .map(|dep| ModuleCid::new(dep.clone()))
                .collect::<Vec<_>>();

            let mut fetched = None;
            for provider_str in metadata.providers {
                if let Ok(peer_id) = provider_str.parse() {
                    if let Ok(Some(bytes)) = self.network.fetch_module(&dep_cid, peer_id).await {
                        fetched = Some(bytes);
                        break;
                    }
                }
            }

            let bytes = match fetched {
                Some(bytes) => bytes,
                None => {
                    warn!("Failed to fetch dependency {} from any provider", dep_cid);
                    continue;
                }
            };

            let info = crate::wasm::loader::ModuleInfo {
                cid: dep_cid.clone(),
                name: metadata.name.clone(),
                version: metadata.version.clone(),
                size: bytes.len(),
                dependencies: dep_dependencies.clone(),
                author: metadata.author.clone(),
                description: metadata.description.clone(),
            };

            let bytes_arc = Arc::new(bytes.clone());
            self.loader.add_to_cache(&dep_cid, info, bytes_arc).await;
            for dep in dep_dependencies {
                if !seen.contains(&dep) {
                    queue.push_back(dep);
                }
            }
        }

        Ok(())
    }

    /// Resolve name to CID
    async fn resolve_name(&self, name: &str) -> Result<Option<String>> {
        let results = self.network.search_modules_by_name(name).await?;

        // results is Vec<ModuleMetadata>
        Ok(results.first().map(|meta| meta.cid.clone()))
    }

    /// Check if string looks like CID
    fn looks_like_cid(s: &str) -> bool {
        s.starts_with('b') && s.len() > 30
    }

    /// Serve frontend HTML (placeholder - will be enhanced)
    async fn serve_frontend(&self, bytes: &[u8], path: &str) -> Response {
        // Check if this is a bundled application (bincode serialized AppBundle)
        if let Ok(bundle) = crate::bundle::AppBundle::from_bytes(bytes) {
            info!(
                "Detected AppBundle: {} assets, {} bytes",
                bundle.assets.len(),
                bundle.metadata().total_size
            );

            // Determine which asset to serve
            let asset_path = if path.is_empty() || path == "/" {
                "index.html"
            } else {
                path.trim_start_matches('/')
            };

            // Try to serve the requested asset
            if let Some(asset_data) = bundle.get_asset(asset_path) {
                let content_type = crate::bundle::AppBundle::content_type_for_path(asset_path);
                info!(
                    "Serving asset: {} ({} bytes, {})",
                    asset_path,
                    asset_data.len(),
                    content_type
                );

                return self.build_asset_response(asset_path, asset_data, content_type);
            }

            // Asset not found in bundle - check if it's a SPA route
            // For SPA apps, fallback to index.html for navigation routes
            if !asset_path.contains('.') && bundle.get_asset("index.html").is_some() {
                info!(
                    "Asset '{}' not found, falling back to index.html (SPA mode)",
                    asset_path
                );
                let index_data = bundle.get_asset("index.html").unwrap();
                return self.build_asset_response("index.html", index_data, "text/html");
            }

            // Asset not found in bundle, return 404 with asset listing
            let asset_list = bundle.asset_paths().join(", ");
            return self.error_response(
                StatusCode::NOT_FOUND,
                &format!(
                    "Asset '{}' not found in bundle. Available assets: {}",
                    asset_path, asset_list
                ),
            );
        }

        // Check if this is a TAR archive (legacy support)
        if Self::is_tar_archive(bytes) {
            // Try to serve index.html from the bundle
            let index_path = if path.is_empty() || path == "/" {
                "index.html"
            } else {
                path
            };

            match Self::extract_from_tar(bytes, index_path).await {
                Ok(Some(html_data)) => {
                    info!(
                        "Serving frontend: {} ({} bytes)",
                        index_path,
                        html_data.len()
                    );
                    self.build_html_response(html_data)
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

    /// Build asset response with security headers and caching
    fn build_asset_response(&self, path: &str, data: &[u8], content_type: &str) -> Response {
        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type);

        // Add security headers for HTML files
        if content_type == "text/html" {
            builder = builder
                .header(
                    "Content-Security-Policy",
                    "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'"
                )
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "SAMEORIGIN")
                .header("Referrer-Policy", "strict-origin-when-cross-origin")
                .header(header::CACHE_CONTROL, "public, max-age=3600, must-revalidate");
        } else {
            // Static assets get long-term caching
            builder = builder
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
        }

        // Add ETag for cache validation
        builder = builder.header(
            header::ETAG,
            format!("\"{}\"", blake3::hash(data).to_hex()),
        );

        builder.body(data.to_vec().into()).unwrap()
    }

    /// Build HTML response with security headers
    fn build_html_response(&self, html_data: Vec<u8>) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .header(
                "Content-Security-Policy",
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'"
            )
            .header("X-Content-Type-Options", "nosniff")
            .header("X-Frame-Options", "SAMEORIGIN")
            .header("Referrer-Policy", "strict-origin-when-cross-origin")
            .header(header::CACHE_CONTROL, "public, max-age=3600, must-revalidate")
            .body(html_data.into())
            .unwrap()
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
            if Self::is_tar_archive(bytes) {
                "TAR bundle"
            } else {
                "Single WASM file"
            }
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
        let path = path.trim_start_matches('/');

        // Check if this is a bundled application (bincode serialized AppBundle)
        if let Ok(bundle) = crate::bundle::AppBundle::from_bytes(bytes) {
            // Try to get the requested asset from bundle
            if let Some(asset_data) = bundle.get_asset(path) {
                let content_type = crate::bundle::AppBundle::content_type_for_path(path);
                info!(
                    "Serving static file '{}' from bundle ({} bytes, {})",
                    path,
                    asset_data.len(),
                    content_type
                );

                return self.build_asset_response(path, asset_data, content_type);
            }

            // Asset not found in bundle - check SPA fallback for navigation routes
            if !path.contains('.') && bundle.get_asset("index.html").is_some() {
                info!(
                    "Static file '{}' not found, falling back to index.html (SPA mode)",
                    path
                );
                let index_data = bundle.get_asset("index.html").unwrap();
                return self.build_asset_response("index.html", index_data, "text/html");
            }

            // Asset not found in bundle
            let asset_list = bundle.asset_paths().join(", ");
            return self.error_response(
                StatusCode::NOT_FOUND,
                &format!(
                    "File '{}' not found in bundle. Available assets: {}",
                    path, asset_list
                ),
            );
        }

        // Check if this is a TAR archive (legacy support)
        if Self::is_tar_archive(bytes) {
            // Extract the requested file from the TAR archive
            match Self::extract_from_tar(bytes, path).await {
                Ok(Some(file_data)) => {
                    info!(
                        "Serving static file '{}' from TAR bundle ({} bytes)",
                        path,
                        file_data.len()
                    );

                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                        .header(
                            header::ETAG,
                            format!("\"{}\"", blake3::hash(&file_data).to_hex()),
                        )
                        .body(file_data.into())
                        .unwrap()
                }
                Ok(None) => self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!("File '{}' not found in bundle", path),
                ),
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
                .header(
                    header::ETAG,
                    format!("\"{}\"", blake3::hash(bytes).to_hex()),
                )
                .body(bytes.to_vec().into())
                .unwrap()
        }
    }

    /// Execute WASM as backend API
    async fn execute_wasm_api(
        &self,
        cid: &str,
        path: &str,
        body: &axum::body::Bytes,
        method: &str,
        query: Option<&str>,
        headers: &axum::http::HeaderMap,
    ) -> Result<Response> {
        use super::io::{WasmRequest, WasmResponse};

        info!(
            "Executing WASM API: {} path: {} method: {} query: {:?}",
            cid, path, method, query
        );

        // Fetch module
        let bytes = self.fetch_module(cid).await?.context("Module not found")?;

        // Create WasmRequest with the actual HTTP method
        // Ensure path has leading slash
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        // Convert body bytes to string (UTF-8)
        let body_str = String::from_utf8_lossy(body).to_string();

        let mut wasm_request = WasmRequest::new(method.to_string(), normalized_path, body_str);

        // Add HTTP headers to WasmRequest
        for (key, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                wasm_request =
                    wasm_request.with_header(key.as_str().to_string(), value_str.to_string());
            }
        }

        // Set content type from headers if present
        if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
            if let Ok(ct_str) = content_type.to_str() {
                wasm_request = wasm_request.with_content_type(ct_str.to_string());
            }
        }

        // Parse and add query parameters if present
        if let Some(query_str) = query {
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    // URL decode the key and value
                    let key = urlencoding::decode(key)
                        .unwrap_or_else(|_| key.into())
                        .to_string();
                    let value = urlencoding::decode(value)
                        .unwrap_or_else(|_| value.into())
                        .to_string();
                    wasm_request = wasm_request.with_query(key, value);
                }
            }
        }

        // Serialize request to JSON
        let request_json = wasm_request
            .to_json()
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
            let component = runtime.load_component(&bytes).context(format!(
                "Failed to load WASI P2 component (size: {} bytes)",
                bytes.len()
            ))?;
            info!("Loaded WASI P2 component for API execution");

            // Create store with stdin containing the request
            let mut store = runtime.create_store_with_stdin(request_json.into_bytes())?;

            // Execute the component using the Command pattern
            match runtime
                .execute_component_command(&mut store, &component)
                .await
            {
                Ok(_) => {
                    info!("Component executed successfully");
                }
                Err(e) => {
                    // Log full error chain for debugging
                    warn!("Component execution failed: {:?}", e);

                    // Build error message with full chain
                    let mut error_msg = format!("{}", e);
                    let mut source = e.source();
                    while let Some(err) = source {
                        error_msg.push_str(&format!("\n  Caused by: {}", err));
                        source = err.source();
                    }

                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(
                            serde_json::json!({
                                "error": "Component execution failed",
                                "message": error_msg,
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
                            if let Some(status) =
                                json_response.get("status").and_then(|v| v.as_u64())
                            {
                                let body = json_response
                                    .get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&stdout_str)
                                    .to_string();

                                let content_type = json_response
                                    .get("content_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("application/json");

                                return Ok(Response::builder()
                                    .status(
                                        StatusCode::from_u16(status as u16)
                                            .unwrap_or(StatusCode::OK),
                                    )
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
        let module = runtime.load_module(&bytes).context(format!(
            "Failed to load Wasm module (size: {} bytes)",
            bytes.len()
        ))?;
        info!("Loaded WASM module for API execution");

        // Create store with stdin containing the request
        let mut store = runtime.create_store_with_stdin(request_json.into_bytes())?;

        // Instantiate with WASI
        let instance = runtime.instantiate_with_wasi(&mut store, &module).await?;
        info!("Instantiated WASM module");

        // Look for API handler function (convention: _handle_request, handle_request, or _start for WASI modules)
        let handler_func_name = if instance.get_func(&mut store, "_handle_request").is_some() {
            "_handle_request"
        } else if instance.get_func(&mut store, "handle_request").is_some() {
            "handle_request"
        } else if instance.get_func(&mut store, "_start").is_some() {
            "_start"
        } else {
            // No standard handler found, return helpful error
            return Ok(Response::builder()
                .status(StatusCode::NOT_IMPLEMENTED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(
                    serde_json::json!({
                        "error": "No API handler found",
                        "message": "Module must export 'handle_request', '_handle_request', or '_start' function",
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
        match runtime
            .execute_function(&mut store, &instance, handler_func_name, &[])
            .await
        {
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
                    })
                    .to_string();

                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(response_text.into())
                        .unwrap());
                }

                // Parse the WASM response from stdout
                let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
                info!("WASM output: {} bytes", stdout_str.len());
                debug!("WASM response: {}", stdout_str);

                match WasmResponse::from_json(&stdout_str) {
                    Ok(wasm_response) => {
                        // Build HTTP response from WasmResponse
                        let mut response_builder = Response::builder().status(
                            StatusCode::from_u16(wasm_response.status).unwrap_or(StatusCode::OK),
                        );

                        // Add content type
                        let content_type = wasm_response
                            .content_type
                            .unwrap_or_else(|| "application/json".to_string());
                        response_builder =
                            response_builder.header(header::CONTENT_TYPE, content_type);

                        // Add custom headers
                        for (key, value) in wasm_response.headers {
                            response_builder = response_builder.header(key, value);
                        }

                        Ok(response_builder.body(wasm_response.body.into()).unwrap())
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
                // Log full error chain for debugging
                let mut error_chain = vec![e.to_string()];
                let mut current_error = e.source();
                while let Some(source) = current_error {
                    error_chain.push(source.to_string());
                    current_error = source.source();
                }
                error!("WASM execution error:");
                for (i, err) in error_chain.iter().enumerate() {
                    error!("  [{}] {}", i, err);
                }

                // Check stderr for error messages from the module
                let stderr_bytes = runtime.get_stderr(&store);
                if !stderr_bytes.is_empty() {
                    let stderr_str = String::from_utf8_lossy(&stderr_bytes);
                    error!("WASM stderr: {}", stderr_str);
                }

                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(
                        serde_json::json!({
                            "error": "Execution failed",
                            "message": error_chain.join(" -> "),
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
