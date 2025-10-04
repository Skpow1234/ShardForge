# ShardForge: Distributed Database System

ShardForge is an ambitious distributed database system built in Rust, designed to provide ACID transactions, SQL compatibility, and horizontal scalability through RAFT consensus and intelligent sharding. The initial release will be CLI-focused, targeting enterprise customers who need high-performance, reliable distributed data storage.

[![Rust](https://img.shields.io/badge/Rust-1.75+-000000.svg?style=for-the-badge&logo=rust)](https://rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-Async%20Runtime-000000.svg?style=for-the-badge&logo=tokio)](https://tokio.rs/)
[![gRPC](https://img.shields.io/badge/gRPC-Protocol%20Buffers-244c5a.svg?style=for-the-badge&logo=grpc)](https://grpc.io/)
[![RocksDB](https://img.shields.io/badge/RocksDB-Storage-orange.svg?style=for-the-badge&logo=rocksdb)](https://rocksdb.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-SQL%20Compatible-4169e1.svg?style=for-the-badge&logo=postgresql)](https://postgresql.org/)
[![Prometheus](https://img.shields.io/badge/Prometheus-Metrics-e6522c.svg?style=for-the-badge&logo=prometheus)](https://prometheus.io/)
[![Docker](https://img.shields.io/badge/Docker-Containerized-2496ed.svg?style=for-the-badge&logo=docker)](https://docker.com/)
[![Kubernetes](https://img.shields.io/badge/Kubernetes-Orchestration-326ce5.svg?style=for-the-badge&logo=kubernetes)](https://kubernetes.io/)
[![RAFT](https://img.shields.io/badge/RAFT-Consensus-00add8.svg?style=for-the-badge&logo=raft)](https://raft.github.io/)
[![OpenSSL](https://img.shields.io/badge/OpenSSL-Security-721412.svg?style=for-the-badge&logo=openssl)](https://openssl.org/)

## Quick Start

### Using Docker (Recommended)

**Prerequisites:** [Docker](https://docker.com/get-started) installed on your system.

```bash
# Create data directory
mkdir -p data

# Pull the latest image
docker pull shardforge/shardforge:latest

# Initialize database
docker run --rm -v $(pwd)/data:/data shardforge/shardforge:latest shardforge init --single-node --data-dir /data

# Start the database server
docker run -d --name shardforge-db -v $(pwd)/data:/data -p 5432:5432 shardforge/shardforge:latest shardforge start --config /data/shardforge.toml

# Check status
docker run --rm --network container:shardforge-db shardforge/shardforge:latest shardforge status

# Execute SQL queries
docker run -it --rm --network container:shardforge-db shardforge/shardforge:latest shardforge sql "SELECT 1 as welcome"

# Stop the database
docker stop shardforge-db && docker rm shardforge-db
```

### Build from Source

**Prerequisites:** [Rust 1.80+](https://rustup.rs/) and system dependencies (OpenSSL, pkg-config).

```bash
# Clone and build
git clone https://github.com/shardforge/shardforge.git
cd shardforge

# Build in release mode
cargo build --release

# Initialize database
./target/release/shardforge init --single-node --data-dir ./data

# Start the database server
./target/release/shardforge start --config shardforge.toml

# Check status
./target/release/shardforge status --detailed

# Execute SQL queries
./target/release/shardforge sql "SELECT 1 as welcome"
```

### Development with Docker Compose

For local development with persistent storage:

```bash
# Start development environment
docker-compose up -d

# Initialize database (first time only)
docker-compose exec shardforge shardforge init --single-node --data-dir /data/shardforge

# Check database status
docker-compose exec shardforge shardforge status --detailed

# Execute SQL queries
docker-compose exec shardforge shardforge sql "SELECT 1 as welcome"

# View logs
docker-compose logs -f shardforge

# Stop environment
docker-compose down
```

> **Note:** ShardForge is currently in Phase 1 (Foundation) of development. The CLI and basic infrastructure are implemented, but full database server functionality, SQL engine, and distributed consensus are planned for future phases.

## Current Implementation Status

### Completed

- Project structure and workspace organization
- Storage engine trait abstraction
- In-memory storage engine with full test coverage
- Core type system (NodeId, Key, Value, etc.)
- CLI command structure and argument parsing
- Configuration management framework
- Comprehensive CI/CD pipeline (6 workflows)
- Docker containerization support

### In Progress

- RocksDB storage engine integration
- Sled storage engine integration
- Configuration loading and validation

### Planned (Phase 1)

- SQL parser and query planner
- Transaction manager with MVCC
- Server implementation (gRPC/TCP)
- Query execution engine
- Indexing support (B-tree, hash)

### Future Phases

- RAFT consensus (Phase 2)
- Distributed sharding (Phase 2)
- Enterprise features (Phase 3)

## Documentation

- **[Overview](docs/overview.md)** - Executive summary and market analysis
- **[Architecture](docs/architecture.md)** - System design and component breakdown
- **[Core Functionalities](docs/functionalities.md)** - RAFT consensus, sharding, transactions, SQL support, and CLI
- **[Technical Implementation](docs/implementation.md)** - Network protocols, storage format, configuration, and error handling
- **[Technologies](docs/technologies.md)** - Core Rust crates and dependencies
- **[Performance](docs/performance.md)** - Benchmarking targets and optimization strategies
- **[Security](docs/security.md)** - Authentication, authorization, and data privacy features
- **[Deployment](docs/deployment.md)** - Installation, configuration, and monitoring
- **[Development Roadmap](docs/roadmap.md)** - Phased development plan (24 months)
- **[Testing Strategy](docs/testing.md)** - Unit testing, integration testing, and chaos engineering
- **[Risk Assessment](docs/risk-assessment.md)** - Technical and market risk analysis
- **[Success Metrics](docs/success-metrics.md)** - Goals and key performance indicators
- **[Conclusion](docs/conclusion.md)** - Final thoughts and project outlook

## Key Differentiators

- **Rust Performance**: Memory safety + zero-cost abstractions
- **CLI-First**: Developer-friendly tooling from day one
- **Open Source**: Community-driven development
- **Modern Architecture**: Built for cloud-native from ground up
