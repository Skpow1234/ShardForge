# ShardForge - Optimized Multi-Stage Docker Build
# Production-ready database with caching and security best practices

# Build stage - using Alpine for better security
FROM rust:1.90-alpine AS builder

# Install build dependencies with security updates
RUN apk add --no-cache \
    protobuf-dev \
    protoc \
    pkgconfig \
    openssl-dev \
    ca-certificates \
    musl-dev

WORKDIR /usr/src/shardforge

# Copy workspace configuration for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY shardforge-core/Cargo.toml ./shardforge-core/
COPY shardforge-config/Cargo.toml ./shardforge-config/
COPY shardforge-storage/Cargo.toml ./shardforge-storage/

# Create dummy source files to build dependencies first (caching optimization)
RUN mkdir -p src bin shardforge-core/src shardforge-config/src shardforge-storage/src && \
    echo "fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > bin/shardforge.rs && \
    echo "fn main() {}" > shardforge-core/src/lib.rs && \
    echo "fn main() {}" > shardforge-config/src/lib.rs && \
    echo "fn main() {}" > shardforge-storage/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --bin shardforge

# Remove dummy files and copy actual source
RUN rm -rf src bin shardforge-core/src shardforge-config/src shardforge-storage/src
COPY . .

# Build final binary (only our code changes, deps already cached)
RUN cargo build --release --bin shardforge

# Runtime stage - distroless for maximum security
FROM gcr.io/distroless/cc-debian12

# Copy binary from builder stage with proper ownership
COPY --from=builder --chown=nonroot:nonroot /usr/src/shardforge/target/release/shardforge /shardforge

# Use nonroot user (built into distroless)
USER nonroot:nonroot
WORKDIR /

# Expose standard database ports
EXPOSE 5432 9090

# Health check for container orchestration (distroless doesn't support HEALTHCHECK)
# HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
#     CMD shardforge --version > /dev/null || exit 1

# Set entrypoint and default command
ENTRYPOINT ["/shardforge"]
CMD ["start"]