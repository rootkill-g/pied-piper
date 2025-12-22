# Multi-stage build for Pied Piper
FROM rust:1.92-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --bin pied-piper

# Runtime stage
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 pied-piper

# Create directories
RUN mkdir -p /home/pied-piper/.pied-piper/storage \
    /home/pied-piper/.pied-piper/cache \
    && chown -R pied-piper:pied-piper /home/pied-piper

# Copy binary from builder
COPY --from=builder /app/target/release/pied-piper /usr/local/bin/pied-piper

# Copy example config
COPY config.example.yaml /home/pied-piper/config.yaml

# Switch to non-root user
USER pied-piper
WORKDIR /home/pied-piper

# Expose ports
# 8080 - HTTP Gateway
# 4001 - libp2p TCP
# 4002 - libp2p QUIC
EXPOSE 8080 4001 4002/udp

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/pied-piper", "health-check", "||", "exit", "1"]

# Default command
CMD ["pied-piper", "gateway", "--listen", "0.0.0.0:8080"]
