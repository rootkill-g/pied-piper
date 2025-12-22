use super::{CrdtOperation, LwwMap, OrSet, Timestamp, Token};
use anyhow::Result;
use libp2p::gossipsub::{IdentTopic, Message, MessageId, TopicHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// CRDT synchronization manager
///
/// Manages distributed state synchronization using GossipSub for propagation.
/// Provides last-write-wins maps and observed-remove sets with automatic merging.
pub struct CrdtSync {
    /// Node identifier for this replica
    node_id: u64,
    /// LWW-Maps indexed by name
    lww_maps: Arc<RwLock<HashMap<String, LwwMap>>>,
    /// OR-Sets indexed by name
    or_sets: Arc<RwLock<HashMap<String, OrSet>>>,
    /// GossipSub topic for CRDT sync
    topic: IdentTopic,
}

/// Message format for CRDT synchronization over GossipSub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtSyncMessage {
    /// Source node ID
    pub node_id: u64,
    /// Target CRDT name (e.g., "user_data", "distributed_cache")
    pub crdt_name: String,
    /// Type of CRDT
    pub crdt_type: CrdtType,
    /// Operations to apply
    pub operations: Vec<CrdtOperation>,
}

/// Type of CRDT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtType {
    LwwMap,
    OrSet,
}

impl CrdtSync {
    /// Create a new CRDT sync manager
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            lww_maps: Arc::new(RwLock::new(HashMap::new())),
            or_sets: Arc::new(RwLock::new(HashMap::new())),
            topic: IdentTopic::new("pied-piper-crdt-sync"),
        }
    }

    /// Get the GossipSub topic for CRDT sync
    pub fn topic(&self) -> &IdentTopic {
        &self.topic
    }

    /// Get or create an LWW-Map with the given name
    pub async fn get_lww_map(&self, name: &str) -> LwwMap {
        let maps = self.lww_maps.read().await;
        if let Some(map) = maps.get(name) {
            map.clone()
        } else {
            drop(maps);
            let mut maps = self.lww_maps.write().await;
            maps.entry(name.to_string())
                .or_insert_with(|| LwwMap::new(self.node_id))
                .clone()
        }
    }

    /// Get or create an OR-Set with the given name
    pub async fn get_or_set(&self, name: &str) -> OrSet {
        let sets = self.or_sets.read().await;
        if let Some(set) = sets.get(name) {
            set.clone()
        } else {
            drop(sets);
            let mut sets = self.or_sets.write().await;
            sets.entry(name.to_string())
                .or_insert_with(|| OrSet::new(self.node_id))
                .clone()
        }
    }

    /// Set a value in an LWW-Map and return the operation for broadcasting
    pub async fn lww_map_set(
        &self,
        name: &str,
        key: String,
        value: Vec<u8>,
    ) -> Result<CrdtOperation> {
        let mut maps = self.lww_maps.write().await;
        let map = maps
            .entry(name.to_string())
            .or_insert_with(|| LwwMap::new(self.node_id));

        let timestamp = Timestamp::now(self.node_id);
        map.set_with_timestamp(key.clone(), value.clone(), timestamp);

        Ok(CrdtOperation::LwwMapSet {
            key,
            value,
            timestamp,
        })
    }

    /// Remove a key from an LWW-Map and return the operation for broadcasting
    pub async fn lww_map_remove(&self, name: &str, key: &str) -> Result<CrdtOperation> {
        let mut maps = self.lww_maps.write().await;
        let map = maps
            .entry(name.to_string())
            .or_insert_with(|| LwwMap::new(self.node_id));

        let timestamp = Timestamp::now(self.node_id);
        map.remove_with_timestamp(key, timestamp);

        Ok(CrdtOperation::LwwMapRemove {
            key: key.to_string(),
            timestamp,
        })
    }

    /// Get a value from an LWW-Map
    pub async fn lww_map_get(&self, name: &str, key: &str) -> Option<Vec<u8>> {
        let maps = self.lww_maps.read().await;
        maps.get(name)
            .and_then(|map| map.get(key).map(|v| v.to_vec()))
    }

    /// Get all keys from an LWW-Map
    pub async fn lww_map_keys(&self, name: &str) -> Vec<String> {
        let maps = self.lww_maps.read().await;
        maps.get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the number of entries in an LWW-Map
    pub async fn lww_map_len(&self, name: &str) -> usize {
        let maps = self.lww_maps.read().await;
        maps.get(name).map(|map| map.len()).unwrap_or(0)
    }

    /// Add an element to an OR-Set and return the operation for broadcasting
    pub async fn or_set_add(&self, name: &str, element: Vec<u8>) -> Result<CrdtOperation> {
        let mut sets = self.or_sets.write().await;
        let set = sets
            .entry(name.to_string())
            .or_insert_with(|| OrSet::new(self.node_id));

        let token = set.add(element.clone());

        Ok(CrdtOperation::OrSetAdd {
            key: name.to_string(),
            value: element,
            token,
        })
    }

    /// Remove an element from an OR-Set and return the operation for broadcasting
    pub async fn or_set_remove(&self, name: &str, element: &[u8]) -> Result<CrdtOperation> {
        let mut sets = self.or_sets.write().await;
        let set = sets
            .entry(name.to_string())
            .or_insert_with(|| OrSet::new(self.node_id));

        // Get the tokens for this element before removing
        let tokens = set.get_tokens(element).unwrap_or_default();
        set.remove(element);

        Ok(CrdtOperation::OrSetRemove {
            key: name.to_string(),
            tokens: tokens.into_iter().collect(),
        })
    }

    /// Check if an element is in an OR-Set
    pub async fn or_set_contains(&self, name: &str, element: &[u8]) -> bool {
        let sets = self.or_sets.read().await;
        sets.get(name)
            .map(|set| set.contains(element))
            .unwrap_or(false)
    }

    /// Get all elements from an OR-Set
    pub async fn or_set_elements(&self, name: &str) -> Vec<Vec<u8>> {
        let sets = self.or_sets.read().await;
        sets.get(name).map(|set| set.elements()).unwrap_or_default()
    }

    /// Get the number of elements in an OR-Set
    pub async fn or_set_len(&self, name: &str) -> usize {
        let sets = self.or_sets.read().await;
        sets.get(name).map(|set| set.len()).unwrap_or(0)
    }

    /// Apply a CRDT operation (received from network)
    pub async fn apply_operation(
        &self,
        crdt_name: &str,
        crdt_type: CrdtType,
        operation: CrdtOperation,
    ) -> Result<()> {
        match (crdt_type, operation) {
            (
                CrdtType::LwwMap,
                CrdtOperation::LwwMapSet {
                    key,
                    value,
                    timestamp,
                },
            ) => {
                let mut maps = self.lww_maps.write().await;
                let map = maps
                    .entry(crdt_name.to_string())
                    .or_insert_with(|| LwwMap::new(self.node_id));
                map.set_with_timestamp(key, value, timestamp);
            }
            (CrdtType::LwwMap, CrdtOperation::LwwMapRemove { key, timestamp }) => {
                let mut maps = self.lww_maps.write().await;
                let map = maps
                    .entry(crdt_name.to_string())
                    .or_insert_with(|| LwwMap::new(self.node_id));
                map.remove_with_timestamp(&key, timestamp);
            }
            (CrdtType::OrSet, CrdtOperation::OrSetAdd { value, token, .. }) => {
                let mut sets = self.or_sets.write().await;
                let set = sets
                    .entry(crdt_name.to_string())
                    .or_insert_with(|| OrSet::new(self.node_id));
                set.add_with_token(value, token);
            }
            (CrdtType::OrSet, CrdtOperation::OrSetRemove { tokens, .. }) => {
                let mut sets = self.or_sets.write().await;
                let set = sets
                    .entry(crdt_name.to_string())
                    .or_insert_with(|| OrSet::new(self.node_id));
                // For remove, we need to track which elements these tokens belong to
                // This is a simplification - in a real implementation, we'd need better tracking
                for element in set.elements() {
                    set.remove_tokens(&element, &tokens);
                }
            }
            _ => {
                anyhow::bail!(
                    "CRDT type mismatch: {:?} does not match operation",
                    crdt_type
                );
            }
        }
        Ok(())
    }

    /// Handle incoming GossipSub message
    pub async fn handle_message(&self, message: &Message) -> Result<()> {
        let sync_msg: CrdtSyncMessage = bincode::deserialize(&message.data)?;

        // Ignore messages from self
        if sync_msg.node_id == self.node_id {
            return Ok(());
        }

        // Apply each operation
        for operation in sync_msg.operations {
            self.apply_operation(&sync_msg.crdt_name, sync_msg.crdt_type, operation)
                .await?;
        }

        Ok(())
    }

    /// Create a sync message for broadcasting
    pub fn create_sync_message(
        &self,
        crdt_name: String,
        crdt_type: CrdtType,
        operations: Vec<CrdtOperation>,
    ) -> CrdtSyncMessage {
        CrdtSyncMessage {
            node_id: self.node_id,
            crdt_name,
            crdt_type,
            operations,
        }
    }

    /// Serialize a sync message for GossipSub
    pub fn serialize_message(&self, message: &CrdtSyncMessage) -> Result<Vec<u8>> {
        Ok(bincode::serialize(message)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lww_map_operations() {
        let sync = CrdtSync::new(1);

        // Set a value
        let _op = sync
            .lww_map_set("test_map", "key1".to_string(), b"value1".to_vec())
            .await
            .unwrap();

        // Get the value
        let value = sync.lww_map_get("test_map", "key1").await;
        assert_eq!(value, Some(b"value1".to_vec()));

        // Update the value
        let _op = sync
            .lww_map_set("test_map", "key1".to_string(), b"value2".to_vec())
            .await
            .unwrap();
        let value = sync.lww_map_get("test_map", "key1").await;
        assert_eq!(value, Some(b"value2".to_vec()));

        // Remove the value
        let _op = sync.lww_map_remove("test_map", "key1").await.unwrap();
        let value = sync.lww_map_get("test_map", "key1").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_or_set_operations() {
        let sync = CrdtSync::new(1);

        // Add elements
        let _op = sync
            .or_set_add("test_set", b"elem1".to_vec())
            .await
            .unwrap();
        let _op = sync
            .or_set_add("test_set", b"elem2".to_vec())
            .await
            .unwrap();

        // Check containment
        assert!(sync.or_set_contains("test_set", b"elem1").await);
        assert!(sync.or_set_contains("test_set", b"elem2").await);
        assert!(!sync.or_set_contains("test_set", b"elem3").await);

        // Check length
        assert_eq!(sync.or_set_len("test_set").await, 2);

        // Remove element
        let _op = sync.or_set_remove("test_set", b"elem1").await.unwrap();
        assert!(!sync.or_set_contains("test_set", b"elem1").await);
        assert_eq!(sync.or_set_len("test_set").await, 1);
    }

    #[tokio::test]
    async fn test_lww_map_sync() {
        let sync1 = CrdtSync::new(1);
        let sync2 = CrdtSync::new(2);

        // Node 1 sets a value
        let op1 = sync1
            .lww_map_set("shared", "key".to_string(), b"value1".to_vec())
            .await
            .unwrap();

        // Node 2 applies the operation
        sync2
            .apply_operation("shared", CrdtType::LwwMap, op1)
            .await
            .unwrap();

        // Both should have the same value
        let value1 = sync1.lww_map_get("shared", "key").await;
        let value2 = sync2.lww_map_get("shared", "key").await;
        assert_eq!(value1, value2);
        assert_eq!(value1, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_or_set_sync() {
        let sync1 = CrdtSync::new(1);
        let sync2 = CrdtSync::new(2);

        // Node 1 adds an element
        let op1 = sync1.or_set_add("shared", b"elem".to_vec()).await.unwrap();

        // Node 2 applies the operation
        sync2
            .apply_operation("shared", CrdtType::OrSet, op1)
            .await
            .unwrap();

        // Both should have the element
        assert!(sync1.or_set_contains("shared", b"elem").await);
        assert!(sync2.or_set_contains("shared", b"elem").await);
    }
}
