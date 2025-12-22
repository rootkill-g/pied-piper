use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// TLS configuration for the gateway
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to TLS certificate file (PEM format)
    pub cert_path: PathBuf,
    /// Path to TLS private key file (PEM format)
    pub key_path: PathBuf,
    /// Enable HTTPS
    pub enabled: bool,
}

impl TlsConfig {
    /// Create a new TLS configuration
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
            enabled: true,
        }
    }

    /// Check if TLS files exist
    pub fn validate(&self) -> Result<()> {
        if !self.cert_path.exists() {
            anyhow::bail!("Certificate file not found: {:?}", self.cert_path);
        }
        if !self.key_path.exists() {
            anyhow::bail!("Private key file not found: {:?}", self.key_path);
        }
        Ok(())
    }

    /// Load TLS configuration for axum-server
    pub async fn build_server_config(&self) -> Result<axum_server::tls_rustls::RustlsConfig> {
        info!("Loading TLS certificate from {:?}", self.cert_path);
        info!("Loading TLS private key from {:?}", self.key_path);

        // Use axum-server's RustlsConfig which handles file loading
        axum_server::tls_rustls::RustlsConfig::from_pem_file(&self.cert_path, &self.key_path)
            .await
            .context("Failed to build TLS config from PEM files")
    }
}

/// Generate a self-signed certificate for development
pub fn generate_self_signed_cert(domain: &str, cert_path: &Path, key_path: &Path) -> Result<()> {
    warn!("Generating self-signed certificate for development");
    warn!("Domain: {}", domain);

    // For production use, we'd use rcgen crate, but for now just log instructions
    info!("To generate a self-signed certificate, run:");
    info!("  openssl req -x509 -newkey rsa:4096 -nodes \\");
    info!("    -keyout {:?} \\", key_path);
    info!("    -out {:?} \\", cert_path);
    info!("    -days 365 -subj '/CN={}'", domain);

    anyhow::bail!(
        "Self-signed certificate generation not implemented. \
         Please use openssl to generate certificates manually."
    )
}

/// Get default certificate directory
pub fn default_cert_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pied-piper")
        .join("certs")
}

/// Ensure certificate directory exists
pub fn ensure_cert_dir() -> Result<PathBuf> {
    let cert_dir = default_cert_dir();
    fs::create_dir_all(&cert_dir)
        .with_context(|| format!("Failed to create cert directory: {:?}", cert_dir))?;
    Ok(cert_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cert_dir() {
        let dir = default_cert_dir();
        assert!(dir.to_string_lossy().contains(".pied-piper"));
        assert!(dir.to_string_lossy().contains("certs"));
    }

    #[test]
    fn test_tls_config_validation() {
        let config = TlsConfig::new(
            PathBuf::from("/nonexistent/cert.pem"),
            PathBuf::from("/nonexistent/key.pem"),
        );
        assert!(config.validate().is_err());
    }
}
