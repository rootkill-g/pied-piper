mod bundle;
mod cli;
mod config;
mod content;
mod crdt;
mod gateway;
mod manifest;
mod metrics;
mod network;
mod package;
mod security;
mod storage;
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

use crate::cli::{Cli, Commands, ConfigAction, PackageAction};
use crate::config::PiedPiperConfig;
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
        Commands::Config { action } => {
            handle_config_command(action, cli.config.as_deref())?;
        }

        Commands::Package { action } => {
            handle_package_command(action).await?;
        }

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
                let parsed: AppManifest =
                    serde_yaml::from_str(&content).context("Failed to parse manifest YAML")?;
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
            let version = version
                .or(meta_version)
                .unwrap_or_else(|| "0.1.0".to_string());
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

                info!(
                    "Bundle created: {} assets, {} total bytes",
                    bundle.assets.len(),
                    bundle.metadata().total_size
                );

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

                let cert_path = tls_cert.unwrap_or_else(|| cert_dir.join("cert.pem"));
                let key_path = tls_key.unwrap_or_else(|| cert_dir.join("key.pem"));

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

            // Clone client for shutdown
            let shutdown_client = client.clone();

            // Spawn network node in background with graceful shutdown support
            let network_handle = tokio::spawn(async move {
                if let Err(e) = node.run().await {
                    error!("P2P node error: {}", e);
                }
            });

            let cache_dir = std::env::current_dir()?.join(".pied-piper").join("modules");
            let loader = Arc::new(ModuleLoader::new(cache_dir).await?);

            // Load security configuration (use defaults if config file not found)
            let security_config = if let Some(config_path) = cli.config.as_deref() {
                match PiedPiperConfig::load(Some(config_path)) {
                    Ok(cfg) => cfg.security,
                    Err(e) => {
                        warn!("Failed to load config, using defaults: {}", e);
                        crate::config::SecurityConfig::default()
                    }
                }
            } else {
                crate::config::SecurityConfig::default()
            };

            let gateway_config = GatewayConfig {
                port,
                https_port,
                index_file: "index.html".to_string(),
                tls_config,
                request_timeout_secs: 30,
            };

            let server = GatewayServer::new(gateway_config, client, loader)
                .with_security(security_config);
            
            // Set up graceful shutdown
            let shutdown_signal = async {
                signal::ctrl_c()
                    .await
                    .expect("Failed to install CTRL+C signal handler");
                info!("Received shutdown signal, gracefully shutting down...");
            };
            
            // Start server with shutdown signal
            tokio::select! {
                result = server.start() => {
                    if let Err(e) = result {
                        error!("Gateway server error: {}", e);
                    }
                }
                _ = shutdown_signal => {
                    info!("Shutdown signal received, stopping server...");
                }
            }
            
            // Signal network node to shut down
            info!("Shutting down network node...");
            if let Err(e) = shutdown_client.shutdown().await {
                warn!("Error shutting down network node: {}", e);
            }
            
            // Wait for network node to finish
            info!("Waiting for network node to complete...");
            let _ = network_handle.await;
            info!("Shutdown complete");
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

/// Handle configuration subcommands
fn handle_config_command(action: ConfigAction, _config_path: Option<&std::path::Path>) -> Result<()> {
    match action {
        ConfigAction::Init { output, format, force } => {
            if output.exists() && !force {
                anyhow::bail!(
                    "Config file '{}' already exists. Use --force to overwrite.",
                    output.display()
                );
            }

            let content = match format.as_str() {
                "yaml" | "yml" => PiedPiperConfig::example_yaml(),
                "toml" => PiedPiperConfig::example_toml(),
                "json" => serde_json::to_string_pretty(&PiedPiperConfig::default())?,
                _ => anyhow::bail!("Unsupported format '{}'. Use yaml, toml, or json.", format),
            };

            std::fs::write(&output, content)
                .with_context(|| format!("Failed to write config to {}", output.display()))?;

            info!("✅ Created example configuration file: {}", output.display());
            println!("Configuration file created: {}", output.display());
            println!("\nEdit the file to customize your settings, then run:");
            println!("  pied-piper --config {} gateway", output.display());
        }

        ConfigAction::Validate { config_file } => {
            if !config_file.exists() {
                anyhow::bail!("Config file not found: {}", config_file.display());
            }

            match PiedPiperConfig::load(Some(&config_file)) {
                Ok(_config) => {
                    info!("✅ Configuration is valid");
                    println!("✅ Configuration file is valid: {}", config_file.display());
                }
                Err(e) => {
                    error!("❌ Configuration validation failed: {}", e);
                    anyhow::bail!("Invalid configuration: {}", e);
                }
            }
        }

        ConfigAction::Show { json } => {
            let config = PiedPiperConfig::load(None)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("{}", serde_yaml::to_string(&config)?);
            }
        }
    }

    Ok(())
}

async fn handle_package_command(action: PackageAction) -> Result<()> {
    use crate::package::{PackageManifest, PiperNetPackage};
    use crate::package::builder::PackageBuilder;
    use crate::package::crypto;
    use crate::network::NetworkNode;

    match action {
        PackageAction::Init { directory, name, package_type, force } => {
            let manifest_path = directory.join("pn.toml");
            
            if manifest_path.exists() && !force {
                anyhow::bail!(
                    "Manifest file '{}' already exists. Use --force to overwrite.",
                    manifest_path.display()
                );
            }

            // Start with example template
            let mut manifest_content = PackageManifest::example();
            
            // Customize with provided options via string replacement
            if let Some(name) = name {
                manifest_content = manifest_content.replace(
                    r#"name = "my-awesome-app""#,
                    &format!(r#"name = "{}""#, name)
                );
            }
            
            manifest_content = manifest_content.replace(
                r#"type = "backend""#,
                &format!(r#"type = "{}""#, package_type.to_lowercase())
            );

            std::fs::write(&manifest_path, manifest_content)
                .with_context(|| format!("Failed to write manifest to {}", manifest_path.display()))?;

            info!("✅ Created package manifest: {}", manifest_path.display());
            println!("📦 Package manifest created: {}", manifest_path.display());
            println!("\nNext steps:");
            println!("  1. Edit pn.toml to customize your package");
            println!("  2. Build your WASM module: cargo build --target wasm32-wasip1 --release");
            println!("  3. Build package: pied-piper package build");
            println!("  4. Deploy: pied-piper package deploy <package-name>.pn");
        }

        PackageAction::Build { manifest, output, key } => {
            if !manifest.exists() {
                anyhow::bail!("Manifest file not found: {}", manifest.display());
            }

            info!("📦 Building package from {}", manifest.display());
            
            // Load manifest to get name and version for default output path
            let manifest_content = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Failed to read manifest: {}", manifest.display()))?;
            let pkg_manifest = PackageManifest::from_toml(&manifest_content)?;
            
            let output_path = output.unwrap_or_else(|| {
                let filename = format!("{}-{}.pn", pkg_manifest.metadata.name, pkg_manifest.metadata.version);
                PathBuf::from(filename)
            });

            // Check if key was provided (before moving it)
            let key_provided = key.is_some();
            
            // Determine encryption key
            let encryption_key = if let Some(key_str) = key {
                // Use provided key (hex encoded)
                let key_bytes = hex::decode(&key_str)
                    .context("Invalid hex-encoded key")?;
                if key_bytes.len() != 32 {
                    anyhow::bail!("Encryption key must be 32 bytes (64 hex characters)");
                }
                let mut key_array = [0u8; 32];
                key_array.copy_from_slice(&key_bytes);
                key_array
            } else {
                // Use network-wide shared key (default for distribution)
                crypto::get_network_key()
            };

            // Build the package
            let mut builder = PackageBuilder::from_manifest_file(&manifest).await?;
            builder.load_module().await?;
            builder.load_assets().await?;
            builder.load_dependencies().await?;
            
            let package = builder.build(&encryption_key)?;
            package.save_to_file(&output_path, &encryption_key).await?;

            info!("✅ Package built successfully: {}", output_path.display());
            println!("✅ Package built: {}", output_path.display());
            println!("📦 Name: {} v{}", pkg_manifest.metadata.name, pkg_manifest.metadata.version);
            println!("🔒 Encrypted with {}", if key_provided { "provided key" } else { "network shared key" });
            println!("\nDeploy with: pied-piper package deploy {}", output_path.display());
        }

        PackageAction::Deploy { package, name, timeout } => {
            if !package.exists() {
                anyhow::bail!("Package file not found: {}", package.display());
            }

            info!("🚀 Deploying package: {}", package.display());
            
            // Read the package file
            let package_bytes = tokio::fs::read(&package).await
                .with_context(|| format!("Failed to read package: {}", package.display()))?;
            
            // Verify it's a valid .pn package
            if package_bytes.len() < 4 || &package_bytes[0..4] != b"PN\x01\x00" {
                anyhow::bail!("Invalid package format. File must be a .pn package.");
            }
            
            // Decrypt package to read manifest using network key
            let decryption_key = crypto::get_network_key();
            
            let pkg = PiperNetPackage::load_from_file(&package, &decryption_key).await
                .context("Failed to decrypt package")?;
            
            let pkg_name = name.unwrap_or_else(|| pkg.manifest.metadata.name.clone());
            let pkg_version = pkg.manifest.metadata.version.clone();
            let pkg_description = pkg.manifest.metadata.description.clone();
            
            println!("📦 Package: {} v{}", pkg_name, pkg_version);
            if let Some(desc) = &pkg_description {
                println!("📝 Description: {}", desc);
            }
            
            // Create network node for deployment
            let config = crate::network::NetworkNodeConfig::default();
            let (client, mut node) = NetworkNode::new(config).await?;
            
            // Start the node
            node.start_listening()?;
            node.bootstrap_dht()?;
            
            // Spawn network node in background
            tokio::spawn(async move {
                if let Err(e) = node.run().await {
                    error!("Network node error: {}", e);
                }
            });
            
            // Wait a bit for DHT bootstrap
            info!("Waiting for DHT bootstrap...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Deploy the .pn package
            info!("Publishing package to network...");
            let cid = client
                .publish_module(
                    package_bytes,
                    Some(pkg_name.clone()),
                    Some(pkg_version.clone()),
                    pkg.manifest.metadata.author.clone(),
                    pkg_description.clone(),
                )
                .await
                .context("Failed to publish package")?;
            
            println!("✅ Package deployed successfully!");
            println!("🔗 CID: {}", cid);
            println!("🌐 Access at:");
            println!("   By name: http://localhost:8080/app/{}", pkg_name);
            println!("   By CID:  http://localhost:8080/cid/{}", cid);
            
            info!("Deployment complete. Keeping node running for {} seconds...", timeout);
            tokio::time::sleep(tokio::time::Duration::from_secs(timeout)).await;
        }

        PackageAction::Verify { package, verbose } => {
            if !package.exists() {
                anyhow::bail!("Package file not found: {}", package.display());
            }

            info!("🔍 Verifying package: {}", package.display());
            
            let package_bytes = tokio::fs::read(&package).await
                .with_context(|| format!("Failed to read package: {}", package.display()))?;
            
            // Check magic bytes
            if package_bytes.len() < 4 {
                anyhow::bail!("Invalid package: file too small");
            }
            
            if &package_bytes[0..4] != b"PN\x01\x00" {
                anyhow::bail!("Invalid package: incorrect magic bytes");
            }

            println!("✅ Valid .pn package format");
            println!("📦 File: {}", package.display());
            println!("📊 Size: {} bytes", package_bytes.len());
            
            if verbose {
                println!("\nNote: Decryption requires the node's peer ID key");
                println!("Use 'pied-piper package extract' to view contents");
            }
        }

        PackageAction::Extract { package, output, key } => {
            if !package.exists() {
                anyhow::bail!("Package file not found: {}", package.display());
            }

            info!("📦 Extracting package: {}", package.display());
            
            // Determine decryption key
            let decryption_key = if let Some(key_str) = key {
                let key_bytes = hex::decode(&key_str)
                    .context("Invalid hex-encoded key")?;
                if key_bytes.len() != 32 {
                    anyhow::bail!("Decryption key must be 32 bytes (64 hex characters)");
                }
                let mut key_array = [0u8; 32];
                key_array.copy_from_slice(&key_bytes);
                key_array
            } else {
                // Use network-wide shared key (default)
                crypto::get_network_key()
            };

            // Load and decrypt package
            let pkg = PiperNetPackage::load_from_file(&package, &decryption_key).await?;

            // Create output directory
            tokio::fs::create_dir_all(&output).await
                .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

            // Extract manifest
            let manifest_path = output.join("pn.toml");
            let manifest_content = pkg.manifest.to_toml()?;
            tokio::fs::write(&manifest_path, manifest_content).await?;

            // Extract module
            let module_bytes = pkg.get_module(&decryption_key)?;
            let module_path = output.join("module.wasm");
            tokio::fs::write(&module_path, module_bytes).await?;

            // Extract assets
            for (asset_path, _encrypted_asset) in &pkg.assets {
                if let Some(asset_bytes) = pkg.get_asset(asset_path, &decryption_key)? {
                    let asset_output_path = output.join(asset_path);
                    
                    // Create parent directories if needed
                    if let Some(parent) = asset_output_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    
                    tokio::fs::write(&asset_output_path, asset_bytes).await?;
                }
            }

            // Extract dependencies
            for (dep_name, _encrypted_dep) in &pkg.dependencies {
                if let Some(dep_bytes) = pkg.get_dependency(dep_name, &decryption_key)? {
                    let dep_path = output.join("dependencies").join(format!("{}.wasm", dep_name));
                    
                    tokio::fs::create_dir_all(output.join("dependencies")).await?;
                    tokio::fs::write(&dep_path, dep_bytes).await?;
                }
            }

            info!("✅ Package extracted to: {}", output.display());
            println!("✅ Package extracted to: {}", output.display());
            println!("📄 Manifest: {}", manifest_path.display());
            println!("📦 Module: {}", module_path.display());
            if !pkg.assets.is_empty() {
                println!("🎨 Assets: {} files", pkg.assets.len());
            }
            if !pkg.dependencies.is_empty() {
                println!("📚 Dependencies: {}", pkg.dependencies.len());
            }
        }
    }

    Ok(())
}
