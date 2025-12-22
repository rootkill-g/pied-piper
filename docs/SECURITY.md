# Security Hardening Guide

This document describes the comprehensive security features implemented in Pied Piper and best practices for deploying securely.

## Table of Contents

- [Overview](#overview)
- [Security Features](#security-features)
  - [Rate Limiting](#rate-limiting)
  - [Request Validation](#request-validation)
  - [DDoS Protection](#ddos-protection)
  - [Security Headers](#security-headers)
  - [Connection Management](#connection-management)
- [Configuration](#configuration)
- [Testing](#testing)
- [Best Practices](#best-practices)
- [Threat Model](#threat-model)

## Overview

Pied Piper implements defense-in-depth security with multiple layers of protection:

1. **Rate Limiting**: Token bucket algorithm prevents abuse
2. **Request Validation**: Input sanitization and validation
3. **DDoS Protection**: Connection limits and timeouts
4. **Security Headers**: Modern web security headers
5. **TLS/HTTPS**: Encrypted communications
6. **WASM Sandboxing**: Isolated execution environment

All security features are enabled by default and can be configured via `config.yaml`.

## Security Features

### Rate Limiting

**Implementation**: Token bucket algorithm with per-IP tracking

**Features**:
- Per-IP rate limiting (default: 60 requests/minute)
- Configurable burst size (default: 10 requests)
- Automatic token refill over time
- Background cleanup of stale buckets
- Health endpoints (`/health`, `/ready`) exempt from rate limiting

**Configuration**:
```yaml
security:
  rate_limit_per_minute: 60    # Requests per minute per IP
  rate_limit_burst: 10          # Burst capacity
```

**How it works**:
```
Client makes request → Check IP's token bucket → 
  If tokens available: Allow (consume 1 token)
  If no tokens: 429 Too Many Requests
  
Tokens refill at rate_limit_per_minute / 60 per second
```

**Testing**:
```bash
# Test rate limiting
for i in {1..70}; do 
  curl http://localhost:8080/app/test
done
# After 60 requests, you'll see 429 responses
```

### Request Validation

**Path Validation**:
- ✅ Prevents path traversal (`../`, `./`, `\\`)
- ✅ Blocks null bytes (`\0`)
- ✅ Detects suspicious patterns (`/etc/`, `/proc/`, `/sys/`)
- ✅ Enforces maximum path depth (default: 10 segments)
- ✅ Validates against directory separators

**Header Validation**:
- ✅ Maximum header size limit (default: 8KB total)
- ✅ Detects header injection (`\r\n` in values)
- ✅ Blocks suspicious user agents (security scanners)
  - sqlmap, nikto, nmap, burp, metasploit, etc.

**File Extension Validation**:
- Whitelist-based approach
- Allowed extensions (default):
  - Web: html, css, js, json
  - WASM: wasm
  - Images: png, jpg, jpeg, gif, svg, ico
  - Fonts: woff, woff2, ttf, otf
  - Docs: txt, md

**Body Size Limiting**:
- Maximum request body size: 16MB (configurable)
- Enforced at middleware layer via `RequestBodyLimitLayer`

**Configuration**:
```yaml
security:
  max_request_body_size: 16777216  # 16MB in bytes
  max_header_size: 8192             # 8KB in bytes
  max_path_depth: 10
  block_suspicious_user_agents: true
  allowed_extensions:
    - html
    - css
    - js
    - wasm
    - json
    # ... more extensions
```

**Example Blocked Requests**:
```bash
# Path traversal
curl http://localhost:8080/../etc/passwd
# Response: 403 Forbidden

# Suspicious user agent
curl -A "sqlmap/1.0" http://localhost:8080/
# Response: 403 Forbidden

# Disallowed extension
curl http://localhost:8080/script.php
# Response: 403 Forbidden
```

### DDoS Protection

**Connection Tracking**:
- Per-IP concurrent connection limits (default: 100)
- Global concurrent request limits (default: 10,000)
- Automatic connection cleanup on request completion
- Real-time connection counting

**Request Timeouts**:
- Request timeout: 30 seconds (configurable)
- Prevents slowloris attacks
- Enforced via `TimeoutLayer` middleware

**Configuration**:
```yaml
security:
  max_connections_per_ip: 100
  max_concurrent_requests: 10000

gateway:
  request_timeout_secs: 30
```

**How it works**:
```
New request → Register connection (check limits) →
  If within limits: Allow
  If exceeds IP limit: 429 Too Many Requests
  If exceeds global limit: 503 Service Unavailable
  
On completion → Unregister connection
```

**Metrics**:
Monitor connection stats:
```bash
curl http://localhost:8080/metrics | grep connection
```

### Security Headers

**Implemented Headers**:

1. **Content-Security-Policy (CSP)**
   ```
   default-src 'self'; 
   script-src 'self' 'unsafe-inline' 'unsafe-eval'; 
   style-src 'self' 'unsafe-inline'; 
   img-src 'self' data: https:; 
   font-src 'self' data:; 
   connect-src 'self'; 
   frame-ancestors 'self'; 
   base-uri 'self'; 
   form-action 'self'
   ```
   - Prevents XSS attacks
   - Restricts resource loading
   - Configurable via `enable_strict_csp`

2. **X-Content-Type-Options**
   ```
   X-Content-Type-Options: nosniff
   ```
   - Prevents MIME type sniffing
   - Always enabled

3. **X-Frame-Options**
   ```
   X-Frame-Options: SAMEORIGIN
   ```
   - Prevents clickjacking
   - Only allows same-origin framing

4. **Referrer-Policy**
   ```
   Referrer-Policy: strict-origin-when-cross-origin
   ```
   - Controls referrer information
   - Protects privacy

5. **X-XSS-Protection**
   ```
   X-XSS-Protection: 1; mode=block
   ```
   - Legacy XSS protection
   - Blocks on detection

6. **Strict-Transport-Security (HSTS)** *(HTTPS only)*
   ```
   Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
   ```
   - Forces HTTPS
   - Prevents downgrade attacks
   - Configurable max-age

**Configuration**:
```yaml
security:
  enable_hsts: true
  hsts_max_age: 31536000  # 1 year
  enable_strict_csp: true
```

**Testing**:
```bash
curl -I http://localhost:8080/
# Check for security headers in response
```

### Connection Management

**Features**:
- Automatic connection tracking per IP
- Real-time connection counting
- Graceful connection cleanup
- Background maintenance tasks

**Implementation Details**:
- Uses `Arc<RwLock<HashMap>>` for thread-safe tracking
- Separate per-IP and global counters
- No memory leaks (connections always cleaned up)
- Health endpoints don't count toward limits

**Monitoring**:
```rust
// Check current connections
let count = connection_tracker.get_connection_count(ip).await;
let total = connection_tracker.get_total_connections().await;
```

## Configuration

### Complete Security Configuration Example

```yaml
security:
  # Request size limits
  max_request_body_size: 16777216  # 16MB
  max_header_size: 8192            # 8KB
  
  # Rate limiting
  rate_limit_per_minute: 60
  rate_limit_burst: 10
  
  # Connection limits
  max_connections_per_ip: 100
  max_concurrent_requests: 10000
  
  # Security headers
  enable_hsts: true
  hsts_max_age: 31536000
  enable_strict_csp: true
  
  # Request validation
  block_suspicious_user_agents: true
  max_path_depth: 10
  
  # CORS (empty = disabled)
  cors_allowed_origins: []
  
  # File extension whitelist
  allowed_extensions:
    - html
    - css
    - js
    - wasm
    - json
    - png
    - jpg
    - jpeg
    - gif
    - svg
    - ico
    - woff
    - woff2
    - ttf
    - otf
    - txt
    - md

gateway:
  request_timeout_secs: 30
```

### Production vs Development

**Production** (strict):
```yaml
security:
  rate_limit_per_minute: 60
  rate_limit_burst: 5
  max_connections_per_ip: 50
  block_suspicious_user_agents: true
  enable_hsts: true
  enable_strict_csp: true
```

**Development** (relaxed):
```yaml
security:
  rate_limit_per_minute: 600
  rate_limit_burst: 100
  max_connections_per_ip: 1000
  block_suspicious_user_agents: false
  enable_hsts: false
  enable_strict_csp: false
```

## Testing

### Unit Tests

All security features have comprehensive unit tests:

```bash
cargo test security
```

**Test Coverage**:
- ✅ Token bucket algorithm
- ✅ Rate limiter per-IP tracking
- ✅ Connection tracker limits
- ✅ Path validation (traversal, null bytes, depth)
- ✅ Extension validation (whitelist)
- ✅ Header validation (size, injection, user agents)
- ✅ Path sanitization
- ✅ Configuration defaults

### Integration Testing

Test rate limiting:
```bash
# Burst test
for i in {1..70}; do curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/; done

# Expected: First 60 return 200, rest return 429
```

Test path validation:
```bash
# Should be blocked
curl http://localhost:8080/../../../etc/passwd
curl http://localhost:8080/path/../admin
curl http://localhost:8080/file%00.txt

# Should work
curl http://localhost:8080/app/test
curl http://localhost:8080/static/app.js
```

Test connection limits:
```bash
# Stress test with Apache Bench
ab -n 10000 -c 200 http://localhost:8080/

# Monitor connections
curl http://localhost:8080/metrics | grep connection
```

### Load Testing

Use tools like `wrk`, `apache bench`, or `bombardier`:

```bash
# wrk
wrk -t4 -c100 -d30s http://localhost:8080/

# apache bench
ab -n 10000 -c 100 http://localhost:8080/

# bombardier
bombardier -c 100 -n 10000 http://localhost:8080/
```

## Best Practices

### 1. Always Use HTTPS in Production

```yaml
gateway:
  https_port: 8443
  tls_cert_path: /path/to/cert.pem
  tls_key_path: /path/to/key.pem
```

Generate certificate:
```bash
# Self-signed (testing only)
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem -out cert.pem \
  -days 365 -subj '/CN=localhost'

# Production: Use Let's Encrypt
certbot certonly --standalone -d yourdomain.com
```

### 2. Monitor Metrics

Enable Prometheus metrics and monitor:
- `http_requests_total` - Request rate
- Connection counts
- Rate limit violations
- Error rates

```bash
curl http://localhost:8080/metrics
```

### 3. Adjust Limits for Your Traffic

Start conservative, then tune based on metrics:

**Low traffic** (< 1000 req/hour):
```yaml
rate_limit_per_minute: 30
max_connections_per_ip: 50
```

**Medium traffic** (1000-10000 req/hour):
```yaml
rate_limit_per_minute: 60
max_connections_per_ip: 100
```

**High traffic** (> 10000 req/hour):
```yaml
rate_limit_per_minute: 120
max_connections_per_ip: 200
```

### 4. Use Reverse Proxy

Deploy behind nginx/caddy for additional protection:

**Nginx example**:
```nginx
server {
    listen 443 ssl http2;
    server_name example.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # Additional rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req zone=api burst=20 nodelay;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### 5. Regular Updates

- Update Rust dependencies regularly
- Monitor security advisories
- Rotate TLS certificates before expiry
- Review logs for suspicious activity

### 6. Defense in Depth

Combine multiple security layers:
1. Firewall (iptables/ufw)
2. Reverse proxy (nginx/caddy)
3. Pied Piper security features
4. Network isolation
5. Monitoring and alerting

## Threat Model

### What Pied Piper Protects Against

✅ **Rate Limiting**:
- Brute force attacks
- Credential stuffing
- API abuse
- Resource exhaustion

✅ **Request Validation**:
- Path traversal (LFI/RFI)
- Directory listing
- Null byte injection
- Header injection
- Malicious user agents

✅ **DDoS Protection**:
- Connection exhaustion
- Slowloris attacks
- Request flooding
- Resource starvation

✅ **Security Headers**:
- XSS attacks
- Clickjacking
- MIME sniffing
- Protocol downgrade

✅ **WASM Sandboxing**:
- Arbitrary code execution
- Memory corruption
- File system access
- Network access

### What Requires Additional Protection

⚠️ **Application Logic**: Pied Piper secures the gateway, but WASM modules must validate their own business logic

⚠️ **Database Security**: If WASM modules use databases, secure them separately

⚠️ **Network Layer**: Use firewalls, VPNs, and network segmentation

⚠️ **Authentication/Authorization**: Implement in your WASM modules or proxy

⚠️ **Zero-Day Vulnerabilities**: Keep dependencies updated

## Security Checklist

Before deploying to production:

- [ ] Enable HTTPS with valid certificate
- [ ] Configure appropriate rate limits for your traffic
- [ ] Enable HSTS header
- [ ] Enable strict CSP
- [ ] Set up monitoring and alerting
- [ ] Deploy behind reverse proxy (nginx/caddy)
- [ ] Configure firewall rules
- [ ] Test all security features
- [ ] Review WASM module permissions
- [ ] Set up log aggregation
- [ ] Document incident response plan
- [ ] Regular security audits
- [ ] Penetration testing

## Incident Response

If you suspect a security issue:

1. **Immediate**: Check metrics for anomalies
   ```bash
   curl http://localhost:8080/metrics
   ```

2. **Investigate**: Review logs for suspicious patterns
   ```bash
   journalctl -u pied-piper | grep -E "403|429|Rate limit"
   ```

3. **Mitigate**: Temporarily tighten limits
   ```yaml
   security:
     rate_limit_per_minute: 10
     max_connections_per_ip: 10
   ```

4. **Block**: Use firewall to block attacking IPs
   ```bash
   ufw deny from <attacker-ip>
   ```

5. **Report**: If it's a vulnerability in Pied Piper, report via GitHub Security Advisories

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [OWASP Secure Headers Project](https://owasp.org/www-project-secure-headers/)
- [Mozilla Web Security Guidelines](https://infosec.mozilla.org/guidelines/web_security)
- [Rust Security Advisory Database](https://rustsec.org/)

## Contact

For security concerns, please open a GitHub issue or contact the maintainers directly.
