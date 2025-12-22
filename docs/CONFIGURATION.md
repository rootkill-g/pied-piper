# Configuration Guide

Pied Piper supports flexible configuration through multiple sources with clear precedence rules.

## Configuration Precedence

Configuration values are loaded in the following order (later sources override earlier ones):

1. **Default values** - Built-in sensible defaults
2. **Configuration file** - YAML, TOML, or JSON format
3. **Environment variables** - Prefixed with `PP_`
4. **CLI arguments** - Command-line flags (highest priority)

## Quick Start

### Generate Example Configuration

```bash
# Generate YAML config (default)
pied-piper config init

# Generate TOML config
pied-piper config init config.toml --format toml

# Generate JSON config
pied-piper config init config.json --format json
```

### Validate Configuration

```bash
pied-piper config validate pied-piper.yaml
```

### View Resolved Configuration

```bash
# Show current configuration with all sources merged
pied-piper config show

# Show as JSON
pied-piper config show --json

# Load from specific file
pied-piper --config my-config.yaml config show
```

## Configuration File Formats

### YAML (Recommended)

```yaml
network:
  tcp_port: 4001
  quic_port: 4002
  enable_mdns: true

gateway:
  port: 8080
  enable_compression: true
```

### TOML

```toml
[network]
tcp_port = 4001
quic_port = 4002
enable_mdns = true

[gateway]
port = 8080
enable_compression = true
```

### JSON

```json
{
  "network": {
    "tcp_port": 4001,
    "quic_port": 4002,
    "enable_mdns": true
  },
  "gateway": {
    "port": 8080,
    "enable_compression": true
  }
}
```

## Environment Variables

All configuration options can be set via environment variables using the `PP_` prefix and underscore separator.

### Examples

```bash
# Set gateway port
export PP_GATEWAY_PORT=9090

# Set network ports
export PP_NETWORK_TCP_PORT=5001
export PP_NETWORK_QUIC_PORT=5002

# Set log level
export PP_LOGGING_LEVEL=debug

# Disable mDNS
export PP_NETWORK_ENABLE_MDNS=false

# Set cache size (in bytes)
export PP_STORAGE_MAX_CACHE_SIZE_BYTES=1073741824  # 1 GB
```

### Nested Configuration

For nested configuration values, use underscores to separate levels:

```bash
PP_NETWORK_IDLE_TIMEOUT_SECS=120
PP_GATEWAY_REQUEST_TIMEOUT_SECS=60
PP_PERFORMANCE_WASM_FUEL_LIMIT=200000000
```

## Using Configuration Files

### With Gateway Command

```bash
# Use config file
pied-piper --config config.yaml gateway

# Override specific values with CLI flags
pied-piper --config config.yaml gateway --tcp-port 5001

# Override with environment variables
PP_GATEWAY_PORT=9090 pied-piper --config config.yaml gateway
```

### Configuration File Locations

Pied Piper looks for configuration files in the following locations (in order):

1. Path specified by `--config` flag
2. `./pied-piper.yaml` (current directory)
3. `./pied-piper.toml`
4. `./pied-piper.json`
5. `~/.config/pied-piper/config.yaml`
6. `/etc/pied-piper/config.yaml`

## Configuration Sections

### Network

P2P network configuration for libp2p.

```yaml
network:
  tcp_port: 4001              # TCP listening port (0 = random)
  quic_port: 4002             # QUIC listening port (0 = random)
  enable_mdns: true           # Local peer discovery
  bootstrap_peers: []         # Initial peers to connect to
  topics: []                  # GossipSub topics
  max_connections: 100        # Connection limit
  idle_timeout_secs: 60       # Idle connection timeout
```

### Gateway

HTTP/HTTPS server configuration.

```yaml
gateway:
  port: 8080                       # HTTP port
  https_port: 0                    # HTTPS port (0 = disabled)
  tls_cert_path: null              # TLS certificate path
  tls_key_path: null               # TLS private key path
  index_file: index.html           # Default index file
  enable_compression: true         # Response compression
  request_timeout_secs: 30         # Request timeout
```

### Storage

Cache and persistent storage configuration.

```yaml
storage:
  data_dir: .pied-piper                 # Main data directory
  cache_dir: null                       # Module cache (default: data_dir/modules)
  dht_dir: null                         # DHT state (default: data_dir)
  max_cache_size_bytes: 536870912       # Cache size limit (512 MB)
  max_cache_entries: 256                # Max cached modules
```

### Performance

Performance tuning options.

```yaml
performance:
  worker_threads: 0                      # Worker threads (0 = auto)
  connection_pool_size: 10               # HTTP pool size per host
  pool_timeout_secs: 90                  # Pool connection timeout
  tcp_keepalive: true                    # TCP keepalive for HTTP
  wasm_fuel_limit: 100000000             # WASM CPU limit
  wasm_memory_limit_bytes: 67108864      # WASM memory limit (64 MB)
```

### Logging

Logging configuration.

```yaml
logging:
  level: info                # Log level: trace, debug, info, warn, error
  json_format: false         # JSON formatted logs
  file_path: null            # Log file path (null = stdout)
```

## Production Deployment

See [`config.production.yaml`](config.production.yaml) for a production-ready configuration example.

### Key Production Settings

1. **Disable mDNS**: Set `network.enable_mdns: false`
2. **Enable HTTPS**: Configure `gateway.https_port`, `tls_cert_path`, `tls_key_path`
3. **JSON Logging**: Set `logging.json_format: true` for log aggregation
4. **Increase Limits**: Adjust `max_connections`, cache sizes, timeouts
5. **Set Bootstrap Peers**: Add production bootstrap nodes

### Systemd Service Example

```ini
[Unit]
Description=Pied Piper Gateway
After=network.target

[Service]
Type=simple
User=pied-piper
Group=pied-piper
WorkingDirectory=/var/lib/pied-piper
ExecStart=/usr/local/bin/pied-piper --config /etc/pied-piper/config.yaml gateway
Restart=on-failure
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/pied-piper /var/cache/pied-piper /var/log/pied-piper

[Install]
WantedBy=multi-user.target
```

### Docker Example

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/pied-piper /usr/local/bin/
COPY config.production.yaml /etc/pied-piper/config.yaml

EXPOSE 8080 8443 4001 4002/udp

CMD ["pied-piper", "--config", "/etc/pied-piper/config.yaml", "gateway"]
```

## Validation

The configuration system validates all values on load:

- TLS paths must exist if HTTPS is enabled
- Log levels must be valid (trace, debug, info, warn, error)
- Numeric limits must be positive
- Port numbers must be valid (0-65535)

Invalid configurations will fail with helpful error messages:

```
❌ Configuration validation failed: Invalid log level 'invalid'. Must be one of: trace, debug, info, warn, error
```

## Troubleshooting

### View Effective Configuration

```bash
# See what configuration is actually being used
pied-piper config show
```

### Test Configuration Changes

```bash
# Validate before deploying
pied-piper config validate new-config.yaml

# Test with dry-run (validate only, don't start)
pied-piper --config new-config.yaml config show
```

### Debug Environment Variables

```bash
# Print all PP_* environment variables
env | grep ^PP_
```

## Examples

See the [`examples/`](examples/) directory for real-world configuration examples:

- `config.example.yaml` - Fully documented with all options
- `config.production.yaml` - Production deployment template
- `config.development.yaml` - Local development settings
