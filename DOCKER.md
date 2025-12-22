# Docker Deployment Guide

This guide explains how to deploy Pied Piper nodes using Docker and Docker Compose for local development, testing, or production environments.

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Scaling](#scaling)
- [Monitoring](#monitoring)
- [Production Deployment](#production-deployment)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Prerequisites

- Docker 20.10+ installed
- Docker Compose v2.0+ installed
- At least 2GB of available RAM
- Ports 8080-8083 and 4001-4032 available

### Start the Network

```bash
# Build and start all nodes
docker compose up -d

# View logs from all nodes
docker compose logs -f

# View logs from specific node
docker compose logs -f node-bootstrap

# Check status
docker compose ps
```

### Access Nodes

Once running, you can access:

- **Bootstrap Node**: http://localhost:8080
- **Node 1**: http://localhost:8081
- **Node 2**: http://localhost:8082
- **Node 3**: http://localhost:8083

### Deploy and Test

```bash
# Deploy an example to bootstrap node
cd examples/wasip1-core/hello-api
cargo build --target wasm32-wasip1 --release
../../target/release/pied-piper deploy \
  --gateway http://localhost:8080 \
  --file target/wasm32-wasip1/release/hello_api.wasm

# Test on different nodes (content will replicate across the network)
curl http://localhost:8080/cid/<CID>
curl http://localhost:8081/cid/<CID>
curl http://localhost:8082/cid/<CID>
```

### Stop the Network

```bash
# Stop all containers
docker compose down

# Stop and remove all data
docker compose down -v
```

## Architecture

### Network Topology

```
┌─────────────────────────────────────────────┐
│          Docker Bridge Network              │
│              172.20.0.0/24                  │
│                                             │
│  ┌──────────────┐         ┌──────────────┐ │
│  │  Bootstrap   │◄───────►│    Node 1    │ │
│  │ 172.20.0.10  │         │ 172.20.0.11  │ │
│  │  Port: 8080  │         │  Port: 8081  │ │
│  └──────────────┘         └──────────────┘ │
│         ▲                         ▲         │
│         │                         │         │
│         │    ┌──────────────┐     │         │
│         └───►│    Node 2    │◄────┘         │
│              │ 172.20.0.12  │               │
│              │  Port: 8082  │               │
│              └──────────────┘               │
│                     ▲                        │
│                     │                        │
│              ┌──────────────┐               │
│              │    Node 3    │               │
│              │ 172.20.0.13  │               │
│              │  Port: 8083  │               │
│              └──────────────┘               │
└─────────────────────────────────────────────┘
```

### Port Mappings

| Service        | HTTP Gateway | libp2p TCP | libp2p QUIC |
|----------------|-------------|------------|-------------|
| Bootstrap      | 8080        | 4001       | 4002/udp    |
| Node 1         | 8081        | 4011       | 4012/udp    |
| Node 2         | 8082        | 4021       | 4022/udp    |
| Node 3         | 8083        | 4031       | 4032/udp    |

### Data Persistence

Each node has its own Docker volume for data persistence:

- `bootstrap-data` → Bootstrap node storage
- `node-1-data` → Node 1 storage
- `node-2-data` → Node 2 storage
- `node-3-data` → Node 3 storage

## Configuration

### Node-Specific Configs

Each node uses its own configuration file:

- `config.bootstrap.yaml` → Bootstrap node
- `config.node1.yaml` → Node 1
- `config.node2.yaml` → Node 2
- `config.node3.yaml` → Node 3

### Customizing Configuration

Edit the configuration files to customize:

```yaml
network:
  tcp_port: 4001
  quic_port: 4002
  enable_mdns: true
  max_connections: 100

gateway:
  port: 8080
  enable_compression: true
  request_timeout_secs: 30

storage:
  max_cache_size_bytes: 536870912  # 512 MB
  max_cache_entries: 256

performance:
  worker_threads: 0  # 0 = auto-detect
  wasm_fuel_limit: 100000000
  wasm_memory_limit_bytes: 67108864  # 64 MB

logging:
  level: info
```

### Environment Variables

Override config via environment variables in `docker-compose.yml`:

```yaml
environment:
  - RUST_LOG=debug
  - PP_GATEWAY_PORT=8080
  - PP_NETWORK_TCP_PORT=4001
```

## Scaling

### Adding More Nodes

To add additional nodes, edit `docker-compose.yml`:

```yaml
  node-4:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: pied-piper-node-4
    hostname: node-4
    environment:
      - RUST_LOG=info
      - NODE_NAME=node-4
    ports:
      - "8084:8080"
      - "4041:4001"
      - "4042:4002/udp"
    volumes:
      - node-4-data:/home/pied-piper/.pied-piper
      - ./config.node4.yaml:/home/pied-piper/config.yaml:ro
    networks:
      pied-piper-network:
        ipv4_address: 172.20.0.14
    depends_on:
      node-bootstrap:
        condition: service_healthy
    restart: unless-stopped

volumes:
  node-4-data:
```

Create `config.node4.yaml` following the same pattern as other node configs.

### Scaling with Docker Compose Scale

For dynamic scaling:

```bash
# Scale to 5 instances (not recommended for production)
docker compose up -d --scale node-1=5
```

⚠️ **Note**: This requires removing static port mappings and using dynamic ports.

## Monitoring

### Health Checks

All nodes have health checks configured:

```bash
# Check health status
docker compose ps

# View health check logs
docker inspect --format='{{json .State.Health}}' pied-piper-bootstrap | jq
```

### View Logs

```bash
# All nodes
docker compose logs -f

# Specific node with timestamps
docker compose logs -f --timestamps node-1

# Last 100 lines
docker compose logs --tail=100 node-bootstrap
```

### Network Statistics

```bash
# Connect to a node
docker exec -it pied-piper-bootstrap /bin/sh

# Check peer connections (once inside container)
# Note: You'll need to implement a CLI command for this
pied-piper peers list
```

### Resource Usage

```bash
# View resource usage
docker stats

# Specific container
docker stats pied-piper-node-1
```

## Production Deployment

### Security Considerations

1. **Change Default Ports**: Use non-standard ports
2. **Enable TLS**: Configure HTTPS in gateway section
3. **Firewall Rules**: Only expose necessary ports
4. **Non-root User**: Already configured in Dockerfile
5. **Resource Limits**: Add to docker-compose.yml:

```yaml
services:
  node-1:
    # ... other config ...
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

### Bootstrap Peers Configuration

For production across multiple hosts, configure bootstrap peers:

**config.node1.yaml**:
```yaml
network:
  enable_mdns: false  # Disable mDNS for internet deployment
  bootstrap_peers:
    - "12D3KooW...@/ip4/203.0.113.1/tcp/4001"
    - "12D3KooW...@/ip4/203.0.113.2/tcp/4001"
```

### HTTPS/TLS Configuration

1. Generate certificates:
```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

2. Update config:
```yaml
gateway:
  https_port: 8443
  tls_cert_path: /home/pied-piper/certs/cert.pem
  tls_key_path: /home/pied-piper/certs/key.pem
```

3. Mount certificates:
```yaml
volumes:
  - ./certs:/home/pied-piper/certs:ro
```

### Persistent Logging

Configure file logging:

```yaml
logging:
  level: info
  json_format: true  # For log aggregation
  file_path: /home/pied-piper/logs/pied-piper.log
```

Mount log directory:
```yaml
volumes:
  - ./logs:/home/pied-piper/logs
```

### Backup and Recovery

#### Backup Node Data

```bash
# Backup all node data
docker run --rm \
  -v bootstrap-data:/data \
  -v $(pwd):/backup \
  debian:bookworm-slim \
  tar czf /backup/bootstrap-backup.tar.gz -C /data .

# Repeat for each node volume
```

#### Restore Node Data

```bash
# Create volume and restore
docker volume create bootstrap-data
docker run --rm \
  -v bootstrap-data:/data \
  -v $(pwd):/backup \
  debian:bookworm-slim \
  tar xzf /backup/bootstrap-backup.tar.gz -C /data
```

## Troubleshooting

### Nodes Not Discovering Each Other

**Symptom**: Nodes don't connect to each other

**Solutions**:
1. Check mDNS is enabled:
   ```bash
   docker compose logs | grep -i mdns
   ```

2. Verify network connectivity:
   ```bash
   docker exec pied-piper-node-1 ping 172.20.0.10
   ```

3. Check bootstrap configuration:
   ```bash
   docker exec pied-piper-node-1 cat /home/pied-piper/config.yaml
   ```

### High Memory Usage

**Symptom**: Containers consuming too much memory

**Solutions**:
1. Reduce cache size in config:
   ```yaml
   storage:
     max_cache_size_bytes: 268435456  # 256 MB
     max_cache_entries: 128
   ```

2. Limit WASM memory:
   ```yaml
   performance:
     wasm_memory_limit_bytes: 33554432  # 32 MB
   ```

3. Add Docker resource limits (see Production section)

### Port Conflicts

**Symptom**: `bind: address already in use`

**Solutions**:
1. Find conflicting process:
   ```bash
   lsof -i :8080
   ```

2. Change port mappings in docker-compose.yml:
   ```yaml
   ports:
     - "9080:8080"  # Use different host port
   ```

### Container Crashes

**Symptom**: Container exits immediately

**Solutions**:
1. Check logs:
   ```bash
   docker compose logs node-1
   ```

2. Run container interactively:
   ```bash
   docker run -it --rm pied-piper-node-1 /bin/sh
   ```

3. Verify config syntax:
   ```bash
   docker exec pied-piper-node-1 pied-piper validate-config /home/pied-piper/config.yaml
   ```

### Build Failures

**Symptom**: Docker build fails

**Solutions**:
1. Clear build cache:
   ```bash
   docker compose build --no-cache
   ```

2. Check disk space:
   ```bash
   df -h
   docker system df
   ```

3. Prune unused images:
   ```bash
   docker system prune -a
   ```

### Network Issues

**Symptom**: Nodes can't reach each other

**Solutions**:
1. Inspect network:
   ```bash
   docker network inspect pied-piper-network
   ```

2. Check DNS resolution:
   ```bash
   docker exec pied-piper-node-1 nslookup bootstrap
   ```

3. Recreate network:
   ```bash
   docker compose down
   docker network prune
   docker compose up -d
   ```

## Advanced Topics

### Using External Network

To connect to nodes outside Docker:

```yaml
networks:
  pied-piper-network:
    driver: bridge
    driver_opts:
      com.docker.network.bridge.enable_ip_masquerade: "true"
```

Configure host networking for bootstrap:
```yaml
node-bootstrap:
  network_mode: host
```

### Multi-Host Deployment

For deployment across multiple machines:

1. Use Docker Swarm or Kubernetes
2. Configure external bootstrap peers
3. Use overlay networks
4. Implement service discovery

See [DEPLOYMENT.md](docs/DEPLOYMENT.md) for details.

### Custom Images

Build with specific Rust version:

```dockerfile
FROM rust:1.94-slim as builder
# ... rest of Dockerfile
```

Or use pre-built images (once published):
```yaml
services:
  node-bootstrap:
    image: pied-piper:latest
```

## References

- [Configuration Guide](docs/CONFIGURATION.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Architecture Overview](docs/ARCHITECTURE.md)
- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)

## Support

For issues and questions:
- GitHub Issues: https://github.com/rootkill-g/pied-piper/issues
- Documentation: https://github.com/rootkill-g/pied-piper/docs
