use anyhow::{Context, Result};
use axum::{
    Router as AxumRouter,
    extract::State,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug};

use crate::network::NetworkNode;
use crate::wasm::ModuleLoader;
use super::resolver::ContentResolver;
use super::handler::RequestHandler;

/// Configuration for the HTTP Gateway
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// HTTP listening address (e.g., "127.0.0.1:8080")
    pub listen_addr: SocketAddr,
    
    /// Enable CORS for browser access
    pub enable_cors: bool,
    
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Default index file (default: "index.html")
    pub index_file: String,
    
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
            request_timeout: 30,
            index_file: "index.html".to_string(),
            verbose: false,
        }
    }
}

/// Shared state for the gateway
#[derive(Clone)]
pub struct GatewayState {
    pub network: Arc<Mutex<NetworkNode>>,
    pub resolver: Arc<ContentResolver>,
    pub handler: Arc<RequestHandler>,
    pub config: GatewayConfig,
}

/// HTTP Gateway Server
/// 
/// Provides HTTP/HTTPS access to applications deployed on the Pied Piper network.
/// Supports both direct CID access and human-readable names.
pub struct GatewayServer {
    config: GatewayConfig,
    state: GatewayState,
}

impl GatewayServer {
    /// Create a new gateway server
    pub async fn new(
        config: GatewayConfig,
        network: NetworkNode,
        loader: Arc<ModuleLoader>,
    ) -> Result<Self> {
        let network = Arc::new(Mutex::new(network));
        
        let resolver = Arc::new(ContentResolver::new(
            network.clone(),
            loader.clone(),
        ));
        
        let handler = Arc::new(RequestHandler::new(
            network.clone(),
            loader,
            config.clone(),
        ));
        
        let state = GatewayState {
            network,
            resolver,
            handler,
            config: config.clone(),
        };
        
        Ok(Self { config, state })
    }
    
    /// Start the gateway server
    pub async fn start(self) -> Result<()> {
        info!("🌐 Starting HTTP Gateway on {}", self.config.listen_addr);
        
        let app = self.create_router();
        
        let listener = tokio::net::TcpListener::bind(&self.config.listen_addr)
            .await
            .context("Failed to bind HTTP server")?;
        
        info!("✅ Gateway listening on http://{}", self.config.listen_addr);
        info!("📡 Ready to serve decentralized applications");
        
        axum::serve(listener, app)
            .await
            .context("Gateway server error")?;
        
        Ok(())
    }
    
    /// Create the Axum router with all routes
    fn create_router(&self) -> AxumRouter {
        AxumRouter::new()
            // Health check endpoint
            .route("/health", get(health_check))
            
            // Gateway info endpoint
            .route("/info", get(gateway_info))
            
            // Application routes
            // Direct CID access: /cid/<cid>/*path
            .route("/cid/:cid", get(handle_cid_request))
            .route("/cid/:cid/*path", get(handle_cid_request_with_path))
            
            // Named app access: /app/<name>/*path
            .route("/app/:name", get(handle_app_request))
            .route("/app/:name/*path", get(handle_app_request_with_path))
            
            // API routes (POST/PUT/DELETE)
            .route("/cid/:cid/api/*path", post(handle_api_request))
            .route("/app/:name/api/*path", post(handle_api_request))
            
            // Root handler (could map to a default app)
            .route("/", get(root_handler))
            
            // 404 handler
            .fallback(not_found_handler)
            
            .with_state(self.state.clone())
    }
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Gateway info endpoint
async fn gateway_info(State(state): State<GatewayState>) -> String {
    let network = state.network.lock().await;
    let peer_id = network.local_peer_id();
    
    format!(
        r#"{{
  "gateway": "Pied Piper HTTP Gateway",
  "version": "0.2.0",
  "peer_id": "{}",
  "status": "online"
}}"#,
        peer_id
    )
}

/// Handle CID-based requests (no path)
async fn handle_cid_request(
    State(state): State<GatewayState>,
    axum::extract::Path(cid): axum::extract::Path<String>,
) -> axum::response::Response {
    debug!("CID request: {}", cid);
    
    state.handler.handle_cid_request(&cid, None).await
}

/// Handle CID-based requests with path
async fn handle_cid_request_with_path(
    State(state): State<GatewayState>,
    axum::extract::Path((cid, path)): axum::extract::Path<(String, String)>,
) -> axum::response::Response {
    debug!("CID request: {} path: {}", cid, path);
    
    state.handler.handle_cid_request(&cid, Some(&path)).await
}

/// Handle named application requests (no path)
async fn handle_app_request(
    State(state): State<GatewayState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> axum::response::Response {
    debug!("App request: {}", name);
    
    state.handler.handle_app_request(&name, None).await
}

/// Handle named application requests with path
async fn handle_app_request_with_path(
    State(state): State<GatewayState>,
    axum::extract::Path((name, path)): axum::extract::Path<(String, String)>,
) -> axum::response::Response {
    debug!("App request: {} path: {}", name, path);
    
    state.handler.handle_app_request(&name, Some(&path)).await
}

/// Handle API requests (POST/PUT/DELETE)
async fn handle_api_request(
    State(state): State<GatewayState>,
    axum::extract::Path((target, path)): axum::extract::Path<(String, String)>,
    body: String,
) -> axum::response::Response {
    debug!("API request: {} path: {}", target, path);
    
    state.handler.handle_api_request(&target, &path, body).await
}

/// Root handler
async fn root_handler() -> axum::response::Html<&'static str> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Pied Piper Gateway</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #2563eb; }
        code { background: #f3f4f6; padding: 2px 6px; border-radius: 3px; }
        .section { margin: 30px 0; }
    </style>
</head>
<body>
    <h1>🌐 Pied Piper HTTP Gateway</h1>
    <p>Welcome to the decentralized web! This gateway provides HTTP access to applications running on the Pied Piper network.</p>
    
    <div class="section">
        <h2>Access Applications</h2>
        <ul>
            <li><strong>By CID:</strong> <code>/cid/&lt;content-id&gt;/</code></li>
            <li><strong>By Name:</strong> <code>/app/&lt;app-name&gt;/</code></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Endpoints</h2>
        <ul>
            <li><code>GET /health</code> - Health check</li>
            <li><code>GET /info</code> - Gateway information</li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Examples</h2>
        <pre>
# Access app by CID
http://localhost:8080/cid/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/

# Access app by name
http://localhost:8080/app/my-awesome-app/

# Call API endpoint
curl -X POST http://localhost:8080/app/my-app/api/hello -d '{"name":"World"}'
        </pre>
    </div>
</body>
</html>"#,
    )
}

/// 404 handler
async fn not_found_handler() -> axum::response::Html<&'static str> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>404 - Not Found</title>
    <style>
        body { font-family: sans-serif; text-align: center; margin-top: 100px; }
        h1 { color: #dc2626; }
    </style>
</head>
<body>
    <h1>404 - Not Found</h1>
    <p>The requested application or resource was not found on the network.</p>
    <p><a href="/">Return to Gateway Home</a></p>
</body>
</html>"#,
    )
}
