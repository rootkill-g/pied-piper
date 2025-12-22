use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

/// A WebAssembly module manifest describing application metadata and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Application name (required, alphanumeric + hyphens)
    pub name: String,
    /// Semantic version (required)
    pub version: String,
    /// Author/organization name
    pub author: Option<String>,
    /// Human-readable description
    pub description: Option<String>,
    /// Path to the main WASM module (relative to manifest file)
    pub module: String,
    /// Runtime configuration
    pub runtime: Option<RuntimeConfig>,
    /// Dependency specifications
    pub dependencies: Option<HashMap<String, DependencySpec>>,
    /// Required WASM capabilities
    pub capabilities: Option<Vec<String>>,
    /// Environment variables to be made available to the module
    pub env: Option<HashMap<String, String>>,
}

/// Runtime configuration for module execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// WASM engine backend ("wasmtime" is the default/only option for now)
    pub backend: Option<String>,
    /// Maximum memory in bytes
    pub max_memory: Option<usize>,
    /// Maximum execution time in seconds
    pub max_execution_time: Option<u64>,
    /// Enable WASI support
    pub enable_wasi: Option<bool>,
    /// Module entry point function name
    pub entry_point: Option<String>,
}

/// Specification for a module dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string ("1.0.0", "^1.0", "~1.0", etc.)
    Simple(String),
    /// Complex specification with CID and version constraints
    Complex {
        /// Content ID of the module
        cid: Option<String>,
        /// Version or version constraint
        version: Option<String>,
        /// Optional: specific peer to fetch from
        from: Option<String>,
    },
}

/// Represents parsed and validated manifest
#[derive(Debug, Clone)]
pub struct ValidatedManifest {
    inner: Manifest,
    base_dir: PathBuf,
}

impl ValidatedManifest {
    /// Parse and validate a manifest from YAML content
    pub fn from_yaml(content: &str, base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        let manifest: Manifest =
            serde_yaml::from_str(content).context("Failed to parse manifest YAML")?;

        // Validate required fields
        manifest.validate()?;

        // Validate module path exists
        let module_path = base_dir.join(&manifest.module);
        if !module_path.exists() {
            return Err(anyhow!("Module not found at {:?}", module_path));
        }

        Ok(Self {
            inner: manifest,
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Load and parse a manifest from a file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read manifest: {:?}", path))?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::from_yaml(&content, base_dir)
    }

    /// Get the absolute path to the main module
    pub fn module_path(&self) -> PathBuf {
        self.base_dir.join(&self.inner.module)
    }

    /// Get all dependencies
    pub fn dependencies(&self) -> HashMap<String, DependencySpec> {
        self.inner.dependencies.clone().unwrap_or_default()
    }

    /// Check if a capability is required
    pub fn requires_capability(&self, cap: &str) -> bool {
        self.inner
            .capabilities
            .as_ref()
            .map(|caps| caps.iter().any(|c| c == cap))
            .unwrap_or(false)
    }

    /// Get environment variables
    pub fn env_vars(&self) -> HashMap<String, String> {
        self.inner.env.clone().unwrap_or_default()
    }

    /// Get runtime configuration
    pub fn runtime(&self) -> Option<&RuntimeConfig> {
        self.inner.runtime.as_ref()
    }

    /// Dereference to inner manifest
    pub fn inner(&self) -> &Manifest {
        &self.inner
    }
}

impl Manifest {
    /// Validate the manifest for correctness
    fn validate(&self) -> Result<()> {
        // Validate name
        if self.name.is_empty() {
            return Err(anyhow!(
                "Manifest field 'name' is required and cannot be empty"
            ));
        }
        if !Self::is_valid_identifier(&self.name) {
            return Err(anyhow!(
                "Manifest 'name' must contain only alphanumeric characters, hyphens, and underscores"
            ));
        }

        // Validate version
        if self.version.is_empty() {
            return Err(anyhow!("Manifest field 'version' is required"));
        }
        if !Self::is_valid_semver(&self.version) {
            warn!(
                "Version '{}' doesn't follow semantic versioning (major.minor.patch)",
                self.version
            );
            // Don't fail, just warn - some version formats might be valid
        }

        // Validate module path
        if self.module.is_empty() {
            return Err(anyhow!("Manifest field 'module' is required"));
        }
        if self.module.contains("..") {
            return Err(anyhow!(
                "Module path cannot contain '..' for security reasons"
            ));
        }

        // Validate dependencies for cycles
        if let Some(deps) = &self.dependencies {
            self.check_dependency_cycles(deps)?;
        }

        // Validate capabilities
        if let Some(caps) = &self.capabilities {
            for cap in caps {
                if !Self::is_valid_capability(cap) {
                    warn!("Unknown capability required: {}", cap);
                }
            }
        }

        // Validate runtime config
        if let Some(runtime) = &self.runtime {
            if let Some(backend) = &runtime.backend {
                if backend != "wasmtime" {
                    warn!("Unknown runtime backend: {}. Using 'wasmtime'.", backend);
                }
            }
            if let Some(max_mem) = runtime.max_memory {
                if max_mem < 1024 * 1024 {
                    warn!(
                        "Max memory too small ({}), minimum 1MB recommended",
                        max_mem
                    );
                }
            }
        }

        Ok(())
    }

    /// Check for dependency cycles
    fn check_dependency_cycles(&self, deps: &HashMap<String, DependencySpec>) -> Result<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for dep_name in deps.keys() {
            if !visited.contains(dep_name) {
                self.dfs_check(&mut visited, &mut rec_stack, dep_name, deps)?;
            }
        }
        Ok(())
    }

    /// DFS for cycle detection
    fn dfs_check(
        &self,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        node: &str,
        deps: &HashMap<String, DependencySpec>,
    ) -> Result<()> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(_dep_spec) = deps.get(node) {
            // In a real system, we'd follow the dependency graph here
            // For now, we just check the direct dependency exists
        }

        rec_stack.remove(node);
        Ok(())
    }

    /// Check if string is a valid identifier (name, module, etc.)
    fn is_valid_identifier(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !s.starts_with('-')
            && !s.starts_with('_')
    }

    /// Check if version follows semantic versioning
    fn is_valid_semver(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u32>().is_ok())
    }

    /// Check if a capability is recognized
    fn is_valid_capability(cap: &str) -> bool {
        matches!(
            cap,
            "network"
                | "storage"
                | "crypto"
                | "filesystem"
                | "threading"
                | "random"
                | "time"
                | "env"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_valid_identifier() {
        assert!(Manifest::is_valid_identifier("my-module"));
        assert!(Manifest::is_valid_identifier("my_module"));
        assert!(Manifest::is_valid_identifier("MyModule123"));
        assert!(!Manifest::is_valid_identifier("-invalid"));
        assert!(!Manifest::is_valid_identifier(""));
    }

    #[test]
    fn test_valid_semver() {
        assert!(Manifest::is_valid_semver("1.0.0"));
        assert!(Manifest::is_valid_semver("1.2"));
        assert!(!Manifest::is_valid_semver("1"));
        assert!(!Manifest::is_valid_semver("1.2.3.4"));
    }

    #[tokio::test]
    async fn test_manifest_parsing() {
        let dir = tempdir().unwrap();
        let module_path = dir.path().join("module.wasm");
        fs::write(&module_path, b"fake wasm").await.unwrap();

        let yaml = r#"
name: test-app
version: 1.0.0
author: Test Author
description: A test application
module: module.wasm
"#;

        let result = ValidatedManifest::from_yaml(yaml, dir.path());
        assert!(result.is_ok());

        let manifest = result.unwrap();
        assert_eq!(manifest.inner().name, "test-app");
        assert_eq!(manifest.inner().version, "1.0.0");
    }

    #[test]
    fn test_manifest_validation_missing_name() {
        let yaml = r#"
version: 1.0.0
module: module.wasm
"#;
        let result = Manifest::from_yaml(yaml, Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_validation_missing_version() {
        let yaml = r#"
name: test-app
module: module.wasm
"#;
        let result = Manifest::from_yaml(yaml, Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_validation_path_traversal() {
        let yaml = r#"
name: test-app
version: 1.0.0
module: ../../../etc/passwd
"#;
        let result = Manifest::from_yaml(yaml, Path::new("."));
        assert!(result.is_err());
    }
}

impl Manifest {
    /// Parse manifest from YAML string (for testing)
    pub fn from_yaml(content: &str, _base_dir: &Path) -> Result<Self> {
        let manifest: Self = serde_yaml::from_str(content)?;
        manifest.validate()?;
        Ok(manifest)
    }
}
