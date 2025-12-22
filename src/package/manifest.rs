/// Package manifest (pn.toml) structure
/// 
/// Similar to Cargo.toml, this defines the metadata and
/// structure of a PiperNet package.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package manifest (pn.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package metadata
    pub metadata: PackageMetadata,
    
    /// Type of package
    #[serde(rename = "type")]
    pub package_type: PackageType,
    
    /// Main entrypoint (WASM module path)
    pub entrypoint: String,
    
    /// List of asset file paths
    #[serde(default)]
    pub assets: Vec<String>,
    
    /// Dependencies (name -> version requirement)
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// Package metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name (e.g., "hello-api")
    pub name: String,
    
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    
    /// Short description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Author name/email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    
    /// License identifier (e.g., "MIT", "Apache-2.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    
    /// Homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    
    /// Repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// Type of package
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// Backend API (WASM only)
    Backend,
    
    /// Frontend web app (HTML/CSS/JS + optional WASM)
    Frontend,
    
    /// Full-stack (backend WASM + frontend assets)
    FullStack,
    
    /// Library (reusable WASM component)
    Library,
}

impl PackageManifest {
    /// Load manifest from pn.toml content
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
    
    /// Serialize manifest to pn.toml format
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
    
    /// Create a minimal manifest with required fields
    pub fn minimal(name: String, version: String, entrypoint: String) -> Self {
        Self {
            metadata: PackageMetadata {
                name,
                version,
                description: None,
                author: None,
                license: None,
                homepage: None,
                repository: None,
            },
            package_type: PackageType::Backend,
            entrypoint,
            assets: Vec::new(),
            dependencies: HashMap::new(),
        }
    }
    
    /// Example manifest for documentation
    pub fn example() -> String {
        r#"type = "backend"
entrypoint = "target/wasm32-wasip1/release/hello-api.wasm"

[metadata]
name = "hello-api"
version = "1.0.0"
description = "Pipey's Hello World API"
author = "Pipey <pipey@piper.net>"
license = "MIT"
homepage = "https://pipey.net/hello-api"
repository = "https://pipergit.com/pipey/hello-api"

[dependencies]
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_manifest_roundtrip() {
        let manifest = PackageManifest::minimal(
            "test-app".to_string(),
            "1.0.0".to_string(),
            "module.wasm".to_string(),
        );
        
        let toml = manifest.to_toml().unwrap();
        let decoded = PackageManifest::from_toml(&toml).unwrap();
        
        assert_eq!(manifest.metadata.name, decoded.metadata.name);
        assert_eq!(manifest.metadata.version, decoded.metadata.version);
    }
    
    #[test]
    fn test_example_manifest() {
        let example = PackageManifest::example();
        let manifest = PackageManifest::from_toml(&example).unwrap();
        
        assert_eq!(manifest.metadata.name, "hello-api");
        assert_eq!(manifest.metadata.version, "1.0.0");
        assert_eq!(manifest.package_type, PackageType::Backend);
    }
}
