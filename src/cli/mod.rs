use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Pied Piper - Decentralized Internet Platform
#[derive(Parser, Debug)]
#[command(name = "pied-piper")]
#[command(about = "A decentralized internet platform for WebAssembly applications", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Pied Piper daemon (node)
    Daemon {
        /// TCP port to listen on
        #[arg(long, default_value = "0")]
        tcp_port: u16,

        /// QUIC port to listen on
        #[arg(long, default_value = "0")]
        quic_port: u16,

        /// Disable mDNS local discovery
        #[arg(long)]
        no_mdns: bool,

        /// Bootstrap peer addresses (format: peer_id@multiaddr)
        #[arg(long)]
        bootstrap: Vec<String>,

        /// Topics to subscribe to
        #[arg(long)]
        topic: Vec<String>,
    },

    /// Show node information
    Info {
        /// Node endpoint
        #[arg(default_value = "http://localhost:8080")]
        endpoint: String,
    },

    /// Deploy a WebAssembly application (placeholder for Phase 3)
    Deploy {
        /// Path to the application manifest
        manifest: PathBuf,
    },
    
    /// Run a WebAssembly module
    Run {
        /// Path to the Wasm module file
        module: PathBuf,
        
        /// Function to execute (default: _start or main)
        #[arg(short, long)]
        function: Option<String>,
        
        /// Maximum memory in MB
        #[arg(long, default_value = "128")]
        max_memory: usize,
        
        /// Maximum execution time in seconds
        #[arg(long, default_value = "30")]
        max_time: u64,
        
        /// Enable fuel metering for CPU limits
        #[arg(long)]
        fuel: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
