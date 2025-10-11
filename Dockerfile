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
# Copy Cargo.toml first
COPY Cargo.toml ./
COPY shardforge-core/Cargo.toml ./shardforge-core/
COPY shardforge-config/Cargo.toml ./shardforge-config/
COPY shardforge-storage/Cargo.toml ./shardforge-storage/

# Try to copy Cargo.lock, but don't fail if it doesn't exist
COPY Cargo.lock* ./

# Ensure Cargo.lock exists (generate if missing)
RUN if [ ! -f Cargo.lock ]; then \
        echo "⚠️ Cargo.lock not found in build context, generating it..." && \
        cargo generate-lockfile; \
    else \
        echo "✅ Cargo.lock found in build context"; \
    fi

# Verify files were copied correctly and show build context
RUN echo "=== Build Context Verification ===" && \
    ls -la && \
    echo "=== Cargo files ===" && \
    ls -la Cargo.toml Cargo.lock && \
    echo "=== Workspace members ===" && \
    ls -la shardforge-*/ && \
    echo "=== Cargo.lock content check ===" && \
    head -5 Cargo.lock && \
    echo "=== Cargo.lock size ===" && \
    wc -l Cargo.lock

# Create dummy source files to build dependencies first (caching optimization)
RUN mkdir -p src bin shardforge-core/src shardforge-config/src shardforge-storage/src && \
    echo "fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > bin/shardforge.rs && \
    echo "fn main() {}" > shardforge-core/src/lib.rs && \
    echo "fn main() {}" > shardforge-config/src/lib.rs && \
    echo "fn main() {}" > shardforge-storage/src/lib.rs

# Build dependencies (cached layer) - use sled features for compatibility
RUN cargo build --release --bin shardforge --features sled --no-default-features

# Remove dummy files and copy actual source
RUN rm -rf src bin shardforge-core/src shardforge-config/src shardforge-storage/src
COPY . .

# Build final binary (only our code changes, deps already cached)
RUN cargo build --release --bin shardforge --features sled --no-default-features

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