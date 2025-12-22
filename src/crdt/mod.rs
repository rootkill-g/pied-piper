//! Conflict-free Replicated Data Types (CRDTs)
//!
//! This module provides CRDT implementations for distributed state management
//! in the Pied Piper network. CRDTs enable eventual consistency across nodes
//! without requiring coordination.

mod lww_map;
mod or_set;
mod sync;
mod types;

pub use lww_map::LwwMap;
pub use or_set::OrSet;
pub use sync::{CrdtSync, CrdtSyncMessage, CrdtType};
pub use types::{CrdtOperation, Timestamp, Token};
