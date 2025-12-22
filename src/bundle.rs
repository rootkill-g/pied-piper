use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Represents a bundled application with code and assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppBundle {
    /// Main WASM module
    pub wasm_module: Vec<u8>,
    /// Static assets (HTML, CSS, JS, images, etc.)
    pub assets: HashMap<String, Vec<u8>>,
    /// Bundle metadata
    pub metadata: BundleMetadata,
}

/// Metadata for an application bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Application name
    pub name: String,
    /// Version
    pub version: String,
    /// Entry point (main WASM file name)
    pub entry_point: String,
    /// Asset paths
    pub asset_paths: Vec<String>,
    /// Total bundle size
    pub total_size: usize,
    /// Creation timestamp
    pub created_at: u64,
}

impl AppBundle {
    /// Create a new bundle from a WASM module and asset directory
    pub async fn new(
        wasm_path: impl AsRef<Path>,
        assets_dir: Option<impl AsRef<Path>>,
        name: String,
        version: String,
    ) -> Result<Self> {
        let wasm_path = wasm_path.as_ref();

        // Load WASM module
        let wasm_module = fs::read(wasm_path)
            .await
            .with_context(|| format!("Failed to read WASM module: {:?}", wasm_path))?;

        info!("Loaded WASM module: {} bytes", wasm_module.len());

        // Load assets if directory provided
        let mut assets = HashMap::new();
        let mut asset_paths = Vec::new();

        if let Some(assets_dir) = assets_dir {
            let assets_dir = assets_dir.as_ref();
            if assets_dir.exists() {
                Self::load_assets_recursive(assets_dir, assets_dir, &mut assets, &mut asset_paths)
                    .await?;
                info!("Loaded {} assets", assets.len());
            }
        }

        let total_size = wasm_module.len() + assets.values().map(|v| v.len()).sum::<usize>();
        let entry_point = wasm_path.file_name().unwrap().to_string_lossy().to_string();

        let metadata = BundleMetadata {
            name,
            version,
            entry_point,
            asset_paths,
            total_size,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        Ok(Self {
            wasm_module,
            assets,
            metadata,
        })
    }

    /// Recursively load assets from directory
    fn load_assets_recursive<'a>(
        base_dir: &'a Path,
        current_dir: &'a Path,
        assets: &'a mut HashMap<String, Vec<u8>>,
        asset_paths: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = fs::read_dir(current_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;

                if metadata.is_file() {
                    // Get relative path from base directory
                    let relative_path = path.strip_prefix(base_dir)?.to_string_lossy().to_string();

                    // Skip hidden files and common build artifacts
                    if relative_path.starts_with('.')
                        || relative_path.contains("/target/")
                        || relative_path.contains("/node_modules/")
                    {
                        continue;
                    }

                    debug!("Loading asset: {}", relative_path);
                    let content = fs::read(&path).await?;
                    assets.insert(relative_path.clone(), content);
                    asset_paths.push(relative_path);
                } else if metadata.is_dir() {
                    // Recurse into subdirectories
                    Self::load_assets_recursive(base_dir, &path, assets, asset_paths).await?;
                }
            }

            Ok(())
        })
    }

    /// Serialize bundle to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize bundle")
    }

    /// Deserialize bundle from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize bundle")
    }

    /// Get asset by path
    pub fn get_asset(&self, path: &str) -> Option<&Vec<u8>> {
        self.assets.get(path)
    }

    /// Get WASM module
    pub fn wasm_module(&self) -> &[u8] {
        &self.wasm_module
    }

    /// Get metadata
    pub fn metadata(&self) -> &BundleMetadata {
        &self.metadata
    }

    /// List all asset paths
    pub fn asset_paths(&self) -> Vec<String> {
        self.metadata.asset_paths.clone()
    }

    /// Get content type for an asset based on extension
    pub fn content_type_for_path(path: &str) -> &'static str {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" | "mjs" => "application/javascript",
            "json" => "application/json",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "wasm" => "application/wasm",
            "txt" => "text/plain",
            "xml" => "application/xml",
            "pdf" => "application/pdf",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "ico" => "image/x-icon",
            _ => "application/octet-stream",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_bundle_creation() {
        let dir = tempdir().unwrap();

        // Create WASM module
        let wasm_path = dir.path().join("module.wasm");
        fs::write(&wasm_path, b"fake wasm").await.unwrap();

        // Create assets directory
        let assets_dir = dir.path().join("assets");
        fs::create_dir(&assets_dir).await.unwrap();
        fs::write(assets_dir.join("index.html"), b"<html></html>")
            .await
            .unwrap();
        fs::write(assets_dir.join("styles.css"), b"body{}")
            .await
            .unwrap();

        // Create bundle
        let bundle = AppBundle::new(
            wasm_path,
            Some(assets_dir),
            "test-app".to_string(),
            "1.0.0".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(bundle.wasm_module().len(), 9);
        assert_eq!(bundle.assets.len(), 2);
        assert!(bundle.get_asset("index.html").is_some());
        assert!(bundle.get_asset("styles.css").is_some());
        assert_eq!(bundle.metadata().name, "test-app");
    }

    #[tokio::test]
    async fn test_bundle_serialization() {
        let dir = tempdir().unwrap();
        let wasm_path = dir.path().join("module.wasm");
        fs::write(&wasm_path, b"fake wasm").await.unwrap();

        let bundle = AppBundle::new(
            wasm_path,
            None::<PathBuf>,
            "test-app".to_string(),
            "1.0.0".to_string(),
        )
        .await
        .unwrap();

        let bytes = bundle.to_bytes().unwrap();
        let deserialized = AppBundle::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.wasm_module(), bundle.wasm_module());
        assert_eq!(deserialized.metadata().name, bundle.metadata().name);
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(AppBundle::content_type_for_path("index.html"), "text/html");
        assert_eq!(AppBundle::content_type_for_path("styles.css"), "text/css");
        assert_eq!(
            AppBundle::content_type_for_path("app.js"),
            "application/javascript"
        );
        assert_eq!(
            AppBundle::content_type_for_path("data.json"),
            "application/json"
        );
        assert_eq!(
            AppBundle::content_type_for_path("module.wasm"),
            "application/wasm"
        );
    }
}
