use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::network::NetworkClient;
use crate::wasm::{ModuleCid, ModuleLoader};

/// Resolves URLs and names to content addresses (CIDs)
///
/// This component handles:
/// - CID validation
/// - Name to CID lookups via DHT
/// - Module availability checking
pub struct ContentResolver {
    network: NetworkClient,
    loader: Arc<ModuleLoader>,
}

impl ContentResolver {
    /// Create a new content resolver
    pub fn new(network: NetworkClient, loader: Arc<ModuleLoader>) -> Self {
        Self { network, loader }
    }

    /// Resolve a name or CID to a valid CID
    ///
    /// # Arguments
    /// * `identifier` - Either a CID (starts with 'b') or a human-readable name
    ///
    /// # Returns
    /// * `Ok(Some(cid))` - Successfully resolved to CID
    /// * `Ok(None)` - Name not found in DHT
    /// * `Err(_)` - Resolution error
    pub async fn resolve(&self, identifier: &str) -> Result<Option<String>> {
        // Check if it's already a CID (simple heuristic)
        if Self::looks_like_cid(identifier) {
            debug!("Identifier looks like CID: {}", identifier);
            return Ok(Some(identifier.to_string()));
        }

        // Otherwise, try to resolve name via DHT
        debug!("Resolving name: {}", identifier);
        self.resolve_name(identifier).await
    }

    /// Check if a string looks like a CID
    fn looks_like_cid(s: &str) -> bool {
        // Basic check: CIDs typically start with 'b' and are 40+ chars
        s.starts_with('b') && s.len() > 30 && s.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Resolve a human-readable name to CID via DHT
    async fn resolve_name(&self, name: &str) -> Result<Option<String>> {
        // Search for modules by name
        let results = self.network.search_modules_by_name(name).await?;

        if results.is_empty() {
            warn!("No modules found for name: {}", name);
            return Ok(None);
        }

        // Return the first matching CID
        // TODO: Handle multiple versions, let user select
        if let Some(module_metadata) = results.first() {
            debug!("Resolved '{}' to CID: {}", name, module_metadata.cid);
            Ok(Some(module_metadata.cid.clone()))
        } else {
            Ok(None)
        }
    }

    /// Check if a module is available (in cache or network)
    pub async fn is_available(&self, cid: &str) -> bool {
        let module_cid = ModuleCid::new(cid.to_string());

        // Check local cache first
        if self.loader.get_from_cache(&module_cid).await.is_some() {
            debug!("Module {} found in cache", cid);
            return true;
        }

        // Check if metadata exists in DHT
        if let Ok(Some(_metadata)) = self.network.find_module_by_cid(&module_cid).await {
            debug!("Module {} found in DHT", cid);
            return true;
        }

        warn!("Module {} not available", cid);
        false
    }

    /// Get module bytes from cache or network
    pub async fn fetch_module(&self, cid: &str) -> Result<Option<Vec<u8>>> {
        let module_cid = ModuleCid::new(cid.to_string());

        // Try cache first
        if let Some((_info, bytes)) = self.loader.get_from_cache(&module_cid).await {
            debug!("Fetched module {} from cache ({} bytes)", cid, bytes.len());
            return Ok(Some(bytes.to_vec()));
        }

        // Try to find in network
        debug!("Module {} not in cache, searching network...", cid);

        // First, check if module metadata exists
        if let Ok(Some(metadata)) = self.network.find_module_by_cid(&module_cid).await {
            debug!("Found metadata for {}, attempting fetch", cid);

            // Try fetch from providers
            for provider_str in metadata.providers {
                if let Ok(peer_id) = provider_str.parse() {
                    if let Ok(Some(bytes)) = self.network.fetch_module(&module_cid, peer_id).await {
                        return Ok(Some(bytes));
                    }
                }
            }

            warn!("Network fetch failed: no reachable providers");
            return Ok(None);
        }

        warn!("Module {} not found in network (no metadata)", cid);
        Ok(None)
    }
}
