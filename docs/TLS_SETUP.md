# TLS/HTTPS Gateway Setup Guide

This guide explains how to configure TLS/HTTPS for the Pied Piper gateway.

## Quick Start

### 1. Generate Self-Signed Certificate (Development)

For local development and testing, you can use a self-signed certificate:

```bash
# Create certificate directory
mkdir -p ~/.pied-piper/certs

# Generate certificate and private key (valid for 365 days)
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout ~/.pied-piper/certs/key.pem \
  -out ~/.pied-piper/certs/cert.pem \
  -days 365 -subj "/CN=localhost"
```

**Note**: Self-signed certificates will show security warnings in browsers. This is expected for development.

### 2. Start Gateway with TLS

```bash
# Use default certificate paths (~/.pied-piper/certs/)
cargo run --release -- gateway --tls

# Or specify custom paths
cargo run --release -- gateway \
  --tls \
  --tls-cert /path/to/cert.pem \
  --tls-key /path/to/key.pem \
  --https-listen 8443
```

### 3. Test the Gateway

```bash
# HTTP (port 8080)
curl http://localhost:8080/health

# HTTPS (port 8443)
curl --insecure https://localhost:8443/health
```

The `--insecure` flag is needed for self-signed certificates. Remove it when using production certificates.

---

## Architecture

### Dual Server Model

The gateway runs both HTTP and HTTPS servers concurrently:

- **HTTP Server**: Port 8080 (default, configurable with `--listen`)
- **HTTPS Server**: Port 8443 (default, configurable with `--https-listen`)

Both servers share the same routes and functionality. You can:
- Use both protocols simultaneously
- Disable HTTP by not exposing port 8080
- Configure different ports as needed

### TLS Implementation

- **Library**: `rustls` (pure Rust TLS 1.2/1.3 implementation)
- **Server**: `axum-server` with TLS support
- **Certificate Format**: PEM (standard for OpenSSL/Let's Encrypt)

---

## Production Deployment

### Using Let's Encrypt Certificates

For production, use Let's Encrypt for free, trusted certificates:

#### 1. Install Certbot

```bash
# macOS
brew install certbot

# Ubuntu/Debian
sudo apt-get install certbot

# CentOS/RHEL
sudo yum install certbot
```

#### 2. Generate Certificates

```bash
# Standalone mode (requires port 80 access)
sudo certbot certonly --standalone -d yourdomain.com

# Certificates will be in:
# /etc/letsencrypt/live/yourdomain.com/fullchain.pem
# /etc/letsencrypt/live/yourdomain.com/privkey.pem
```

#### 3. Copy Certificates

```bash
# Copy to pied-piper directory
sudo cp /etc/letsencrypt/live/yourdomain.com/fullchain.pem ~/.pied-piper/certs/cert.pem
sudo cp /etc/letsencrypt/live/yourdomain.com/privkey.pem ~/.pied-piper/certs/key.pem
sudo chown $USER:$USER ~/.pied-piper/certs/*.pem
```

#### 4. Start Gateway

```bash
cargo run --release -- gateway --tls
```

#### 5. Certificate Renewal

Let's Encrypt certificates expire after 90 days. Set up automatic renewal:

```bash
# Test renewal
sudo certbot renew --dry-run

# Add to crontab (runs daily)
echo "0 0 * * * certbot renew --quiet && cp /etc/letsencrypt/live/yourdomain.com/*.pem ~/.pied-piper/certs/ && systemctl restart pied-piper" | crontab -
```

**Note**: Current version requires manual server restart after certificate renewal. Automatic reload is planned for a future release.

---

## Configuration Options

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--tls` | disabled | Enable TLS/HTTPS support |
| `--tls-cert <PATH>` | `~/.pied-piper/certs/cert.pem` | Path to TLS certificate file (PEM format) |
| `--tls-key <PATH>` | `~/.pied-piper/certs/key.pem` | Path to TLS private key file (PEM format) |
| `--https-listen <PORT>` | `8443` | HTTPS server port |
| `--listen <PORT>` | `8080` | HTTP server port |

### Example Configurations

#### Development (Self-Signed)
```bash
cargo run -- gateway --tls
```

#### Production (Custom Ports)
```bash
cargo run --release -- gateway \
  --tls \
  --listen 80 \
  --https-listen 443 \
  --tls-cert /etc/ssl/certs/pied-piper.pem \
  --tls-key /etc/ssl/private/pied-piper.key
```

#### HTTP Only (No TLS)
```bash
cargo run -- gateway
# Only port 8080 will be used
```

#### HTTPS Only (Disable HTTP)
```bash
# Start with TLS
cargo run -- gateway --tls

# Use firewall to block port 8080
sudo ufw deny 8080
```

---

## Troubleshooting

### Certificate File Not Found

```
Error: TLS certificate file not found at /Users/user/.pied-piper/certs/cert.pem
```

**Solution**: Generate certificates using the Quick Start instructions above.

### Permission Denied

```
Error: Permission denied (os error 13)
```

**Solution**: Ensure certificate files are readable:
```bash
chmod 644 ~/.pied-piper/certs/cert.pem
chmod 600 ~/.pied-piper/certs/key.pem
```

### Port Already in Use

```
Error: Address already in use (os error 48)
```

**Solution**: Another process is using port 8080 or 8443:
```bash
# Find process using port
lsof -i :8443

# Kill process
kill -9 <PID>

# Or use different port
cargo run -- gateway --tls --https-listen 9443
```

### Browser Certificate Warning

```
Your connection is not private (NET::ERR_CERT_AUTHORITY_INVALID)
```

**For Development**: Click "Advanced" → "Proceed to localhost (unsafe)" - this is normal for self-signed certificates.

**For Production**: Use Let's Encrypt or a trusted CA certificate.

### Certificate Expired

Let's Encrypt certificates expire after 90 days:

```bash
# Check expiration
openssl x509 -in ~/.pied-piper/certs/cert.pem -noout -enddate

# Renew with certbot
sudo certbot renew

# Copy new certificates
sudo cp /etc/letsencrypt/live/yourdomain.com/*.pem ~/.pied-piper/certs/

# Restart gateway
```

---

## Security Best Practices

### Certificate Management
- ✅ Use Let's Encrypt for production (free, trusted, auto-renewable)
- ✅ Keep private keys secure (`chmod 600` on key.pem)
- ✅ Never commit certificates to version control
- ✅ Rotate certificates before expiration
- ✅ Monitor certificate expiration dates

### TLS Configuration
- ✅ TLS 1.2 and 1.3 enabled (rustls default)
- ✅ Strong cipher suites (rustls default)
- ✅ No SSL 3.0 or TLS 1.0/1.1 (deprecated, insecure)
- 🔄 HSTS headers (planned for future release)
- 🔄 OCSP stapling (planned for future release)

### Network Security
- Use firewall rules to restrict access
- Consider using a reverse proxy (nginx, Caddy) for additional features
- Enable rate limiting for public endpoints
- Use VPN or IP whitelisting for internal services

---

## Future Enhancements

### Planned Features
- **ACME Protocol Support**: Automatic Let's Encrypt certificate provisioning
- **Automatic Renewal**: Certificate renewal without server restart
- **HTTP → HTTPS Redirect**: Configurable redirect from HTTP to HTTPS
- **HSTS Headers**: HTTP Strict Transport Security support
- **SNI Support**: Multiple domains on single server
- **OCSP Stapling**: Certificate revocation status
- **Certificate Monitoring**: Expiration alerts and notifications

### Contributing
If you'd like to help implement these features, see the [Contributing Guide](../README.md#contributing).

---

## References

- [rustls Documentation](https://docs.rs/rustls/)
- [Let's Encrypt User Guide](https://letsencrypt.org/docs/)
- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [TLS Best Practices](https://github.com/ssllabs/research/wiki/SSL-and-TLS-Deployment-Best-Practices)

---

## Support

For issues or questions:
1. Check [docs/TESTING_GUIDE.md](./TESTING_GUIDE.md) for testing procedures
2. Review [PHASE_3A_PROGRESS.md](../PHASE_3A_PROGRESS.md) for implementation details
3. Open an issue on GitHub with:
   - Your configuration
   - Certificate generation commands used
   - Full error messages
   - Output of `openssl version`
