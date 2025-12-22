//! Security hardening module for the Pied Piper gateway
//!
//! This module provides:
//! - Rate limiting per IP address
//! - Request validation and sanitization
//! - DDoS protection mechanisms
//! - Security headers enforcement
//! - Request size limits

use anyhow::{Context, Result};
use axum::http::{HeaderMap, StatusCode};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum request body size in bytes (default: 16MB)
    pub max_request_body_size: usize,

    /// Maximum request header size in bytes (default: 8KB)
    pub max_header_size: usize,

    /// Rate limit: requests per minute per IP (default: 60)
    pub rate_limit_per_minute: u32,

    /// Rate limit: burst size (default: 10)
    pub rate_limit_burst: u32,

    /// Maximum concurrent connections per IP (default: 100)
    pub max_connections_per_ip: usize,

    /// Maximum concurrent requests globally (default: 10000)
    pub max_concurrent_requests: usize,

    /// Request timeout in seconds (default: 30)
    pub request_timeout_secs: u64,

    /// Enable HSTS header
    pub enable_hsts: bool,

    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u64,

    /// Enable strict CSP
    pub enable_strict_csp: bool,

    /// Allowed origins for CORS (empty = no CORS)
    pub cors_allowed_origins: Vec<String>,

    /// Block suspicious user agents
    pub block_suspicious_user_agents: bool,

    /// Maximum path depth to prevent path traversal
    pub max_path_depth: usize,

    /// Allowed file extensions for static files
    pub allowed_extensions: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_request_body_size: 16 * 1024 * 1024, // 16MB
            max_header_size: 8 * 1024,                // 8KB
            rate_limit_per_minute: 60,
            rate_limit_burst: 10,
            max_connections_per_ip: 100,
            max_concurrent_requests: 10000,
            request_timeout_secs: 30,
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            enable_strict_csp: true,
            cors_allowed_origins: vec![],
            block_suspicious_user_agents: true,
            max_path_depth: 10,
            allowed_extensions: vec![
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
            ],
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate_per_minute: u32) -> Self {
        Self {
            tokens: capacity as f64,
            last_refill: Instant::now(),
            capacity: capacity as f64,
            refill_rate: refill_rate_per_minute as f64 / 60.0, // Convert to per-second
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        // Refill tokens based on time elapsed
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        // Try to consume tokens
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<IpAddr, TokenBucket>>>,
    config: SecurityConfig,
}

impl RateLimiter {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if request is allowed for the given IP
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), StatusCode> {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets.entry(ip).or_insert_with(|| {
            TokenBucket::new(self.config.rate_limit_burst, self.config.rate_limit_per_minute)
        });

        if bucket.try_consume(1.0) {
            Ok(())
        } else {
            warn!("Rate limit exceeded for IP: {}", ip);
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }

    /// Cleanup old buckets (call periodically)
    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let threshold = Instant::now() - Duration::from_secs(300); // 5 minutes

        buckets.retain(|_, bucket| bucket.last_refill > threshold);

        debug!("Rate limiter cleanup: {} buckets remaining", buckets.len());
    }
}

/// Connection tracker for DDoS protection
pub struct ConnectionTracker {
    connections: Arc<RwLock<HashMap<IpAddr, usize>>>,
    total_connections: Arc<RwLock<usize>>,
    config: SecurityConfig,
}

impl ConnectionTracker {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            total_connections: Arc::new(RwLock::new(0)),
            config,
        }
    }

    /// Try to register a new connection
    pub async fn register_connection(&self, ip: IpAddr) -> Result<(), StatusCode> {
        let mut connections = self.connections.write().await;
        let mut total = self.total_connections.write().await;

        // Check global limit
        if *total >= self.config.max_concurrent_requests {
            warn!("Global connection limit reached");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // Check per-IP limit
        let count = connections.entry(ip).or_insert(0);
        if *count >= self.config.max_connections_per_ip {
            warn!("Connection limit exceeded for IP: {}", ip);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        *count += 1;
        *total += 1;

        Ok(())
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, ip: IpAddr) {
        let mut connections = self.connections.write().await;
        let mut total = self.total_connections.write().await;

        if let Some(count) = connections.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                connections.remove(&ip);
            }
        }

        *total = total.saturating_sub(1);
    }

    /// Get current connection count for an IP
    pub async fn get_connection_count(&self, ip: IpAddr) -> usize {
        let connections = self.connections.read().await;
        *connections.get(&ip).unwrap_or(&0)
    }

    /// Get total connection count
    pub async fn get_total_connections(&self) -> usize {
        *self.total_connections.read().await
    }
}

/// Request validator
pub struct RequestValidator {
    config: SecurityConfig,
}

impl RequestValidator {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// Validate request path for security issues
    pub fn validate_path(&self, path: &str) -> Result<(), StatusCode> {
        // Check for null bytes
        if path.contains('\0') {
            warn!("Path contains null byte: {}", path);
            return Err(StatusCode::BAD_REQUEST);
        }

        // Check for path traversal attempts
        if path.contains("..") {
            warn!("Path traversal attempt detected: {}", path);
            return Err(StatusCode::FORBIDDEN);
        }

        // Check path depth
        let depth = path.trim_start_matches('/').split('/').count();
        if depth > self.config.max_path_depth {
            warn!("Path depth exceeds limit: {} (depth: {})", path, depth);
            return Err(StatusCode::BAD_REQUEST);
        }

        // Check for suspicious patterns
        let suspicious_patterns = [
            "/etc/", "/proc/", "/sys/", "/dev/", "/../", "/./", "//", "\\\\",
        ];

        for pattern in &suspicious_patterns {
            if path.contains(pattern) {
                warn!("Suspicious pattern in path: {} (pattern: {})", path, pattern);
                return Err(StatusCode::FORBIDDEN);
            }
        }

        Ok(())
    }

    /// Validate file extension
    pub fn validate_extension(&self, path: &str) -> Result<(), StatusCode> {
        // If no extension, it's likely a route (allow it)
        if !path.contains('.') {
            return Ok(());
        }

        if let Some(ext) = path.rsplit('.').next() {
            let ext_lower = ext.to_lowercase();
            if self.config.allowed_extensions.contains(&ext_lower) {
                Ok(())
            } else {
                warn!("Disallowed file extension: {}", ext);
                Err(StatusCode::FORBIDDEN)
            }
        } else {
            Ok(())
        }
    }

    /// Validate headers for security issues
    pub fn validate_headers(&self, headers: &HeaderMap) -> Result<(), StatusCode> {
        // Check total header size
        let total_size: usize = headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.len())
            .sum();

        if total_size > self.config.max_header_size {
            warn!("Headers exceed size limit: {} bytes", total_size);
            return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
        }

        // Check for suspicious user agents
        if self.config.block_suspicious_user_agents {
            if let Some(user_agent) = headers.get("user-agent") {
                if let Ok(ua) = user_agent.to_str() {
                    let ua_lower = ua.to_lowercase();
                    let suspicious = [
                        "sqlmap",
                        "nikto",
                        "nmap",
                        "masscan",
                        "nessus",
                        "burp",
                        "metasploit",
                        "havij",
                        "acunetix",
                        "qualys",
                    ];

                    for pattern in &suspicious {
                        if ua_lower.contains(pattern) {
                            warn!("Suspicious user agent detected: {}", ua);
                            return Err(StatusCode::FORBIDDEN);
                        }
                    }
                }
            }
        }

        // Check for header injection attempts
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                if value_str.contains('\r') || value_str.contains('\n') {
                    warn!(
                        "Header injection attempt detected: {}: {}",
                        name.as_str(),
                        value_str
                    );
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }

        Ok(())
    }

    /// Sanitize path by removing dangerous characters
    pub fn sanitize_path(&self, path: &str) -> String {
        path.chars()
            .filter(|c| c.is_alphanumeric() || *c == '/' || *c == '-' || *c == '_' || *c == '.')
            .collect()
    }
}

/// Security middleware state
pub struct SecurityMiddleware {
    pub rate_limiter: Arc<RateLimiter>,
    pub connection_tracker: Arc<ConnectionTracker>,
    pub validator: Arc<RequestValidator>,
    pub config: SecurityConfig,
}

impl SecurityMiddleware {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: Arc::new(RateLimiter::new(config.clone())),
            connection_tracker: Arc::new(ConnectionTracker::new(config.clone())),
            validator: Arc::new(RequestValidator::new(config.clone())),
            config,
        }
    }

    /// Get security headers to add to responses
    pub fn get_security_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            ("X-Content-Type-Options", "nosniff".to_string()),
            ("X-Frame-Options", "SAMEORIGIN".to_string()),
            (
                "Referrer-Policy",
                "strict-origin-when-cross-origin".to_string(),
            ),
            ("X-XSS-Protection", "1; mode=block".to_string()),
        ];

        if self.config.enable_hsts {
            headers.push((
                "Strict-Transport-Security",
                format!(
                    "max-age={}; includeSubDomains; preload",
                    self.config.hsts_max_age
                ),
            ));
        }

        if self.config.enable_strict_csp {
            headers.push((
                "Content-Security-Policy",
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'self'; base-uri 'self'; form-action 'self'".to_string(),
            ));
        }

        headers
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(rate_limiter: Arc<RateLimiter>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                rate_limiter.cleanup_old_buckets().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_request_body_size, 16 * 1024 * 1024);
        assert_eq!(config.rate_limit_per_minute, 60);
        assert!(config.enable_hsts);
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 60);

        // Should be able to consume initial capacity
        for _ in 0..10 {
            assert!(bucket.try_consume(1.0));
        }

        // Should be empty now
        assert!(!bucket.try_consume(1.0));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = SecurityConfig {
            rate_limit_burst: 5,
            rate_limit_per_minute: 60,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Should allow initial burst
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }

        // Should be rate limited
        assert!(limiter.check_rate_limit(ip).await.is_err());
    }

    #[tokio::test]
    async fn test_connection_tracker() {
        let config = SecurityConfig {
            max_connections_per_ip: 3,
            max_concurrent_requests: 10,
            ..Default::default()
        };

        let tracker = ConnectionTracker::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Register connections
        assert!(tracker.register_connection(ip).await.is_ok());
        assert!(tracker.register_connection(ip).await.is_ok());
        assert!(tracker.register_connection(ip).await.is_ok());

        // Should hit limit
        assert!(tracker.register_connection(ip).await.is_err());

        // Unregister one
        tracker.unregister_connection(ip).await;

        // Should work again
        assert!(tracker.register_connection(ip).await.is_ok());
    }

    #[test]
    fn test_path_validation() {
        let config = SecurityConfig::default();
        let validator = RequestValidator::new(config);

        // Valid paths
        assert!(validator.validate_path("/api/users").is_ok());
        assert!(validator.validate_path("/static/app.js").is_ok());
        assert!(validator.validate_path("/").is_ok());

        // Invalid paths
        assert!(validator.validate_path("/../etc/passwd").is_err());
        assert!(validator.validate_path("/etc/shadow").is_err());
        assert!(validator.validate_path("/path/with/null\0byte").is_err());
        assert!(validator.validate_path("path//double//slash").is_err());
    }

    #[test]
    fn test_extension_validation() {
        let config = SecurityConfig::default();
        let validator = RequestValidator::new(config);

        // Allowed extensions
        assert!(validator.validate_extension("/app.js").is_ok());
        assert!(validator.validate_extension("/style.css").is_ok());
        assert!(validator.validate_extension("/module.wasm").is_ok());

        // Disallowed extensions
        assert!(validator.validate_extension("/script.php").is_err());
        assert!(validator.validate_extension("/file.exe").is_err());
        assert!(validator.validate_extension("/shell.sh").is_err());
    }

    #[test]
    fn test_path_sanitization() {
        let config = SecurityConfig::default();
        let validator = RequestValidator::new(config);

        assert_eq!(
            validator.sanitize_path("/api/user<script>"),
            "/api/userscript"
        );
        assert_eq!(validator.sanitize_path("/path;rm -rf"), "/pathrm-rf");
        assert_eq!(validator.sanitize_path("/normal_path-123"), "/normal_path-123");
    }

    #[test]
    fn test_header_validation() {
        let config = SecurityConfig::default();
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers.insert("user-agent", "Mozilla/5.0".parse().unwrap());
        headers.insert("accept", "text/html".parse().unwrap());

        assert!(validator.validate_headers(&headers).is_ok());

        // Test suspicious user agent
        let mut bad_headers = HeaderMap::new();
        bad_headers.insert("user-agent", "sqlmap/1.0".parse().unwrap());
        assert!(validator.validate_headers(&bad_headers).is_err());
    }
}
