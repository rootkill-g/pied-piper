// Content distribution module for WebAssembly modules
// Handles publishing, discovery, and fetching of WASM modules over libp2p

pub mod discovery;
pub mod protocol;
pub mod provider;
pub mod publisher;

pub use discovery::ModuleDiscovery;
pub use protocol::{ContentProtocol, ModuleRequest, ModuleResponse};
pub use provider::ModuleProvider;
pub use publisher::ModulePublisher;
