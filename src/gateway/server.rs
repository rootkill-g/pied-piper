use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, State, ws::WebSocketUpgrade, ConnectInfo},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{debug, info, warn};

use super::handler::RequestHandler;
use super::resolver::ContentResolver;
use super::tls::TlsConfig;
use super::websocket::WsHandler;
use crate::metrics::Metrics;
use crate::network::NetworkClient;
use crate::security::SecurityMiddleware;
use crate::wasm::ModuleLoader;

/// Configuration for the HTTP Gateway
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub port: u16,
    pub https_port: Option<u16>,
    pub index_file: String,
    pub tls_config: Option<TlsConfig>,
    pub request_timeout_secs: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            https_port: None,
            index_file: "index.html".to_string(),
            tls_config: None,
            request_timeout_secs: 30,
        }
    }
}

/// Shared state for the gateway
#[derive(Clone)]
struct GatewayState {
    network: NetworkClient,
    #[allow(dead_code)]
    loader: Arc<ModuleLoader>,
    #[allow(dead_code)]
    config: GatewayConfig,
    #[allow(dead_code)]
    resolver: Arc<ContentResolver>,
    handler: Arc<RequestHandler>,
    ws_handler: Arc<WsHandler>,
    metrics: Arc<Metrics>,
    security: Arc<SecurityMiddleware>,
}

/// HTTP Gateway Server
pub struct GatewayServer {
    config: GatewayConfig,
    network: NetworkClient,
    loader: Arc<ModuleLoader>,
    security_config: crate::config::SecurityConfig,
}

impl GatewayServer {
    /// Create a new Gateway Server
    pub fn new(config: GatewayConfig, network: NetworkClient, loader: Arc<ModuleLoader>) -> Self {
        Self {
            config,
            network,
            loader,
            security_config: crate::config::SecurityConfig::default(),
        }
    }

    /// Create a new Gateway Server with custom security config
    pub fn with_security(mut self, security_config: crate::config::SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Start the Gateway Server
    pub async fn start(&self) -> Result<()> {
        let state = self.create_state();

        // Start background cleanup task for rate limiter
        SecurityMiddleware::start_cleanup_task(state.security.rate_limiter.clone());

        info!("🔒 Security enabled: rate_limit={}/min, max_body={}MB", 
            self.security_config.rate_limit_per_minute,
            self.security_config.max_request_body_size / (1024 * 1024));

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .route("/info", get(info_handler))
            .route("/metrics", get(metrics_handler))
            // WebSocket endpoints
            .route("/ws/cid/:cid", get(handle_ws_cid))
            .route("/ws/app/:name", get(handle_ws_app))
            // Direct CID access
            .route("/cid/:cid", any(handle_cid_request))
            .route("/cid/:cid/*path", any(handle_cid_request_with_path))
            // Named app access
            .route("/app/:name", any(handle_app_request))
            .route("/app/:name/*path", any(handle_app_request_with_path))
            // Root and Fallback
            .route("/", get(root_handler))
            .fallback(not_found_handler)
            // Security, compression, and timeout middleware stack
            .layer(
                ServiceBuilder::new()
                    .layer(middleware::from_fn_with_state(state.clone(), security_middleware))
                    .layer(RequestBodyLimitLayer::new(self.security_config.max_request_body_size))
                    .layer(TimeoutLayer::new(Duration::from_secs(self.config.request_timeout_secs)))
                    .layer(CompressionLayer::new())
            )
            // Add state
            .with_state(state);

        // Start HTTP server
        let http_handle = {
            let app = app.clone();
            let port = self.config.port;
            tokio::spawn(async move {
                info!("🌐 Starting HTTP Gateway on port {}", port);
                let addr = SocketAddr::from(([0, 0, 0, 0], port));
                info!("✅ HTTP Gateway listening on http://{}", addr);

                axum_server::bind(addr)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .context("HTTP Gateway server error")
            })
        };

        // Start HTTPS server if TLS is configured
        let https_handle = if let Some(tls_config) = &self.config.tls_config {
            let https_port = self.config.https_port.unwrap_or(8443);

            info!("🔒 TLS/HTTPS enabled");
            tls_config.validate()?;

            let rustls_config = tls_config.build_server_config().await?;
            let app = app.clone();

            Some(tokio::spawn(async move {
                info!("🔐 Starting HTTPS Gateway on port {}", https_port);
                let addr = SocketAddr::from(([0, 0, 0, 0], https_port));
                info!("✅ HTTPS Gateway listening on https://{}", addr);

                axum_server::bind_rustls(addr, rustls_config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .context("HTTPS Gateway server error")
            }))
        } else {
            None
        };

        // Wait for either server to complete (or error)
        if let Some(https_handle) = https_handle {
            tokio::select! {
                result = http_handle => result??,
                result = https_handle => result??,
            }
        } else {
            http_handle.await??;
        }

        Ok(())
    }

    /// Create shared state for handlers
    fn create_state(&self) -> Arc<GatewayState> {
        let metrics = Arc::new(Metrics::new().expect("Failed to create metrics"));
        
        // Convert config::SecurityConfig to security::SecurityConfig
        let security_config = crate::security::SecurityConfig {
            max_request_body_size: self.security_config.max_request_body_size,
            max_header_size: self.security_config.max_header_size,
            rate_limit_per_minute: self.security_config.rate_limit_per_minute,
            rate_limit_burst: self.security_config.rate_limit_burst,
            max_connections_per_ip: self.security_config.max_connections_per_ip,
            max_concurrent_requests: self.security_config.max_concurrent_requests,
            request_timeout_secs: self.config.request_timeout_secs,
            enable_hsts: self.security_config.enable_hsts,
            hsts_max_age: self.security_config.hsts_max_age,
            enable_strict_csp: self.security_config.enable_strict_csp,
            cors_allowed_origins: self.security_config.cors_allowed_origins.clone(),
            block_suspicious_user_agents: self.security_config.block_suspicious_user_agents,
            max_path_depth: self.security_config.max_path_depth,
            allowed_extensions: self.security_config.allowed_extensions.clone(),
        };

        Arc::new(GatewayState {
            network: self.network.clone(),
            loader: self.loader.clone(),
            config: self.config.clone(),
            resolver: Arc::new(ContentResolver::new(
                self.network.clone(),
                self.loader.clone(),
            )),
            handler: Arc::new(
                RequestHandler::new(
                    self.network.clone(),
                    self.loader.clone(),
                    self.config.clone(),
                )
                .with_metrics(metrics.clone())
            ),
            ws_handler: Arc::new(WsHandler::new(self.network.clone(), self.loader.clone())),
            metrics,
            security: Arc::new(SecurityMiddleware::new(security_config)),
        })
    }
}

// --- Security Middleware ---

async fn security_middleware(
    State(state): State<Arc<GatewayState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = addr.ip();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();

    // Skip security checks for health endpoints
    if path == "/health" || path == "/ready" {
        return Ok(next.run(req).await);
    }

    // Rate limiting
    state.security.rate_limiter.check_rate_limit(ip).await?;

    // Connection tracking
    state.security.connection_tracker.register_connection(ip).await?;

    // Path validation
    state.security.validator.validate_path(&path)?;

    // Extension validation
    state.security.validator.validate_extension(&path)?;

    // Header validation
    state.security.validator.validate_headers(&headers)?;

    // Execute request
    let response = next.run(req).await;

    // Unregister connection
    state.security.connection_tracker.unregister_connection(ip).await;

    Ok(response)
}

// --- Handlers ---

async fn health_check(State(state): State<Arc<GatewayState>>) -> &'static str {
    state.metrics.http_requests_total
        .with_label_values(&["GET", "/health", "200"])
        .inc();
    "OK"
}

async fn readiness_check(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    // Check if the node has at least one peer connection
    let peer_count = state.network.peer_count().await;
    
    state.metrics.http_requests_total
        .with_label_values(&["GET", "/ready", if peer_count > 0 { "200" } else { "503" }])
        .inc();
    
    if peer_count > 0 {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "ready": true,
                "peer_count": peer_count,
                "message": "Gateway is ready to accept traffic"
            }))
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "ready": false,
                "peer_count": 0,
                "message": "Gateway is not ready - no peer connections"
            }))
        )
    }
}

async fn info_handler(State(state): State<Arc<GatewayState>>) -> Json<serde_json::Value> {
    let peer_id = state.network.local_peer_id();

    state.metrics.http_requests_total
        .with_label_values(&["GET", "/info", "200"])
        .inc();

    Json(serde_json::json!({
        "gateway": "Pied Piper HTTP Gateway",
        "version": env!("CARGO_PKG_VERSION"),
        "peer_id": peer_id.to_string(),
        "status": "online"
    }))
}

async fn metrics_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let metrics_text = state.metrics.export();
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics_text,
    )
}

async fn handle_cid_request(
    State(state): State<Arc<GatewayState>>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    axum::extract::Path(cid): axum::extract::Path<String>,
    query: axum::extract::RawQuery,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let query_str = query.0.as_deref();
    let cid_normalized = cid.trim_end_matches('/');
    debug!("CID request: {} method: {}", cid_normalized, method);

    state
        .handler
        .handle_cid_request(
            cid_normalized,
            None,
            method.as_str(),
            query_str,
            &headers,
            &body,
        )
        .await
}

async fn handle_cid_request_with_path(
    State(state): State<Arc<GatewayState>>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    axum::extract::Path((cid, path)): axum::extract::Path<(String, String)>,
    query: axum::extract::RawQuery,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let query_str = query.0.as_deref();
    let cid_normalized = cid.trim_end_matches('/');
    debug!("CID request: {} path: {}", cid_normalized, path);

    state
        .handler
        .handle_cid_request(
            cid_normalized,
            Some(&path),
            method.as_str(),
            query_str,
            &headers,
            &body,
        )
        .await
}

async fn handle_app_request(
    State(state): State<Arc<GatewayState>>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
    query: axum::extract::RawQuery,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let query_str = query.0.as_deref();
    let name_normalized = name.trim_end_matches('/');
    debug!("App request: {} method: {}", name_normalized, method);

    state
        .handler
        .handle_app_request(
            name_normalized,
            None,
            method.as_str(),
            query_str,
            &headers,
            &body,
        )
        .await
}

async fn handle_app_request_with_path(
    State(state): State<Arc<GatewayState>>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    axum::extract::Path((name, path)): axum::extract::Path<(String, String)>,
    query: axum::extract::RawQuery,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let query_str = query.0.as_deref();
    let name_normalized = name.trim_end_matches('/');
    debug!("App request: {} path: {}", name_normalized, path);

    state
        .handler
        .handle_app_request(
            name_normalized,
            Some(&path),
            method.as_str(),
            query_str,
            &headers,
            &body,
        )
        .await
}

async fn root_handler() -> axum::response::Html<&'static str> {
    axum::response::Html(
        r#"
<!DOCTYPE html>
<html>
<head><title>Pied Piper Gateway</title></head>
<body>
<h1>🌐 Pied Piper HTTP Gateway</h1>
<p>Running with internal Network Client.</p>
</body>
</html>
"#,
    )
}

async fn not_found_handler() -> axum::response::Html<&'static str> {
    axum::response::Html("<h1>404 - Not Found</h1>")
}

/// WebSocket handler for CID-based modules
async fn handle_ws_cid(
    State(state): State<Arc<GatewayState>>,
    Path(cid): Path<String>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    debug!("WebSocket upgrade request for CID: {}", cid);
    state.ws_handler.clone().handle_ws_cid(ws, cid).await
}

/// WebSocket handler for named applications
async fn handle_ws_app(
    State(state): State<Arc<GatewayState>>,
    Path(name): Path<String>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    debug!("WebSocket upgrade request for app: {}", name);
    state.ws_handler.clone().handle_ws_app(ws, name).await
}
