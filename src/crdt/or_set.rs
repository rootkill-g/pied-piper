use super::types::{Timestamp, Token};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Observed-Remove Set (OR-Set)
/// 
/// A CRDT that allows add and remove operations to be concurrent.
/// Each add operation is tagged with a unique token.
/// An element is present in the set if it has at least one token that hasn't been removed.
/// Removes are explicit - they specify which tokens to remove.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrSet {
    /// Elements with their add tokens: element -> set of tokens
    added: HashMap<Vec<u8>, HashSet<Token>>,
    /// Elements with their remove tokens: element -> set of tokens
    removed: HashMap<Vec<u8>, HashSet<Token>>,
    /// Node identifier for this replica
    node_id: u64,
    /// Sequence counter for generating unique tokens
    seq_counter: u32,
}

impl OrSet {
    /// Create a new OR-Set with the given node ID
    pub fn new(node_id: u64) -> Self {
        Self {
            added: HashMap::new(),
            removed: HashMap::new(),
            node_id,
            seq_counter: 0,
        }
    }

    /// Add an element to the set
    /// Returns the token assigned to this add operation
    pub fn add(&mut self, element: Vec<u8>) -> Token {
        let timestamp = Timestamp::now(self.node_id);
        let token = Token::new(timestamp, self.seq_counter);
        self.seq_counter = self.seq_counter.wrapping_add(1);
        
        self.added
            .entry(element)
            .or_insert_with(HashSet::new)
            .insert(token.clone());
        
        token
    }

    /// Add an element with an explicit token (for syncing)
    pub fn add_with_token(&mut self, element: Vec<u8>, token: Token) {
        self.added
            .entry(element)
            .or_insert_with(HashSet::new)
            .insert(token);
    }

    /// Remove an element from the set
    /// This observes the current tokens and adds them to the removed set
    pub fn remove(&mut self, element: &[u8]) {
        if let Some(tokens) = self.added.get(element) {
            let removed_tokens = self.removed
                .entry(element.to_vec())
                .or_insert_with(HashSet::new);
            for token in tokens {
                removed_tokens.insert(token.clone());
            }
        }
    }

    /// Remove specific tokens for an element (for syncing)
    pub fn remove_tokens(&mut self, element: &[u8], tokens: &[Token]) {
        let removed_tokens = self.removed
            .entry(element.to_vec())
            .or_insert_with(HashSet::new);
        for token in tokens {
            removed_tokens.insert(token.clone());
        }
    }

    /// Check if an element is in the set
    /// An element is present if it has added tokens that haven't been removed
    pub fn contains(&self, element: &[u8]) -> bool {
        if let Some(add_tokens) = self.added.get(element) {
            if let Some(remove_tokens) = self.removed.get(element) {
                // Element is present if there are tokens that haven't been removed
                add_tokens.iter().any(|t| !remove_tokens.contains(t))
            } else {
                // No removes, so present if there are adds
                !add_tokens.is_empty()
            }
        } else {
            false
        }
    }

    /// Get all elements in the set
    pub fn elements(&self) -> Vec<Vec<u8>> {
        self.added
            .iter()
            .filter(|(element, add_tokens)| {
                if let Some(remove_tokens) = self.removed.get(*element) {
                    // Include if there are add tokens that haven't been removed
                    add_tokens.iter().any(|t| !remove_tokens.contains(t))
                } else {
                    // No removes, include if there are adds
                    !add_tokens.is_empty()
                }
            })
            .map(|(element, _)| element.clone())
            .collect()
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.added
            .iter()
            .filter(|(element, add_tokens)| {
                if let Some(remove_tokens) = self.removed.get(*element) {
                    add_tokens.iter().any(|t| !remove_tokens.contains(t))
                } else {
                    !add_tokens.is_empty()
                }
            })
            .count()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get tokens for an element
    pub fn get_tokens(&self, element: &[u8]) -> Option<HashSet<Token>> {
        if let Some(add_tokens) = self.added.get(element) {
            if let Some(remove_tokens) = self.removed.get(element) {
                // Return only the tokens that haven't been removed
                let active: HashSet<Token> = add_tokens
                    .iter()
                    .filter(|t| !remove_tokens.contains(t))
                    .cloned()
                    .collect();
                if active.is_empty() {
                    None
                } else {
                    Some(active)
                }
            } else {
                Some(add_tokens.clone())
            }
        } else {
            None
        }
    }

    /// Merge with another OR-Set (CRDT merge operation)
    /// Union all add and remove tokens
    pub fn merge(&mut self, other: &OrSet) {
        // Merge added tokens
        for (element, other_tokens) in &other.added {
            let tokens = self
                .added
                .entry(element.clone())
                .or_insert_with(HashSet::new);
            
            for token in other_tokens {
                tokens.insert(token.clone());
            }
        }
        
        // Merge removed tokens
        for (element, other_tokens) in &other.removed {
            let tokens = self
                .removed
                .entry(element.clone())
                .or_insert_with(HashSet::new);
            
            for token in other_tokens {
                tokens.insert(token.clone());
            }
        }
    }

    /// Get the raw element map (for serialization/sync)
    pub fn raw_elements(&self) -> &HashMap<Vec<u8>, HashSet<Token>> {
        &self.added
    }

    /// Clear all elements
    pub fn clear(&mut self) {
        self.added.clear();
        self.removed.clear();
        self.seq_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut set = OrSet::new(1);
        
        set.add(b"elem1".to_vec());
        set.add(b"elem2".to_vec());
        
        assert!(set.contains(b"elem1"));
        assert!(set.contains(b"elem2"));
        assert!(!set.contains(b"elem3"));
        assert_eq!(set.len(), 2);
        
        set.remove(b"elem1");
        assert!(!set.contains(b"elem1"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_concurrent_adds() {
        let mut set1 = OrSet::new(1);
        let mut set2 = OrSet::new(2);
        
        // Both nodes add the same element
        let token1 = set1.add(b"elem".to_vec());
        let token2 = set2.add(b"elem".to_vec());
        
        // Tokens should be different
        assert_ne!(token1, token2);
        
        // Merge
        set1.merge(&set2);
        
        // Element should have both tokens
        let tokens = set1.get_tokens(b"elem").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(tokens.contains(&token1));
        assert!(tokens.contains(&token2));
    }

    #[test]
    fn test_remove_after_merge() {
        let mut set1 = OrSet::new(1);
        let mut set2 = OrSet::new(2);
        
        // Node 1 adds element
        set1.add(b"elem".to_vec());
        
        // Node 2 doesn't have it yet
        assert!(!set2.contains(b"elem"));
        
        // Node 2 gets the add via merge
        set2.merge(&set1);
        assert!(set2.contains(b"elem"));
        
        // Node 2 removes it
        set2.remove(b"elem");
        assert!(!set2.contains(b"elem"));
        
        // Node 1 merges the remove
        set1.merge(&set2);
        assert!(!set1.contains(b"elem"));
    }

    #[test]
    fn test_add_wins_over_remove() {
        let mut set1 = OrSet::new(1);
        let mut set2 = OrSet::new(2);
        
        // Both start with the element
        let token = set1.add(b"elem".to_vec());
        set2.add_with_token(b"elem".to_vec(), token.clone());
        
        // Node 1 removes it
        set1.remove(b"elem");
        
        // Node 2 adds it again (concurrent with remove)
        let token2 = set2.add(b"elem".to_vec());
        
        // After merge, the new add should win
        set1.merge(&set2);
        set2.merge(&set1);
        
        // Both should have the element (because of the new token)
        assert!(set1.contains(b"elem"));
        assert!(set2.contains(b"elem"));
        
        // The old token should be gone, but new token should be present
        let tokens = set1.get_tokens(b"elem").unwrap();
        assert!(!tokens.contains(&token));
        assert!(tokens.contains(&token2));
    }

    #[test]
    fn test_merge_convergence() {
        let mut set1 = OrSet::new(1);
        let mut set2 = OrSet::new(2);
        
        set1.add(b"a".to_vec());
        set1.add(b"b".to_vec());
        
        set2.add(b"b".to_vec());
        set2.add(b"c".to_vec());
        
        // Merge both ways
        let mut merged1 = set1.clone();
        let mut merged2 = set2.clone();
        
        merged1.merge(&set2);
        merged2.merge(&set1);
        
        // Both should converge to same state
        assert_eq!(merged1.len(), merged2.len());
        assert!(merged1.contains(b"a"));
        assert!(merged1.contains(b"b"));
        assert!(merged1.contains(b"c"));
        assert!(merged2.contains(b"a"));
        assert!(merged2.contains(b"b"));
        assert!(merged2.contains(b"c"));
    }

    #[test]
    fn test_elements_iteration() {
        let mut set = OrSet::new(1);
        
        set.add(b"elem1".to_vec());
        set.add(b"elem2".to_vec());
        set.add(b"elem3".to_vec());
        
        let elements = set.elements();
        assert_eq!(elements.len(), 3);
        
        // Check all elements are present
        assert!(elements.contains(&b"elem1".to_vec()));
        assert!(elements.contains(&b"elem2".to_vec()));
        assert!(elements.contains(&b"elem3".to_vec()));
    }

    #[test]
    fn test_serialization() {
        let mut set = OrSet::new(1);
        set.add(b"elem1".to_vec());
        set.add(b"elem2".to_vec());
        
        // Serialize
        let serialized = bincode::serialize(&set).unwrap();
        
        // Deserialize
        let deserialized: OrSet = bincode::deserialize(&serialized).unwrap();
        
        assert!(deserialized.contains(b"elem1"));
        assert!(deserialized.contains(b"elem2"));
        assert_eq!(deserialized.len(), 2);
    }
}
