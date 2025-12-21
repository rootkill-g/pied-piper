use libp2p::{gossipsub, identify, kad, mdns, ping, request_response, swarm::NetworkBehaviour};
use crate::content::protocol::ContentProtocol;

/// The main network behaviour combining all libp2p protocols
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PiedPiperEvent")]
pub struct PiedPiperBehaviour {
    /// Kademlia DHT for content routing and peer discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

    /// mDNS for local peer discovery
    pub mdns: mdns::tokio::Behaviour,

    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,

    /// Ping protocol for connection keep-alive
    pub ping: ping::Behaviour,

    /// GossipSub for pub/sub messaging
    pub gossipsub: gossipsub::Behaviour,
    
    /// Request-Response protocol for module distribution
    pub content: ContentProtocol,
}

/// Events emitted by the network behaviour
#[derive(Debug)]
pub enum PiedPiperEvent {
    Kademlia(kad::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
    Ping(ping::Event),
    Gossipsub(gossipsub::Event),
    Content(request_response::Event<crate::content::protocol::ModuleRequest, crate::content::protocol::ModuleResponse>),
}

impl From<kad::Event> for PiedPiperEvent {
    fn from(event: kad::Event) -> Self {
        PiedPiperEvent::Kademlia(event)
    }
}

impl From<mdns::Event> for PiedPiperEvent {
    fn from(event: mdns::Event) -> Self {
        PiedPiperEvent::Mdns(event)
    }
}

impl From<identify::Event> for PiedPiperEvent {
    fn from(event: identify::Event) -> Self {
        PiedPiperEvent::Identify(event)
    }
}

impl From<ping::Event> for PiedPiperEvent {
    fn from(event: ping::Event) -> Self {
        PiedPiperEvent::Ping(event)
    }
}

impl From<gossipsub::Event> for PiedPiperEvent {
    fn from(event: gossipsub::Event) -> Self {
        PiedPiperEvent::Gossipsub(event)
    }
}

impl From<request_response::Event<crate::content::protocol::ModuleRequest, crate::content::protocol::ModuleResponse>> for PiedPiperEvent {
    fn from(event: request_response::Event<crate::content::protocol::ModuleRequest, crate::content::protocol::ModuleResponse>) -> Self {
        PiedPiperEvent::Content(event)
    }
}
