use anyhow::{Context, Result};
use libp2p::{
    kad::{Record, RecordKey, Quorum},
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::wasm::loader::{ModuleCid, ModuleInfo};

/// Module metadata stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub cid: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub size: usize,
    pub dependencies: Vec<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub providers: Vec<String>, // PeerIds as strings
    pub published_at: u64, // Unix timestamp
}

/// Publisher for WebAssembly modules to the network
pub struct ModulePublisher {
    local_peer_id: PeerId,
}

impl ModulePublisher {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self { local_peer_id }
    }
    
    /// Publish module metadata to DHT
    /// Returns the DHT key where the metadata was published
    pub fn create_metadata_record(
        &self,
        module_info: &ModuleInfo,
        module_bytes: &[u8],
    ) -> Result<Record> {
        let metadata = ModuleMetadata {
            cid: module_info.cid.to_string(),
            name: module_info.name.clone(),
            version: module_info.version.clone(),
            size: module_bytes.len(),
            dependencies: module_info.dependencies.iter().map(|d| d.to_string()).collect(),
            author: module_info.author.clone(),
            description: module_info.description.clone(),
            providers: vec![self.local_peer_id.to_string()],
            published_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Serialize metadata
        let value = serde_json::to_vec(&metadata)
            .context("Failed to serialize module metadata")?;
        
        // Create DHT key from CID
        let key = RecordKey::new(&format!("module:{}", module_info.cid));
        
        // Create DHT record
        let record = Record {
            key,
            value,
            publisher: Some(self.local_peer_id),
            expires: None, // No expiration for now
        };
        
        let name_display = module_info.name.as_deref().unwrap_or("unnamed");
        info!(
            "Created metadata record for module {} ({})",
            name_display, module_info.cid
        );
        
        Ok(record)
    }
    
    /// Create a name-to-CID mapping record for module discovery
    pub fn create_name_record(
        &self,
        name: &str,
        version: &str,
        cid: &ModuleCid,
    ) -> Result<Record> {
        // Create DHT key from name:version
        let key = RecordKey::new(&format!("name:{}:{}", name, version));
        
        // Value is just the CID
        let value = cid.to_string().into_bytes();
        
        let record = Record {
            key,
            value,
            publisher: Some(self.local_peer_id),
            expires: None,
        };
        
        info!("Created name record for {}:{} -> {}", name, version, cid);
        
        Ok(record)
    }
    
    /// Create announcement message for GossipSub
    pub fn create_announcement_message(
        &self,
        module_info: &ModuleInfo,
    ) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Announcement {
            r#type: String,
            cid: String,
            name: Option<String>,
            version: Option<String>,
            provider: String,
            timestamp: u64,
        }
        
        let announcement = Announcement {
            r#type: "module_published".to_string(),
            cid: module_info.cid.to_string(),
            name: module_info.name.clone(),
            version: module_info.version.clone(),
            provider: self.local_peer_id.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        serde_json::to_vec(&announcement)
            .context("Failed to serialize announcement")
    }
}

/// Topic name for module announcements
pub const MODULE_ANNOUNCEMENTS_TOPIC: &str = "pied-piper/modules/announcements";

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_metadata_record() {
        let peer_id = PeerId::random();
        let publisher = ModulePublisher::new(peer_id);
        
        let cid = ModuleCid::from_bytes(b"test module");
        let info = ModuleInfo {
            cid: cid.clone(),
            name: "test-module".to_string(),
            version: "1.0.0".to_string(),
            size: 100,
            dependencies: vec![],
            author: Some("test".to_string()),
            description: Some("Test module".to_string()),
        };
        
        let record = publisher.create_metadata_record(&info, b"test module");
        assert!(record.is_ok());
    }
}
