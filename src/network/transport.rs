// Transport configuration (placeholder for future enhancements)
// This will contain custom transport configurations and optimizations

use anyhow::Result;

/// Transport configuration options
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Enable QUIC transport
    pub enable_quic: bool,

    /// Enable TCP transport
    pub enable_tcp: bool,

    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            enable_quic: true,
            enable_tcp: true,
            connection_timeout: 30,
        }
    }
}
