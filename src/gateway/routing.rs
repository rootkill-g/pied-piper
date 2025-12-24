/// Production-ready routing utilities for the Pied Piper gateway
///
/// This module provides robust routing functionality with:
/// - CID validation
/// - Path normalization and security
/// - Method validation
/// - URL encoding/decoding
/// - Comprehensive error handling

use axum::http::{Method, StatusCode};
use std::collections::HashSet;
use tracing::{debug, warn};

/// Result type for routing operations
pub type RoutingResult<T> = Result<T, RoutingError>;

/// Routing-specific errors
#[derive(Debug, Clone)]
pub enum RoutingError {
    InvalidCID(String),
    InvalidPath(String),
    MethodNotAllowed(String),
    InvalidEncoding(String),
}

impl RoutingError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            RoutingError::InvalidCID(_) => StatusCode::BAD_REQUEST,
            RoutingError::InvalidPath(_) => StatusCode::BAD_REQUEST,
            RoutingError::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            RoutingError::InvalidEncoding(_) => StatusCode::BAD_REQUEST,
        }
    }

    pub fn message(&self) -> String {
        match self {
            RoutingError::InvalidCID(msg) => format!("Invalid CID: {}", msg),
            RoutingError::InvalidPath(msg) => format!("Invalid path: {}", msg),
            RoutingError::MethodNotAllowed(msg) => format!("Method not allowed: {}", msg),
            RoutingError::InvalidEncoding(msg) => format!("Invalid URL encoding: {}", msg),
        }
    }
}

/// CID validator
pub struct CIDValidator;

impl CIDValidator {
    /// Validate that a string is a valid CID (Content Identifier)
    /// 
    /// Validates:
    /// - Starts with 'b' (base32)
    /// - Correct length (typically 59 characters for CIDv1 base32)
    /// - Contains only valid base32 characters (a-z, 2-7)
    pub fn validate(cid: &str) -> RoutingResult<()> {
        // Check minimum length
        if cid.len() < 30 {
            warn!("CID too short: {} (length: {})", cid, cid.len());
            return Err(RoutingError::InvalidCID(
                "CID too short (minimum 30 characters)".to_string(),
            ));
        }

        // Check maximum length (prevent DoS)
        if cid.len() > 100 {
            warn!("CID too long: {} (length: {})", cid, cid.len());
            return Err(RoutingError::InvalidCID(
                "CID too long (maximum 100 characters)".to_string(),
            ));
        }

        // Must start with 'b' for base32 CIDv1
        if !cid.starts_with('b') {
            warn!("CID does not start with 'b': {}", cid);
            return Err(RoutingError::InvalidCID(
                "CID must start with 'b' (base32 encoding)".to_string(),
            ));
        }

        // Validate characters (base32: a-z, 2-7)
        for ch in cid.chars() {
            if !ch.is_ascii_lowercase() && !matches!(ch, '2'..='7') {
                warn!("Invalid character in CID: {} (char: {})", cid, ch);
                return Err(RoutingError::InvalidCID(format!(
                    "Invalid character '{}' in CID (must be lowercase a-z or 2-7)",
                    ch
                )));
            }
        }

        debug!("CID validated successfully: {}", cid);
        Ok(())
    }

    /// Check if a string looks like a valid CID (fast check)
    pub fn looks_valid(s: &str) -> bool {
        s.starts_with('b') && s.len() >= 30 && s.len() <= 100 && s.chars().all(|c| c.is_ascii_lowercase() || matches!(c, '2'..='7'))
    }
}

/// Path sanitizer and validator
pub struct PathSanitizer;

impl PathSanitizer {
    /// Normalize and validate a path
    ///
    /// Normalizes:
    /// - Removes multiple consecutive slashes
    /// - Removes leading/trailing slashes
    /// - Resolves . and .. components
    /// - Decodes URL encoding
    ///
    /// Validates:
    /// - No directory traversal attempts
    /// - No null bytes
    /// - No suspicious patterns
    pub fn normalize(path: &str) -> RoutingResult<String> {
        // Check for null bytes
        if path.contains('\0') {
            warn!("Path contains null byte: {:?}", path);
            return Err(RoutingError::InvalidPath(
                "Path contains null byte".to_string(),
            ));
        }

        // Decode URL encoding
        let decoded = match urlencoding::decode(path) {
            Ok(s) => s.to_string(),
            Err(e) => {
                warn!("Failed to decode path: {} (error: {})", path, e);
                return Err(RoutingError::InvalidEncoding(format!(
                    "Invalid URL encoding: {}",
                    e
                )));
            }
        };

        // Check for directory traversal BEFORE normalization
        if decoded.contains("..") {
            warn!("Directory traversal attempt detected: {}", decoded);
            return Err(RoutingError::InvalidPath(
                "Directory traversal not allowed".to_string(),
            ));
        }

        // Split path into components
        let components: Vec<&str> = decoded
            .split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .collect();

        // Check each component for suspicious patterns
        for component in &components {
            // Check for hidden files
            if component.starts_with('.') && component.len() > 1 {
                warn!("Hidden file access attempt: {}", component);
                return Err(RoutingError::InvalidPath(
                    "Access to hidden files not allowed".to_string(),
                ));
            }

            // Check for special characters
            for ch in component.chars() {
                if !ch.is_alphanumeric() && !matches!(ch, '-' | '_' | '.') {
                    warn!(
                        "Invalid character in path component: {} (char: {})",
                        component, ch
                    );
                    return Err(RoutingError::InvalidPath(format!(
                        "Invalid character '{}' in path",
                        ch
                    )));
                }
            }
        }

        // Join components back
        let normalized = components.join("/");

        // Final validation
        Self::validate_security(&normalized)?;

        debug!("Path normalized: {} -> {}", path, normalized);
        Ok(normalized)
    }

    /// Validate path for security issues
    fn validate_security(path: &str) -> RoutingResult<()> {
        // Suspicious patterns
        let suspicious_patterns = [
            "/etc/", "/proc/", "/sys/", "/dev/", "/../", "/./", "//", "\\\\", "/root/",
            "/var/", "/tmp/", "/bin/", "/usr/", "/opt/", "/boot/", "/home/",
        ];

        for pattern in &suspicious_patterns {
            if path.contains(pattern) {
                warn!("Suspicious pattern in path: {} (pattern: {})", path, pattern);
                return Err(RoutingError::InvalidPath(format!(
                    "Suspicious pattern detected: {}",
                    pattern
                )));
            }
        }

        // Check path depth (prevent deeply nested paths)
        let depth = path.split('/').filter(|c| !c.is_empty()).count();
        if depth > 10 {
            warn!("Path depth exceeds limit: {} (depth: {})", path, depth);
            return Err(RoutingError::InvalidPath(format!(
                "Path depth {} exceeds maximum of 10",
                depth
            )));
        }

        Ok(())
    }

    /// Sanitize path by removing potentially dangerous characters
    /// Use this as a fallback for legacy paths
    pub fn sanitize(path: &str) -> String {
        path.chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
            .collect()
    }
}

/// HTTP method validator for routes
pub struct MethodValidator;

impl MethodValidator {
    /// Validate that a method is allowed for CID-based routes
    pub fn validate_cid_route(method: &Method, has_path: bool) -> RoutingResult<()> {
        match (method.as_str(), has_path) {
            // Root CID access - allow GET and HEAD for bundles/frontends
            ("GET" | "HEAD", false) => Ok(()),
            
            // CID with path - static assets or API calls
            ("GET" | "HEAD", true) => Ok(()),
            
            // API methods (POST, PUT, DELETE) only for paths
            ("POST" | "PUT" | "DELETE" | "PATCH", true) => Ok(()),
            
            // OPTIONS for CORS preflight
            ("OPTIONS", _) => Ok(()),
            
            _ => {
                warn!(
                    "Method {} not allowed for CID route (has_path: {})",
                    method, has_path
                );
                Err(RoutingError::MethodNotAllowed(format!(
                    "Method {} not allowed for this route type",
                    method
                )))
            }
        }
    }

    /// Validate that a method is allowed for named app routes
    pub fn validate_app_route(method: &Method) -> RoutingResult<()> {
        // Named apps are typically APIs, allow more methods
        match method.as_str() {
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => Ok(()),
            _ => {
                warn!("Method {} not allowed for app route", method);
                Err(RoutingError::MethodNotAllowed(format!(
                    "Method {} not allowed for app routes",
                    method
                )))
            }
        }
    }

    /// Get allowed methods for a route type
    pub fn allowed_methods_cid(has_path: bool) -> HashSet<Method> {
        let mut methods = HashSet::new();
        methods.insert(Method::GET);
        methods.insert(Method::HEAD);
        methods.insert(Method::OPTIONS);
        
        if has_path {
            methods.insert(Method::POST);
            methods.insert(Method::PUT);
            methods.insert(Method::DELETE);
            methods.insert(Method::PATCH);
        }
        
        methods
    }

    /// Get allowed methods for app routes
    pub fn allowed_methods_app() -> HashSet<Method> {
        let mut methods = HashSet::new();
        methods.insert(Method::GET);
        methods.insert(Method::POST);
        methods.insert(Method::PUT);
        methods.insert(Method::DELETE);
        methods.insert(Method::PATCH);
        methods.insert(Method::HEAD);
        methods.insert(Method::OPTIONS);
        methods
    }
}

/// File extension validator
pub struct ExtensionValidator;

impl ExtensionValidator {
    /// Get the file extension from a path
    pub fn get_extension(path: &str) -> Option<String> {
        path.rfind('.')
            .and_then(|pos| path.get(pos + 1..))
            .map(|ext| ext.to_lowercase())
    }

    /// Check if an extension is allowed
    pub fn is_allowed(path: &str) -> bool {
        // If no extension, allow (could be an API endpoint)
        let ext = match Self::get_extension(path) {
            Some(e) => e,
            None => return true,
        };

        // Allowed extensions for web assets and WASM
        let allowed = [
            "html", "htm", "css", "js", "json", "wasm", "wat", "png", "jpg", "jpeg", "gif",
            "svg", "ico", "woff", "woff2", "ttf", "eot", "txt", "xml", "pdf", "webp", "mp4",
            "webm", "ogg", "mp3", "wav", "zip", "tar", "gz",
        ];

        allowed.contains(&ext.as_str())
    }

    /// Check if a path has a suspicious extension
    pub fn is_suspicious(path: &str) -> bool {
        let ext = match Self::get_extension(path) {
            Some(e) => e,
            None => return false,
        };

        // Dangerous extensions that should never be served
        let suspicious = [
            "php", "asp", "aspx", "jsp", "cgi", "pl", "py", "rb", "sh", "bash", "exe", "dll",
            "so", "dylib", "bat", "cmd", "vbs", "ps1",
        ];

        suspicious.contains(&ext.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_validation() {
        // Valid CIDs
        assert!(CIDValidator::validate("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").is_ok());
        assert!(CIDValidator::validate("brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka").is_ok());

        // Invalid CIDs
        assert!(CIDValidator::validate("abc123").is_err()); // Too short
        assert!(CIDValidator::validate("bafybeig").is_err()); // Too short
        assert!(CIDValidator::validate("aafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").is_err()); // Wrong prefix
        assert!(CIDValidator::validate("bafybeig_invalid_chars!@#").is_err()); // Invalid chars
    }

    #[test]
    fn test_path_normalization() {
        // Valid paths
        assert_eq!(PathSanitizer::normalize("/app.js").unwrap(), "app.js");
        assert_eq!(PathSanitizer::normalize("styles.css").unwrap(), "styles.css");
        assert_eq!(PathSanitizer::normalize("/api/users").unwrap(), "api/users");

        // Path with multiple slashes
        assert_eq!(PathSanitizer::normalize("//api///users//").unwrap(), "api/users");

        // Invalid paths (parent directory traversal)
        assert!(PathSanitizer::normalize("/../etc/passwd").is_err());
        
        // Absolute paths should succeed after normalization
        assert_eq!(PathSanitizer::normalize("/etc/shadow").unwrap(), "etc/shadow");
        
        // Null bytes should fail
        assert!(PathSanitizer::normalize("path/with/null\0byte").is_err());
    }

    #[test]
    fn test_url_encoding() {
        // URL-encoded spaces are decoded but then fail validation (spaces not allowed)
        assert!(PathSanitizer::normalize("/hello%20world.txt").is_err());
        
        // URL-encoded slashes are decoded successfully
        assert_eq!(PathSanitizer::normalize("/api%2Fusers").unwrap(), "api/users");
    }

    #[test]
    fn test_method_validation() {
        // Valid methods for CID routes
        assert!(MethodValidator::validate_cid_route(&Method::GET, false).is_ok());
        assert!(MethodValidator::validate_cid_route(&Method::GET, true).is_ok());
        assert!(MethodValidator::validate_cid_route(&Method::POST, true).is_ok());

        // Invalid methods
        assert!(MethodValidator::validate_cid_route(&Method::POST, false).is_err());
        assert!(MethodValidator::validate_cid_route(&Method::DELETE, false).is_err());
    }

    #[test]
    fn test_extension_validation() {
        // Allowed extensions
        assert!(ExtensionValidator::is_allowed("/app.js"));
        assert!(ExtensionValidator::is_allowed("/style.css"));
        assert!(ExtensionValidator::is_allowed("/module.wasm"));
        assert!(ExtensionValidator::is_allowed("/api")); // No extension

        // Suspicious extensions
        assert!(ExtensionValidator::is_suspicious("/script.php"));
        assert!(ExtensionValidator::is_suspicious("/file.exe"));
        assert!(ExtensionValidator::is_suspicious("/shell.sh"));
        assert!(!ExtensionValidator::is_suspicious("/app.js"));
    }

    #[test]
    fn test_path_depth_limit() {
        let deep_path = "/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z";
        assert!(PathSanitizer::normalize(deep_path).is_err());

        let normal_path = "/a/b/c/d/e";
        assert!(PathSanitizer::normalize(normal_path).is_ok());
    }

    #[test]
    fn test_hidden_files() {
        assert!(PathSanitizer::normalize("/.htaccess").is_err());
        assert!(PathSanitizer::normalize("/.env").is_err());
        assert!(PathSanitizer::normalize("/.git/config").is_err());
        assert!(PathSanitizer::normalize("/normal.txt").is_ok());
    }
}
