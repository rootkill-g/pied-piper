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

    /// Deploy a WebAssembly application
    Deploy {
        /// Path to the WASM module file or manifest.yaml
        manifest: PathBuf,

        /// Optional assets directory (HTML, CSS, JS, etc.)
        #[arg(short, long)]
        assets: Option<PathBuf>,

        /// Name of the module (overrides manifest)
        #[arg(short, long)]
        name: Option<String>,

        /// Version of the module (overrides manifest)
        #[arg(short, long)]
        version: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Author
        #[arg(short, long)]
        author: Option<String>,
    },

    /// Search for modules by name
    Search {
        /// Module name to search for
        name: String,

        /// Maximum time to wait for results in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },

    /// Run a WebAssembly module
    Run {
        /// Path to the Wasm module file or CID
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

    /// Start HTTP gateway server
    Gateway {
        /// HTTP listening address
        #[arg(long, default_value = "127.0.0.1:8080")]
        listen: String,

        /// HTTPS listening address (if TLS enabled)
        #[arg(long, default_value = "127.0.0.1:8443")]
        https_listen: String,

        /// Enable TLS/HTTPS
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate file (PEM format)
        #[arg(long)]
        tls_cert: Option<PathBuf>,

        /// Path to TLS private key file (PEM format)
        #[arg(long)]
        tls_key: Option<PathBuf>,

        /// TCP port for P2P network
        #[arg(long, default_value = "0")]
        tcp_port: u16,

        /// QUIC port for P2P network
        #[arg(long, default_value = "0")]
        quic_port: u16,

        /// Bootstrap peer addresses
        #[arg(long)]
        bootstrap: Vec<String>,

        /// Enable CORS
        #[arg(long, default_value = "true")]
        cors: bool,

        /// Request timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
    },

    /// Configuration management commands
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Generate an example configuration file
    Init {
        /// Output file path
        #[arg(default_value = "pied-piper.yaml")]
        output: PathBuf,

        /// Configuration format (yaml, toml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },

    /// Validate a configuration file
    Validate {
        /// Configuration file to validate
        config_file: PathBuf,
    },

    /// Show current configuration (resolved with env vars)
    Show {
        /// Show as JSON
        #[arg(long)]
        json: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
