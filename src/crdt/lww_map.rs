use super::types::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Last-Write-Wins Map (LWW-Map)
/// 
/// A CRDT that resolves conflicts using timestamps - the most recent write wins.
/// Each key is associated with a value and a timestamp.
/// During merge, the value with the highest timestamp is kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwMap {
    /// Current state: key -> (value, timestamp)
    entries: HashMap<String, (Vec<u8>, Timestamp)>,
    /// Node identifier for this replica
    node_id: u64,
}

impl LwwMap {
    /// Create a new LWW-Map with the given node ID
    pub fn new(node_id: u64) -> Self {
        Self {
            entries: HashMap::new(),
            node_id,
        }
    }

    /// Set a key-value pair with the current timestamp
    pub fn set(&mut self, key: String, value: Vec<u8>) {
        let timestamp = Timestamp::now(self.node_id);
        self.set_with_timestamp(key, value, timestamp);
    }

    /// Set a key-value pair with an explicit timestamp (for syncing)
    pub fn set_with_timestamp(&mut self, key: String, value: Vec<u8>, timestamp: Timestamp) {
        match self.entries.get(&key) {
            Some((_, existing_ts)) if *existing_ts > timestamp => {
                // Existing value is newer, keep it
                return;
            }
            _ => {
                // New value is newer or key doesn't exist
                self.entries.insert(key, (value, timestamp));
            }
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.entries.get(key).map(|(value, _)| value.as_slice())
    }

    /// Get a value with its timestamp
    pub fn get_with_timestamp(&self, key: &str) -> Option<(&[u8], Timestamp)> {
        self.entries.get(key).map(|(value, ts)| (value.as_slice(), *ts))
    }

    /// Remove a key with the current timestamp
    pub fn remove(&mut self, key: &str) {
        let timestamp = Timestamp::now(self.node_id);
        self.remove_with_timestamp(key, timestamp);
    }

    /// Remove a key with an explicit timestamp
    pub fn remove_with_timestamp(&mut self, key: &str, timestamp: Timestamp) {
        match self.entries.get(key) {
            Some((_, existing_ts)) if *existing_ts > timestamp => {
                // Existing value is newer, keep it
                return;
            }
            _ => {
                // Remove the key
                self.entries.remove(key);
            }
        }
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Merge with another LWW-Map (CRDT merge operation)
    /// For each key, keep the value with the highest timestamp
    pub fn merge(&mut self, other: &LwwMap) {
        for (key, (value, timestamp)) in &other.entries {
            self.set_with_timestamp(key.clone(), value.clone(), *timestamp);
        }
    }

    /// Get the raw entries (for serialization/sync)
    pub fn entries(&self) -> &HashMap<String, (Vec<u8>, Timestamp)> {
        &self.entries
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut map = LwwMap::new(1);
        
        map.set("key1".to_string(), b"value1".to_vec());
        assert_eq!(map.get("key1"), Some(b"value1".as_slice()));
        assert_eq!(map.len(), 1);
        
        map.set("key2".to_string(), b"value2".to_vec());
        assert_eq!(map.len(), 2);
        
        map.remove("key1");
        assert_eq!(map.get("key1"), None);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_timestamp_conflicts() {
        let mut map = LwwMap::new(1);
        
        let ts1 = Timestamp::new(1000, 1);
        let ts2 = Timestamp::new(2000, 1);
        
        // Add with earlier timestamp
        map.set_with_timestamp("key".to_string(), b"old".to_vec(), ts1);
        assert_eq!(map.get("key"), Some(b"old".as_slice()));
        
        // Add with later timestamp - should replace
        map.set_with_timestamp("key".to_string(), b"new".to_vec(), ts2);
        assert_eq!(map.get("key"), Some(b"new".as_slice()));
        
        // Try to add with earlier timestamp - should be ignored
        map.set_with_timestamp("key".to_string(), b"older".to_vec(), ts1);
        assert_eq!(map.get("key"), Some(b"new".as_slice()));
    }

    #[test]
    fn test_merge() {
        let mut map1 = LwwMap::new(1);
        let mut map2 = LwwMap::new(2);
        
        let ts1 = Timestamp::new(1000, 1);
        let ts2 = Timestamp::new(2000, 2);
        let ts3 = Timestamp::new(1500, 1);
        
        map1.set_with_timestamp("a".to_string(), b"value_a1".to_vec(), ts1);
        map1.set_with_timestamp("b".to_string(), b"value_b1".to_vec(), ts3);
        
        map2.set_with_timestamp("a".to_string(), b"value_a2".to_vec(), ts2);
        map2.set_with_timestamp("c".to_string(), b"value_c2".to_vec(), ts2);
        
        // Merge map2 into map1
        map1.merge(&map2);
        
        // map1 should have:
        // - "a" -> "value_a2" (ts2 > ts1)
        // - "b" -> "value_b1" (only in map1)
        // - "c" -> "value_c2" (only in map2)
        assert_eq!(map1.get("a"), Some(b"value_a2".as_slice()));
        assert_eq!(map1.get("b"), Some(b"value_b1".as_slice()));
        assert_eq!(map1.get("c"), Some(b"value_c2".as_slice()));
        assert_eq!(map1.len(), 3);
    }

    #[test]
    fn test_merge_bidirectional() {
        let mut map1 = LwwMap::new(1);
        let mut map2 = LwwMap::new(2);
        
        let ts1 = Timestamp::new(1000, 1);
        let ts2 = Timestamp::new(2000, 2);
        
        map1.set_with_timestamp("key".to_string(), b"value1".to_vec(), ts1);
        map2.set_with_timestamp("key".to_string(), b"value2".to_vec(), ts2);
        
        // Merge both ways
        let mut merged1 = map1.clone();
        let mut merged2 = map2.clone();
        
        merged1.merge(&map2);
        merged2.merge(&map1);
        
        // Both should converge to the same state (ts2 wins)
        assert_eq!(merged1.get("key"), Some(b"value2".as_slice()));
        assert_eq!(merged2.get("key"), Some(b"value2".as_slice()));
    }

    #[test]
    fn test_serialization() {
        let mut map = LwwMap::new(1);
        map.set("key1".to_string(), b"value1".to_vec());
        map.set("key2".to_string(), b"value2".to_vec());
        
        // Serialize
        let serialized = bincode::serialize(&map).unwrap();
        
        // Deserialize
        let deserialized: LwwMap = bincode::deserialize(&serialized).unwrap();
        
        assert_eq!(deserialized.get("key1"), Some(b"value1".as_slice()));
        assert_eq!(deserialized.get("key2"), Some(b"value2".as_slice()));
        assert_eq!(deserialized.len(), 2);
    }
}
