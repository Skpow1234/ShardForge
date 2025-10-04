# Multi-stage Docker build for ShardForge
# Use Debian bookworm-slim for better security control
FROM debian:bookworm-slim AS builder

# Install Rust and build dependencies with security updates
RUN apt-get update && apt-get upgrade -y && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/shardforge

# Copy dependency manifests
    COPY Cargo.toml ./

# Copy all workspace members
    COPY shardforge-core/ shardforge-core/
    COPY shardforge-config/ shardforge-config/
    COPY shardforge-storage/ shardforge-storage/
    COPY src/ src/
    COPY bin/ bin/

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies with security updates
RUN apt-get update && apt-get upgrade -y && apt-get install -y \
    ca-certificates \
    openssl \
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

# Health check (simple version check until status command is fully implemented)
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD shardforge --version > /dev/null 2>&1 || exit 1

# Default command
CMD ["shardforge", "start"]
