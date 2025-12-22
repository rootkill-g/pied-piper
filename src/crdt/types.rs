use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for CRDT operations
/// Combines a logical timestamp with a node ID for total ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp {
    /// Logical timestamp (typically system time in milliseconds)
    pub time: u64,
    /// Node identifier to break ties
    pub node_id: u64,
}

impl Timestamp {
    pub fn new(time: u64, node_id: u64) -> Self {
        Self { time, node_id }
    }

    /// Create a timestamp from the current system time
    pub fn now(node_id: u64) -> Self {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self { time, node_id }
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.time, self.node_id)
    }
}

/// Unique token for OR-Set operations
/// Each add operation gets a unique token to distinguish it from removes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token {
    /// Timestamp of the add operation
    pub timestamp: Timestamp,
    /// Sequence number for multiple adds in the same millisecond
    pub seq: u32,
}

impl Token {
    pub fn new(timestamp: Timestamp, seq: u32) -> Self {
        Self { timestamp, seq }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.timestamp, self.seq)
    }
}

/// Operation types for CRDT synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtOperation {
    /// LWW-Map set operation
    LwwMapSet {
        key: String,
        value: Vec<u8>,
        timestamp: Timestamp,
    },
    /// LWW-Map remove operation
    LwwMapRemove { key: String, timestamp: Timestamp },
    /// OR-Set add operation
    OrSetAdd {
        key: String,
        value: Vec<u8>,
        token: Token,
    },
    /// OR-Set remove operation
    OrSetRemove { key: String, tokens: Vec<Token> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = Timestamp::new(1000, 1);
        let ts2 = Timestamp::new(1000, 2);
        let ts3 = Timestamp::new(1001, 1);

        assert!(ts1 < ts2); // Same time, different node
        assert!(ts1 < ts3); // Different time
        assert!(ts2 < ts3);
    }

    #[test]
    fn test_timestamp_now() {
        let ts1 = Timestamp::now(1);
        let ts2 = Timestamp::now(1);

        assert!(ts2.time >= ts1.time);
    }

    #[test]
    fn test_token_creation() {
        let ts = Timestamp::new(1000, 1);
        let token = Token::new(ts, 0);

        assert_eq!(token.timestamp, ts);
        assert_eq!(token.seq, 0);
    }
}
