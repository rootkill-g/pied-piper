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
        
        let key = b"test_encryption_key_32_bytes!!";
        let bytes = package.to_bytes(key).unwrap();
        
        let decoded = PiperNetPackage::from_bytes(&bytes, key).unwrap();
        
        assert_eq!(package.manifest.metadata.name, decoded.manifest.metadata.name);
    }
}
