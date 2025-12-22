use anyhow::{Context, Result};
use libp2p::{
    PeerId,
    kad::{QueryId, RecordKey},
    request_response::OutboundRequestId,
};
use std::collections::HashMap;
use tokio::sync::oneshot;
use tracing::{debug, info};

use super::protocol::SearchResult;
use super::publisher::{ModuleMetadata, NameRegistration};
use crate::wasm::loader::ModuleCid;

/// Tracks ongoing discovery queries
#[derive(Debug)]
pub struct DiscoveryQuery {
    pub query_type: QueryType,
    pub responder: Option<ClientResponder>,
    pub initiated_at: std::time::Instant,
}

#[derive(Debug)]
pub enum ClientResponder {
    Metadata(oneshot::Sender<anyhow::Result<Option<ModuleMetadata>>>),
    Search(oneshot::Sender<anyhow::Result<Vec<ModuleMetadata>>>),
    SearchResults(oneshot::Sender<anyhow::Result<SearchResponse>>),
    Bytes(oneshot::Sender<anyhow::Result<Option<Vec<u8>>>>),
}

#[derive(Debug, Clone)]
pub enum QueryType {
    /// Looking up module metadata by CID
    ModuleMetadata { cid: String },

    /// Looking up CID by name:version
    ModuleName { name: String, version: Option<String> },

    /// Searching modules by name via request-response
    SearchByName { name: String },

    /// Searching for providers of a module
    Providers { cid: String },
}

/// Module discovery service
pub struct ModuleDiscovery {
    /// Pending DHT queries
    pending_queries: HashMap<QueryId, DiscoveryQuery>,

    /// Pending request-response queries
    pending_requests: HashMap<OutboundRequestId, DiscoveryQuery>,
}

impl ModuleDiscovery {
    pub fn new() -> Self {
        Self {
            pending_queries: HashMap::new(),
            pending_requests: HashMap::new(),
        }
    }

    /// Create a DHT key for module metadata lookup
    pub fn metadata_key(cid: &ModuleCid) -> RecordKey {
        RecordKey::new(&format!("module:{}", cid))
    }

    /// Create a DHT key for name-to-CID lookup
    pub fn name_key(name: &str, version: &str) -> RecordKey {
        RecordKey::new(&format!("name:{}:{}", name, version))
    }

    /// Create a DHT key for name-to-CID lookup (latest)
    pub fn name_key_latest(name: &str) -> RecordKey {
        RecordKey::new(&format!("name:{}", name))
    }
    
    /// Create a DHT key for persistent name lookup
    pub fn persistent_name_key(name: &str) -> RecordKey {
        RecordKey::new(&format!("persistent-name:{}", name))
    }

    /// Register a pending DHT query
    pub fn register_dht_query(
        &mut self,
        query_id: QueryId,
        query_type: QueryType,
        responder: Option<ClientResponder>,
    ) {
        debug!("Registered DHT query {:?}: {:?}", query_id, query_type);
        self.pending_queries.insert(
            query_id,
            DiscoveryQuery {
                query_type,
                responder,
                initiated_at: std::time::Instant::now(),
            },
        );
    }

    /// Register a pending request-response query
    pub fn register_request(
        &mut self,
        request_id: OutboundRequestId,
        query_type: QueryType,
        responder: Option<ClientResponder>,
    ) {
        debug!("Registered request {:?}: {:?}", request_id, query_type);
        self.pending_requests.insert(
            request_id,
            DiscoveryQuery {
                query_type,
                responder,
                initiated_at: std::time::Instant::now(),
            },
        );
    }

    /// Complete a DHT query and return its type and responder
    pub fn complete_dht_query(
        &mut self,
        query_id: &QueryId,
    ) -> Option<(QueryType, Option<ClientResponder>)> {
        self.pending_queries.remove(query_id).map(|q| {
            let duration = q.initiated_at.elapsed();
            info!("DHT query {:?} completed in {:?}", query_id, duration);
            (q.query_type, q.responder)
        })
    }

    /// Complete a request-response query
    pub fn complete_request(
        &mut self,
        request_id: &OutboundRequestId,
    ) -> Option<(QueryType, Option<ClientResponder>)> {
        self.pending_requests.remove(request_id).map(|q| {
            let duration = q.initiated_at.elapsed();
            info!("Request {:?} completed in {:?}", request_id, duration);
            (q.query_type, q.responder)
        })
    }

    /// Parse module metadata from DHT record
    pub fn parse_metadata(&self, data: &[u8]) -> Result<ModuleMetadata> {
        serde_json::from_slice(data).context("Failed to parse module metadata")
    }

    /// Parse CID from name record
    pub fn parse_cid(&self, data: &[u8]) -> Result<String> {
        String::from_utf8(data.to_vec()).context("Failed to parse CID from name record")
    }
    
    /// Parse persistent name registration
    pub fn parse_name_registration(&self, data: &[u8]) -> Result<NameRegistration> {
        serde_json::from_slice(data).context("Failed to parse name registration")
    }

    /// Get statistics about pending queries
    pub fn stats(&self) -> DiscoveryStats {
        DiscoveryStats {
            pending_dht_queries: self.pending_queries.len(),
            pending_requests: self.pending_requests.len(),
        }
    }

    /// Clean up queries older than timeout
    pub fn cleanup_stale_queries(&mut self, timeout: std::time::Duration) {
        let now = std::time::Instant::now();

        self.pending_queries.retain(|id, query| {
            if now.duration_since(query.initiated_at) > timeout {
                info!("Removing stale DHT query {:?}", id);
                false
            } else {
                true
            }
        });

        self.pending_requests.retain(|id, query| {
            if now.duration_since(query.initiated_at) > timeout {
                info!("Removing stale request {:?}", id);
                false
            } else {
                true
            }
        });
    }
}

impl Default for ModuleDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub pending_dht_queries: usize,
    pub pending_requests: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub peer_id: PeerId,
    pub results: Vec<SearchResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_key() {
        let cid = ModuleCid::from_bytes(b"test");
        let key = ModuleDiscovery::metadata_key(&cid);
        assert!(key.as_ref().starts_with(b"module:"));
    }

    #[test]
    fn test_name_key() {
        let key = ModuleDiscovery::name_key("my-module", "1.0.0");
        assert_eq!(key.as_ref(), b"name:my-module:1.0.0");
    }

    #[test]
    fn test_name_key_latest() {
        let key = ModuleDiscovery::name_key_latest("my-module");
        assert_eq!(key.as_ref(), b"name:my-module");
    }
}
