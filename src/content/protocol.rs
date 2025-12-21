use libp2p::{
    request_response::{self, ProtocolSupport},
    StreamProtocol,
};
use serde::{Deserialize, Serialize};

/// Protocol for requesting and providing WASM modules
pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/pied-piper/module/1.0.0");

/// Request for a WASM module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleRequest {
    /// Request module bytes by CID
    GetModule { cid: String },
    
    /// Request module metadata by CID
    GetModuleInfo { cid: String },
    
    /// Search for modules by name
    SearchByName { name: String },
    
    /// List all modules a peer provides
    ListModules,
}

/// Response to a module request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleResponse {
    /// Module bytes
    Module {
        cid: String,
        bytes: Vec<u8>,
    },
    
    /// Module metadata
    ModuleInfo {
        cid: String,
        name: Option<String>,
        version: Option<String>,
        size: usize,
        dependencies: Vec<String>,
        author: Option<String>,
        description: Option<String>,
    },
    
    /// Search results
    SearchResults {
        modules: Vec<SearchResult>,
    },
    
    /// List of available modules
    ModuleList {
        cids: Vec<String>,
    },
    
    /// Module not found
    NotFound { cid: String },
    
    /// Error occurred
    Error { message: String },
}

/// Search result for a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub cid: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Content distribution protocol behaviour
pub type ContentProtocol = request_response::cbor::Behaviour<ModuleRequest, ModuleResponse>;

/// Create a new content distribution protocol
pub fn new_content_protocol() -> ContentProtocol {
    request_response::cbor::Behaviour::new(
        [(PROTOCOL_NAME, ProtocolSupport::Full)],
        request_response::Config::default(),
    )
}
