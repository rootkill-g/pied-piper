mod cli;
mod network;
mod wasm;

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
        
        Commands::Run {
            module,
            function,
            max_memory,
            max_time,
            fuel,
        } => {
            info!("Running Wasm module: {:?}", module);
            
            if let Err(e) = run_wasm_module(module, function, max_memory, max_time, fuel).await {
                error!("Failed to run Wasm module: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Run a WebAssembly module
async fn run_wasm_module(
    module_path: std::path::PathBuf,
    function_name: Option<String>,
    max_memory_mb: usize,
    max_time_secs: u64,
    enable_fuel: bool,
) -> Result<()> {
    use std::time::Duration;
    use wasm::{WasmRuntime, WasmRuntimeConfig, ModuleLoader, Sandbox, ResourceLimits, ExecutionContext};
    
    info!("Loading module from: {:?}", module_path);
    
    // Create runtime configuration
    let config = WasmRuntimeConfig {
        max_memory_bytes: max_memory_mb * 1024 * 1024,
        max_execution_time: Duration::from_secs(max_time_secs),
        enable_async: true,
        enable_wasi: true,
        enable_fuel,
        initial_fuel: if enable_fuel { 10_000_000 } else { 0 },
    };
    
    // Create runtime
    let runtime = WasmRuntime::new(config)?;
    info!("Created Wasm runtime");
    
    // Create module loader
    let cache_dir = std::env::temp_dir().join("pied-piper-cache");
    let loader = ModuleLoader::new(cache_dir).await?;
    
    // Load module
    let (module_info, module_bytes) = loader.load_from_file(module_path).await?;
    info!("Loaded module: {} ({}  bytes)", module_info.cid, module_info.size);
    
    // Load Wasm module
    let module = runtime.load_module(&module_bytes)?;
    info!("Parsed Wasm module");
    
    // Validate module against sandbox limits
    let limits = ResourceLimits {
        max_memory_bytes: max_memory_mb * 1024 * 1024,
        max_execution_time: Duration::from_secs(max_time_secs),
        max_fuel: if enable_fuel { 10_000_000 } else { 0 },
        max_stack_depth: 1024,
        max_table_elements: 10_000,
    };
    
    let sandbox = Sandbox::new(limits.clone());
    sandbox.validate_module(&module)?;
    info!("Module passed sandbox validation");
    
    // Create store
    let mut store = runtime.create_store()?;
    
    // Create execution context
    let ctx = ExecutionContext::new(limits);
    
    // Instantiate module with WASI
    let instance = runtime.instantiate_with_wasi(&mut store, &module).await?;
    info!("Instantiated module with WASI");
    
    // Determine which function to call
    let func_name = function_name.unwrap_or_else(|| {
        // Try _start first (WASI), then main
        if instance.get_func(&mut store, "_start").is_some() {
            "_start".to_string()
        } else {
            "main".to_string()
        }
    });
    
    info!("Executing function: {}", func_name);
    
    // Execute the function
    let start_time = std::time::Instant::now();
    let results = runtime.execute_function(&mut store, &instance, &func_name, &[]).await?;
    let duration = start_time.elapsed();
    
    info!("Execution completed in {:?}", duration);
    
    // Print results
    if !results.is_empty() {
        println!("\nResults:");
        for (i, result) in results.iter().enumerate() {
            println!("  [{}]: {:?}", i, result);
        }
    }
    
    // Print fuel consumed if enabled
    if enable_fuel {
        if let Some(remaining_fuel) = runtime.get_remaining_fuel(&store) {
            let consumed = ctx.fuel_consumed(remaining_fuel);
            println!("\nFuel consumed: {} (remaining: {})", consumed, remaining_fuel);
        }
    }
    
    println!("\nExecution time: {:?}", duration);
    println!("Module CID: {}", module_info.cid);
    
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
