# Pied Piper Deployment Guide

This guide covers deploying Pied Piper in various environments, from local development to production cloud infrastructure.

## Table of Contents

- [Quick Deploy](#quick-deploy)
- [Binary Installation](#binary-installation)
- [Docker Deployment](#docker-deployment)
- [systemd Service](#systemd-service)
- [Docker Compose](#docker-compose-full-stack)
- [Cloud Providers](#cloud-providers)
- [Kubernetes](#kubernetes)
- [Production Checklist](#production-checklist)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Quick Deploy

### One-Line Install

```bash
curl -fsSL https://raw.githubusercontent.com/rootkill-g/pied-piper/main/install.sh | sh
```

### Build from Source

```bash
# Clone repository
git clone https://github.com/rootkill-g/pied-piper
cd pied-piper

# Build release binary
cargo build --release

# Install (optional)
sudo cp target/release/pied-piper /usr/local/bin/

# Verify
pied-piper --version
```

## Binary Installation

### Supported Platforms

- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

### Download Pre-built Binary

```bash
# Linux x86_64
wget https://github.com/rootkill-g/pied-piper/releases/latest/download/pied-piper-linux-x86_64.tar.gz
tar xzf pied-piper-linux-x86_64.tar.gz
sudo mv pied-piper /usr/local/bin/

# macOS (Apple Silicon)
wget https://github.com/rootkill-g/pied-piper/releases/latest/download/pied-piper-macos-aarch64.tar.gz
tar xzf pied-piper-macos-aarch64.tar.gz
sudo mv pied-piper /usr/local/bin/
```

### Basic Configuration

Create `/etc/pied-piper/config.yaml`:

```yaml
network:
  tcp_port: 4001
  quic_port: 4002
  enable_mdns: true

gateway:
  port: 8080
  https_port: 8443
  request_timeout_secs: 30

security:
  rate_limit_per_minute: 60
  max_connections_per_ip: 100
  enable_hsts: true
  enable_strict_csp: true

storage:
  data_dir: /var/lib/pied-piper
  max_cache_size_bytes: 536870912  # 512MB

logging:
  level: info
  json_format: false
```

### Run

```bash
pied-piper gateway --config /etc/pied-piper/config.yaml
```

## Docker Deployment

### Single Container

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder

WORKDIR /build
COPY . .

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/pied-piper /usr/local/bin/

# Create data directory
RUN mkdir -p /var/lib/pied-piper

# Expose ports
EXPOSE 8080 8443 4001 4002

# Run as non-root
RUN useradd -m -u 1000 pied-piper
USER pied-piper

WORKDIR /home/pied-piper

ENTRYPOINT ["pied-piper"]
CMD ["gateway", "--listen", "0.0.0.0:8080"]
```

**Build and Run:**
```bash
# Build image
docker build -t pied-piper:latest .

# Run container
docker run -d \
  --name pied-piper \
  -p 8080:8080 \
  -p 8443:8443 \
  -p 4001:4001 \
  -p 4002:4002/udp \
  -v pied-piper-data:/var/lib/pied-piper \
  pied-piper:latest
```

### With Custom Configuration

```bash
# Create config directory
mkdir -p ./config

# Create config.yaml (see example above)
vim ./config/config.yaml

# Run with mounted config
docker run -d \
  --name pied-piper \
  -p 8080:8080 \
  -v ./config:/etc/pied-piper:ro \
  -v pied-piper-data:/var/lib/pied-piper \
  pied-piper:latest gateway --config /etc/pied-piper/config.yaml
```

### Docker Logs

```bash
# View logs
docker logs -f pied-piper

# Export metrics
docker exec pied-piper curl http://localhost:8080/metrics
```

## systemd Service

### Installation

Create `/etc/systemd/system/pied-piper.service`:

```ini
[Unit]
Description=Pied Piper P2P Gateway
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=pied-piper
Group=pied-piper

# Binary location
ExecStart=/usr/local/bin/pied-piper gateway \
  --config /etc/pied-piper/config.yaml \
  --listen 0.0.0.0:8080

# Working directory
WorkingDirectory=/var/lib/pied-piper

# Restart on failure
Restart=on-failure
RestartSec=5s

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/pied-piper
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

# Environment
Environment=RUST_LOG=info,pied_piper=debug
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=multi-user.target
```

### Create User and Directories

```bash
# Create service user
sudo useradd -r -s /bin/false pied-piper

# Create directories
sudo mkdir -p /etc/pied-piper
sudo mkdir -p /var/lib/pied-piper
sudo mkdir -p /var/log/pied-piper

# Set permissions
sudo chown -R pied-piper:pied-piper /var/lib/pied-piper
sudo chown -R pied-piper:pied-piper /var/log/pied-piper
sudo chmod 755 /var/lib/pied-piper
```

### Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable on boot
sudo systemctl enable pied-piper

# Start service
sudo systemctl start pied-piper

# Check status
sudo systemctl status pied-piper

# View logs
sudo journalctl -u pied-piper -f
```

### Service Management

```bash
# Stop
sudo systemctl stop pied-piper

# Restart
sudo systemctl restart pied-piper

# Disable
sudo systemctl disable pied-piper

# View recent logs
sudo journalctl -u pied-piper --since "1 hour ago"
```

## Docker Compose (Full Stack)

Deploy Pied Piper with Prometheus and Grafana monitoring.

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  pied-piper:
    image: pied-piper:latest
    build: .
    container_name: pied-piper
    ports:
      - "8080:8080"
      - "8443:8443"
      - "4001:4001"
      - "4002:4002/udp"
    volumes:
      - ./config:/etc/pied-piper:ro
      - pied-piper-data:/var/lib/pied-piper
    environment:
      - RUST_LOG=info,pied_piper=debug
    command: gateway --config /etc/pied-piper/config.yaml
    restart: unless-stopped
    networks:
      - pied-piper-net
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/usr/share/prometheus/console_libraries'
      - '--web.console.templates=/usr/share/prometheus/consoles'
    restart: unless-stopped
    networks:
      - pied-piper-net

  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
      - ./grafana/datasources:/etc/grafana/provisioning/datasources:ro
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_INSTALL_PLUGINS=
    restart: unless-stopped
    networks:
      - pied-piper-net

volumes:
  pied-piper-data:
  prometheus-data:
  grafana-data:

networks:
  pied-piper-net:
    driver: bridge
```

**prometheus.yml:**
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'pied-piper'
    static_configs:
      - targets: ['pied-piper:8080']
    metrics_path: '/metrics'
```

**Start Stack:**
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down

# Stop and remove volumes
docker-compose down -v
```

**Access:**
- Pied Piper: http://localhost:8080
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/admin)

## Cloud Providers

### AWS (EC2)

**Launch EC2 Instance:**

```bash
# Ubuntu 22.04 LTS, t3.medium or larger
# Open ports: 22 (SSH), 80, 443, 4001, 4002

# Connect via SSH
ssh -i your-key.pem ubuntu@<instance-ip>

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/rootkill-g/pied-piper
cd pied-piper
cargo build --release

# Install binary
sudo cp target/release/pied-piper /usr/local/bin/

# Set up systemd service (see above)
# Configure nginx reverse proxy (optional)
```

**Elastic Beanstalk:**

Create `Dockerrun.aws.json`:
```json
{
  "AWSEBDockerrunVersion": "1",
  "Image": {
    "Name": "your-registry/pied-piper:latest"
  },
  "Ports": [
    {
      "ContainerPort": 8080,
      "HostPort": 80
    }
  ],
  "Volumes": [
    {
      "HostDirectory": "/var/app/data",
      "ContainerDirectory": "/var/lib/pied-piper"
    }
  ]
}
```

Deploy:
```bash
eb init
eb create pied-piper-env
eb deploy
```

### Google Cloud Platform (GCP)

**Compute Engine:**

```bash
# Create VM
gcloud compute instances create pied-piper \
  --machine-type=e2-medium \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --boot-disk-size=20GB \
  --tags=http-server,https-server

# Configure firewall
gcloud compute firewall-rules create allow-pied-piper \
  --allow=tcp:8080,tcp:8443,tcp:4001,udp:4002

# SSH and install (same as AWS)
gcloud compute ssh pied-piper
```

**Cloud Run:**

```bash
# Build and push to GCR
gcloud builds submit --tag gcr.io/PROJECT_ID/pied-piper

# Deploy to Cloud Run
gcloud run deploy pied-piper \
  --image gcr.io/PROJECT_ID/pied-piper \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --port 8080 \
  --memory 512Mi \
  --cpu 1
```

### Azure

**Virtual Machine:**

```bash
# Create VM
az vm create \
  --resource-group pied-piper-rg \
  --name pied-piper-vm \
  --image UbuntuLTS \
  --size Standard_B2s \
  --admin-username azureuser \
  --generate-ssh-keys

# Open ports
az vm open-port --port 8080 --resource-group pied-piper-rg --name pied-piper-vm
az vm open-port --port 4001 --resource-group pied-piper-rg --name pied-piper-vm

# SSH and install
az vm show --resource-group pied-piper-rg --name pied-piper-vm -d --query publicIps -o tsv
ssh azureuser@<ip-address>
```

**Container Instances:**

```bash
# Create container
az container create \
  --resource-group pied-piper-rg \
  --name pied-piper \
  --image your-registry/pied-piper:latest \
  --dns-name-label pied-piper \
  --ports 8080 4001 \
  --cpu 1 \
  --memory 1
```

### DigitalOcean

**Droplet:**

```bash
# Create via CLI
doctl compute droplet create pied-piper \
  --image ubuntu-22-04-x64 \
  --size s-2vcpu-2gb \
  --region nyc3 \
  --ssh-keys YOUR_SSH_KEY_ID

# Or use One-Click Docker Droplet
# Then deploy via Docker Compose
```

**App Platform:**

Create `app.yaml`:
```yaml
name: pied-piper
services:
  - name: api
    github:
      repo: rootkill-g/pied-piper
      branch: main
    dockerfile_path: Dockerfile
    http_port: 8080
    instance_count: 1
    instance_size_slug: basic-xxs
    routes:
      - path: /
```

Deploy:
```bash
doctl apps create --spec app.yaml
```

## Kubernetes

**Deployment:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pied-piper
  labels:
    app: pied-piper
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pied-piper
  template:
    metadata:
      labels:
        app: pied-piper
    spec:
      containers:
      - name: pied-piper
        image: pied-piper:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 4001
          name: p2p-tcp
        - containerPort: 4002
          name: p2p-udp
          protocol: UDP
        env:
        - name: RUST_LOG
          value: "info,pied_piper=debug"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: data
          mountPath: /var/lib/pied-piper
        - name: config
          mountPath: /etc/pied-piper
          readOnly: true
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: pied-piper-data
      - name: config
        configMap:
          name: pied-piper-config
---
apiVersion: v1
kind: Service
metadata:
  name: pied-piper
spec:
  selector:
    app: pied-piper
  type: LoadBalancer
  ports:
  - name: http
    port: 80
    targetPort: 8080
  - name: https
    port: 443
    targetPort: 8443
  - name: p2p-tcp
    port: 4001
    targetPort: 4001
  - name: p2p-udp
    port: 4002
    targetPort: 4002
    protocol: UDP
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: pied-piper-config
data:
  config.yaml: |
    network:
      tcp_port: 4001
      quic_port: 4002
    gateway:
      port: 8080
    security:
      rate_limit_per_minute: 120
      max_connections_per_ip: 200
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: pied-piper-data
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

**Deploy:**
```bash
kubectl apply -f pied-piper-deployment.yaml
kubectl get pods -l app=pied-piper
kubectl logs -f deployment/pied-piper
```

## Production Checklist

### Security

- [ ] Enable HTTPS with valid TLS certificate
- [ ] Configure rate limiting appropriately
- [ ] Enable HSTS and security headers
- [ ] Set up firewall rules (allow only necessary ports)
- [ ] Run as non-root user
- [ ] Keep dependencies updated
- [ ] Enable audit logging

### Performance

- [ ] Configure appropriate resource limits (CPU, memory)
- [ ] Enable connection pooling
- [ ] Set up caching (if applicable)
- [ ] Use compression (gzip/brotli)
- [ ] Optimize WASM module sizes

### Reliability

- [ ] Set up health checks
- [ ] Configure automatic restarts
- [ ] Implement graceful shutdown
- [ ] Test failover scenarios
- [ ] Back up DHT state and storage

### Monitoring

- [ ] Set up Prometheus metrics collection
- [ ] Configure Grafana dashboards
- [ ] Set up alerting (PagerDuty, Slack, etc.)
- [ ] Monitor error rates
- [ ] Track P2P peer count

### Networking

- [ ] Configure DNS records
- [ ] Set up reverse proxy (nginx/caddy)
- [ ] Enable HTTP/2
- [ ] Configure WebSocket support
- [ ] Test NAT traversal

### Operations

- [ ] Document deployment procedure
- [ ] Set up CI/CD pipeline
- [ ] Implement rolling updates
- [ ] Test backup/restore procedures
- [ ] Create runbook for common issues

## Monitoring

### Prometheus Metrics

Available at `http://localhost:8080/metrics`:

```promql
# Request rate
rate(http_requests_total[5m])

# Error rate
rate(http_requests_total{status=~"5.."}[5m])

# P2P peers
network_peers_connected

# Cache hit rate
rate(content_cache_hits_total[5m]) / rate(content_fetches_total[5m])

# WebSocket connections
websocket_connections

# WASM execution time
histogram_quantile(0.95, rate(wasm_execution_duration_seconds_bucket[5m]))
```

### Grafana Dashboard

Import the provided dashboard from `grafana/dashboards/pied-piper.json` or create custom panels:

**Key Metrics:**
- HTTP request rate and latency
- Error rates by endpoint
- P2P peer count and DHT stats
- WASM execution metrics
- Storage size and operations
- WebSocket connections

### Logging

**Structured Logging:**
```yaml
logging:
  level: info  # trace, debug, info, warn, error
  json_format: true  # JSON for log aggregation
  file_path: /var/log/pied-piper/app.log
```

**Log Aggregation:**
Use tools like:
- Loki (Grafana)
- Elasticsearch + Kibana (ELK)
- Datadog
- CloudWatch (AWS)

## Troubleshooting

### Port Already in Use

```bash
# Find process using port
sudo lsof -i :8080
sudo netstat -tulpn | grep 8080

# Kill process
sudo kill -9 <PID>
```

### Permission Denied

```bash
# Check file permissions
ls -la /var/lib/pied-piper

# Fix ownership
sudo chown -R pied-piper:pied-piper /var/lib/pied-piper

# Fix permissions
sudo chmod 755 /var/lib/pied-piper
```

### High Memory Usage

```bash
# Check memory
free -h

# Monitor process
top -p $(pgrep pied-piper)

# Reduce cache size in config
storage:
  max_cache_size_bytes: 268435456  # 256MB instead of 512MB
```

### P2P Connection Issues

```bash
# Check firewall
sudo ufw status
sudo firewall-cmd --list-all

# Allow ports
sudo ufw allow 4001/tcp
sudo ufw allow 4002/udp

# Check connectivity
nc -zv <peer-ip> 4001
```

### Certificate Errors

```bash
# Check certificate validity
openssl x509 -in cert.pem -text -noout

# Check certificate expiry
openssl x509 -in cert.pem -noout -dates

# Renew Let's Encrypt
sudo certbot renew
```

### Slow Response Times

```bash
# Check metrics
curl http://localhost:8080/metrics | grep duration

# Increase timeouts
gateway:
  request_timeout_secs: 60

# Enable debug logging
RUST_LOG=debug pied-piper gateway
```

## Support

For deployment issues:
- Check documentation: https://github.com/rootkill-g/pied-piper/docs
- Open an issue: https://github.com/rootkill-g/pied-piper/issues
- Community forum: (TBD)

---

**Last Updated:** December 22, 2025  
**Version:** 0.5.0
