use anyhow::Result;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// Represents a peer stored in the DHT routing table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPeer {
    /// The peer's ID
    pub peer_id: String,
    /// The peer's multiaddrs
    pub addresses: Vec<String>,
    /// When this peer was last seen
    pub last_seen: u64,
}

/// Serializable DHT state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedDhtState {
    /// Version for future compatibility
    pub version: u32,
    /// Our local peer ID (for validation)
    pub local_peer_id: String,
    /// Peers in the routing table
    pub peers: Vec<PersistedPeer>,
    /// Timestamp of when this was saved
    pub saved_at: u64,
}

/// Manager for DHT persistence
pub struct KademliaPersistence {
    db_path: PathBuf,
}

impl KademliaPersistence {
    /// Create a new DHT persistence manager
    pub fn new(base_dir: &Path) -> Self {
        let db_path = base_dir.join("kademlia.json");
        Self { db_path }
    }

    /// Save the current DHT routing table state
    pub async fn save(&self, local_peer_id: PeerId, peers: Vec<(PeerId, Vec<libp2p::Multiaddr>)>) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let persisted_peers: Vec<PersistedPeer> = peers
            .iter()
            .map(|(peer_id, addresses)| PersistedPeer {
                peer_id: peer_id.to_string(),
                addresses: addresses.iter().map(|a| a.to_string()).collect(),
                last_seen: now,
            })
            .collect();

        let state = PersistedDhtState {
            version: 1,
            local_peer_id: local_peer_id.to_string(),
            peers: persisted_peers,
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&self.db_path, json).await?;
        debug!("Persisted DHT state with {} peers to {:?}", state.peers.len(), self.db_path);

        Ok(())
    }

    /// Load persisted DHT state
    pub async fn load(&self, local_peer_id: PeerId) -> Result<Vec<(PeerId, Vec<libp2p::Multiaddr>)>> {
        if !self.db_path.exists() {
            debug!("No persisted DHT state found at {:?}", self.db_path);
            return Ok(vec![]);
        }

        let json = fs::read_to_string(&self.db_path).await?;
        let state: PersistedDhtState = serde_json::from_str(&json)?;

        // Validate that this state is for our peer
        if state.local_peer_id != local_peer_id.to_string() {
            warn!(
                "Persisted DHT state is for different peer (stored: {}, current: {}). Skipping.",
                state.local_peer_id, local_peer_id
            );
            return Ok(vec![]);
        }

        // Reconstruct peer list
        let mut result = vec![];
        for persisted_peer in state.peers {
            match persisted_peer.peer_id.parse::<PeerId>() {
                Ok(peer_id) => {
                    let addresses: Result<Vec<libp2p::Multiaddr>, _> = persisted_peer
                        .addresses
                        .iter()
                        .map(|addr| addr.parse::<libp2p::Multiaddr>())
                        .collect();

                    match addresses {
                        Ok(addrs) => {
                            result.push((peer_id, addrs));
                        }
                        Err(e) => {
                            warn!("Failed to parse multiaddr for peer {}: {}", persisted_peer.peer_id, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse peer ID {}: {}", persisted_peer.peer_id, e);
                }
            }
        }

        debug!("Loaded {} peers from persisted DHT state", result.len());
        Ok(result)
    }

    /// Clear persisted DHT state
    pub async fn clear(&self) -> Result<()> {
        if self.db_path.exists() {
            fs::remove_file(&self.db_path).await?;
            debug!("Cleared persisted DHT state");
        }
        Ok(())
    }

    /// Get the path to the DHT database
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_and_load_dht_state() {
        let dir = tempdir().unwrap();
        let persistence = KademliaPersistence::new(dir.path());
        let peer_id = PeerId::random();

        let peers = vec![(
            PeerId::random(),
            vec!["/ip4/127.0.0.1/tcp/30333".parse().unwrap()],
        )];

        persistence.save(peer_id, peers.clone()).await.unwrap();
        let loaded = persistence.load(peer_id).await.unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].1.len(), 1);
    }

    #[tokio::test]
    async fn test_peer_id_mismatch() {
        let dir = tempdir().unwrap();
        let persistence = KademliaPersistence::new(dir.path());
        let peer_id_1 = PeerId::random();
        let peer_id_2 = PeerId::random();

        let peers = vec![(
            PeerId::random(),
            vec!["/ip4/127.0.0.1/tcp/30333".parse().unwrap()],
        )];

        persistence.save(peer_id_1, peers).await.unwrap();
        let loaded = persistence.load(peer_id_2).await.unwrap();

        // Should return empty list due to peer ID mismatch
        assert_eq!(loaded.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_dht_state() {
        let dir = tempdir().unwrap();
        let persistence = KademliaPersistence::new(dir.path());
        let peer_id = PeerId::random();

        let peers = vec![(
            PeerId::random(),
            vec!["/ip4/127.0.0.1/tcp/30333".parse().unwrap()],
        )];

        persistence.save(peer_id, peers).await.unwrap();
        assert!(persistence.db_path().exists());

        persistence.clear().await.unwrap();
        assert!(!persistence.db_path().exists());
    }
}
