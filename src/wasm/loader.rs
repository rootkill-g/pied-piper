use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Content identifier for a Wasm module (using Blake3)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleCid(pub String);

impl ModuleCid {
    /// Generate a CID from module bytes using Blake3
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        // Use base32 encoding of the hash
        let cid = multibase::encode(multibase::Base::Base32Lower, hash.as_bytes());
        ModuleCid(cid)
    }
    
    /// Get the CID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModuleCid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata about a Wasm module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Content identifier
    pub cid: ModuleCid,
    
    /// Module name (optional)
    pub name: Option<String>,
    
    /// Module version (optional)
    pub version: Option<String>,
    
    /// Module size in bytes
    pub size: usize,
    
    /// Module dependencies (CIDs of other modules)
    pub dependencies: Vec<ModuleCid>,
    
    /// Module author (optional)
    pub author: Option<String>,
    
    /// Module description (optional)
    pub description: Option<String>,
}

/// Module loader with caching
pub struct ModuleLoader {
    /// Cache directory for downloaded modules
    cache_dir: PathBuf,
    
    /// In-memory cache of module info
    info_cache: Arc<RwLock<HashMap<ModuleCid, ModuleInfo>>>,
    
    /// In-memory cache of module bytes
    bytes_cache: Arc<RwLock<HashMap<ModuleCid, Arc<Vec<u8>>>>>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir)
            .await
            .context("Failed to create cache directory")?;
        
        Ok(Self {
            cache_dir,
            info_cache: Arc::new(RwLock::new(HashMap::new())),
            bytes_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Load a module from local filesystem
    pub async fn load_from_file(&self, path: PathBuf) -> Result<(ModuleInfo, Arc<Vec<u8>>)> {
        // Read the module bytes
        let bytes = fs::read(&path)
            .await
            .context(format!("Failed to read module from {:?}", path))?;
        
        // Generate CID
        let cid = ModuleCid::from_bytes(&bytes);
        
        // Create module info
        let info = ModuleInfo {
            cid: cid.clone(),
            name: path.file_stem().and_then(|s| s.to_str()).map(String::from),
            version: None,
            size: bytes.len(),
            dependencies: vec![],
            author: None,
            description: None,
        };
        
        let bytes_arc = Arc::new(bytes);
        
        // Cache the module
        {
            let mut info_cache = self.info_cache.write().await;
            info_cache.insert(cid.clone(), info.clone());
        }
        
        {
            let mut bytes_cache = self.bytes_cache.write().await;
            bytes_cache.insert(cid.clone(), bytes_arc.clone());
        }
        
        Ok((info, bytes_arc))
    }
    
    /// Load a module from bytes
    pub async fn load_from_bytes(&self, bytes: Vec<u8>, name: Option<String>) -> Result<(ModuleInfo, Arc<Vec<u8>>)> {
        let cid = ModuleCid::from_bytes(&bytes);
        
        let info = ModuleInfo {
            cid: cid.clone(),
            name,
            version: None,
            size: bytes.len(),
            dependencies: vec![],
            author: None,
            description: None,
        };
        
        let bytes_arc = Arc::new(bytes);
        
        // Cache the module
        {
            let mut info_cache = self.info_cache.write().await;
            info_cache.insert(cid.clone(), info.clone());
        }
        
        {
            let mut bytes_cache = self.bytes_cache.write().await;
            bytes_cache.insert(cid.clone(), bytes_arc.clone());
        }
        
        Ok((info, bytes_arc))
    }
    
    /// Get a module from cache by CID
    pub async fn get_from_cache(&self, cid: &ModuleCid) -> Option<(ModuleInfo, Arc<Vec<u8>>)> {
        let info = {
            let info_cache = self.info_cache.read().await;
            info_cache.get(cid).cloned()
        };
        
        let bytes = {
            let bytes_cache = self.bytes_cache.read().await;
            bytes_cache.get(cid).cloned()
        };
        
        match (info, bytes) {
            (Some(info), Some(bytes)) => Some((info, bytes)),
            _ => None,
        }
    }
    
    /// Check if a module is in cache
    pub async fn is_cached(&self, cid: &ModuleCid) -> bool {
        let info_cache = self.info_cache.read().await;
        info_cache.contains_key(cid)
    }
    
    /// Save a module to disk cache
    pub async fn save_to_disk(&self, cid: &ModuleCid, bytes: &[u8]) -> Result<PathBuf> {
        let file_path = self.cache_dir.join(format!("{}.wasm", cid.as_str()));
        
        fs::write(&file_path, bytes)
            .await
            .context("Failed to write module to disk")?;
        
        Ok(file_path)
    }
    
    /// Load a module from disk cache
    pub async fn load_from_disk(&self, cid: &ModuleCid) -> Result<Arc<Vec<u8>>> {
        let file_path = self.cache_dir.join(format!("{}.wasm", cid.as_str()));
        
        let bytes = fs::read(&file_path)
            .await
            .context("Failed to read module from disk")?;
        
        let bytes_arc = Arc::new(bytes);
        
        // Update memory cache
        {
            let mut bytes_cache = self.bytes_cache.write().await;
            bytes_cache.insert(cid.clone(), bytes_arc.clone());
        }
        
        Ok(bytes_arc)
    }
    
    /// Clear the in-memory cache
    pub async fn clear_memory_cache(&self) {
        let mut info_cache = self.info_cache.write().await;
        info_cache.clear();
        
        let mut bytes_cache = self.bytes_cache.write().await;
        bytes_cache.clear();
    }
    
    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let info_count = self.info_cache.read().await.len();
        let bytes_count = self.bytes_cache.read().await.len();
        (info_count, bytes_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_module_cid_generation() {
        let data = b"hello world";
        let cid1 = ModuleCid::from_bytes(data);
        let cid2 = ModuleCid::from_bytes(data);
        
        assert_eq!(cid1, cid2);
        assert!(!cid1.as_str().is_empty());
    }
    
    #[tokio::test]
    async fn test_module_loader_creation() {
        let temp_dir = tempdir().unwrap();
        let loader = ModuleLoader::new(temp_dir.path().to_path_buf()).await;
        assert!(loader.is_ok());
    }
    
    #[tokio::test]
    async fn test_load_from_bytes() {
        let temp_dir = tempdir().unwrap();
        let loader = ModuleLoader::new(temp_dir.path().to_path_buf()).await.unwrap();
        
        let test_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]; // Wasm magic
        let result = loader.load_from_bytes(test_bytes, Some("test".to_string())).await;
        
        assert!(result.is_ok());
        let (info, _) = result.unwrap();
        assert_eq!(info.name, Some("test".to_string()));
    }
}
