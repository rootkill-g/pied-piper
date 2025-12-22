use anyhow::{Context, Result};
use futures::future::join_all;
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, futures::StreamExt, gossipsub, identify, kad, mdns,
    noise, ping, request_response, swarm::SwarmEvent, tcp, yamux,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::fs;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use super::behaviour::{PiedPiperBehaviour, PiedPiperEvent};
use super::command::NetworkCommand;
use super::kademlia_persistence::KademliaPersistence;
use crate::content::protocol::PROTOCOL_NAME;
use crate::content::publisher::ModuleMetadata;
use crate::content::{ModuleDiscovery, ModuleProvider, ModulePublisher};
use crate::wasm::loader::{ModuleCid, ModuleInfo, ModuleLoader};
use std::sync::Arc;

/// Custom error type to signal shutdown
#[derive(Debug)]
struct ShutdownSignal;

impl std::fmt::Display for ShutdownSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shutdown signal received")
    }
}

impl std::error::Error for ShutdownSignal {}


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

/// Client handle for interacting with the NetworkNode service
#[derive(Clone)]
pub struct NetworkClient {
    command_tx: mpsc::Sender<NetworkCommand>,
    local_peer_id: PeerId,
}

impl NetworkClient {
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Publish a WebAssembly module to the network
    pub async fn publish_module(
        &self,
        module_bytes: Vec<u8>,
        name: Option<String>,
        version: Option<String>,
        author: Option<String>,
        description: Option<String>,
    ) -> Result<ModuleCid> {
        let cid = ModuleCid::from_bytes(&module_bytes);

        // Create module info
        let module_info = ModuleInfo {
            cid: cid.clone(),
            name,
            version,
            size: module_bytes.len(),
            dependencies: vec![],
            author,
            description,
        };

        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::ProvideModule {
                info: module_info,
                bytes: module_bytes,
                response: tx,
            })
            .await
            .context("Failed to send ProvideModule command")?;

        rx.await
            .context("Failed to receive ProvideModule response")?
    }

    /// Find a module by its CID using DHT
    /// Returns ModuleMetadata which includes provider list
    pub async fn find_module_by_cid(&self, cid: &ModuleCid) -> Result<Option<ModuleMetadata>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::FindModule {
                cid: cid.clone(),
                response: tx,
            })
            .await
            .context("Failed to send FindModule command")?;

        rx.await.context("Failed to receive FindModule response")?
    }

    /// Search for modules by name
    pub async fn search_modules_by_name(&self, name: &str) -> Result<Vec<ModuleMetadata>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::SearchModules {
                name: name.to_string(),
                response: tx,
            })
            .await
            .context("Failed to send SearchModules command")?;

        rx.await
            .context("Failed to receive SearchModules response")?
    }

    /// Fetch a module from the network by CID
    pub async fn fetch_module(&self, cid: &ModuleCid, peer_id: PeerId) -> Result<Option<Vec<u8>>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::FetchModule {
                cid: cid.clone(),
                peer_id,
                response: tx,
            })
            .await
            .context("Failed to send FetchModule command")?;

        rx.await.context("Failed to receive FetchModule response")?
    }

    /// Register a persistent name for a module
    pub async fn register_name(
        &self,
        name: String,
        cid: ModuleCid,
        version: Option<String>,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::RegisterName {
                name,
                cid,
                version,
                response: tx,
            })
            .await
            .context("Failed to send RegisterName command")?;

        rx.await
            .context("Failed to receive RegisterName response")?
    }

    /// Resolve a persistent name to a CID
    pub async fn resolve_name(&self, name: &str) -> Result<Option<String>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::ResolveName {
                name: name.to_string(),
                response: tx,
            })
            .await
            .context("Failed to send ResolveName command")?;

        rx.await.context("Failed to receive ResolveName response")?
    }

    /// Find all available versions of a module by name
    pub async fn find_versions(&self, name: &str) -> Result<Vec<ModuleMetadata>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::FindVersions {
                name: name.to_string(),
                response: tx,
            })
            .await
            .context("Failed to send FindVersions command")?;

        rx.await
            .context("Failed to receive FindVersions response")?
    }

    /// Find the best matching version for a requirement
    /// Requirement format: "^1.0.0", "~1.2.3", ">=2.0.0", "latest"
    pub async fn find_best_version(
        &self,
        name: &str,
        requirement: &str,
    ) -> Result<Option<ModuleMetadata>> {
        let (tx, rx) = oneshot::channel();

        self.command_tx
            .send(NetworkCommand::FindBestVersion {
                name: name.to_string(),
                requirement: requirement.to_string(),
                response: tx,
            })
            .await
            .context("Failed to send FindBestVersion command")?;

        rx.await
            .context("Failed to receive FindBestVersion response")?
    }
    
    /// Get the number of connected peers
    pub async fn peer_count(&self) -> usize {
        let (tx, rx) = oneshot::channel();

        // Send command and handle potential errors gracefully
        if let Err(_) = self.command_tx
            .send(NetworkCommand::GetPeerCount { response: tx })
            .await
        {
            return 0; // Return 0 if command channel is closed
        }

        rx.await.unwrap_or(0) // Return 0 if response fails
    }
    
    /// Shutdown the network node gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Sending shutdown signal to network node");
        self.command_tx
            .send(NetworkCommand::Shutdown)
            .await
            .context("Failed to send Shutdown command")?;
        Ok(())
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
    command_rx: mpsc::Receiver<NetworkCommand>,
    // Keep relay transport alive to prevent panic
    _relay_transport: libp2p::relay::client::Transport,
    // DHT persistence manager
    dht_persistence: KademliaPersistence,
}

impl NetworkNode {
    /// Create a new network node with the given configuration
    /// Returns a tuple of (Client, NodeService)
    pub async fn new(config: NetworkNodeConfig) -> Result<(NetworkClient, Self)> {
        info!("Creating new network node");

        // Generate or load keypair
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer ID: {}", local_peer_id);

        // Create behaviour and relay transport
        let (behaviour, relay_transport) = Self::create_behaviour(&local_key, &config)?;

        // Build the swarm
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

        let cache_dir = std::env::current_dir()?.join(".pied-piper").join("modules");
        let loader = Arc::new(ModuleLoader::new(cache_dir).await?);
        let provider = ModuleProvider::new(loader.clone());

        // Initialize DHT persistence
        let dht_dir = std::env::current_dir()?.join(".pied-piper");
        fs::create_dir_all(&dht_dir).await?;
        let dht_persistence = KademliaPersistence::new(&dht_dir);

        // Create command channel (size 32)
        let (command_tx, command_rx) = mpsc::channel(32);

        let client = NetworkClient {
            command_tx,
            local_peer_id,
        };

        let node = Self {
            swarm,
            config,
            publisher,
            discovery,
            provider,
            loader,
            command_rx,
            _relay_transport: relay_transport,
            dht_persistence,
        };

        Ok((client, node))
    }

    /// Create the network behaviour with all protocols
    /// Returns (Behaviour, RelayTransport) - transport must be kept alive
    fn create_behaviour(
        key: &libp2p::identity::Keypair,
        config: &NetworkNodeConfig,
    ) -> Result<(PiedPiperBehaviour, libp2p::relay::client::Transport)> {
        let peer_id = PeerId::from(key.public());

        // Set up Kademlia DHT
        let mut kademlia_config = kad::Config::default();
        kademlia_config.set_query_timeout(Duration::from_secs(5 * 60));
        let store = kad::store::MemoryStore::new(peer_id);
        let mut kademlia = kad::Behaviour::with_config(peer_id, store, kademlia_config);

        for (peer_id, addr) in &config.bootstrap_peers {
            kademlia.add_address(peer_id, addr.clone());
        }

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        let identify = identify::Behaviour::new(
            identify::Config::new("/pied-piper/1.0.0".to_string(), key.public())
                .with_agent_version(format!("pied-piper/{}", env!("CARGO_PKG_VERSION"))),
        );

        let ping = ping::Behaviour::new(ping::Config::new());

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|message| {
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

        for topic_name in &config.topics {
            let topic = gossipsub::IdentTopic::new(topic_name);
            gossipsub.subscribe(&topic)?;
        }

        let content = request_response::Behaviour::new(
            [(PROTOCOL_NAME, request_response::ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        // Initialize relay client for NAT traversal
        // The relay client new() function returns (Transport, Behaviour)
        let (relay_transport, relay) = libp2p::relay::client::new(peer_id);

        // Initialize DCUTR for hole-punching
        let dcutr = libp2p::dcutr::Behaviour::new(peer_id);

        let behaviour = PiedPiperBehaviour {
            kademlia,
            mdns,
            identify,
            ping,
            gossipsub,
            content,
            relay,
            dcutr,
        };

        Ok((behaviour, relay_transport))
    }

    /// Start listening on the configured ports
    pub fn start_listening(&mut self) -> Result<Vec<Multiaddr>> {
        let mut listening_addrs = Vec::new();

        let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", self.config.tcp_port)
            .parse()
            .context("Failed to parse TCP address")?;

        self.swarm.listen_on(tcp_addr.clone())?;
        info!("Listening on TCP: {}", tcp_addr);
        listening_addrs.push(tcp_addr);

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
        if let Err(err) = self.swarm.behaviour_mut().kademlia.bootstrap() {
            warn!("DHT bootstrap skipped: {}", err);
        }
        Ok(())
    }

    /// Run the network event loop
    pub async fn run(mut self) -> Result<()> {
        // Handle listening logic:
        // We assume start_listening was called before run, or we call it here.
        // But main.rs calls start_listening on "node" which is now gone.
        // Wait, main.rs calls start_listening on NetworkNodes.
        // My main.rs rewrite will call start_listening on the *service* struct?
        // No, run consumes self.
        // So main.rs needs to call start_listening on the service before consuming it.
        // So start_listening must remain on NetworkNode Service.

        info!("Starting network event loop");

        // Load persisted DHT state and add peers back
        let local_peer_id = self.swarm.local_peer_id().clone();
        match self.dht_persistence.load(local_peer_id).await {
            Ok(peers) => {
                debug!("Loaded {} peers from DHT persistence", peers.len());
                for (peer_id, addresses) in peers {
                    for addr in addresses {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to load persisted DHT state: {}", e);
            }
        }

        self.dial_bootstrap_peers();

        if let Err(err) = self.reprovide_cached_modules().await {
            warn!("Failed to re-provide cached modules: {}", err);
        }

        let mut bootstrap_interval = tokio::time::interval(Duration::from_secs(30));
        let mut replication_interval = tokio::time::interval(Duration::from_secs(300));
        let mut dht_persistence_interval = tokio::time::interval(Duration::from_secs(300)); // Save every 5 minutes

        loop {
            tokio::select! {
                _ = bootstrap_interval.tick() => {
                    self.dial_bootstrap_peers();
                }
                _ = replication_interval.tick() => {
                    if let Err(err) = self.reprovide_cached_modules().await {
                        warn!("Failed to re-provide cached modules: {}", err);
                    }
                }
                _ = dht_persistence_interval.tick() => {
                    // Save DHT state periodically (inline to avoid Send issues)
                    let local_peer_id = *self.swarm.local_peer_id();
                    let peers = vec![];  // Kademlia doesn't expose routing table directly
                    if let Err(err) = self.dht_persistence.save(local_peer_id, peers).await {
                        warn!("Failed to persist DHT state: {}", err);
                    }
                }
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(event) => {
                            if let Err(e) = self.handle_behaviour_event(event).await {
                                warn!("Error handling behaviour event: {}", e);
                            }
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            debug!("Connection established with {}", peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            debug!("Connection closed with {}", peer_id);
                        }
                        _ => {}
                    }
                }

                // Handle commands from client
                Some(command) = self.command_rx.recv() => {
                    if let Err(e) = self.handle_command(command).await {
                        // If shutdown command, break the loop and return Ok
                        if matches!(e.downcast_ref::<ShutdownSignal>(), Some(_)) {
                            info!("Network node shutting down gracefully");
                            return Ok(());
                        }
                        warn!("Error handling command: {}", e);
                    }
                }
            }
        }
    }

    /// Handle commands from the client
    async fn handle_command(&mut self, command: NetworkCommand) -> Result<()> {
        match command {
            NetworkCommand::ProvideModule {
                info,
                bytes,
                response,
            } => {
                let bytes_arc = Arc::new(bytes.clone());

                // 1. Cache locally
                self.loader
                    .add_to_cache(&info.cid, info.clone(), bytes_arc.clone())
                    .await;

                // 2. Add to Provider
                self.provider
                    .provide_module(info.clone(), bytes_arc)
                    .await?;

                // 3. Create metadata record
                let record = self.publisher.create_metadata_record(&info, &bytes)?;
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, kad::Quorum::One)?;

                // 3b. Publish provider record for content routing
                let provider_key = ModuleDiscovery::metadata_key(&info.cid);
                if let Err(err) = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(provider_key)
                {
                    warn!("Failed to start providing {}: {}", info.cid, err);
                }

                // 4. Create name record if applicable
                if let Some(name) = &info.name {
                    let latest_record =
                        self.publisher.create_latest_name_record(name, &info.cid)?;
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .put_record(latest_record, kad::Quorum::One)?;

                    if let Some(version) = &info.version {
                        let name_record = self
                            .publisher
                            .create_name_record(name, version, &info.cid)?;
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .put_record(name_record, kad::Quorum::One)?;
                    }
                }

                // 5. Announce via GossipSub
                if let Ok(msg) = self.publisher.create_announcement_message(&info) {
                    let topic = gossipsub::IdentTopic::new(
                        crate::content::publisher::MODULE_ANNOUNCEMENTS_TOPIC,
                    );
                    let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, msg);
                }

                let _ = response.send(Ok(info.cid));
            }

            NetworkCommand::FindModule { cid, response } => {
                let key = ModuleDiscovery::metadata_key(&cid);
                let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);

                use crate::content::discovery::ClientResponder;
                self.discovery.register_dht_query(
                    query_id,
                    crate::content::discovery::QueryType::ModuleMetadata {
                        cid: cid.to_string(),
                    },
                    Some(ClientResponder::Metadata(response)),
                );
            }

            NetworkCommand::SearchModules { name, response } => {
                let (module_name, version) = split_name_query(&name);
                if let Some(version) = version.as_deref() {
                    let key = ModuleDiscovery::name_key(&module_name, version);
                    let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);

                    use crate::content::discovery::ClientResponder;
                    self.discovery.register_dht_query(
                        query_id,
                        crate::content::discovery::QueryType::ModuleName {
                            name: module_name,
                            version: Some(version.to_string()),
                        },
                        Some(ClientResponder::Search(response)),
                    );
                } else {
                    let local_infos = self.provider.search_by_name(&module_name).await;
                    let local_peer = self.swarm.local_peer_id().to_string();
                    let mut local_metadata = Vec::new();
                    for info in local_infos {
                        local_metadata.push(ModuleMetadata {
                            cid: info.cid.to_string(),
                            name: info.name.clone(),
                            version: info.version.clone(),
                            size: info.size,
                            dependencies: info.dependencies.iter().map(|d| d.to_string()).collect(),
                            author: info.author.clone(),
                            description: info.description.clone(),
                            providers: vec![local_peer.clone()],
                            published_at: 0,
                        });
                    }

                    let peers: Vec<PeerId> = self.swarm.connected_peers().cloned().collect();

                    if peers.is_empty() {
                        let _ = response.send(Ok(local_metadata));
                        return Ok(());
                    }

                    let mut receivers = Vec::new();
                    for peer_id in peers {
                        let request = crate::content::protocol::ModuleRequest::SearchByName {
                            name: module_name.clone(),
                        };
                        let request_id = self
                            .swarm
                            .behaviour_mut()
                            .content
                            .send_request(&peer_id, request);

                        let (tx, rx) = oneshot::channel();
                        use crate::content::discovery::ClientResponder;
                        self.discovery.register_request(
                            request_id,
                            crate::content::discovery::QueryType::SearchByName {
                                name: module_name.clone(),
                            },
                            Some(ClientResponder::SearchResults(tx)),
                        );
                        receivers.push(rx);
                    }

                    tokio::spawn(async move {
                        let mut results = local_metadata;
                        let mut seen = std::collections::HashSet::new();
                        for meta in &results {
                            seen.insert(meta.cid.clone());
                        }

                        let wait_all = join_all(receivers);
                        let response_sets =
                            match tokio::time::timeout(Duration::from_secs(3), wait_all).await {
                                Ok(responses) => responses,
                                Err(_) => Vec::new(),
                            };

                        for response in response_sets {
                            let response = match response {
                                Ok(Ok(response)) => response,
                                _ => continue,
                            };

                            for module in response.results {
                                if !seen.insert(module.cid.clone()) {
                                    continue;
                                }

                                results.push(ModuleMetadata {
                                    cid: module.cid,
                                    name: module.name,
                                    version: module.version,
                                    size: 0,
                                    dependencies: vec![],
                                    author: None,
                                    description: module.description,
                                    providers: vec![response.peer_id.to_string()],
                                    published_at: 0,
                                });
                            }
                        }

                        let _ = response.send(Ok(results));
                    });
                }
            }

            NetworkCommand::FetchModule {
                cid,
                peer_id,
                response,
            } => {
                let request = crate::content::protocol::ModuleRequest::GetModule {
                    cid: cid.to_string(),
                };

                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .content
                    .send_request(&peer_id, request);

                use crate::content::discovery::ClientResponder;
                self.discovery.register_request(
                    request_id,
                    crate::content::discovery::QueryType::Providers {
                        cid: cid.to_string(),
                    },
                    Some(ClientResponder::Bytes(response)),
                );
            }

            NetworkCommand::RegisterName {
                name,
                cid,
                version,
                response,
            } => {
                info!("Registering persistent name: {} -> {}", name, cid);

                // Create persistent name registration record
                let record = self
                    .publisher
                    .register_persistent_name(&name, &cid, version)?;

                // Store in DHT with Quorum::One (will be replicated)
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, kad::Quorum::One)?;

                let _ = response.send(Ok(()));
            }

            NetworkCommand::ResolveName { name, response } => {
                info!("Resolving persistent name: {}", name);

                // Query DHT for persistent name
                let key = ModuleDiscovery::persistent_name_key(&name);
                let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);

                // Create a one-shot channel for the result
                let (tx, rx) = oneshot::channel();

                use crate::content::discovery::ClientResponder;
                self.discovery.register_dht_query(
                    query_id,
                    crate::content::discovery::QueryType::ModuleName {
                        name: name.clone(),
                        version: None,
                    },
                    Some(ClientResponder::Metadata(tx)),
                );

                // Spawn task to convert ModuleMetadata response to CID string
                tokio::spawn(async move {
                    match rx.await {
                        Ok(Ok(Some(metadata))) => {
                            let _ = response.send(Ok(Some(metadata.cid)));
                        }
                        Ok(Ok(None)) => {
                            let _ = response.send(Ok(None));
                        }
                        Ok(Err(e)) => {
                            let _ = response.send(Err(e));
                        }
                        Err(e) => {
                            let _ = response.send(Err(anyhow::anyhow!("Channel error: {}", e)));
                        }
                    }
                });
            }

            NetworkCommand::FindVersions { name, response } => {
                info!("Finding all versions for module: {}", name);

                // Search local provider first
                let local_infos = self.provider.search_by_name(&name).await;
                let local_peer = self.swarm.local_peer_id().to_string();
                let mut results = Vec::new();

                for info in local_infos {
                    results.push(ModuleMetadata {
                        cid: info.cid.to_string(),
                        name: info.name.clone(),
                        version: info.version.clone(),
                        size: info.size,
                        dependencies: info.dependencies.iter().map(|d| d.to_string()).collect(),
                        author: info.author.clone(),
                        description: info.description.clone(),
                        providers: vec![local_peer.clone()],
                        published_at: 0,
                    });
                }

                // TODO: Query network peers for additional versions
                // For now, return local results
                let _ = response.send(Ok(results));
            }

            NetworkCommand::FindBestVersion {
                name,
                requirement,
                response,
            } => {
                info!("Finding best version for {}: {}", name, requirement);

                // Search local provider
                let local_infos = self.provider.search_by_name(&name).await;
                let local_peer = self.swarm.local_peer_id().to_string();
                let mut modules = Vec::new();

                for info in local_infos {
                    modules.push(ModuleMetadata {
                        cid: info.cid.to_string(),
                        name: info.name.clone(),
                        version: info.version.clone(),
                        size: info.size,
                        dependencies: info.dependencies.iter().map(|d| d.to_string()).collect(),
                        author: info.author.clone(),
                        description: info.description.clone(),
                        providers: vec![local_peer.clone()],
                        published_at: 0,
                    });
                }

                // Handle "latest" special case
                if requirement == "latest" {
                    use crate::wasm::loader::version::find_latest;

                    let versions: Vec<String> =
                        modules.iter().filter_map(|m| m.version.clone()).collect();

                    if let Some(best_version) = find_latest(&versions) {
                        let best_module = modules
                            .into_iter()
                            .find(|m| m.version.as_deref() == Some(&best_version));
                        let _ = response.send(Ok(best_module));
                    } else {
                        let _ = response.send(Ok(None));
                    }
                } else {
                    // Use semver matching
                    use crate::wasm::loader::version::find_best_match;

                    let versions: Vec<String> =
                        modules.iter().filter_map(|m| m.version.clone()).collect();

                    match find_best_match(&versions, &requirement) {
                        Ok(Some(best_version)) => {
                            let best_module = modules
                                .into_iter()
                                .find(|m| m.version.as_deref() == Some(&best_version));
                            let _ = response.send(Ok(best_module));
                        }
                        Ok(None) => {
                            let _ = response.send(Ok(None));
                        }
                        Err(e) => {
                            let _ = response.send(Err(e));
                        }
                    }
                }
            }

            NetworkCommand::GetPeerId { response } => {
                let _ = response.send(*self.swarm.local_peer_id());
            }
            
            NetworkCommand::GetPeerCount { response } => {
                let peer_count = self.swarm.connected_peers().count();
                let _ = response.send(peer_count);
            }
            
            NetworkCommand::Shutdown => {
                info!("Received shutdown command");
                return Err(anyhow::Error::new(ShutdownSignal));
            }
        }
        Ok(())
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
            PiedPiperEvent::Relay(event) => {
                self.handle_relay_event(event).await?;
            }
            PiedPiperEvent::Dcutr(event) => {
                self.handle_dcutr_event(event).await?;
            }
        }
        Ok(())
    }

    /// Handle Kademlia DHT events
    async fn handle_kademlia_event(&mut self, event: kad::Event) -> Result<()> {
        match event {
            kad::Event::OutboundQueryProgressed { id, result, .. } => {
                use crate::content::discovery::ClientResponder;

                if let Some((query_type, responder)) = self.discovery.complete_dht_query(&id) {
                    match (query_type, result, responder) {
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))),
                            Some(ClientResponder::Metadata(tx)),
                        ) => {
                            if let Ok(metadata) =
                                self.discovery.parse_metadata(&record.record.value)
                            {
                                let _ = tx.send(Ok(Some(metadata)));
                            } else {
                                let _ =
                                    tx.send(Err(anyhow::anyhow!("Failed to parse metadata value")));
                            }
                        }
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))),
                            Some(ClientResponder::Search(tx)),
                        ) => {
                            if let Ok(metadata) =
                                self.discovery.parse_metadata(&record.record.value)
                            {
                                let _ = tx.send(Ok(vec![metadata]));
                            } else {
                                let _ =
                                    tx.send(Err(anyhow::anyhow!("Failed to parse metadata value")));
                            }
                        }
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Ok(
                                kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                            )),
                            Some(ClientResponder::Metadata(tx)),
                        ) => {
                            let _ = tx.send(Ok(None));
                        }
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Ok(
                                kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                            )),
                            Some(ClientResponder::Search(tx)),
                        ) => {
                            let _ = tx.send(Ok(vec![]));
                        }
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Err(e)),
                            Some(ClientResponder::Metadata(tx)),
                        ) => {
                            warn!("DHT Query failed: {:?}", e);
                            let _ = tx.send(Ok(None));
                        }
                        (
                            crate::content::discovery::QueryType::ModuleMetadata { .. },
                            kad::QueryResult::GetRecord(Err(e)),
                            Some(ClientResponder::Search(tx)),
                        ) => {
                            warn!("DHT Query failed: {:?}", e);
                            let _ = tx.send(Ok(vec![]));
                        }
                        (
                            crate::content::discovery::QueryType::ModuleName { .. },
                            kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))),
                            Some(ClientResponder::Search(tx)),
                        ) => match self.discovery.parse_cid(&record.record.value) {
                            Ok(cid) => {
                                let module_cid = ModuleCid::new(cid.clone());
                                let key = ModuleDiscovery::metadata_key(&module_cid);
                                let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);
                                self.discovery.register_dht_query(
                                    query_id,
                                    crate::content::discovery::QueryType::ModuleMetadata { cid },
                                    Some(ClientResponder::Search(tx)),
                                );
                            }
                            Err(err) => {
                                let _ = tx.send(Err(err));
                            }
                        },
                        (
                            crate::content::discovery::QueryType::ModuleName { .. },
                            kad::QueryResult::GetRecord(Ok(
                                kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                            )),
                            Some(ClientResponder::Search(tx)),
                        ) => {
                            let _ = tx.send(Ok(vec![]));
                        }
                        (
                            crate::content::discovery::QueryType::ModuleName { .. },
                            kad::QueryResult::GetRecord(Err(e)),
                            Some(ClientResponder::Search(tx)),
                        ) => {
                            warn!("DHT Query failed: {:?}", e);
                            let _ = tx.send(Ok(vec![]));
                        }
                        _ => {}
                    }
                }
            }
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
            _ => {}
        }
        Ok(())
    }

    /// Handle mDNS discovery events
    async fn handle_mdns_event(&mut self, event: mdns::Event) -> Result<()> {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, multiaddr) in list {
                    // Add discovered peer to Kademlia
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, multiaddr.clone());

                    // Dial the peer
                    if let Err(_) = self.swarm.dial(multiaddr) {
                        // ignore dial errors for mdns
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle identify protocol events
    async fn handle_identify_event(&mut self, event: identify::Event) -> Result<()> {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                info!(
                    "Identified peer: {} - Agent: {}",
                    peer_id, info.agent_version
                );

                // Add peer's listen addresses to Kademlia
                for addr in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
            _ => {}
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
            _ => {}
        }
        Ok(())
    }

    /// Handle content distribution events
    async fn handle_content_event(
        &mut self,
        event: request_response::Event<
            crate::content::protocol::ModuleRequest,
            crate::content::protocol::ModuleResponse,
        >,
    ) -> Result<()> {
        use request_response::{Event as RREvent, Message};

        match event {
            RREvent::Message { peer, message, .. } => match message {
                Message::Request {
                    request, channel, ..
                } => {
                    info!("Received module request from {}: {:?}", peer, request);
                    let response = self.provider.handle_request(request).await;
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .content
                        .send_response(channel, response);
                }
                Message::Response {
                    response,
                    request_id,
                } => {
                    use crate::content::discovery::ClientResponder;

                    info!("Received module response from {}: {:?}", peer, response);
                    if let Some((_query_type, responder)) =
                        self.discovery.complete_request(&request_id)
                    {
                        match (response, responder) {
                            (
                                crate::content::protocol::ModuleResponse::Module { bytes, .. },
                                Some(ClientResponder::Bytes(tx)),
                            ) => {
                                let _ = tx.send(Ok(Some(bytes)));
                            }
                            (
                                crate::content::protocol::ModuleResponse::NotFound { .. },
                                Some(ClientResponder::Bytes(tx)),
                            ) => {
                                let _ = tx.send(Ok(None));
                            }
                            (
                                crate::content::protocol::ModuleResponse::Error { message },
                                Some(ClientResponder::Bytes(tx)),
                            ) => {
                                let _ = tx.send(Err(anyhow::anyhow!("Remote error: {}", message)));
                            }
                            (
                                crate::content::protocol::ModuleResponse::SearchResults { modules },
                                Some(ClientResponder::SearchResults(tx)),
                            ) => {
                                let _ = tx.send(Ok(crate::content::discovery::SearchResponse {
                                    peer_id: peer,
                                    results: modules,
                                }));
                            }
                            _ => {}
                        }
                    }
                }
            },
            RREvent::OutboundFailure {
                request_id, error, ..
            } => {
                warn!("Outbound request failed: {:?}", error);
                use crate::content::discovery::ClientResponder;
                if let Some((_, responder)) = self.discovery.complete_request(&request_id) {
                    match responder {
                        Some(ClientResponder::Bytes(tx)) => {
                            let _ = tx.send(Err(anyhow::anyhow!("Outbound failure: {:?}", error)));
                        }
                        Some(ClientResponder::SearchResults(tx)) => {
                            let _ = tx.send(Err(anyhow::anyhow!("Outbound failure: {:?}", error)));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle Circuit Relay events
    async fn handle_relay_event(&mut self, event: libp2p::relay::client::Event) -> Result<()> {
        use libp2p::relay::client::Event;

        match event {
            Event::ReservationReqAccepted {
                relay_peer_id,
                renewal,
                limit,
            } => {
                if renewal {
                    info!("Relay reservation renewed with {}", relay_peer_id);
                } else {
                    info!(
                        "Relay reservation accepted with {} (limit: {:?})",
                        relay_peer_id, limit
                    );
                }
            }
            Event::OutboundCircuitEstablished {
                relay_peer_id,
                limit,
            } => {
                info!(
                    "Outbound circuit established via {} (limit: {:?})",
                    relay_peer_id, limit
                );
            }
            Event::InboundCircuitEstablished { src_peer_id, limit } => {
                info!(
                    "Inbound circuit established from {} (limit: {:?})",
                    src_peer_id, limit
                );
            }
        }
        Ok(())
    }

    /// Handle DCUTR (hole-punching) events
    async fn handle_dcutr_event(&mut self, event: libp2p::dcutr::Event) -> Result<()> {
        info!("DCUTR event: {:?}", event);
        Ok(())
    }

    fn dial_bootstrap_peers(&mut self) {
        for (peer_id, addr) in &self.config.bootstrap_peers {
            if self.swarm.is_connected(peer_id) {
                continue;
            }

            self.swarm
                .behaviour_mut()
                .kademlia
                .add_address(peer_id, addr.clone());

            if let Err(err) = self.swarm.dial(addr.clone()) {
                warn!(
                    "Failed to dial bootstrap peer {} at {}: {}",
                    peer_id, addr, err
                );
            }
        }
    }

    async fn reprovide_cached_modules(&mut self) -> Result<()> {
        let cached = self.loader.list_cached_modules().await?;
        if cached.is_empty() {
            return Ok(());
        }

        info!("Re-providing {} cached modules", cached.len());

        for cid in cached {
            let bytes = match self.loader.load_from_disk(&cid).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    warn!("Failed to load cached module {}: {}", cid, err);
                    continue;
                }
            };

            let info = match self.loader.load_module_info(&cid).await? {
                Some(info) => info,
                None => ModuleInfo {
                    cid: cid.clone(),
                    name: None,
                    version: None,
                    size: bytes.len(),
                    dependencies: vec![],
                    author: None,
                    description: None,
                },
            };

            self.loader
                .add_to_cache(&cid, info.clone(), bytes.clone())
                .await;
            self.provider
                .provide_module(info.clone(), bytes.clone())
                .await?;

            let record = self
                .publisher
                .create_metadata_record(&info, bytes.as_ref())?;
            self.swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, kad::Quorum::One)?;

            let provider_key = ModuleDiscovery::metadata_key(&info.cid);
            if let Err(err) = self
                .swarm
                .behaviour_mut()
                .kademlia
                .start_providing(provider_key)
            {
                warn!("Failed to start providing {}: {}", info.cid, err);
            }

            if let Some(name) = &info.name {
                let latest_record = self.publisher.create_latest_name_record(name, &info.cid)?;
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(latest_record, kad::Quorum::One)?;

                if let Some(version) = &info.version {
                    let name_record = self
                        .publisher
                        .create_name_record(name, version, &info.cid)?;
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .put_record(name_record, kad::Quorum::One)?;
                }
            }
        }

        Ok(())
    }
}

fn split_name_query(query: &str) -> (String, Option<String>) {
    let query = query.trim();
    if let Some((name, version)) = query.split_once('@') {
        return normalize_name_version(name, version, query);
    }
    if let Some((name, version)) = query.split_once(':') {
        return normalize_name_version(name, version, query);
    }
    (query.to_string(), None)
}

fn normalize_name_version(name: &str, version: &str, fallback: &str) -> (String, Option<String>) {
    let name = name.trim();
    let version = version.trim();
    if name.is_empty() {
        return (fallback.to_string(), None);
    }
    if version.is_empty() {
        return (name.to_string(), None);
    }
    (name.to_string(), Some(version.to_string()))
}
