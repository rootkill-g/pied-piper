pub mod handler;
pub mod io;
pub mod resolver;
pub mod router;
/// HTTP Gateway for serving WebAssembly applications
///
/// This module provides an HTTP server that bridges traditional web browsers
/// to the decentralized Pied Piper network. It handles:
/// - URL to CID resolution
/// - Static file serving (HTML/CSS/JS)
/// - API request routing to WASM backends
/// - Multi-module application support
pub mod server;
pub mod tls;
pub mod websocket;

pub use handler::{RequestHandler, WasmHandler};
pub use io::{WasmRequest, WasmResponse};
pub use resolver::ContentResolver;
pub use router::Router;
pub use server::{GatewayConfig, GatewayServer};
pub use tls::{TlsConfig, default_cert_dir, ensure_cert_dir};
pub use websocket::WsHandler;
