//! Network layer implementation using libp2p
//!
//! This module provides the peer-to-peer networking functionality including:
//! - Transport protocols (QUIC, TCP)
//! - Encryption (Noise)
//! - Multiplexing (Yamux)
//! - Peer discovery (mDNS, Kademlia DHT)
//! - Content routing
//! - Pub/Sub messaging

mod behaviour;
mod node;
mod transport;

pub use behaviour::PiedPiperBehaviour;
pub use node::{NetworkNode, NetworkNodeConfig};
