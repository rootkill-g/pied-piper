use anyhow::{Context, Result};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::info;

use crate::metrics::Metrics;

/// Content identifier for a Wasm module (using Blake3)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleCid(pub String);

impl ModuleCid {
    /// Create a ModuleCid from an existing CID string
    pub fn new(cid: String) -> Self {
        ModuleCid(cid)
    }

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

/// Module loader with LRU caching
pub struct ModuleLoader {
    /// Cache directory for downloaded modules
    cache_dir: PathBuf,

    /// In-memory LRU cache of module info
    info_cache: Arc<RwLock<LruCache<ModuleCid, ModuleInfo>>>,

    /// In-memory LRU cache of module bytes
    bytes_cache: Arc<RwLock<LruCache<ModuleCid, Arc<Vec<u8>>>>>,

    /// Current memory usage in bytes
    current_bytes: Arc<RwLock<usize>>,

    /// Maximum cached bytes (512 MB default)
    max_bytes: usize,
    
    /// Metrics for tracking cache performance
    metrics: Option<Arc<Metrics>>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir)
            .await
            .context("Failed to create cache directory")?;

        // Max 256 entries in LRU cache
        let max_entries = NonZeroUsize::new(256).unwrap();

        Ok(Self {
            cache_dir,
            info_cache: Arc::new(RwLock::new(LruCache::new(max_entries))),
            bytes_cache: Arc::new(RwLock::new(LruCache::new(max_entries))),
            current_bytes: Arc::new(RwLock::new(0)),
            max_bytes: 512 * 1024 * 1024, // 512 MB
            metrics: None,
        })
    }
    
    /// Set metrics for this loader
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
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
        self.insert_into_cache(cid.clone(), info.clone(), bytes_arc.clone())
            .await;

        Ok((info, bytes_arc))
    }

    /// Load a module from bytes
    pub async fn load_from_bytes(
        &self,
        bytes: Vec<u8>,
        name: Option<String>,
    ) -> Result<(ModuleInfo, Arc<Vec<u8>>)> {
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
        self.insert_into_cache(cid.clone(), info.clone(), bytes_arc.clone())
            .await;

        Ok((info, bytes_arc))
    }

    /// Get a module from cache by CID
    pub async fn get_from_cache(&self, cid: &ModuleCid) -> Option<(ModuleInfo, Arc<Vec<u8>>)> {
        // Check in-memory cache first (LRU automatically updates on get)
        let info = {
            let mut info_cache = self.info_cache.write().await;
            info_cache.get(cid).cloned()
        };

        let bytes = {
            let mut bytes_cache = self.bytes_cache.write().await;
            bytes_cache.get(cid).cloned()
        };

        match (info, bytes) {
            (Some(info), Some(bytes)) => {
                info!("Module {} found in memory cache", cid);
                
                // Track cache hit
                if let Some(metrics) = &self.metrics {
                    metrics.content_cache_hits.inc();
                }
                
                Some((info, bytes))
            }
            _ => {
                // Track cache miss
                if let Some(metrics) = &self.metrics {
                    metrics.content_cache_misses.inc();
                }
                
                // Try loading from disk
                info!("Module {} not in memory, trying disk...", cid);
                match self.load_from_disk(cid).await {
                    Ok(bytes) => {
                        // Create basic module info since we don't have metadata on disk
                        let info = ModuleInfo {
                            cid: cid.clone(),
                            name: None,
                            version: None,
                            size: bytes.len(),
                            dependencies: vec![],
                            author: None,
                            description: None,
                        };

                        // Add to memory cache for future lookups
                        self.insert_into_cache(cid.clone(), info.clone(), bytes.clone())
                            .await;

                        info!("Module {} loaded from disk", cid);
                        Some((info, bytes))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load module {} from disk: {}", cid, e);
                        None
                    }
                }
            }
        }
    }

    /// Check if a module is in cache
    pub async fn is_cached(&self, cid: &ModuleCid) -> bool {
        let info_cache = self.info_cache.read().await;
        info_cache.contains(cid)
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

        Ok(bytes_arc)
    }

    /// Add a module to the cache (used during deployment)
    pub async fn add_to_cache(&self, cid: &ModuleCid, info: ModuleInfo, bytes: Arc<Vec<u8>>) {
        info!("Adding module {} to cache", cid);

        // Add to in-memory cache
        self.insert_into_cache(cid.clone(), info, bytes.clone())
            .await;

        // Also save to disk for persistence
        if let Err(e) = self.save_to_disk(cid, &bytes).await {
            tracing::warn!("Failed to save module {} to disk: {}", cid, e);
        }
    }

    /// Clear the in-memory cache
    pub async fn clear_memory_cache(&self) {
        let mut info_cache = self.info_cache.write().await;
        info_cache.clear();

        let mut bytes_cache = self.bytes_cache.write().await;
        bytes_cache.clear();

        let mut current_bytes = self.current_bytes.write().await;
        *current_bytes = 0;
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let info_count = self.info_cache.read().await.len();
        let bytes_count = self.bytes_cache.read().await.len();
        
        // Update metrics
        if let Some(metrics) = &self.metrics {
            metrics.content_modules_cached.set(bytes_count as i64);
        }
        
        (info_count, bytes_count)
    }

    /// Load a module with all its dependencies
    /// Returns a list of (ModuleInfo, bytes) tuples in dependency order
    pub async fn load_with_dependencies(
        &self,
        cid: &ModuleCid,
    ) -> Result<Vec<(ModuleInfo, Arc<Vec<u8>>)>> {
        let mut loaded = HashMap::new();
        let mut result = Vec::new();

        // Use DFS to resolve dependencies
        self.resolve_dependencies_recursive(cid, &mut loaded, &mut result)
            .await?;

        Ok(result)
    }

    /// Recursive helper for dependency resolution
    fn resolve_dependencies_recursive<'a>(
        &'a self,
        cid: &'a ModuleCid,
        loaded: &'a mut HashMap<ModuleCid, ()>,
        result: &'a mut Vec<(ModuleInfo, Arc<Vec<u8>>)>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            // Skip if already loaded
            if loaded.contains_key(cid) {
                return Ok(());
            }

            // Get module from cache
            let (info, bytes) = self
                .get_from_cache(cid)
                .await
                .ok_or_else(|| anyhow::anyhow!("Module {} not found in cache", cid))?;

            // Mark as loaded to prevent cycles
            loaded.insert(cid.clone(), ());

            // Recursively load dependencies first
            for dep_cid in &info.dependencies.clone() {
                self.resolve_dependencies_recursive(dep_cid, loaded, result)
                    .await?;
            }

            // Add current module after dependencies
            result.push((info, bytes));

            Ok(())
        })
    }

    /// Update module metadata (including dependencies)
    pub async fn update_module_info(&self, cid: &ModuleCid, info: ModuleInfo) -> Result<()> {
        // Save metadata to disk first
        let metadata_path = self.cache_dir.join(format!("{}.json", cid.as_str()));
        let json = serde_json::to_string_pretty(&info)?;
        fs::write(&metadata_path, json).await?;

        // Then update cache
        let mut info_cache = self.info_cache.write().await;
        info_cache.put(cid.clone(), info);

        Ok(())
    }

    /// Load module metadata from disk
    pub async fn load_module_info(&self, cid: &ModuleCid) -> Result<Option<ModuleInfo>> {
        let metadata_path = self.cache_dir.join(format!("{}.json", cid.as_str()));

        if metadata_path.exists() {
            let json = fs::read_to_string(&metadata_path).await?;
            let info: ModuleInfo = serde_json::from_str(&json)?;

            // Update cache
            let mut info_cache = self.info_cache.write().await;
            info_cache.put(cid.clone(), info.clone());

            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    /// List cached module CIDs from disk
    pub async fn list_cached_modules(&self) -> Result<Vec<ModuleCid>> {
        let mut entries = fs::read_dir(&self.cache_dir)
            .await
            .context("Failed to read cache directory")?;
        let mut cids = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
                continue;
            }

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                cids.push(ModuleCid::new(stem.to_string()));
            }
        }

        Ok(cids)
    }

    async fn insert_into_cache(&self, cid: ModuleCid, info: ModuleInfo, bytes: Arc<Vec<u8>>) {
        let mut info_cache = self.info_cache.write().await;
        let mut bytes_cache = self.bytes_cache.write().await;
        let mut current_bytes = self.current_bytes.write().await;

        if let Some(existing) = bytes_cache.peek(&cid) {
            *current_bytes = current_bytes.saturating_sub(existing.len());
        }

        info_cache.put(cid.clone(), info);
        bytes_cache.put(cid.clone(), bytes.clone());
        *current_bytes = current_bytes.saturating_add(bytes.len());

        drop(bytes_cache);
        drop(info_cache);
        drop(current_bytes);

        // LRU cache handles ordering automatically on put/get
        self.evict_if_needed().await;
    }

    async fn evict_if_needed(&self) {
        loop {
            let bytes = {
                let current_bytes = self.current_bytes.read().await;
                *current_bytes
            };

            if bytes <= self.max_bytes {
                break;
            }

            // Pop the least recently used entry
            let (evicted_cid, evicted_bytes) = {
                let mut bytes_cache = self.bytes_cache.write().await;
                match bytes_cache.pop_lru() {
                    Some((cid, bytes)) => (cid, bytes.len()),
                    None => break,
                }
            };

            // Remove from info cache and update byte count
            let mut info_cache = self.info_cache.write().await;
            let mut current_bytes = self.current_bytes.write().await;
            
            info_cache.pop(&evicted_cid);
            *current_bytes = current_bytes.saturating_sub(evicted_bytes);
        }
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
        let loader = ModuleLoader::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let test_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]; // Wasm magic
        let result = loader
            .load_from_bytes(test_bytes, Some("test".to_string()))
            .await;

        assert!(result.is_ok());
        let (info, _) = result.unwrap();
        assert_eq!(info.name, Some("test".to_string()));
    }
}

/// Version matching utilities for dependency resolution
pub mod version {
    use anyhow::{Context, Result};
    use semver::{Version, VersionReq};

    /// Parse a version string into a semver Version
    pub fn parse_version(version_str: &str) -> Result<Version> {
        Version::parse(version_str)
            .with_context(|| format!("Invalid semver version: {}", version_str))
    }

    /// Parse a version requirement string (e.g., "^1.0.0", "~1.2.3", ">=2.0.0")
    pub fn parse_requirement(req_str: &str) -> Result<VersionReq> {
        VersionReq::parse(req_str)
            .with_context(|| format!("Invalid version requirement: {}", req_str))
    }

    /// Check if a version satisfies a requirement
    pub fn matches(version: &str, requirement: &str) -> Result<bool> {
        let version = parse_version(version)?;
        let req = parse_requirement(requirement)?;
        Ok(req.matches(&version))
    }

    /// Find the best matching version from a list of available versions
    /// Returns the highest version that satisfies the requirement
    pub fn find_best_match(available: &[String], requirement: &str) -> Result<Option<String>> {
        let req = parse_requirement(requirement)?;

        let mut matching_versions: Vec<Version> = available
            .iter()
            .filter_map(|v| Version::parse(v).ok())
            .filter(|v| req.matches(v))
            .collect();

        if matching_versions.is_empty() {
            return Ok(None);
        }

        // Sort in descending order to get the highest version
        matching_versions.sort_by(|a, b| b.cmp(a));

        Ok(Some(matching_versions[0].to_string()))
    }

    /// Find the latest version from a list (highest semver)
    pub fn find_latest(available: &[String]) -> Option<String> {
        let mut versions: Vec<Version> = available
            .iter()
            .filter_map(|v| Version::parse(v).ok())
            .collect();

        if versions.is_empty() {
            return None;
        }

        versions.sort_by(|a, b| b.cmp(a));
        Some(versions[0].to_string())
    }

    /// Check if a version string is a valid semver version
    pub fn is_valid_version(version_str: &str) -> bool {
        Version::parse(version_str).is_ok()
    }

    /// Check if a requirement string is valid
    pub fn is_valid_requirement(req_str: &str) -> bool {
        VersionReq::parse(req_str).is_ok()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_version() {
            assert!(parse_version("1.0.0").is_ok());
            assert!(parse_version("0.1.2").is_ok());
            assert!(parse_version("invalid").is_err());
        }

        #[test]
        fn test_parse_requirement() {
            assert!(parse_requirement("^1.0.0").is_ok());
            assert!(parse_requirement("~1.2.3").is_ok());
            assert!(parse_requirement(">=2.0.0").is_ok());
            assert!(parse_requirement("1.0.0").is_ok());
            assert!(parse_requirement("invalid").is_err());
        }

        #[test]
        fn test_matches() {
            assert!(matches("1.2.3", "^1.0.0").unwrap());
            assert!(matches("1.2.3", "~1.2.0").unwrap());
            assert!(matches("2.0.0", ">=2.0.0").unwrap());
            assert!(!matches("0.9.0", "^1.0.0").unwrap());
        }

        #[test]
        fn test_find_best_match() {
            let available = vec![
                "1.0.0".to_string(),
                "1.1.0".to_string(),
                "1.2.0".to_string(),
                "2.0.0".to_string(),
            ];

            // ^1.0.0 should match 1.2.0 (highest 1.x)
            assert_eq!(
                find_best_match(&available, "^1.0.0").unwrap(),
                Some("1.2.0".to_string())
            );

            // ~1.1.0 should match 1.1.0
            assert_eq!(
                find_best_match(&available, "~1.1.0").unwrap(),
                Some("1.1.0".to_string())
            );

            // >=2.0.0 should match 2.0.0
            assert_eq!(
                find_best_match(&available, ">=2.0.0").unwrap(),
                Some("2.0.0".to_string())
            );

            // ^3.0.0 should match nothing
            assert_eq!(find_best_match(&available, "^3.0.0").unwrap(), None);
        }

        #[test]
        fn test_find_latest() {
            let available = vec![
                "1.0.0".to_string(),
                "2.1.0".to_string(),
                "1.5.0".to_string(),
                "2.0.0".to_string(),
            ];

            assert_eq!(find_latest(&available), Some("2.1.0".to_string()));
        }

        #[test]
        fn test_find_latest_empty() {
            let available: Vec<String> = vec![];
            assert_eq!(find_latest(&available), None);
        }

        #[test]
        fn test_is_valid_version() {
            assert!(is_valid_version("1.0.0"));
            assert!(is_valid_version("0.1.2"));
            assert!(!is_valid_version("invalid"));
            assert!(!is_valid_version("1.0"));
        }

        #[test]
        fn test_is_valid_requirement() {
            assert!(is_valid_requirement("^1.0.0"));
            assert!(is_valid_requirement("~1.2.3"));
            assert!(is_valid_requirement(">=2.0.0"));
            assert!(!is_valid_requirement("invalid"));
        }
    }
}
