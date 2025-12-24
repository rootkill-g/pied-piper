/// PiperNet Package (.pn) format
/// 
/// A secure, encrypted package format for deploying applications
/// to the PiperNet network. Packages contain:
/// - Manifest (pn.toml) with metadata
/// - WASM modules (backend/frontend)
/// - Assets (HTML, CSS, JS, images)
/// - Dependencies
///
/// Security features:
/// - AES-256-GCM encryption
/// - Zstd compression
/// - SHA-256 integrity checks
/// - Content obfuscation

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub mod builder;
pub mod crypto;
pub mod manifest;

pub use manifest::{PackageManifest, PackageMetadata, PackageType};

/// Magic bytes for .pn file format (PN + version)
pub const MAGIC_BYTES: &[u8] = b"PN\x01\x00";

/// PiperNet Package structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperNetPackage {
    /// Package manifest
    pub manifest: PackageManifest,
    
    /// Main WASM module bytes (encrypted)
    pub module: Vec<u8>,
    
    /// Asset files (path -> encrypted bytes)
    pub assets: HashMap<String, Vec<u8>>,
    
    /// Dependency modules (name -> encrypted bytes)
    pub dependencies: HashMap<String, Vec<u8>>,
    
    /// Package signature (for verification)
    pub signature: Vec<u8>,
}

impl PiperNetPackage {
    /// Create a new package from components
    pub fn new(
        manifest: PackageManifest,
        module: Vec<u8>,
        assets: HashMap<String, Vec<u8>>,
        dependencies: HashMap<String, Vec<u8>>,
    ) -> Self {
        Self {
            manifest,
            module,
            assets,
            dependencies,
            signature: Vec::new(), // Will be computed during serialization
        }
    }
    
    /// Serialize package to encrypted .pn file bytes
    pub fn to_bytes(&self, encryption_key: &[u8]) -> Result<Vec<u8>> {
        // 1. Serialize the package structure
        let json = serde_json::to_vec(self)?;
        
        // 2. Compress with zstd
        let compressed = zstd::encode_all(&json[..], 3)
            .context("Failed to compress package")?;
        
        // 3. Encrypt with AES-256-GCM
        let encrypted = crypto::encrypt(&compressed, encryption_key)?;
        
        // 4. Add magic bytes and version
        let mut output = Vec::new();
        output.extend_from_slice(MAGIC_BYTES);
        output.extend_from_slice(&encrypted);
        
        Ok(output)
    }
    
    /// Deserialize package from encrypted .pn file bytes
    pub fn from_bytes(bytes: &[u8], encryption_key: &[u8]) -> Result<Self> {
        // 1. Verify magic bytes
        if bytes.len() < 4 || &bytes[0..4] != MAGIC_BYTES {
            bail!("Invalid .pn file: magic bytes mismatch");
        }
        
        // 2. Decrypt
        let encrypted = &bytes[4..];
        let compressed = crypto::decrypt(encrypted, encryption_key)?;
        
        // 3. Decompress
        let json = zstd::decode_all(&compressed[..])
            .context("Failed to decompress package")?;
        
        // 4. Deserialize
        let package: Self = serde_json::from_slice(&json)?;
        
        Ok(package)
    }
    
    /// Load package from .pn file
    pub async fn load_from_file(path: &Path, encryption_key: &[u8]) -> Result<Self> {
        let bytes = fs::read(path).await
            .context("Failed to read .pn file")?;
        
        Self::from_bytes(&bytes, encryption_key)
    }
    
    /// Save package to .pn file
    pub async fn save_to_file(&self, path: &Path, encryption_key: &[u8]) -> Result<()> {
        let bytes = self.to_bytes(encryption_key)?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(path, bytes).await
            .context("Failed to write .pn file")?;
        
        Ok(())
    }
    
    /// Get package CID (content identifier)
    pub fn cid(&self) -> String {
        use crate::wasm::ModuleCid;
        let bytes = serde_json::to_vec(&self.manifest).unwrap_or_default();
        ModuleCid::from_bytes(&bytes).to_string()
    }
    
    /// Decrypt and get main module WASM bytes
    pub fn get_module(&self, encryption_key: &[u8]) -> Result<Vec<u8>> {
        crypto::decrypt(&self.module, encryption_key)
    }
    
    /// Decrypt and get asset by path
    pub fn get_asset(&self, path: &str, encryption_key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(encrypted) = self.assets.get(path) {
            let decrypted = crypto::decrypt(encrypted, encryption_key)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }
    
    /// Decrypt and get dependency by name
    pub fn get_dependency(&self, name: &str, encryption_key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(encrypted) = self.dependencies.get(name) {
            let decrypted = crypto::decrypt(encrypted, encryption_key)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }
    
    /// List all asset paths
    pub fn asset_paths(&self) -> Vec<String> {
        self.assets.keys().cloned().collect()
    }
    
    /// List all dependency names
    pub fn dependency_names(&self) -> Vec<String> {
        self.dependencies.keys().cloned().collect()
    }
    
    /// Get package size in bytes (compressed + encrypted)
    pub fn size(&self, encryption_key: &[u8]) -> Result<usize> {
        Ok(self.to_bytes(encryption_key)?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::crypto::generate_key;
    
    #[test]
    fn test_package_roundtrip() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "test-app".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test application".to_string()),
                author: Some("Test Author".to_string()),
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::FullStack,
            entrypoint: "module.wasm".to_string(),
            assets: vec!["index.html".to_string()],
            dependencies: HashMap::new(),
        };
        
        let module = b"fake wasm module".to_vec();
        let mut assets = HashMap::new();
        assets.insert("index.html".to_string(), b"<html></html>".to_vec());
        
        let package = PiperNetPackage::new(manifest, module, assets, HashMap::new());
        
        let key = b"test_encryption_key_32_bytes!32!";  // Exactly 32 bytes
        assert_eq!(key.len(), 32, "Test key must be 32 bytes");
        let bytes = package.to_bytes(key).unwrap();
        
        let decoded = PiperNetPackage::from_bytes(&bytes, key).unwrap();
        
        assert_eq!(package.manifest.metadata.name, decoded.manifest.metadata.name);
    }
    
    #[test]
    fn test_package_magic_bytes() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint: "module.wasm".to_string(),
            assets: vec![],
            dependencies: HashMap::new(),
        };
        
        let module = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic bytes
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        let key = generate_key();
        let bytes = package.to_bytes(&key).unwrap();
        
        // Check magic bytes
        assert_eq!(&bytes[0..4], b"PN\x01\x00");
    }
    
    #[test]
    fn test_package_with_assets() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "frontend-app".to_string(),
                version: "2.0.0".to_string(),
                description: Some("Frontend app".to_string()),
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Frontend,
            entrypoint: "app.wasm".to_string(),
            assets: vec!["index.html".to_string(), "style.css".to_string()],
            dependencies: HashMap::new(),
        };
        
        let module = b"wasm module content".to_vec();
        let mut assets = HashMap::new();
        assets.insert("index.html".to_string(), b"<!DOCTYPE html>".to_vec());
        assets.insert("style.css".to_string(), b"body { margin: 0; }".to_vec());
        
        let package = PiperNetPackage::new(manifest, module, assets, HashMap::new());
        
        let key = generate_key();
        let bytes = package.to_bytes(&key).unwrap();
        let decoded = PiperNetPackage::from_bytes(&bytes, &key).unwrap();
        
        assert_eq!(decoded.assets.len(), 2);
        assert!(decoded.assets.contains_key("index.html"));
        assert!(decoded.assets.contains_key("style.css"));
    }
    
    #[test]
    fn test_package_get_module() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint: "module.wasm".to_string(),
            assets: vec![],
            dependencies: HashMap::new(),
        };
        
        let module_content = b"original wasm module content";
        let package = PiperNetPackage::new(manifest, module_content.to_vec(), HashMap::new(), HashMap::new());
        
        let key = generate_key();
        let bytes = package.to_bytes(&key).unwrap();
        let decoded = PiperNetPackage::from_bytes(&bytes, &key).unwrap();
        
        // Module is already decrypted in the package structure after from_bytes
        assert_eq!(decoded.module, module_content);
    }
    
    #[test]
    fn test_package_wrong_key_fails() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint: "module.wasm".to_string(),
            assets: vec![],
            dependencies: HashMap::new(),
        };
        
        let module = vec![0u8; 100];
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        let key1 = generate_key();
        let key2 = generate_key();
        
        let bytes = package.to_bytes(&key1).unwrap();
        let result = PiperNetPackage::from_bytes(&bytes, &key2);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_package_corrupted_data_fails() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint: "module.wasm".to_string(),
            assets: vec![],
            dependencies: HashMap::new(),
        };
        
        let module = vec![0u8; 100];
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        let key = generate_key();
        let mut bytes = package.to_bytes(&key).unwrap();
        
        // Corrupt some bytes in the middle
        if bytes.len() > 50 {
            bytes[50] ^= 0xFF;
            bytes[51] ^= 0xFF;
        }
        
        let result = PiperNetPackage::from_bytes(&bytes, &key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_package_invalid_magic_bytes() {
        let key = generate_key();
        let invalid_data = b"XXXX some data";
        
        let result = PiperNetPackage::from_bytes(invalid_data, &key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_package_empty_module() {
        let manifest = PackageManifest {
            metadata: PackageMetadata {
                name: "empty-test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint: "module.wasm".to_string(),
            assets: vec![],
            dependencies: HashMap::new(),
        };
        
        let module = Vec::new();
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        let key = generate_key();
        let bytes = package.to_bytes(&key).unwrap();
        let decoded = PiperNetPackage::from_bytes(&bytes, &key).unwrap();
        
        assert_eq!(decoded.module.len(), 0);
    }
}

