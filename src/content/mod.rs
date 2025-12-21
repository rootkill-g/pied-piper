// Content distribution module for WebAssembly modules
// Handles publishing, discovery, and fetching of WASM modules over libp2p

pub mod protocol;
pub mod publisher;
pub mod discovery;
pub mod provider;

pub use protocol::{ContentProtocol, ModuleRequest, ModuleResponse};
pub use publisher::ModulePublisher;
pub use discovery::ModuleDiscovery;
pub use provider::ModuleProvider;
