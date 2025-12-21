use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::wasm::loader::{ModuleCid, ModuleInfo, ModuleLoader};
use super::protocol::{ModuleRequest, ModuleResponse, SearchResult};

/// Provides WebAssembly modules to other peers
pub struct ModuleProvider {
    /// Module loader for accessing cached modules
    loader: Arc<ModuleLoader>,
    
    /// Modules we're actively providing (CID -> module bytes)
    provided_modules: Arc<RwLock<HashMap<String, Arc<Vec<u8>>>>>,
    
    /// Module metadata (CID -> info)
    module_info: Arc<RwLock<HashMap<String, ModuleInfo>>>,
}

impl ModuleProvider {
    pub fn new(loader: Arc<ModuleLoader>) -> Self {
        Self {
            loader,
            provided_modules: Arc::new(RwLock::new(HashMap::new())),
            module_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a module to be provided to the network
    pub async fn provide_module(
        &self,
        info: ModuleInfo,
        bytes: Arc<Vec<u8>>,
    ) -> Result<()> {
        let cid = info.cid.to_string();
        let name_display = info.name.as_deref().unwrap_or("unnamed");
        
        info!("Providing module {} ({}) to network", name_display, cid);
        
        // Store in memory
        self.provided_modules.write().await.insert(cid.clone(), bytes);
        self.module_info.write().await.insert(cid, info);
        
        Ok(())
    }
    
    /// Handle an incoming module request
    pub async fn handle_request(&self, request: ModuleRequest) -> ModuleResponse {
        match request {
            ModuleRequest::GetModule { cid } => {
                self.handle_get_module(&cid).await
            }
            
            ModuleRequest::GetModuleInfo { cid } => {
                self.handle_get_module_info(&cid).await
            }
            
            ModuleRequest::SearchByName { name } => {
                self.handle_search_by_name(&name).await
            }
            
            ModuleRequest::ListModules => {
                self.handle_list_modules().await
            }
        }
    }
    
    async fn handle_get_module(&self, cid: &str) -> ModuleResponse {
        debug!("Handling GetModule request for CID: {}", cid);
        
        let modules = self.provided_modules.read().await;
        
        match modules.get(cid) {
            Some(bytes) => {
                info!("Serving module {} ({} bytes)", cid, bytes.len());
                ModuleResponse::Module {
                    cid: cid.to_string(),
                    bytes: bytes.as_ref().clone(),
                }
            }
            None => {
                debug!("Module {} not found", cid);
                ModuleResponse::NotFound {
                    cid: cid.to_string(),
                }
            }
        }
    }
    
    async fn handle_get_module_info(&self, cid: &str) -> ModuleResponse {
        debug!("Handling GetModuleInfo request for CID: {}", cid);
        
        let info_map = self.module_info.read().await;
        
        match info_map.get(cid) {
            Some(info) => {
                ModuleResponse::ModuleInfo {
                    cid: info.cid.to_string(),
                    name: info.name.clone(),
                    version: info.version.clone(),
                    size: info.size,
                    dependencies: info.dependencies.iter().map(|d| d.to_string()).collect(),
                    author: info.author.clone(),
                    description: info.description.clone(),
                }
            }
            None => {
                ModuleResponse::NotFound {
                    cid: cid.to_string(),
                }
            }
        }
    }
    
    async fn handle_search_by_name(&self, name: &str) -> ModuleResponse {
        debug!("Handling SearchByName request for: {}", name);
        
        let info_map = self.module_info.read().await;
        
        let results: Vec<SearchResult> = info_map
            .values()
            .filter(|info| {
                info.name
                    .as_ref()
                    .map(|n| n.contains(name))
                    .unwrap_or(false)
            })
            .map(|info| SearchResult {
                cid: info.cid.to_string(),
                name: info.name.clone(),
                version: info.version.clone(),
                description: info.description.clone(),
            })
            .collect();
        
        info!("Found {} modules matching '{}'", results.len(), name);
        
        ModuleResponse::SearchResults { modules: results }
    }
    
    async fn handle_list_modules(&self) -> ModuleResponse {
        debug!("Handling ListModules request");
        
        let modules = self.provided_modules.read().await;
        let cids: Vec<String> = modules.keys().cloned().collect();
        
        info!("Listing {} modules", cids.len());
        
        ModuleResponse::ModuleList { cids }
    }
    
    /// Get list of all provided module CIDs
    pub async fn provided_cids(&self) -> Vec<String> {
        self.provided_modules.read().await.keys().cloned().collect()
    }
    
    /// Check if we're providing a specific module
    pub async fn is_providing(&self, cid: &str) -> bool {
        self.provided_modules.read().await.contains_key(cid)
    }
    
    /// Stop providing a module
    pub async fn stop_providing(&self, cid: &str) -> Result<()> {
        info!("Stopping provision of module {}", cid);
        
        self.provided_modules.write().await.remove(cid);
        self.module_info.write().await.remove(cid);
        
        Ok(())
    }
    
    /// Get statistics about provided modules
    pub async fn stats(&self) -> ProviderStats {
        let modules = self.provided_modules.read().await;
        let total_size: usize = modules.values().map(|v| v.len()).sum();
        
        ProviderStats {
            modules_count: modules.len(),
            total_bytes: total_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub modules_count: usize,
    pub total_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_provide_and_get_module() {
        let loader = Arc::new(ModuleLoader::new("/tmp/test"));
        let provider = ModuleProvider::new(loader);
        
        let cid = ModuleCid::from_bytes(b"test");
        let info = ModuleInfo {
            cid: cid.clone(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            size: 4,
            dependencies: vec![],
            author: None,
            description: None,
        };
        
        let bytes = Arc::new(b"test".to_vec());
        
        provider.provide_module(info, bytes.clone()).await.unwrap();
        
        let request = ModuleRequest::GetModule {
            cid: cid.to_string(),
        };
        
        let response = provider.handle_request(request).await;
        
        match response {
            ModuleResponse::Module { cid: _, bytes: resp_bytes } => {
                assert_eq!(resp_bytes, *bytes);
            }
            _ => panic!("Expected Module response"),
        }
    }
}
