mod cli;
mod network;
mod wasm;
mod content;

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
            
            // Read the WASM file
            let module_bytes = tokio::fs::read(&manifest).await?;
            info!("Loaded {} bytes from {}", module_bytes.len(), manifest.display());
            
            // Extract module name from file path
            let module_name = manifest
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            
            // Create a minimal node for deployment
            let config = NetworkNodeConfig::default();
            let mut node = NetworkNode::new(config).await?;
            
            info!("Local Peer ID: {}", node.local_peer_id());
            
            // Start listening
            let addrs = node.start_listening()?;
            info!("Listening on {} addresses", addrs.len());
            
            // Publish the module to the network
            info!("Publishing module to network...");
            let cid = node.publish_module(
                module_bytes,
                module_name.clone(),
                Some("1.0.0".to_string()),
                None,
                Some(format!("Module deployed from {}", manifest.display())),
            ).await?;
            
            println!("\n✅ Module deployed successfully!");
            println!("📦 Module Name: {}", module_name.as_deref().unwrap_or("unnamed"));
            println!("🔗 CID: {}", cid);
            println!("🆔 Provider Peer ID: {}", node.local_peer_id());
            println!("\nTo run this module on another node:");
            println!("  pied-piper run {} --function <function_name>", cid);
            
            // Keep node running briefly to allow DHT propagation
            info!("Keeping node alive for DHT propagation...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        Commands::Search { name, timeout } => {
            info!("Searching for modules with name: {}", name);
            
            // Create a temporary network node for searching
            let config = NetworkNodeConfig::default();
            let mut node = NetworkNode::new(config).await?;
            
            info!("Created network node: {}", node.local_peer_id());
            
            // Start listening
            let addrs = node.start_listening()?;
            info!("Listening on {} addresses", addrs.len());
            
            println!("\n🔍 Searching for modules matching '{}'...", name);
            println!("⏱️  Timeout: {} seconds", timeout);
            
            // Search for modules
            let results = node.search_modules_by_name(&name).await?;
            
            if results.is_empty() {
                println!("\n⚠️  No modules found matching '{}'", name);
                println!("\nNote: Module search requires:");
                println!("  1. Active provider nodes with deployed modules");
                println!("  2. Proper network connectivity and DHT propagation");
                println!("  3. Bootstrap peers for peer discovery");
                println!("\nTry deploying a module first with:");
                println!("  pied-piper deploy <path-to-wasm>");
            } else {
                println!("\n✅ Found {} module(s):\n", results.len());
                for (i, info) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, info.name.as_deref().unwrap_or("unnamed"));
                    println!("   CID: {}", info.cid);
                    println!("   Version: {}", info.version.as_deref().unwrap_or("unknown"));
                    println!("   Size: {} bytes", info.size);
                    if let Some(desc) = &info.description {
                        println!("   Description: {}", desc);
                    }
                    println!();
                }
            }
            
            // Keep node alive briefly for any pending network operations
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        
        Commands::Run {
            module,
            function,
            max_memory,
            max_time,
            fuel,
        } => {
            info!("Running Wasm module: {:?}", module);
            
            // Check if module is a CID (starts with 'b' and doesn't have path separators)
            let module_str = module.to_string_lossy();
            let is_cid = !module_str.contains('/') && !module_str.contains('\\') && module_str.starts_with('b');
            
            if is_cid {
                // Fetch module from network
                info!("Module appears to be a CID, fetching from network");
                if let Err(e) = run_wasm_from_network(
                    module_str.to_string(),
                    function,
                    max_memory,
                    max_time,
                    fuel,
                ).await {
                    error!("Failed to run Wasm module from network: {}", e);
                    return Err(e);
                }
            } else {
                // Load module from local file
                if let Err(e) = run_wasm_module(module, function, max_memory, max_time, fuel).await {
                    error!("Failed to run Wasm module: {}", e);
                    return Err(e);
                }
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

/// Run a WebAssembly module fetched from the network by CID
async fn run_wasm_from_network(
    cid_str: String,
    function_name: Option<String>,
    max_memory_mb: usize,
    max_time_secs: u64,
    enable_fuel: bool,
) -> Result<()> {
    use std::time::Duration;
    use wasm::{WasmRuntime, WasmRuntimeConfig, ModuleLoader, Sandbox, ResourceLimits, ExecutionContext, ModuleCid};
    
    info!("Fetching module with CID: {}", cid_str);
    
    // Create a temporary network node to fetch the module
    let config = NetworkNodeConfig::default();
    let mut node = NetworkNode::new(config).await?;
    
    info!("Created network node: {}", node.local_peer_id());
    
    // Start listening
    let addrs = node.start_listening()?;
    info!("Listening on {} addresses", addrs.len());
    
    // Create module loader
    let cache_dir = std::env::temp_dir().join("pied-piper-cache");
    let loader = ModuleLoader::new(cache_dir).await?;
    
    // Check if module is in cache
    let cid = ModuleCid(cid_str.clone());
    if let Some((module_info, module_bytes)) = loader.get_from_cache(&cid).await {
        info!("Found module in cache!");
        return execute_wasm_bytes(
            module_bytes.to_vec(),
            module_info,
            function_name,
            max_memory_mb,
            max_time_secs,
            enable_fuel,
        ).await;
    }
    
    info!("Module not in cache, searching network...");
    
    // Try to find the module on the network
    // First, try to discover providers
    let _search_result = node.find_module_by_cid(&cid).await?;
    
    // In a production implementation, we would:
    // 1. Wait for DHT query results in the event loop
    // 2. Get list of providers
    // 3. Try to fetch from each provider
    // 4. Verify CID matches
    // 5. Cache the module
    
    // For now, provide helpful error message
    println!("\n⚠️  Network fetching is partially implemented.");
    println!("To run a module from the network, you need to:");
    println!("1. Have the module provider node running");
    println!("2. Know the provider's peer ID");
    println!("3. Ensure nodes can discover each other (same network/bootstrap)");
    println!("\nFor now, make sure the module is cached locally by deploying it first.");
    
    anyhow::bail!("Module {} not found in cache. Network discovery is still being implemented.", cid_str)
}

/// Execute WebAssembly module from bytes
async fn execute_wasm_bytes(
    module_bytes: Vec<u8>,
    module_info: wasm::loader::ModuleInfo,
    function_name: Option<String>,
    max_memory_mb: usize,
    max_time_secs: u64,
    enable_fuel: bool,
) -> Result<()> {
    use std::time::Duration;
    use wasm::{WasmRuntime, WasmRuntimeConfig, Sandbox, ResourceLimits, ExecutionContext};
    
    info!("Executing module: {} ({} bytes)", module_info.cid, module_bytes.len());
    
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
