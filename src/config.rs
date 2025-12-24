use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Main configuration for Pied Piper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiedPiperConfig {
    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,

    /// Gateway/HTTP server configuration
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Cache and storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Performance tuning configuration
    #[serde(default)]
    pub performance: PerformanceConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,
}

/// Network P2P configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// TCP listening port (0 for random)
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,

    /// QUIC listening port (0 for random)
    #[serde(default = "default_quic_port")]
    pub quic_port: u16,

    /// Enable mDNS for local peer discovery
    #[serde(default = "default_true")]
    pub enable_mdns: bool,

    /// Bootstrap peers (comma-separated: peer_id@multiaddr)
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,

    /// GossipSub topics to subscribe to
    #[serde(default)]
    pub topics: Vec<String>,

    /// Maximum number of connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Connection idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

/// Gateway HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// HTTP server port
    #[serde(default = "default_http_port")]
    pub port: u16,

    /// HTTPS server port (0 to disable)
    #[serde(default)]
    pub https_port: u16,

    /// Path to TLS certificate file
    #[serde(default)]
    pub tls_cert_path: Option<PathBuf>,

    /// Path to TLS private key file
    #[serde(default)]
    pub tls_key_path: Option<PathBuf>,

    /// Default index file for directory requests
    #[serde(default = "default_index_file")]
    pub index_file: String,

    /// Enable response compression
    #[serde(default = "default_true")]
    pub enable_compression: bool,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,
}

/// Storage and cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory for cache and persistence
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Module cache directory
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    /// DHT state directory
    #[serde(default)]
    pub dht_dir: Option<PathBuf>,

    /// Maximum cache size in bytes
    #[serde(default = "default_cache_size")]
    pub max_cache_size_bytes: usize,

    /// Maximum number of cached modules
    #[serde(default = "default_cache_entries")]
    pub max_cache_entries: usize,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads (0 for auto)
    #[serde(default)]
    pub worker_threads: usize,

    /// HTTP connection pool size per host
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,

    /// HTTP connection pool timeout in seconds
    #[serde(default = "default_pool_timeout")]
    pub pool_timeout_secs: u64,

    /// Enable TCP keepalive for HTTP connections
    #[serde(default = "default_true")]
    pub tcp_keepalive: bool,

    /// WASM execution fuel limit (0 for unlimited)
    #[serde(default = "default_fuel_limit")]
    pub wasm_fuel_limit: u64,

    /// WASM memory limit in bytes
    #[serde(default = "default_wasm_memory_limit")]
    pub wasm_memory_limit_bytes: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable JSON formatted logs
    #[serde(default)]
    pub json_format: bool,

    /// Log file path (empty for stdout only)
    #[serde(default)]
    pub file_path: Option<PathBuf>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_request_body_size: usize,

    /// Maximum request header size in bytes
    #[serde(default = "default_max_header_size")]
    pub max_header_size: usize,

    /// Rate limit: requests per minute per IP
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,

    /// Rate limit: burst size
    #[serde(default = "default_burst_size")]
    pub rate_limit_burst: u32,

    /// Maximum concurrent connections per IP
    #[serde(default = "default_max_connections_per_ip")]
    pub max_connections_per_ip: usize,

    /// Maximum concurrent requests globally
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,

    /// Enable HSTS header
    #[serde(default = "default_true")]
    pub enable_hsts: bool,

    /// HSTS max-age in seconds
    #[serde(default = "default_hsts_max_age")]
    pub hsts_max_age: u64,

    /// Enable strict CSP
    #[serde(default = "default_true")]
    pub enable_strict_csp: bool,

    /// Allowed origins for CORS (empty = no CORS)
    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,

    /// Block suspicious user agents
    #[serde(default = "default_true")]
    pub block_suspicious_user_agents: bool,

    /// Maximum path depth
    #[serde(default = "default_max_path_depth")]
    pub max_path_depth: usize,

    /// Allowed file extensions for static files
    #[serde(default = "default_allowed_extensions")]
    pub allowed_extensions: Vec<String>,
}

// Default value functions
fn default_tcp_port() -> u16 {
    0
}
fn default_quic_port() -> u16 {
    0
}
fn default_true() -> bool {
    true
}
fn default_max_connections() -> usize {
    100
}
fn default_idle_timeout() -> u64 {
    60
}
fn default_http_port() -> u16 {
    8080
}
fn default_index_file() -> String {
    "index.html".to_string()
}
fn default_request_timeout() -> u64 {
    30
}
fn default_data_dir() -> PathBuf {
    PathBuf::from(".pied-piper")
}
fn default_cache_size() -> usize {
    512 * 1024 * 1024 // 512 MB
}
fn default_cache_entries() -> usize {
    256
}
fn default_connection_pool_size() -> usize {
    10
}
fn default_pool_timeout() -> u64 {
    90
}
fn default_fuel_limit() -> u64 {
    100_000_000
}
fn default_wasm_memory_limit() -> usize {
    64 * 1024 * 1024 // 64 MB
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_max_body_size() -> usize {
    16 * 1024 * 1024 // 16 MB
}
fn default_max_header_size() -> usize {
    8 * 1024 // 8 KB
}
fn default_rate_limit() -> u32 {
    60
}
fn default_burst_size() -> u32 {
    10
}
fn default_max_connections_per_ip() -> usize {
    100
}
fn default_max_concurrent_requests() -> usize {
    10000
}
fn default_hsts_max_age() -> u64 {
    31536000 // 1 year
}
fn default_max_path_depth() -> usize {
    10
}
fn default_allowed_extensions() -> Vec<String> {
    vec![
        "html".to_string(),
        "css".to_string(),
        "js".to_string(),
        "wasm".to_string(),
        "json".to_string(),
        "png".to_string(),
        "jpg".to_string(),
        "jpeg".to_string(),
        "gif".to_string(),
        "svg".to_string(),
        "ico".to_string(),
        "woff".to_string(),
        "woff2".to_string(),
        "ttf".to_string(),
        "otf".to_string(),
        "txt".to_string(),
        "md".to_string(),
    ]
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            tcp_port: default_tcp_port(),
            quic_port: default_quic_port(),
            enable_mdns: default_true(),
            bootstrap_peers: vec![],
            topics: vec![],
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_http_port(),
            https_port: 0,
            tls_cert_path: None,
            tls_key_path: None,
            index_file: default_index_file(),
            enable_compression: default_true(),
            request_timeout_secs: default_request_timeout(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            cache_dir: None,
            dht_dir: None,
            max_cache_size_bytes: default_cache_size(),
            max_cache_entries: default_cache_entries(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0,
            connection_pool_size: default_connection_pool_size(),
            pool_timeout_secs: default_pool_timeout(),
            tcp_keepalive: default_true(),
            wasm_fuel_limit: default_fuel_limit(),
            wasm_memory_limit_bytes: default_wasm_memory_limit(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: false,
            file_path: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_request_body_size: default_max_body_size(),
            max_header_size: default_max_header_size(),
            rate_limit_per_minute: default_rate_limit(),
            rate_limit_burst: default_burst_size(),
            max_connections_per_ip: default_max_connections_per_ip(),
            max_concurrent_requests: default_max_concurrent_requests(),
            enable_hsts: default_true(),
            hsts_max_age: default_hsts_max_age(),
            enable_strict_csp: default_true(),
            cors_allowed_origins: vec![],
            block_suspicious_user_agents: default_true(),
            max_path_depth: default_max_path_depth(),
            allowed_extensions: default_allowed_extensions(),
        }
    }
}

impl Default for PiedPiperConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            gateway: GatewayConfig::default(),
            storage: StorageConfig::default(),
            performance: PerformanceConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl PiedPiperConfig {
    /// Load configuration with precedence: CLI args > ENV vars > Config file > Defaults
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let mut builder = config::Config::builder();

        // 1. Start with defaults
        let defaults = Self::default();
        builder = builder.add_source(config::File::from_str(
            &serde_json::to_string(&defaults)?,
            config::FileFormat::Json,
        ));

        // 2. Load from config file if provided
        if let Some(path) = config_path {
            if path.exists() {
                info!("Loading configuration from: {}", path.display());
                let format = Self::detect_format(path)?;
                builder = builder.add_source(config::File::from(path).format(format));
            } else {
                warn!("Config file not found: {}, using defaults", path.display());
            }
        }

        // 3. Load from environment variables with PP_ prefix
        builder = builder.add_source(
            config::Environment::with_prefix("PP")
                .separator("_")
                .try_parsing(true),
        );

        // Build and deserialize
        let config = builder
            .build()
            .context("Failed to build configuration")?;

        let mut pied_piper_config: PiedPiperConfig = config
            .try_deserialize()
            .context("Failed to deserialize configuration")?;

        // Resolve derived paths
        pied_piper_config.resolve_paths();

        // Validate configuration
        pied_piper_config.validate()?;

        Ok(pied_piper_config)
    }

    /// Detect configuration file format from extension
    fn detect_format(path: &Path) -> Result<config::FileFormat> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => Ok(config::FileFormat::Yaml),
            Some("toml") => Ok(config::FileFormat::Toml),
            Some("json") => Ok(config::FileFormat::Json),
            _ => anyhow::bail!(
                "Unsupported config file format. Use .yaml, .toml, or .json"
            ),
        }
    }

    /// Resolve derived paths from data_dir
    fn resolve_paths(&mut self) {
        // Set cache_dir if not specified
        if self.storage.cache_dir.is_none() {
            self.storage.cache_dir = Some(self.storage.data_dir.join("modules"));
        }

        // Set dht_dir if not specified
        if self.storage.dht_dir.is_none() {
            self.storage.dht_dir = Some(self.storage.data_dir.clone());
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate TLS configuration
        if self.gateway.https_port > 0 {
            if self.gateway.tls_cert_path.is_none() {
                anyhow::bail!("HTTPS enabled but tls_cert_path not specified");
            }
            if self.gateway.tls_key_path.is_none() {
                anyhow::bail!("HTTPS enabled but tls_key_path not specified");
            }
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            anyhow::bail!(
                "Invalid log level '{}'. Must be one of: trace, debug, info, warn, error",
                self.logging.level
            );
        }

        // Validate cache size
        if self.storage.max_cache_size_bytes == 0 {
            anyhow::bail!("max_cache_size_bytes must be greater than 0");
        }
        if self.storage.max_cache_entries == 0 {
            anyhow::bail!("max_cache_entries must be greater than 0");
        }

        // Validate performance settings
        if self.performance.wasm_memory_limit_bytes == 0 {
            anyhow::bail!("wasm_memory_limit_bytes must be greater than 0");
        }

        Ok(())
    }

    /// Generate example configuration file
    pub fn example_yaml() -> String {
        let example = Self {
            network: NetworkConfig {
                tcp_port: 4001,
                quic_port: 4002,
                enable_mdns: true,
                bootstrap_peers: vec![
                    "12D3KooWExamplePeer@/ip4/203.0.113.1/tcp/4001".to_string(),
                ],
                topics: vec!["pied-piper".to_string()],
                max_connections: 100,
                idle_timeout_secs: 60,
            },
            gateway: GatewayConfig {
                port: 8080,
                https_port: 8443,
                tls_cert_path: Some(PathBuf::from("/path/to/cert.pem")),
                tls_key_path: Some(PathBuf::from("/path/to/key.pem")),
                index_file: "index.html".to_string(),
                enable_compression: true,
                request_timeout_secs: 30,
            },
            storage: StorageConfig {
                data_dir: PathBuf::from(".pied-piper"),
                cache_dir: Some(PathBuf::from(".pied-piper/modules")),
                dht_dir: Some(PathBuf::from(".pied-piper")),
                max_cache_size_bytes: 512 * 1024 * 1024,
                max_cache_entries: 256,
            },
            performance: PerformanceConfig {
                worker_threads: 0,
                connection_pool_size: 10,
                pool_timeout_secs: 90,
                tcp_keepalive: true,
                wasm_fuel_limit: 100_000_000,
                wasm_memory_limit_bytes: 64 * 1024 * 1024,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json_format: false,
                file_path: None,
            },
            security: SecurityConfig::default(),
        };

        serde_yaml::to_string(&example).unwrap_or_else(|_| "# Error generating example".to_string())
    }

    /// Generate example TOML configuration
    pub fn example_toml() -> String {
        let example = Self {
            network: NetworkConfig {
                tcp_port: 4001,
                quic_port: 4002,
                enable_mdns: true,
                bootstrap_peers: vec![
                    "12D3KooWExamplePeer@/ip4/203.0.113.1/tcp/4001".to_string(),
                ],
                topics: vec!["pied-piper".to_string()],
                max_connections: 100,
                idle_timeout_secs: 60,
            },
            gateway: GatewayConfig {
                port: 8080,
                https_port: 8443,
                tls_cert_path: Some(PathBuf::from("/path/to/cert.pem")),
                tls_key_path: Some(PathBuf::from("/path/to/key.pem")),
                index_file: "index.html".to_string(),
                enable_compression: true,
                request_timeout_secs: 30,
            },
            storage: StorageConfig {
                data_dir: PathBuf::from(".pied-piper"),
                cache_dir: Some(PathBuf::from(".pied-piper/modules")),
                dht_dir: Some(PathBuf::from(".pied-piper")),
                max_cache_size_bytes: 512 * 1024 * 1024,
                max_cache_entries: 256,
            },
            performance: PerformanceConfig {
                worker_threads: 0,
                connection_pool_size: 10,
                pool_timeout_secs: 90,
                tcp_keepalive: true,
                wasm_fuel_limit: 100_000_000,
                wasm_memory_limit_bytes: 64 * 1024 * 1024,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json_format: false,
                file_path: None,
            },
            security: SecurityConfig::default(),
        };

        toml::to_string_pretty(&example).unwrap_or_else(|_| "# Error generating example".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = PiedPiperConfig::default();
        assert_eq!(config.gateway.port, 8080);
        assert_eq!(config.network.tcp_port, 0);
        assert_eq!(config.storage.max_cache_entries, 256);
    }

    #[test]
    fn test_load_from_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
network:
  tcp_port: 4001
  quic_port: 4002
gateway:
  port: 9090
storage:
  max_cache_entries: 512
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = PiedPiperConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.network.tcp_port, 4001);
        assert_eq!(config.gateway.port, 9090);
        assert_eq!(config.storage.max_cache_entries, 512);
    }

    #[test]
    fn test_validate_https_requires_tls() {
        let mut config = PiedPiperConfig::default();
        config.gateway.https_port = 8443;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tls_cert_path"));
    }

    #[test]
    fn test_validate_log_level() {
        let mut config = PiedPiperConfig::default();
        config.logging.level = "invalid".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid log level"));
    }

    #[test]
    fn test_resolve_paths() {
        let mut config = PiedPiperConfig::default();
        config.storage.data_dir = PathBuf::from("/custom/data");
        config.storage.cache_dir = None;
        config.storage.dht_dir = None;

        config.resolve_paths();

        assert_eq!(config.storage.cache_dir, Some(PathBuf::from("/custom/data/modules")));
        assert_eq!(config.storage.dht_dir, Some(PathBuf::from("/custom/data")));
    }

    #[test]
    fn test_example_yaml_generation() {
        let yaml = PiedPiperConfig::example_yaml();
        assert!(yaml.contains("network:"));
        assert!(yaml.contains("gateway:"));
        assert!(yaml.contains("tcp_port:"));
    }

    #[test]
    fn test_example_toml_generation() {
        let toml_str = PiedPiperConfig::example_toml();
        assert!(toml_str.contains("[network]"));
        assert!(toml_str.contains("[gateway]"));
        assert!(toml_str.contains("tcp_port"));
    }
    
    #[test]
    fn test_bootstrap_peers_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
network:
  tcp_port: 4001
  bootstrap_peers:
    - "/ip4/127.0.0.1/tcp/4000"
    - "/ip4/192.168.1.1/tcp/4000"
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = PiedPiperConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.network.bootstrap_peers.len(), 2);
        assert_eq!(config.network.bootstrap_peers[0], "/ip4/127.0.0.1/tcp/4000");
    }
    
    #[test]
    fn test_mdns_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
network:
  tcp_port: 4001
  enable_mdns: false
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = PiedPiperConfig::load(Some(&config_path)).unwrap();
        assert!(!config.network.enable_mdns);
    }
    
    #[test]
    fn test_network_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
network:
  tcp_port: 4001
  quic_port: 4002
  max_connections: 500
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = PiedPiperConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.network.tcp_port, 4001);
        assert_eq!(config.network.quic_port, 4002);
        assert_eq!(config.network.max_connections, 500);
    }
    
    #[test]
    fn test_invalid_yaml_fails() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = "invalid: yaml: content: {";
        fs::write(&config_path, yaml_content).unwrap();

        let result = PiedPiperConfig::load(Some(&config_path));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_missing_config_file_uses_default() {
        let result = PiedPiperConfig::load(Some(&PathBuf::from("/nonexistent/config.yaml")));
        // Should return default config when file doesn't exist
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_log_level_validation() {
        let mut config = PiedPiperConfig::default();
        
        config.logging.level = "trace".to_string();
        assert!(config.validate().is_ok());
        
        config.logging.level = "debug".to_string();
        assert!(config.validate().is_ok());
        
        config.logging.level = "info".to_string();
        assert!(config.validate().is_ok());
        
        config.logging.level = "warn".to_string();
        assert!(config.validate().is_ok());
        
        config.logging.level = "error".to_string();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_connection_limits() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
network:
  tcp_port: 4001
  max_connections: 500
  idle_timeout_secs: 120
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = PiedPiperConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.network.max_connections, 500);
        assert_eq!(config.network.idle_timeout_secs, 120);
    }
}

