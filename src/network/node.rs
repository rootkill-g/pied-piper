use anyhow::{Context, Result};
use libp2p::{
    futures::StreamExt, gossipsub, identify, kad, mdns, noise, ping, request_response,
    swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing::{debug, info, warn};

use super::behaviour::{PiedPiperBehaviour, PiedPiperEvent};
use crate::content::protocol::PROTOCOL_NAME;
use crate::content::{ModulePublisher, ModuleDiscovery, ModuleProvider};
use crate::wasm::loader::{ModuleInfo, ModuleCid, ModuleLoader};
use std::sync::Arc;

/// Configuration for the network node
#[derive(Debug, Clone)]
pub struct NetworkNodeConfig {
    /// TCP listening port (0 for random)
    pub tcp_port: u16,

    /// QUIC listening port (0 for random)
    pub quic_port: u16,

    /// Enable mDNS for local discovery
    pub enable_mdns: bool,

    /// Bootstrap peers for DHT
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,

    /// GossipSub topics to subscribe to
    pub topics: Vec<String>,
}

impl Default for NetworkNodeConfig {
    fn default() -> Self {
        Self {
            tcp_port: 0,
            quic_port: 0,
            enable_mdns: true,
            bootstrap_peers: vec![],
            topics: vec![],
        }
    }
}

/// Main network node handling all P2P communication
pub struct NetworkNode {
    swarm: Swarm<PiedPiperBehaviour>,
    config: NetworkNodeConfig,
    publisher: ModulePublisher,
    discovery: ModuleDiscovery,
    provider: ModuleProvider,
    loader: Arc<ModuleLoader>,
}

impl NetworkNode {
    /// Create a new network node with the given configuration
    pub async fn new(config: NetworkNodeConfig) -> Result<Self> {
        info!("Creating new network node");

        // Generate or load keypair (for now, generate new one)
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer ID: {}", local_peer_id);

        // Create behaviour first to handle errors properly
        let behaviour = Self::create_behaviour(&local_key, &config)?;

        // Build the swarm with transport
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_behaviour(|_key| behaviour)?
            .with_swarm_config(|cfg: libp2p::swarm::Config| {
                cfg.with_idle_connection_timeout(Duration::from_secs(60))
            })
            .build();

        // Create content distribution components
        let publisher = ModulePublisher::new(local_peer_id);
        let discovery = ModuleDiscovery::new();
        
        // Create a module loader for the provider
        let cache_dir = std::env::current_dir()?.join(".pied-piper").join("modules");
        let loader = Arc::new(ModuleLoader::new(cache_dir).await?);
        let provider = ModuleProvider::new(loader.clone());

        Ok(Self {
            swarm,
            config,
            publisher,
            discovery,
            provider,
            loader,
        })
    }

    /// Create the network behaviour with all protocols
    fn create_behaviour(
        key: &libp2p::identity::Keypair,
        config: &NetworkNodeConfig,
    ) -> Result<PiedPiperBehaviour> {
        let peer_id = PeerId::from(key.public());

        // Set up Kademlia DHT
        let mut kademlia_config = kad::Config::default();
        kademlia_config.set_query_timeout(Duration::from_secs(5 * 60));
        let store = kad::store::MemoryStore::new(peer_id);
        let mut kademlia = kad::Behaviour::with_config(peer_id, store, kademlia_config);

        // Add bootstrap peers to Kademlia
        for (peer_id, addr) in &config.bootstrap_peers {
            kademlia.add_address(peer_id, addr.clone());
        }

        // Set up mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        // Set up identify protocol
        let identify = identify::Behaviour::new(
            identify::Config::new("/pied-piper/1.0.0".to_string(), key.public())
                .with_agent_version(format!("pied-piper/{}", env!("CARGO_PKG_VERSION"))),
        );

        // Set up ping protocol
        let ping = ping::Behaviour::new(ping::Config::new());

        // Set up GossipSub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|message| {
                // Use message content hash as ID
                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            })
            .build()
            .context("Failed to build GossipSub config")?;

        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(key.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create GossipSub behaviour: {}", e))?;

        // Subscribe to configured topics
        for topic_name in &config.topics {
            let topic = gossipsub::IdentTopic::new(topic_name);
            gossipsub.subscribe(&topic)?;
            info!("Subscribed to topic: {}", topic_name);
        }
        
        // Set up request-response protocol for module distribution
        let content = request_response::Behaviour::new(
            [(PROTOCOL_NAME, request_response::ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        Ok(PiedPiperBehaviour {
            kademlia,
            mdns,
            identify,
            ping,
            gossipsub,
            content,
        })
    }

    /// Start listening on the configured ports
    pub fn start_listening(&mut self) -> Result<Vec<Multiaddr>> {
        let mut listening_addrs = Vec::new();

        // Listen on TCP
        let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", self.config.tcp_port)
            .parse()
            .context("Failed to parse TCP address")?;

        self.swarm.listen_on(tcp_addr.clone())?;
        info!("Listening on TCP: {}", tcp_addr);
        listening_addrs.push(tcp_addr);

        // Listen on QUIC
        let quic_addr: Multiaddr = format!("/ip4/0.0.0.0/udp/{}/quic-v1", self.config.quic_port)
            .parse()
            .context("Failed to parse QUIC address")?;

        self.swarm.listen_on(quic_addr.clone())?;
        info!("Listening on QUIC: {}", quic_addr);
        listening_addrs.push(quic_addr);

        Ok(listening_addrs)
    }

    /// Bootstrap the DHT
    pub fn bootstrap_dht(&mut self) -> Result<()> {
        info!("Bootstrapping DHT");
        self.swarm.behaviour_mut().kademlia.bootstrap()?;
        Ok(())
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Get the list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.swarm.connected_peers().copied().collect()
    }

    /// Publish a WebAssembly module to the network
    /// Returns the CID of the published module
    pub async fn publish_module(
        &mut self,
        module_bytes: Vec<u8>,
        name: Option<String>,
        version: Option<String>,
        author: Option<String>,
        description: Option<String>,
    ) -> Result<ModuleCid> {
        info!("Publishing module to network");

        // Calculate module CID
        let cid = ModuleCid::from_bytes(&module_bytes);
        
        // Create module info
        let module_info = ModuleInfo {
            cid: cid.clone(),
            name: name.clone(),
            version,
            size: module_bytes.len(),
            dependencies: vec![],
            author,
            description,
        };

        // Cache the module in the loader so the gateway can access it
        info!("Caching module {} locally", cid);
        let bytes_arc = Arc::new(module_bytes.clone());
        self.loader.add_to_cache(&cid, module_info.clone(), bytes_arc.clone()).await;

        // Store module in provider
        self.provider
            .provide_module(module_info.clone(), bytes_arc)
            .await?;

        // Create DHT record for module metadata
        let metadata_record = self
            .publisher
            .create_metadata_record(&module_info, &module_bytes)?;

        // Publish to DHT
        self.swarm
            .behaviour_mut()
            .kademlia
            .put_record(metadata_record, kad::Quorum::One)?;

        info!("Module {} published with CID: {}", 
            module_info.name.as_deref().unwrap_or("unnamed"), 
            cid
        );

        // Create and broadcast announcement via GossipSub
        let announcement = self
            .publisher
            .create_announcement_message(&module_info)?;

        let topic = gossipsub::IdentTopic::new(crate::content::publisher::MODULE_ANNOUNCEMENTS_TOPIC);
        
        // Subscribe to announcements topic if not already subscribed
        let _ = self.swarm.behaviour_mut().gossipsub.subscribe(&topic);
        
        // Publish announcement (non-fatal if no peers are subscribed)
        match self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, announcement) {
            Ok(_) => {
                info!("Module announcement published to network");
            }
            Err(e) => {
                // Log but don't fail - module is still stored locally and in DHT
                tracing::warn!("Failed to broadcast announcement (no peers available): {:?}", e);
                info!("Module is still stored locally and available for serving");
            }
        }

        Ok(cid)
    }

    /// Find a module by its CID using DHT
    /// Returns module metadata if found
    pub async fn find_module_by_cid(&mut self, cid: &ModuleCid) -> Result<Option<ModuleInfo>> {
        info!("Searching for module with CID: {}", cid);

        // Query DHT for module metadata
        let key = ModuleDiscovery::metadata_key(cid);
        let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);

        // Register the query
        self.discovery.register_dht_query(
            query_id,
            crate::content::discovery::QueryType::ModuleMetadata {
                cid: cid.to_string(),
            },
        );

        // In a real implementation, we'd wait for the query result asynchronously
        // For now, return None - the result will come through the event loop
        Ok(None)
    }

    /// Search for modules by name
    /// Returns list of matching modules
    pub async fn search_modules_by_name(&mut self, name: &str) -> Result<Vec<ModuleInfo>> {
        info!("Searching for modules with name: {}", name);

        // In a real implementation, we'd:
        // 1. Query DHT for name -> CID mapping
        // 2. Then fetch metadata for each CID
        // For now, return empty list - results will come through event loop

        Ok(vec![])
    }

    /// Fetch a module from the network by CID
    /// Returns the module bytes if found
    pub async fn fetch_module(&mut self, cid: &ModuleCid, peer: PeerId) -> Result<Option<Vec<u8>>> {
        info!("Fetching module {} from peer {}", cid, peer);

        // Create a module request
        let request = crate::content::protocol::ModuleRequest::GetModule {
            cid: cid.to_string(),
        };

        // Send request to peer
        let request_id = self
            .swarm
            .behaviour_mut()
            .content
            .send_request(&peer, request);

        // Register the request
        self.discovery.register_request(
            request_id,
            crate::content::discovery::QueryType::Providers {
                cid: cid.to_string(),
            },
        );

        // In a real implementation, we'd wait for the response asynchronously
        // For now, return None - the response will come through the event loop
        Ok(None)
    }

    /// Run the network event loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting network event loop");

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => {
                    self.handle_behaviour_event(event).await?;
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    num_established,
                    ..
                } => {
                    info!(
                        "Connection established with {} at {} (total: {})",
                        peer_id,
                        endpoint.get_remote_address(),
                        num_established
                    );
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    debug!("Connection closed with {}: {:?}", peer_id, cause);
                }
                SwarmEvent::IncomingConnection { .. } => {
                    debug!("Incoming connection");
                }
                SwarmEvent::IncomingConnectionError { error, .. } => {
                    warn!("Incoming connection error: {}", error);
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    warn!("Outgoing connection error to {:?}: {}", peer_id, error);
                }
                _ => {}
            }
        }
    }

    /// Handle behaviour-specific events
    async fn handle_behaviour_event(&mut self, event: PiedPiperEvent) -> Result<()> {
        match event {
            PiedPiperEvent::Kademlia(event) => {
                self.handle_kademlia_event(event).await?;
            }
            PiedPiperEvent::Mdns(event) => {
                self.handle_mdns_event(event).await?;
            }
            PiedPiperEvent::Identify(event) => {
                self.handle_identify_event(event).await?;
            }
            PiedPiperEvent::Ping(event) => {
                debug!("Ping event: {:?}", event);
            }
            PiedPiperEvent::Gossipsub(event) => {
                self.handle_gossipsub_event(event).await?;
            }
            PiedPiperEvent::Content(event) => {
                self.handle_content_event(event).await?;
            }
        }
        Ok(())
    }

    /// Handle Kademlia DHT events
    async fn handle_kademlia_event(&mut self, event: kad::Event) -> Result<()> {
        match event {
            kad::Event::RoutingUpdated {
                peer,
                is_new_peer,
                addresses,
                ..
            } => {
                if is_new_peer {
                    info!(
                        "New peer added to routing table: {} ({:?})",
                        peer, addresses
                    );
                }
            }
            kad::Event::OutboundQueryProgressed { result, .. } => {
                debug!("Kademlia query progressed: {:?}", result);
            }
            _ => {
                debug!("Kademlia event: {:?}", event);
            }
        }
        Ok(())
    }

    /// Handle mDNS discovery events
    async fn handle_mdns_event(&mut self, event: mdns::Event) -> Result<()> {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, multiaddr) in list {
                    info!("mDNS discovered peer: {} at {}", peer_id, multiaddr);

                    // Add discovered peer to Kademlia
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, multiaddr.clone());

                    // Dial the peer
                    if let Err(e) = self.swarm.dial(multiaddr) {
                        warn!("Failed to dial discovered peer {}: {}", peer_id, e);
                    }
                }
            }
            mdns::Event::Expired(list) => {
                for (peer_id, multiaddr) in list {
                    debug!("mDNS peer expired: {} at {}", peer_id, multiaddr);
                }
            }
        }
        Ok(())
    }

    /// Handle identify protocol events
    async fn handle_identify_event(&mut self, event: identify::Event) -> Result<()> {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                info!(
                    "Identified peer: {} - Agent: {} - Protocol: {}",
                    peer_id, info.agent_version, info.protocol_version
                );

                // Add peer's listen addresses to Kademlia
                for addr in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
            identify::Event::Sent { peer_id, .. } => {
                debug!("Sent identify info to peer: {}", peer_id);
            }
            identify::Event::Pushed { peer_id, .. } => {
                debug!("Pushed identify info to peer: {}", peer_id);
            }
            identify::Event::Error { peer_id, error, .. } => {
                warn!("Identify error with peer {}: {}", peer_id, error);
            }
        }
        Ok(())
    }

    /// Handle GossipSub events
    async fn handle_gossipsub_event(&mut self, event: gossipsub::Event) -> Result<()> {
        match event {
            gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            } => {
                info!(
                    "Received message from {}: {:?} (ID: {})",
                    propagation_source,
                    String::from_utf8_lossy(&message.data),
                    message_id
                );
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                info!("Peer {} subscribed to topic: {}", peer_id, topic);
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                info!("Peer {} unsubscribed from topic: {}", peer_id, topic);
            }
            _ => {
                debug!("GossipSub event: {:?}", event);
            }
        }
        Ok(())
    }
    
    /// Handle content distribution events
    async fn handle_content_event(&mut self, event: request_response::Event<crate::content::protocol::ModuleRequest, crate::content::protocol::ModuleResponse>) -> Result<()> {
        use request_response::{Message, Event as RREvent};
        
        match event {
            RREvent::Message { peer, message, connection_id: _ } => {
                match message {
                    Message::Request { request, channel, .. } => {
                        info!("Received module request from {}: {:?}", peer, request);
                        // Request handling will be done by ModuleProvider in the application layer
                        // For now, just log it
                    }
                    Message::Response { response, .. } => {
                        info!("Received module response from {}: {:?}", peer, response);
                        // Response handling will be done by the application layer
                    }
                }
            }
            RREvent::OutboundFailure { peer, request_id, error, .. } => {
                warn!("Outbound request {:?} to {} failed: {:?}", request_id, peer, error);
            }
            RREvent::InboundFailure { peer, error, .. } => {
                warn!("Inbound request from {} failed: {:?}", peer, error);
            }
            RREvent::ResponseSent { peer, .. } => {
                debug!("Response sent to {}", peer);
            }
        }
        Ok(())
    }
}