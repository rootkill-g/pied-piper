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
mod command;
mod node;
mod transport;
mod kademlia_persistence;

pub use behaviour::PiedPiperBehaviour;
pub use command::NetworkCommand;
pub use node::{NetworkClient, NetworkNode, NetworkNodeConfig};
pub use kademlia_persistence::KademliaPersistence;
