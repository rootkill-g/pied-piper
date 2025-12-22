# Network Discovery & Peer Connection Guide

## Overview

Pied Piper nodes need to discover each other to form a decentralized network. This guide explains how peer discovery works and how to configure it for different deployment scenarios.

## Discovery Methods

### 1. **mDNS (Local Network Discovery)**

**Use Case:** Development, local testing, same LAN/WiFi network

**How it works:**
- Nodes broadcast presence on local network via multicast DNS
- Automatically discovers peers on same network (within seconds)
- Zero configuration required

**Configuration:**
```yaml
network:
  enable_mdns: true  # Enable for local/dev environments
```

**Limitations:**
- Only works on same local network
- Doesn't work across internet/WAN
- Not suitable for production deployments

---

### 2. **Bootstrap Peers (Production Recommended)**

**Use Case:** Production, internet-wide deployment, public networks

**How it works:**
1. You run 1+ **bootstrap nodes** with static, known addresses
2. New nodes connect to bootstrap nodes first
3. Bootstrap nodes introduce new nodes to the DHT (Distributed Hash Table)
4. After DHT bootstrap, nodes discover each other automatically

**Current Implementation:**
- Bootstrap peers are dialed every 30 seconds (automatic reconnection)
- Peers are added to Kademlia DHT routing table
- Once in DHT, nodes discover others through Kademlia lookups

**Configuration:**
```yaml
network:
  enable_mdns: false  # Disable mDNS for production
  bootstrap_peers:
    # Format: peer_id@multiaddr
    - 12D3KooWRBhwfeP8p3FEj6txR6xZJfPDa1N3vFfN5cNNp5Xr1e4F@/ip4/bootstrap.example.com/tcp/4001
    - 12D3KooWRBhwfeP8p3FEj6txR6xZJfPDa1N3vFfN5cNNp5Xr1e4F@/ip6/2001:db8::1/tcp/4001
    - 12D3KooWAnotherPeer@/ip4/203.0.113.50/tcp/4001
    - 12D3KooWBackupNode@/dns4/backup.example.com/tcp/4001
```

**Bootstrap Node Requirements:**
- **Static IP or DNS**: Must have stable, reachable address
- **High Uptime**: Should be always online
- **Public Ports**: TCP 4001 and UDP 4002 must be accessible
- **Good Bandwidth**: Handles initial connection surge

---

### 3. **DHT (Distributed Hash Table)**

**Use Case:** Automatic peer discovery after bootstrap

**How it works:**
- Kademlia DHT stores network routing information
- Nodes maintain routing table of known peers
- Queries propagate through network to find content/peers
- Self-organizing, no central directory

**Built-in Features:**
- DHT state persistence (survives restarts)
- Automatic re-bootstrap every 30 seconds
- Peer routing table maintenance
- Content provider records

---

## Production Setup Guide

### Step 1: Deploy Bootstrap Nodes

Choose 2-3 reliable servers for bootstrap nodes:

```bash
# On bootstrap-1.example.com
pied-piper gateway \
  --tcp-port 4001 \
  --quic-port 4002 \
  --listen 0.0.0.0:8080

# Note the Peer ID from logs:
# INFO pied_piper::network::node: Local peer ID: 12D3KooWRBhw...
```

Save bootstrap node info:
- Peer ID: `12D3KooWRBhw...`
- Address: `/ip4/203.0.113.10/tcp/4001`

### Step 2: Configure Regular Nodes

Update `config.production.yaml`:

```yaml
network:
  tcp_port: 4001
  quic_port: 4002
  enable_mdns: false  # IMPORTANT: Disable mDNS for production
  
  bootstrap_peers:
    # Add your bootstrap nodes here
    - 12D3KooWRBhwfeP8p3FEj6txR6xZJfPDa1N3vFfN5cNNp5Xr1e4F@/ip4/203.0.113.10/tcp/4001
    - 12D3KooWAnotherPeer@/ip4/203.0.113.11/tcp/4001
    - 12D3KooWBackupNode@/dns4/backup.example.com/tcp/4001
  
  topics:
    - pied-piper-prod
  max_connections: 500
```

### Step 3: Deploy Regular Nodes

```bash
pied-piper --config config.production.yaml gateway
```

**What happens:**
1. Node starts, listens on ports 4001 (TCP) and 4002 (QUIC)
2. Connects to bootstrap peers within seconds
3. Exchanges peer info through DHT
4. Discovers other nodes automatically
5. Maintains connections (re-dials every 30s if needed)

---

## Verifying Connectivity

### Check Local Peer ID

```bash
pied-piper info
# Output: Local Peer ID: 12D3KooW...
```

### Check Connected Peers

```bash
# Via /ready endpoint
curl http://localhost:8080/ready | jq .

# Output:
{
  "ready": true,
  "peer_count": 5,
  "message": "Gateway is ready"
}
```

### Check Metrics

```bash
curl http://localhost:8080/metrics | grep peer

# Output:
# network_connected_peers 5
# network_dht_peers 42
```

### Monitor Logs

```bash
# Look for these log messages:
# ✅ INFO pied_piper::network::node: Connection established with 12D3KooW...
# ✅ INFO libp2p_mdns::behaviour: discovered peer ...
# ✅ INFO pied_piper::network::node: Listening on /ip4/...
```

---

## Common Issues & Solutions

### Issue: "No known peers" Warning

```
WARN pied_piper::network::node: DHT bootstrap skipped: No known peers.
```

**Cause:** No bootstrap peers configured and mDNS disabled

**Solution:**
1. Add bootstrap peers to config
2. OR enable mDNS for local testing
3. Verify bootstrap peer addresses are reachable

### Issue: Can't Connect to Bootstrap Peers

**Troubleshooting:**
```bash
# Test connectivity
nc -zv bootstrap.example.com 4001

# Check firewall
sudo iptables -L -n | grep 4001

# Verify DNS resolution
dig bootstrap.example.com

# Check if bootstrap node is running
curl http://bootstrap.example.com:8080/health
```

**Common Causes:**
- Firewall blocking ports 4001/4002
- Bootstrap node not running
- Wrong peer ID in config
- DNS not resolving
- NAT not configured

### Issue: Peers Connect Then Disconnect

**Causes:**
- Idle timeout too short
- NAT/firewall dropping connections
- Bootstrap node overloaded

**Solutions:**
```yaml
network:
  idle_timeout_secs: 120  # Increase timeout
  max_connections: 500    # Increase limit on bootstrap nodes
```

---

## Network Architecture Examples

### Small Deployment (3-10 nodes)

```
┌─────────────────┐
│  Bootstrap Node │  (1 node, static IP)
│  12D3KooW...    │
└────────┬────────┘
         │
    ┌────┼─────┬──────┐
    │    │     │      │
  ┌─▼─┐ ┌▼──┐ ┌▼───┐ ┌▼───┐
  │N1 │ │N2 │ │N3  │ │... │  (Regular nodes)
  └───┘ └───┘ └────┘ └────┘
```

**Config:** Single bootstrap peer, all nodes connect to it

### Medium Deployment (10-100 nodes)

```
┌──────┐  ┌──────┐  ┌──────┐
│Boot1 │  │Boot2 │  │Boot3 │  (3 bootstrap nodes)
└───┬──┘  └───┬──┘  └───┬──┘
    │    ╱    │    ╲    │
    │   ╱     │     ╲   │
  ┌─▼─┬▼─┬────▼─┬────▼─┬▼──┐
  │N1 │N2│ N3   │ N4   │...│  (Regular nodes)
  └───┴──┴──────┴──────┴───┘
       ╲ DHT Mesh Network ╱
```

**Config:** 3 bootstrap peers, nodes connect to any available

### Large Deployment (100+ nodes)

```
       ┌─────────────┐
       │  DNS Pool   │  (bootstrap.example.com)
       │  Load Bal   │
       └──────┬──────┘
              │
    ┌─────────┼──────────┐
    │         │          │
┌───▼──┐  ┌───▼──┐  ┌───▼──┐
│Boot1 │  │Boot2 │  │Boot3 │  (Bootstrap cluster)
└──────┘  └──────┘  └──────┘
    │         │         │
════════════════════════════════
    │ DHT Network         │
    ├──────────┬──────────┤
  ┌─▼─┐  ┌────▼───┐  ┌───▼──┐
  │N1 │  │Region1 │  │Region│
  │N2 │  │Nodes   │  │ N... │  (100+ nodes)
  └───┘  └────────┘  └──────┘
```

**Config:**
- DNS-based bootstrap peer discovery
- Geographic distribution
- Load balancing across bootstrap nodes

---

## Environment Variables

Quick overrides for testing:

```bash
# Override bootstrap peers
export PP_NETWORK_BOOTSTRAP_PEERS="12D3KooW...@/ip4/1.2.3.4/tcp/4001,12D3KooW...@/ip4/5.6.7.8/tcp/4001"

# Enable mDNS for local testing
export PP_NETWORK_ENABLE_MDNS=true

# Change ports
export PP_NETWORK_TCP_PORT=5001
export PP_NETWORK_QUIC_PORT=5002
```

---

## Best Practices

### For Bootstrap Nodes

✅ **Do:**
- Run on stable infrastructure (cloud VMs, dedicated servers)
- Use static IPs or stable DNS names
- Monitor uptime and connectivity
- Keep bootstrap node list small (2-5 nodes)
- Document peer IDs and addresses
- Set high connection limits

❌ **Don't:**
- Run on dynamic IPs
- Use same nodes for heavy application workloads
- Change peer IDs frequently (breaks configs)
- Forget to open firewall ports

### For Regular Nodes

✅ **Do:**
- Configure at least 2 bootstrap peers
- Test connectivity before deployment
- Monitor peer count metrics
- Enable DHT persistence
- Set appropriate timeouts

❌ **Don't:**
- Enable mDNS in production
- Run without any bootstrap peers
- Forget to open P2P ports in firewall
- Use untrusted bootstrap peers

---

## Security Considerations

### Bootstrap Peer Trust

⚠️ **Important:** Bootstrap peers can:
- See all nodes connecting to network
- Potentially censor connections (but not content)
- Monitor network topology

**Mitigation:**
- Run your own bootstrap nodes
- Use multiple independent bootstrap nodes
- Implement peer reputation system (future work)

### Firewall Configuration

```bash
# Allow inbound P2P connections
sudo ufw allow 4001/tcp comment 'Pied Piper TCP'
sudo ufw allow 4002/udp comment 'Pied Piper QUIC'

# Optional: Restrict to specific IPs
sudo ufw allow from 203.0.113.0/24 to any port 4001 proto tcp
```

---

## Testing Network Discovery

### Local Testing (Same Machine)

```bash
# Terminal 1: Start node with mDNS
pied-piper gateway --tcp-port 4001 --quic-port 4002

# Terminal 2: Start second node
pied-piper gateway --tcp-port 4011 --quic-port 4012

# Both should discover each other via mDNS
# Check: curl http://localhost:8080/ready
```

### Internet Testing

```bash
# Node 1 (bootstrap)
pied-piper gateway --tcp-port 4001 --quic-port 4002
# Note peer ID: 12D3KooWABC...

# Node 2 (anywhere on internet)
pied-piper gateway \
  --tcp-port 4001 \
  --quic-port 4002 \
  --bootstrap "12D3KooWABC...@/ip4/YOUR_PUBLIC_IP/tcp/4001"

# Check connectivity
curl http://localhost:8080/ready | jq .peer_count
```

---

## Future Improvements

Planned enhancements for network discovery:

- [ ] **DHT Bootstrap Servers**: Public bootstrap node infrastructure
- [ ] **Rendezvous Protocol**: Topic-based peer discovery
- [ ] **Peer Exchange (PEX)**: Learn peers from connected peers
- [ ] **DNS Discovery**: Discover peers via DNS TXT records
- [ ] **WebRTC Support**: Browser-to-node connections
- [ ] **Circuit Relay**: Connect nodes behind NAT
- [ ] **Peer Reputation**: Trust-based peer selection

---

## Summary

**For Production:**
1. **Disable mDNS**: Set `enable_mdns: false`
2. **Configure Bootstrap Peers**: Add 2-3 stable nodes
3. **Open Firewall**: Allow TCP 4001 and UDP 4002
4. **Monitor**: Check peer_count via `/ready` endpoint

**Quick Start:**
```yaml
network:
  enable_mdns: false
  bootstrap_peers:
    - PEER_ID@/ip4/BOOTSTRAP_IP/tcp/4001
```

That's it! Once one node connects to a bootstrap peer, it automatically discovers and connects to all other nodes in the network through the DHT. 🎉
