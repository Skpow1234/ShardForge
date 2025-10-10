# ShardForge Docker Guide

## Quick Start

```bash
# Build and start in production mode
make docker-build
make docker-up

# Or use docker-compose directly
docker-compose --profile production up -d
```

## Available Profiles

### 🚀 Production (`--profile production`)

- Single node setup
- Optimized for performance
- Health checks enabled
- Resource limits configured

```bash
docker-compose --profile production up -d
```

### 🔧 Development (`--profile dev`)

- Hot reload enabled
- Debug logging
- Source code mounted
- Development-friendly settings

```bash
docker-compose --profile dev up -d
```

### ⚡ Performance Testing (`--profile perf`)

- High resource allocation
- Performance monitoring
- Optimized for benchmarking

```bash
docker-compose --profile perf up -d
```

### 🌐 Cluster (`--profile cluster`)

- 3-node cluster setup
- Inter-node communication
- Load balancing ready

```bash
docker-compose --profile cluster up -d
```

### 📊 Monitoring (`--profile monitoring`)

- Prometheus metrics collection
- Grafana dashboards
- Observability stack

```bash
docker-compose --profile monitoring up -d
```

## Port Mapping

| Service | Port | Description |
|---------|------|-------------|
| ShardForge (Main) | 5432 | PostgreSQL protocol |
| ShardForge (Main) | 9090 | gRPC API |
| ShardForge (Dev) | 5433 | PostgreSQL protocol |
| ShardForge (Dev) | 9091 | gRPC API |
| ShardForge (Perf) | 5434 | PostgreSQL protocol |
| ShardForge (Perf) | 9092 | gRPC API |
| Node 2 | 5435 | PostgreSQL protocol |
| Node 2 | 9093 | gRPC API |
| Node 3 | 5436 | PostgreSQL protocol |
| Node 3 | 9094 | gRPC API |
| Prometheus | 9095 | Metrics collection |
| Grafana | 3000 | Web dashboard |

## Make Commands

```bash
# Docker operations
make docker-build          # Build Docker image
make docker-up            # Start production mode
make docker-down          # Stop all services
make docker-dev           # Start development mode
make docker-perf          # Start performance mode
make docker-cluster       # Start cluster mode
make docker-monitoring    # Start monitoring stack
make docker-logs          # Show logs
make docker-clean         # Clean up resources
```

## Configuration

### Environment Variables

- `RUST_LOG`: Log level (debug, info, warn, error)
- `RUST_BACKTRACE`: Backtrace level (0, 1, full)
- `SHARDFORGE_NODE_ID`: Unique node identifier
- `SHARDFORGE_CLUSTER_SEEDS`: Cluster seed nodes

### Volumes

- `shardforge-data`: Persistent data storage
- `./shardforge.toml`: Configuration file
- `./monitoring/`: Monitoring configuration

## Health Checks

All services include health checks that verify:

- Container is running
- ShardForge binary responds
- Service is ready to accept connections

## Resource Limits

| Profile | Memory Limit | CPU Limit | Memory Reserve | CPU Reserve |
|---------|-------------|-----------|----------------|-------------|
| Production | 2GB | 1.0 CPU | 512MB | 0.5 CPU |
| Development | 2GB | 1.0 CPU | 512MB | 0.5 CPU |
| Performance | 4GB | 2.0 CPU | 1GB | 1.0 CPU |
| Cluster | 2GB | 1.0 CPU | 512MB | 0.5 CPU |

## Troubleshooting

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f shardforge
```

### Check Health

```bash
# Service status
docker-compose ps

# Health check details
docker inspect shardforge_shardforge_1 | grep -A 10 Health
```

### Clean Restart

```bash
# Stop and remove everything
make docker-clean

# Rebuild and start
make docker-build
make docker-up
```

## Security

### Built-in Security Features

- **Non-root user execution** (UID 1000)
- **Minimal runtime dependencies** (slim base images)
- **Security updates** applied during build
- **Network isolation** with custom bridge
- **Resource limits** to prevent resource exhaustion
- **Health monitoring** for service availability
- **File permissions** properly set (755 for directories, 755 for binary)

### Security Scanning

```bash
# Scan for vulnerabilities
make docker-scan

# Build and scan in one command
make docker-secure

# Manual scanning with Docker Scout
docker scout cves shardforge:latest

# Manual scanning with Trivy
trivy image shardforge:latest
```

### Security Best Practices

1. **Regular updates**: Base images are updated with security patches
2. **Minimal attack surface**: Only essential packages installed
3. **Non-privileged execution**: Runs as non-root user
4. **Resource constraints**: Memory and CPU limits prevent DoS
5. **Network isolation**: Services run in isolated network
6. **Health checks**: Automatic service monitoring

## Performance Tips

1. **Use SSD storage** for data volumes
2. **Allocate sufficient memory** for your workload
3. **Enable monitoring** for production deployments
4. **Use cluster mode** for high availability
5. **Monitor resource usage** with Grafana dashboards
