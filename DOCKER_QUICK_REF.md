# Docker Quick Reference

## Quick Commands

```bash
# Start the network
make up

# Stop the network
make down

# View logs
make logs

# Check health
make health

# Full restart
make restart

# Clean everything
make clean
```

## Network Access

| Node      | HTTP Gateway          | libp2p TCP | libp2p QUIC |
|-----------|-----------------------|------------|-------------|
| Bootstrap | http://localhost:8080 | 4001       | 4002        |
| Node 1    | http://localhost:8081 | 4011       | 4012        |
| Node 2    | http://localhost:8082 | 4021       | 4022        |
| Node 3    | http://localhost:8083 | 4031       | 4032        |

## Common Operations

### Deploy Module

```bash
# Build example
cd examples/wasip1-core/hello-api
cargo build --target wasm32-wasip1 --release

# Deploy to any node
./target/release/pied-piper deploy \
  --gateway http://localhost:8080 \
  --file target/wasm32-wasip1/release/hello_api.wasm
```

### Test Replication

```bash
# Deploy to bootstrap
CID=$(./target/release/pied-piper deploy \
  --gateway http://localhost:8080 \
  --file module.wasm | grep CID | cut -d' ' -f4)

# Access from different nodes (content replicates)
curl http://localhost:8080/cid/$CID
curl http://localhost:8081/cid/$CID
curl http://localhost:8082/cid/$CID
```

### View Node Logs

```bash
# All nodes
make logs

# Specific node
make logs-bootstrap
make logs-node1
make logs-node2
make logs-node3

# Follow specific service
docker compose logs -f node-1
```

### Shell Access

```bash
# Connect to node
make shell-bootstrap
make shell-node1
make shell-node2
make shell-node3

# Or directly
docker exec -it pied-piper-node-1 /bin/bash
```

### Resource Monitoring

```bash
# Container stats
make stats

# Continuous monitoring
docker stats

# Disk usage
make disk-usage
```

### Network Testing

```bash
# Test connectivity
make test-network

# Ping between nodes
docker exec pied-piper-node-1 ping 172.20.0.10
```

## Troubleshooting

### Nodes Won't Start

```bash
# Check logs
make logs

# Rebuild
docker compose build --no-cache
make up
```

### Port Conflicts

```bash
# Find what's using port 8080
lsof -i :8080

# Or change ports in docker-compose.yml
```

### Reset Everything

```bash
# Stop and remove all data
make clean

# Restart fresh
make build
make up
```

### Network Issues

```bash
# Check network
docker network inspect pied-piper-network

# Recreate network
docker compose down
docker network prune
docker compose up -d
```

## Advanced Usage

### Add More Nodes

Edit `docker-compose.yml`:

```yaml
node-4:
  # Copy node-3 config and adjust:
  # - Container name: pied-piper-node-4
  # - Ports: 8084:8080, 4041:4001, 4042:4002
  # - IP: 172.20.0.14
  # - Volume: node-4-data
```

### Custom Configuration

```bash
# Edit node configs
vim config.bootstrap.yaml
vim config.node1.yaml

# Restart to apply
make restart
```

### Production Settings

```yaml
# Add to docker-compose.yml under each service
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '1'
      memory: 1G
```

### Backup Data

```bash
# Backup all nodes
make backup

# Manual backup
docker run --rm \
  -v pied-piper_bootstrap-data:/data \
  -v $(pwd):/backup \
  debian:bookworm-slim \
  tar czf /backup/bootstrap.tar.gz -C /data .
```

### Enable HTTPS

1. Generate certificates:
```bash
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes
```

2. Update config:
```yaml
gateway:
  https_port: 8443
  tls_cert_path: /home/pied-piper/certs/cert.pem
  tls_key_path: /home/pied-piper/certs/key.pem
```

3. Mount certs in docker-compose.yml:
```yaml
volumes:
  - ./certs:/home/pied-piper/certs:ro
```

## See Also

- [DOCKER.md](DOCKER.md) - Complete Docker guide
- [CONFIGURATION.md](docs/CONFIGURATION.md) - Configuration reference
- [DEPLOYMENT.md](docs/DEPLOYMENT.md) - Production deployment
- [README.md](README.md) - Main documentation
