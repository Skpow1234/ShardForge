# Multi-stage Docker build for ShardForge
FROM rust:1.80-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/shardforge

    # Copy dependency manifests
    COPY Cargo.toml ./

# Copy source code
COPY src/ src/
COPY shardforge-core/src/ shardforge-core/src/
COPY shardforge-config/src/ shardforge-config/src/
COPY shardforge-storage/src/ shardforge-storage/src/
COPY shardforge-cli/src/ shardforge-cli/src/
COPY bin/ bin/

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash shardforge

# Copy binary from builder stage
COPY --from=builder /usr/src/shardforge/target/x86_64-unknown-linux-gnu/release/shardforge /usr/local/bin/

# Create data directories
RUN mkdir -p /data/shardforge && chown -R shardforge:shardforge /data/shardforge

# Switch to non-root user
USER shardforge
WORKDIR /data/shardforge

# Expose ports
EXPOSE 5432 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD shardforge status --format json | grep -q '"status":"healthy"'

# Default command
CMD ["shardforge", "start"]
