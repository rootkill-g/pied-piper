/// HTTP Gateway for serving WebAssembly applications
/// 
/// This module provides an HTTP server that bridges traditional web browsers
/// to the decentralized Pied Piper network. It handles:
/// - URL to CID resolution
/// - Static file serving (HTML/CSS/JS)
/// - API request routing to WASM backends
/// - Multi-module application support

pub mod server;
pub mod router;
pub mod resolver;
pub mod handler;

pub use server::{GatewayServer, GatewayConfig};
pub use router::Router;
pub use resolver::ContentResolver;
pub use handler::{RequestHandler, WasmHandler};
