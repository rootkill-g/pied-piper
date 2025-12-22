mod bundle;
mod cli;
mod content;
mod crdt;
mod gateway;
mod manifest;
mod network;
mod wasm;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::{Cli, Commands};
use crate::gateway::{GatewayConfig, GatewayServer};
use crate::network::{NetworkNode, NetworkNodeConfig};
use crate::wasm::loader::ModuleCid;
use crate::wasm::{ModuleLoader, WasmRuntime, WasmRuntimeConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,pied_piper=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse_args();

    match cli.command {
        Commands::Daemon {
            tcp_port,
            quic_port,
            bootstrap,
            no_mdns,
            topic,
        } => {
            let config = NetworkNodeConfig {
                tcp_port,
                quic_port,
                enable_mdns: !no_mdns,
                bootstrap_peers: parse_peers(bootstrap)?,
                topics: topic,
                ..Default::default()
            };

            let (client, mut node) = NetworkNode::new(config).await?;
            let _ = node.start_listening()?; // Start listening before run
            let _ = node.bootstrap_dht()?;

            // Spawn network node in background task
            tokio::spawn(async move {
                if let Err(e) = node.run().await {
                    error!("Network node error: {}", e);
                }
            });

            info!("Daemon started. Peer ID: {}", client.local_peer_id());

            // Wait for signal
            signal::ctrl_c().await?;
            info!("Shutting down...");
        }

        Commands::Info { endpoint: _ } => {
            // Just start a node and print info then exit
            let config = NetworkNodeConfig::default();
            let (client, _node) = NetworkNode::new(config).await?;
            println!("Local Peer ID: {}", client.local_peer_id());
        }

        Commands::Deploy {
            manifest,
            assets,
            name,
            version,
            author,
            description,
        } => {
            use crate::bundle::AppBundle;

            let (module_path, manifest_meta) = if is_manifest_path(&manifest) {
                let content = std::fs::read_to_string(&manifest)
                    .with_context(|| format!("Failed to read manifest: {:?}", manifest))?;
                let parsed: AppManifest = serde_yaml::from_str(&content)
                    .context("Failed to parse manifest YAML")?;
                let module_path = resolve_manifest_module(&manifest, &parsed)?;
                (module_path, Some(parsed))
            } else {
                (manifest.clone(), None)
            };

            let meta_name = manifest_meta.as_ref().and_then(|m| m.name.clone());
            let meta_version = manifest_meta.as_ref().and_then(|m| m.version.clone());
            let meta_author = manifest_meta.as_ref().and_then(|m| m.author.clone());
            let meta_description = manifest_meta.as_ref().and_then(|m| m.description.clone());

            let name = name.or(meta_name).unwrap_or_else(|| "unnamed".to_string());
            let version = version.or(meta_version).unwrap_or_else(|| "0.1.0".to_string());
            let author = author.or(meta_author);
            let description = description.or(meta_description);

            // Create bundle if assets are provided
            let bytes = if let Some(assets_dir) = assets {
                info!("Creating bundle with assets from {:?}", assets_dir);
                let bundle = AppBundle::new(
                    &module_path,
                    Some(&assets_dir),
                    name.clone(),
                    version.clone(),
                )
                .await?;
                
                info!("Bundle created: {} assets, {} total bytes", 
                    bundle.assets.len(), 
                    bundle.metadata().total_size);
                
                bundle.to_bytes()?
            } else {
                // Just deploy WASM module
                std::fs::read(&module_path)
                    .with_context(|| format!("Failed to read file: {:?}", module_path))?
            };

            // We need a running node to deploy
            let config = NetworkNodeConfig::default();
            let (client, mut node) = NetworkNode::new(config.clone()).await?;
            // Listen on random port
            let _ = node.start_listening()?;
            let _ = node.bootstrap_dht()?;

            // Spawn node
            tokio::spawn(async move {
                node.run().await.unwrap();
            });

            info!("Deploying module from {:?}", module_path);
            let cid = client
                .publish_module(bytes, Some(name), Some(version), author, description)
                .await?;

            println!("Module deployed successfully!");
            println!("CID: {}", cid);
        }

        Commands::Search { name, timeout } => {
            let config = NetworkNodeConfig::default();
            let (client, mut node) = NetworkNode::new(config).await?;
            let _ = node.start_listening()?;
            let _ = node.bootstrap_dht()?;

            tokio::spawn(async move {
                node.run().await.unwrap();
            });

            info!("Searching for '{}'", name);
            let results = match tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                client.search_modules_by_name(&name),
            )
            .await
            {
                Ok(results) => results?,
                Err(_) => {
                    println!("Search timed out after {} seconds.", timeout);
                    return Ok(());
                }
            };

            if results.is_empty() {
                println!("No modules found.");
            } else {
                println!("Found {} modules:", results.len());
                for module in results {
                    println!(
                        "- {} @ {} (CID: {})",
                        module.name.as_deref().unwrap_or("unnamed"),
                        module.version.as_deref().unwrap_or("unknown"),
                        module.cid
                    );
                }
            }
        }

        Commands::Run {
            module,
            function,
            max_memory,
            max_time,
            fuel,
        } => {
            // Check if input is a local file
            let path = module.clone();
            if path.exists() {
                let bytes = std::fs::read(&path)?;
                run_wasm_bytes(bytes, function, vec![], max_memory, max_time, fuel).await?;
            } else {
                // Treat as CID and fetch from network
                let config = NetworkNodeConfig::default();
                let (client, mut node) = NetworkNode::new(config).await?;
                let _ = node.start_listening()?;
                let _ = node.bootstrap_dht()?;

                tokio::spawn(async move {
                    node.run().await.unwrap();
                });

                info!("Fetching module {}...", module.display());
                let cid = ModuleCid::new(module.to_string_lossy().to_string());

                // For fetch, we first need to find providers
                if let Some(metadata) = client.find_module_by_cid(&cid).await? {
                    info!("Found metadata for {}", module.display());

                    let mut fetched = false;
                    for provider in metadata.providers {
                        if let Ok(peer_id) = provider.parse() {
                            info!("Fetching from {}", peer_id);
                            if let Ok(Some(bytes)) = client.fetch_module(&cid, peer_id).await {
                                run_wasm_bytes(
                                    bytes,
                                    function.clone(),
                                    vec![],
                                    max_memory,
                                    max_time,
                                    fuel,
                                )
                                .await?;
                                fetched = true;
                                break;
                            }
                        }
                    }
                    if !fetched {
                        anyhow::bail!("Failed to fetch module from any provider");
                    }
                } else {
                    anyhow::bail!("Module not found in network");
                }
            }
        }

        Commands::Gateway {
            listen,
            https_listen,
            tls,
            tls_cert,
            tls_key,
            tcp_port,
            quic_port,
            bootstrap,
            cors: _,
            timeout: _,
        } => {
            let addr: SocketAddr = listen
                .parse()
                .context("Invalid listen address, expected host:port")?;
            let port = addr.port();
            
            // Parse HTTPS address if TLS is enabled
            let https_port = if tls {
                let https_addr: SocketAddr = https_listen
                    .parse()
                    .context("Invalid HTTPS listen address, expected host:port")?;
                Some(https_addr.port())
            } else {
                None
            };
            
            // Setup TLS configuration if enabled
            let tls_config = if tls {
                use crate::gateway::{TlsConfig, ensure_cert_dir};
                
                let cert_dir = ensure_cert_dir()?;
                
                let cert_path = tls_cert
                    .unwrap_or_else(|| cert_dir.join("cert.pem"));
                let key_path = tls_key
                    .unwrap_or_else(|| cert_dir.join("key.pem"));
                
                if !cert_path.exists() || !key_path.exists() {
                    error!("TLS certificate or key not found!");
                    error!("Expected: {:?} and {:?}", cert_path, key_path);
                    error!("");
                    error!("To generate a self-signed certificate for testing:");
                    error!("  mkdir -p {:?}", cert_dir);
                    error!("  openssl req -x509 -newkey rsa:4096 -nodes \\");
                    error!("    -keyout {:?} \\", key_path);
                    error!("    -out {:?} \\", cert_path);
                    error!("    -days 365 -subj '/CN=localhost'");
                    error!("");
                    anyhow::bail!("TLS files not found");
                }
                
                Some(TlsConfig::new(cert_path, key_path))
            } else {
                None
            };
            
            let config = NetworkNodeConfig {
                tcp_port,
                quic_port,
                bootstrap_peers: parse_peers(bootstrap)?,
                ..Default::default()
            };

            let (client, mut node) = NetworkNode::new(config).await?;
            let _ = node.start_listening()?;
            let _ = node.bootstrap_dht()?;

            tokio::spawn(async move {
                if let Err(e) = node.run().await {
                    error!("P2P node error: {}", e);
                }
            });

            let cache_dir = std::env::current_dir()?.join(".pied-piper").join("modules");
            let loader = Arc::new(ModuleLoader::new(cache_dir).await?);

            let gateway_config = GatewayConfig {
                port,
                https_port,
                index_file: "index.html".to_string(),
                tls_config,
            };

            let server = GatewayServer::new(gateway_config, client, loader);
            server.start().await?;
        }
    }

    Ok(())
}

fn parse_peers(peers: Vec<String>) -> Result<Vec<(libp2p::PeerId, libp2p::Multiaddr)>> {
    let mut parsed = Vec::new();
    for peer in peers {
        // Simple manual parsing or use Multiaddr parsing if it contains peer ID
        // For now assume format /ip4/.../tcp/.../p2p/PEER_ID
        let multiaddr: libp2p::Multiaddr = peer.parse()?;
        if let Some(libp2p::multiaddr::Protocol::P2p(peer_id)) = multiaddr.iter().last() {
            parsed.push((peer_id, multiaddr));
        } else {
            // Warn or skip
        }
    }
    Ok(parsed)
}

async fn run_wasm_bytes(
    bytes: Vec<u8>,
    function: Option<String>,
    _args: Vec<String>,
    max_memory: usize,
    max_time: u64,
    fuel: bool,
) -> Result<()> {
    let config = WasmRuntimeConfig {
        enable_wasi: true,
        max_memory_bytes: max_memory * 1024 * 1024,
        max_execution_time: std::time::Duration::from_secs(max_time),
        enable_fuel: fuel,
        ..Default::default()
    };

    let runtime = WasmRuntime::new(config)?;

    // Check if component or module
    let is_component =
        bytes.len() >= 8 && bytes[0..4] == [0x00, 0x61, 0x73, 0x6d] && bytes[4] == 0x0d;

    if is_component {
        warn!("Running WASM components via CLI is limited. Use gateway for APIs.");
        let component = runtime.load_component(&bytes)?;
        let mut store = runtime.create_store()?;
        // CLI execution of components is complex (Command vs Reactor)
        // For now, we only support specific command exports or _start
        runtime
            .execute_component_command(&mut store, &component)
            .await?;
    } else {
        let module = runtime.load_module(&bytes)?;
        let mut store = runtime.create_store()?;
        let instance = runtime.instantiate_with_wasi(&mut store, &module).await?;

        let func_name = function.as_deref().unwrap_or("_start");
        runtime
            .execute_function(&mut store, &instance, func_name, &[])
            .await?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct AppManifest {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
    module: Option<String>,
    runtime: Option<AppRuntime>,
}

#[derive(Debug, Deserialize)]
struct AppRuntime {
    backend: Option<AppBackend>,
}

#[derive(Debug, Deserialize)]
struct AppBackend {
    module: Option<String>,
}

fn is_manifest_path(path: &PathBuf) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml") | Some("yml")
    )
}

fn resolve_manifest_module(path: &PathBuf, manifest: &AppManifest) -> Result<PathBuf> {
    let module = manifest
        .module
        .as_ref()
        .cloned()
        .or_else(|| manifest.runtime.as_ref()?.backend.as_ref()?.module.clone())
        .ok_or_else(|| anyhow::anyhow!("Manifest missing module path"))?;

    let base_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    Ok(base_dir.join(module))
}
