# Deployment & Operations

## 1. Deployment Models

### Single-Node Development

```bash
# Quick start for development
shardforge init --single-node --data-dir ./data
shardforge start --config shardforge.toml
```

### Multi-Node Production

```bash
# Initialize first node
shardforge cluster init --node-id node-1 --bind 10.0.0.1:5432

# Add additional nodes
shardforge cluster join --cluster 10.0.0.1:5432 --node-id node-2 --bind 10.0.0.2:5432
shardforge cluster join --cluster 10.0.0.1:5432 --node-id node-3 --bind 10.0.0.3:5432
```

## 2. Monitoring Integration

### Metrics Export

- **Prometheus** metrics endpoint
- **StatsD** protocol support
- **OpenTelemetry** distributed tracing
- **Custom dashboard** templates for Grafana

### Key Metrics

```rust
pub struct ClusterMetrics {
    // Performance metrics
    pub queries_per_second: Counter,
    pub query_latency: Histogram,
    pub transaction_throughput: Counter,

    // Cluster health
    pub node_count: Gauge,
    pub leader_elections: Counter,
    pub network_partitions: Counter,

    // Storage metrics
    pub disk_usage: Gauge,
    pub compaction_time: Histogram,
    pub cache_hit_rate: Gauge,
}
```
