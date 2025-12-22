/// Router for mapping URLs to content
///
/// This module handles:
/// - Route pattern matching
/// - Path parameter extraction
/// - Query string parsing
use std::collections::HashMap;

/// URL Router for content and API endpoints
pub struct Router {
    routes: HashMap<String, RouteHandler>,
}

/// Route handler function type
pub type RouteHandler = fn(&RouteContext) -> RouteResult;

/// Context passed to route handlers
pub struct RouteContext {
    pub path: String,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
}

/// Result returned from route handlers
pub enum RouteResult {
    Content(Vec<u8>, String), // (bytes, content_type)
    Redirect(String),
    NotFound,
    Error(String),
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a route handler
    pub fn route(&mut self, pattern: &str, handler: RouteHandler) {
        self.routes.insert(pattern.to_string(), handler);
    }

    /// Match a path to a route handler
    pub fn match_route(&self, path: &str) -> Option<&RouteHandler> {
        // Simple exact match for now
        // TODO: Implement pattern matching with wildcards
        self.routes.get(path)
    }

    /// Parse path parameters
    pub fn parse_params(pattern: &str, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        for (i, part) in pattern_parts.iter().enumerate() {
            if part.starts_with(':') {
                if let Some(value) = path_parts.get(i) {
                    let key = part[1..].to_string();
                    params.insert(key, value.to_string());
                }
            }
        }

        params
    }

    /// Parse query string
    pub fn parse_query(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    key.to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }

        params
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
