//! Conflict-free Replicated Data Types (CRDTs)
//! 
//! This module provides CRDT implementations for distributed state management
//! in the Pied Piper network. CRDTs enable eventual consistency across nodes
//! without requiring coordination.

mod types;
mod lww_map;
mod or_set;
mod sync;

pub use types::{Timestamp, Token, CrdtOperation};
pub use lww_map::LwwMap;
pub use or_set::OrSet;
pub use sync::{CrdtSync, CrdtSyncMessage, CrdtType};
