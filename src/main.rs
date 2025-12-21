mod cli;
mod network;

use anyhow::Result;
use cli::{Cli, Commands};
use network::{NetworkNode, NetworkNodeConfig};
use tracing::{error, info};
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Set up logging
    let log_level = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)))
        .init();

    info!("Pied Piper v{}", env!("CARGO_PKG_VERSION"));

    // Handle commands
    match cli.command {
        Commands::Daemon {
            tcp_port,
            quic_port,
            no_mdns,
            bootstrap,
            topic,
        } => {
            info!("Starting Pied Piper daemon");

            // Parse bootstrap peers
            let bootstrap_peers = parse_bootstrap_peers(&bootstrap)?;

            // Create node configuration
            let config = NetworkNodeConfig {
                tcp_port,
                quic_port,
                enable_mdns: !no_mdns,
                bootstrap_peers,
                topics: topic,
            };

            // Create and start the network node
            let mut node = NetworkNode::new(config).await?;

            info!("Local Peer ID: {}", node.local_peer_id());

            // Start listening
            let addrs = node.start_listening()?;
            info!("Listening on {} addresses", addrs.len());

            // Bootstrap DHT if we have bootstrap peers
            if !bootstrap.is_empty() {
                node.bootstrap_dht()?;
            }

            info!("Node is ready and running");

            // Run the event loop
            if let Err(e) = node.run().await {
                error!("Node error: {}", e);
                return Err(e);
            }
        }

        Commands::Info { endpoint } => {
            info!("Querying node at: {}", endpoint);
            println!("Node info endpoint: {}", endpoint);
            println!("(Info command not yet fully implemented - coming in Phase 3)");
        }

        Commands::Deploy { manifest } => {
            info!("Deploying application from: {:?}", manifest);
            println!("Deploy command will be implemented in Phase 3");
            println!("Manifest: {:?}", manifest);
        }
    }

    Ok(())
}

/// Parse bootstrap peer strings into (PeerId, Multiaddr) tuples
fn parse_bootstrap_peers(peers: &[String]) -> Result<Vec<(libp2p::PeerId, libp2p::Multiaddr)>> {
    use std::str::FromStr;

    let mut result = Vec::new();

    for peer_str in peers {
        // Expected format: peer_id@multiaddr or just multiaddr (if it includes /p2p/peer_id)
        if let Some((peer_id_str, addr_str)) = peer_str.split_once('@') {
            let peer_id = libp2p::PeerId::from_str(peer_id_str)?;
            let addr = libp2p::Multiaddr::from_str(addr_str)?;
            result.push((peer_id, addr));
        } else {
            // Try to parse as multiaddr and extract peer ID
            let addr = libp2p::Multiaddr::from_str(peer_str)?;

            // Extract peer ID from multiaddr if present
            if let Some(libp2p::multiaddr::Protocol::P2p(peer_id)) = addr
                .iter()
                .find(|p| matches!(p, libp2p::multiaddr::Protocol::P2p(_)))
            {
                result.push((peer_id, addr));
            } else {
                anyhow::bail!("Bootstrap peer address must include peer ID");
            }
        }
    }

    Ok(result)
}
