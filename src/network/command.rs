use crate::content::publisher::ModuleMetadata;
use crate::wasm::loader::{ModuleCid, ModuleInfo};
use libp2p::PeerId;
use tokio::sync::oneshot;

/// Commands sent to the network node
#[derive(Debug)]
pub enum NetworkCommand {
    /// Start providing a module
    ProvideModule {
        info: ModuleInfo,
        bytes: Vec<u8>,
        response: oneshot::Sender<anyhow::Result<ModuleCid>>,
    },

    /// Find a module by CID
    FindModule {
        cid: ModuleCid,
        response: oneshot::Sender<anyhow::Result<Option<ModuleMetadata>>>,
    },

    /// Search for modules by name
    SearchModules {
        name: String,
        response: oneshot::Sender<anyhow::Result<Vec<ModuleMetadata>>>,
    },

    /// Fetch a module from a specific peer
    FetchModule {
        cid: ModuleCid,
        peer_id: PeerId,
        response: oneshot::Sender<anyhow::Result<Option<Vec<u8>>>>,
    },
    
    /// Register a persistent name for a module
    RegisterName {
        name: String,
        cid: ModuleCid,
        version: Option<String>,
        response: oneshot::Sender<anyhow::Result<()>>,
    },
    
    /// Resolve a persistent name to a CID
    ResolveName {
        name: String,
        response: oneshot::Sender<anyhow::Result<Option<String>>>,
    },
    
    /// Find all versions of a module by name
    FindVersions {
        name: String,
        response: oneshot::Sender<anyhow::Result<Vec<ModuleMetadata>>>,
    },
    
    /// Find best matching version for a requirement
    FindBestVersion {
        name: String,
        requirement: String,
        response: oneshot::Sender<anyhow::Result<Option<ModuleMetadata>>>,
    },

    /// Get local peer ID
    GetPeerId { response: oneshot::Sender<PeerId> },
}
