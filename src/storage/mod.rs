use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use tracing::{debug, info, warn};

/// Persistent key-value storage for WASM modules
#[derive(Clone)]
pub struct PersistentStorage {
    /// In-memory cache
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Path to storage directory
    storage_path: PathBuf,
}

impl PersistentStorage {
    /// Create a new persistent storage
    pub async fn new<P: AsRef<Path>>(storage_path: P) -> Result<Self> {
        let storage_path = storage_path.as_ref().to_path_buf();
        
        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_path)
            .await
            .context("Failed to create storage directory")?;

        let mut cache = HashMap::new();

        // Load existing data from disk
        if let Ok(mut entries) = fs::read_dir(&storage_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("dat") {
                    if let Ok(data) = fs::read(&path).await {
                        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                            // Decode the key from hex filename
                            if let Ok(key_bytes) = hex::decode(filename) {
                                if let Ok(key) = String::from_utf8(key_bytes) {
                                    cache.insert(key.clone(), data);
                                    debug!("Loaded storage key: {}", key);
                                }
                            }
                        }
                    }
                }
            }
        }

        info!("Loaded {} keys from persistent storage", cache.len());

        Ok(Self {
            cache: Arc::new(RwLock::new(cache)),
            storage_path,
        })
    }

    /// Get a value from storage
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    /// Set a value in storage (persists to disk)
    pub async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        // Update in-memory cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), value.clone());
        }

        // Persist to disk
        let filename = hex::encode(key.as_bytes());
        let file_path = self.storage_path.join(format!("{}.dat", filename));
        
        fs::write(&file_path, &value)
            .await
            .context("Failed to write storage file")?;

        debug!("Persisted storage key: {} ({} bytes)", key, value.len());
        Ok(())
    }

    /// Delete a value from storage
    pub async fn delete(&self, key: &str) -> Result<()> {
        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.remove(key);
        }

        // Delete from disk
        let filename = hex::encode(key.as_bytes());
        let file_path = self.storage_path.join(format!("{}.dat", filename));
        
        if file_path.exists() {
            fs::remove_file(&file_path)
                .await
                .context("Failed to delete storage file")?;
        }

        debug!("Deleted storage key: {}", key);
        Ok(())
    }

    /// Get number of keys in storage
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if storage is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Get the underlying HashMap for compatibility with existing code
    pub fn as_hashmap(&self) -> Arc<RwLock<HashMap<String, Vec<u8>>>> {
        self.cache.clone()
    }

    /// Synchronous get - for use in host functions
    pub fn get_sync(&self, key: &str) -> Option<Vec<u8>> {
        // Directly access the cache without async
        tokio::task::block_in_place(|| {
            let cache = self.cache.blocking_read();
            let result = cache.get(key).cloned();
            debug!("Storage get_sync: {} -> {} bytes", key, result.as_ref().map(|v| v.len()).unwrap_or(0));
            result
        })
    }

    /// Synchronous set - for use in host functions
    pub fn set_sync(&self, key: &str, value: Vec<u8>) -> Result<()> {
        // Update cache synchronously
        tokio::task::block_in_place(|| {
            let mut cache = self.cache.blocking_write();
            cache.insert(key.to_string(), value.clone());
        });
        
        // Persist to disk asynchronously (fire and forget for now)
        let filename = hex::encode(key.as_bytes());
        let file_path = self.storage_path.join(format!("{}.dat", filename));
        
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(e) = tokio::fs::write(&file_path, &value).await {
                    warn!("Failed to persist storage key {}: {}", key, e);
                } else {
                    debug!("Persisted storage key: {} ({} bytes)", key, value.len());
                }
            })
        });
        
        Ok(())
    }

    /// Synchronous delete - for use in host functions
    pub fn delete_sync(&self, key: &str) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.delete(key).await
            })
        })
    }

    /// Synchronous len - for use in host functions
    pub fn len_sync(&self) -> usize {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.len().await
            })
        })
    }
}
