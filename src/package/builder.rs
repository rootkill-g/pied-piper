/// Package builder for creating .pn files
/// 
/// This module provides utilities for building PiperNet packages
/// from source files, similar to `cargo build`.

use super::{PiperNetPackage, PackageManifest, crypto};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, debug};

/// Builder for creating .pn packages
pub struct PackageBuilder {
    /// Package manifest
    manifest: PackageManifest,
    
    /// Base directory for resolving relative paths
    base_dir: PathBuf,
    
    /// Collected WASM module bytes
    module_bytes: Option<Vec<u8>>,
    
    /// Collected asset files (path -> bytes)
    asset_files: HashMap<String, Vec<u8>>,
    
    /// Collected dependencies (name -> bytes)
    dependency_files: HashMap<String, Vec<u8>>,
}

impl PackageBuilder {
    /// Create a new package builder from manifest
    pub fn new(manifest: PackageManifest, base_dir: PathBuf) -> Self {
        Self {
            manifest,
            base_dir,
            module_bytes: None,
            asset_files: HashMap::new(),
            dependency_files: HashMap::new(),
        }
    }
    
    /// Load manifest from pn.toml file
    pub async fn from_manifest_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).await
            .context("Failed to read pn.toml")?;
        
        let manifest = PackageManifest::from_toml(&content)
            .context("Failed to parse pn.toml")?;
        
        let base_dir = path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid manifest path"))?
            .to_path_buf();
        
        Ok(Self::new(manifest, base_dir))
    }
    
    /// Load the main WASM module
    pub async fn load_module(&mut self) -> Result<&mut Self> {
        let module_path = self.base_dir.join(&self.manifest.entrypoint);
        
        info!("Loading WASM module from {:?}", module_path);
        
        let bytes = fs::read(&module_path).await
            .with_context(|| format!("Failed to read module: {:?}", module_path))?;
        
        // Verify it's a valid WASM module
        if bytes.len() < 4 || &bytes[0..4] != b"\0asm" {
            bail!("Invalid WASM module: {:?}", module_path);
        }
        
        debug!("Loaded WASM module: {} bytes", bytes.len());
        self.module_bytes = Some(bytes);
        
        Ok(self)
    }
    
    /// Load all asset files
    pub async fn load_assets(&mut self) -> Result<&mut Self> {
        for asset_path in &self.manifest.assets {
            let full_path = self.base_dir.join(asset_path);
            
            debug!("Loading asset: {:?}", full_path);
            
            let bytes = fs::read(&full_path).await
                .with_context(|| format!("Failed to read asset: {:?}", full_path))?;
            
            self.asset_files.insert(asset_path.clone(), bytes);
        }
        
        info!("Loaded {} assets", self.asset_files.len());
        Ok(self)
    }
    
    /// Load dependency modules
    pub async fn load_dependencies(&mut self) -> Result<&mut Self> {
        // For now, dependencies are expected to be in a "deps" directory
        // In the future, this could fetch from the network
        
        for (dep_name, _version_req) in &self.manifest.dependencies {
            let dep_path = self.base_dir.join("deps").join(format!("{}.wasm", dep_name));
            
            if dep_path.exists() {
                debug!("Loading dependency: {:?}", dep_path);
                
                let bytes = fs::read(&dep_path).await
                    .with_context(|| format!("Failed to read dependency: {:?}", dep_path))?;
                
                self.dependency_files.insert(dep_name.clone(), bytes);
            } else {
                debug!("Dependency {} not found locally, will fetch from network", dep_name);
            }
        }
        
        info!("Loaded {} dependencies", self.dependency_files.len());
        Ok(self)
    }
    
    /// Build the package with encryption
    pub fn build(&self, encryption_key: &[u8]) -> Result<PiperNetPackage> {
        let module = self.module_bytes.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Module not loaded"))?;
        
        info!("Building package: {}", self.manifest.metadata.name);
        
        // Encrypt all components
        let encrypted_module = crypto::encrypt(module, encryption_key)?;
        
        let mut encrypted_assets = HashMap::new();
        for (path, bytes) in &self.asset_files {
            let encrypted = crypto::encrypt(bytes, encryption_key)?;
            encrypted_assets.insert(path.clone(), encrypted);
        }
        
        let mut encrypted_deps = HashMap::new();
        for (name, bytes) in &self.dependency_files {
            let encrypted = crypto::encrypt(bytes, encryption_key)?;
            encrypted_deps.insert(name.clone(), encrypted);
        }
        
        let package = PiperNetPackage::new(
            self.manifest.clone(),
            encrypted_module,
            encrypted_assets,
            encrypted_deps,
        );
        
        info!("Package built successfully");
        Ok(package)
    }
    
    /// Build and save to .pn file
    pub async fn build_and_save(&self, output_path: &Path, encryption_key: &[u8]) -> Result<PathBuf> {
        let package = self.build(encryption_key)?;
        
        let output_path = if output_path.extension().is_none() {
            output_path.with_extension("pn")
        } else {
            output_path.to_path_buf()
        };
        
        package.save_to_file(&output_path, encryption_key).await?;
        
        info!("Package saved to: {:?}", output_path);
        Ok(output_path)
    }
}

/// Quick build helper function
pub async fn build_package(
    manifest_path: &Path,
    output_path: &Path,
    encryption_key: &[u8],
) -> Result<PathBuf> {
    let mut builder = PackageBuilder::from_manifest_file(manifest_path).await?;
    
    builder
        .load_module().await?
        .load_assets().await?
        .load_dependencies().await?;
    
    builder.build_and_save(output_path, encryption_key).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::manifest::{PackageMetadata, PackageType};
    
    #[tokio::test]
    async fn test_builder_creation() {
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
        
        let builder = PackageBuilder::new(manifest, PathBuf::from("/tmp"));
        assert_eq!(builder.manifest.metadata.name, "test");
    }
}
