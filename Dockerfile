# Optimized multi-stage Docker build for ShardForge
FROM rust:1.90-bookworm AS builder

WORKDIR /usr/src/shardforge

# Copy manifests first for better caching
COPY Cargo.toml ./
COPY shardforge-core/Cargo.toml ./shardforge-core/
COPY shardforge-config/Cargo.toml ./shardforge-config/
COPY shardforge-storage/Cargo.toml ./shardforge-storage/

# Create dummy main files to build dependencies
RUN mkdir -p src bin && \
    echo "fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > bin/shardforge.rs && \
    mkdir -p shardforge-core/src && echo "fn main() {}" > shardforge-core/src/lib.rs && \
    mkdir -p shardforge-config/src && echo "fn main() {}" > shardforge-config/src/lib.rs && \
    mkdir -p shardforge-storage/src && echo "fn main() {}" > shardforge-storage/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release

# Remove dummy files
RUN rm -rf src bin shardforge-core/src shardforge-config/src shardforge-storage/src

# Copy actual source code
COPY src/ src/
COPY bin/ bin/
COPY shardforge-core/src/ shardforge-core/src/
COPY shardforge-config/src/ shardforge-config/src/
COPY shardforge-storage/src/ shardforge-storage/src/
COPY proto/ proto/
COPY build.rs ./

# Build the final application (only our code, deps already built)
RUN touch src/lib.rs && cargo build --release

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd --create-home --shell /bin/bash --uid 1000 shardforge

# Copy binary from builder stage
COPY --from=builder /usr/src/shardforge/target/release/shardforge /usr/local/bin/shardforge

# Set proper permissions and create directories
RUN mkdir -p /data/shardforge && chown -R shardforge:shardforge /data/shardforge

# Switch to non-root user
USER shardforge
WORKDIR /data/shardforge

# Expose ports
EXPOSE 5432 9090

# Optimized health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD shardforge --version > /dev/null || exit 1

# Set entrypoint
ENTRYPOINT ["shardforge"]
CMD ["start"]
