# Deployment & Operations Guide

## 1. Deployment Architectures

### Single-Node Development Setup

**Quick Start for Development**:

```bash
# Initialize single-node cluster
shardforge init --single-node --data-dir ./data --config shardforge.toml

# Start the database
shardforge start --config shardforge.toml

# Connect via CLI
shardforge sql --database default
```

**Development Configuration**:

```toml
[cluster]
name = "dev-cluster"
node_id = "dev-node-1"
data_directory = "./data"
bind_address = "127.0.0.1:5432"

[storage]
engine = "rocksdb"
block_cache_size_mb = 128

[logging]
level = "debug"
```

### Multi-Node Production Clusters

#### Manual Cluster Bootstrap

```bash
# Initialize seed node
shardforge cluster bootstrap \
  --node-id node-001 \
  --bind 10.0.1.10:5432 \
  --advertise 10.0.1.10:5432 \
  --cluster-members "10.0.1.10:5432,10.0.1.11:5432,10.0.1.12:5432" \
  --data-dir /data/shardforge

# Join additional nodes
shardforge cluster join \
  --node-id node-002 \
  --bind 10.0.1.11:5432 \
  --advertise 10.0.1.11:5432 \
  --seed-nodes "10.0.1.10:5432" \
  --data-dir /data/shardforge

shardforge cluster join \
  --node-id node-003 \
  --bind 10.0.1.12:5432 \
  --advertise 10.0.1.12:5432 \
  --seed-nodes "10.0.1.10:5432" \
  --data-dir /data/shardforge
```

#### Automated Cluster Deployment

```bash
# Using deployment scripts
./deploy-cluster.sh \
  --nodes 5 \
  --region us-west-2 \
  --instance-type c5.4xlarge \
  --data-disks 4 \
  --replication-factor 3
```

## 2. Infrastructure as Code

### Terraform Deployment

```hcl
# main.tf
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.0"
    }
  }
}

module "shardforge_cluster" {
  source = "./modules/shardforge-cluster"

  cluster_name    = "production"
  node_count      = 5
  instance_type   = "c5.4xlarge"
  data_volume_size = 1000

  vpc_id          = aws_vpc.main.id
  subnet_ids      = aws_subnet.private[*].id
  security_groups = [aws_security_group.shardforge.id]

  tags = {
    Environment = "production"
    Project     = "shardforge"
  }
}
```

### Kubernetes Manifests

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: shardforge-cluster
  namespace: database
spec:
  serviceName: shardforge
  replicas: 5
  selector:
    matchLabels:
      app: shardforge
  template:
    metadata:
      labels:
        app: shardforge
    spec:
      containers:
      - name: shardforge
        image: shardforge/shardforge:latest
        ports:
        - containerPort: 5432
          name: sql
        - containerPort: 9090
          name: metrics
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: CLUSTER_MEMBERS
          value: "shardforge-0.shardforge.database.svc.cluster.local:5432,shardforge-1.shardforge.database.svc.cluster.local:5432,shardforge-2.shardforge.database.svc.cluster.local:5432,shardforge-3.shardforge.database.svc.cluster.local:5432,shardforge-4.shardforge.database.svc.cluster.local:5432"
        volumeMounts:
        - name: data
          mountPath: /data
        - name: config
          mountPath: /etc/shardforge
          readOnly: true
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "8Gi"
            cpu: "2"
          limits:
            memory: "16Gi"
            cpu: "4"
      volumes:
      - name: config
        configMap:
          name: shardforge-config
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: "fast-ssd"
      resources:
        requests:
          storage: 1Ti
---
apiVersion: v1
kind: Service
metadata:
  name: shardforge-cluster
  namespace: database
spec:
  type: ClusterIP
  ports:
  - port: 5432
    targetPort: 5432
    name: sql
  - port: 9090
    targetPort: 9090
    name: metrics
  selector:
    app: shardforge
```

### Helm Chart

```yaml
# values.yaml
cluster:
  name: production
  replicas: 5

image:
  repository: shardforge/shardforge
  tag: "latest"
  pullPolicy: IfNotPresent

config:
  raft:
    electionTimeoutMs: 150
    heartbeatIntervalMs: 50
  storage:
    engine: rocksdb
    blockCacheSizeMb: 2048
  monitoring:
    enabled: true

persistence:
  enabled: true
  storageClass: "fast-ssd"
  size: 1Ti

service:
  type: ClusterIP
  sqlPort: 5432
  metricsPort: 9090
```

## 3. Configuration Management

### Hierarchical Configuration System

```toml
# /etc/shardforge/cluster.toml - System-wide defaults
[cluster]
name = "production-cluster"
data_directory = "/data/shardforge"

[storage]
engine = "rocksdb"
compression = "lz4"

[security]
tls_enabled = true
certificate_path = "/etc/ssl/certs/shardforge.crt"
private_key_path = "/etc/ssl/private/shardforge.key"

# ~/.shardforge/user.toml - User overrides
[logging]
level = "debug"

# shardforge.toml - Environment-specific config
[cluster]
node_id = "node-001"
bind_address = "10.0.1.10:5432"

[monitoring]
metrics_enabled = true
tracing_enabled = true
```

### Configuration Validation

```rust
#[derive(Debug, Validate)]
pub struct ClusterConfig {
    #[validate(length(min = 1, max = 63))]
    pub name: String,

    #[validate(length(min = 1, max = 63))]
    pub node_id: String,

    #[validate]
    pub network: NetworkConfig,

    #[validate]
    pub storage: StorageConfig,
}

#[derive(Debug, Validate)]
pub struct NetworkConfig {
    #[validate(custom = "validate_socket_addr")]
    pub bind_address: String,

    #[validate(range(min = 1, max = 65535))]
    pub max_connections: u32,

    #[validate(range(min = 1, max = 300))]
    pub connection_timeout_sec: u32,
}

impl ClusterConfig {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        // Cross-field validation
        if self.network.max_connections > 10000 {
            return Err(ValidationErrors::new());
        }

        // Environment-specific validation
        if self.cluster.name == "production" && !self.security.tls_enabled {
            return Err(ValidationErrors::new());
        }

        Ok(())
    }
}
```

## 4. Monitoring & Observability

### Comprehensive Metrics Collection

```rust
#[derive(Debug)]
pub struct MetricsRegistry {
    // System metrics
    pub cpu_usage: Gauge,
    pub memory_usage: Gauge,
    pub disk_usage: Gauge,
    pub network_io: Counter,

    // Database metrics
    pub active_connections: Gauge,
    pub queries_per_second: Counter,
    pub query_latency: Histogram,
    pub transaction_throughput: Counter,

    // Cluster metrics
    pub node_count: Gauge,
    pub leader_elections: Counter,
    pub replication_lag: Histogram,

    // Storage metrics
    pub storage_size: Gauge,
    pub compaction_time: Histogram,
    pub cache_hit_rate: Gauge,
    pub write_amplification: Gauge,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            cpu_usage: register_gauge!("shardforge_cpu_usage_percent"),
            memory_usage: register_gauge!("shardforge_memory_usage_bytes"),
            // ... initialize other metrics
        }
    }

    pub async fn collect_system_metrics(&self) {
        // CPU usage
        let cpu = sys_info::cpu_usage().unwrap_or(0.0);
        self.cpu_usage.set(cpu);

        // Memory usage
        let mem = sys_info::mem_info().unwrap();
        self.memory_usage.set(mem.total as f64 - mem.free as f64);

        // Disk usage
        let disk = sys_info::disk_info().unwrap();
        self.disk_usage.set(disk.total as f64 - disk.free as f64);
    }
}
```

### Distributed Tracing

```rust
use tracing::{info, instrument};
use opentelemetry::trace::{Tracer, Span};

#[instrument(name = "query_execution", fields(query_id = %query.id))]
pub async fn execute_query(&self, query: Query) -> Result<QueryResult, Error> {
    let span = tracing::info_span!("execute_query",
        query_id = %query.id,
        query_type = %query.query_type,
        user_id = %query.user_id,
    );

    let _enter = span.enter();

    // Parse query
    let parsed_query = self.parser.parse(&query.sql).instrument(info_span!("parse_query")).await?;

    // Optimize query
    let optimized_plan = self.optimizer.optimize(&parsed_query).instrument(info_span!("optimize_query")).await?;

    // Execute query
    let result = self.executor.execute(&optimized_plan).instrument(info_span!("execute_plan")).await?;

    info!(rows_returned = result.rows.len(), execution_time_ms = result.execution_time.as_millis());

    Ok(result)
}
```

### Alerting & Dashboards

```yaml
# Prometheus alerting rules
groups:
- name: shardforge
  rules:
  - alert: ShardForgeNodeDown
    expr: up{job="shardforge"} == 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "ShardForge node {{ $labels.instance }} is down"
      description: "ShardForge node has been down for more than 5 minutes."

  - alert: ShardForgeHighLatency
    expr: histogram_quantile(0.99, rate(shardforge_query_latency_bucket[5m])) > 1
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High query latency on {{ $labels.instance }}"
      description: "99th percentile query latency is {{ $value }}s"

  - alert: ShardForgeReplicationLag
    expr: shardforge_replication_lag_seconds > 30
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "High replication lag on {{ $labels.instance }}"
```

## 5. Backup & Disaster Recovery

### Multi-Modal Backup Strategy

```bash
# Full backup
shardforge backup create \
  --type full \
  --database production \
  --destination s3://backups/shardforge/production \
  --compression zstd \
  --encryption aes256 \
  --retention 30d

# Incremental backup
shardforge backup create \
  --type incremental \
  --database production \
  --parent-backup backup-2024-01-01-full \
  --destination s3://backups/shardforge/production

# Point-in-time recovery
shardforge backup restore \
  --backup-id backup-2024-01-01-full \
  --point-in-time "2024-01-01 14:30:00" \
  --target-database production_restored \
  --destination /data/shardforge/restored
```

### Automated Backup Scheduling

```yaml
# Kubernetes CronJob for automated backups
apiVersion: batch/v1
kind: CronJob
metadata:
  name: shardforge-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: shardforge/shardforge:latest
            command:
            - shardforge
            - backup
            - create
            - --type
            - incremental
            - --database
            - production
            - --destination
            - s3://backups/shardforge/production
            env:
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: backup-credentials
                  key: access-key
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: backup-credentials
                  key: secret-key
          restartPolicy: OnFailure
```

### Disaster Recovery Testing

```bash
# Disaster recovery drill
#!/bin/bash
echo "Starting disaster recovery test..."

# 1. Create backup
echo "Creating backup..."
shardforge backup create --type full --database test --destination /tmp/backup

# 2. Simulate disaster (kill all nodes)
echo "Simulating disaster..."
docker-compose down

# 3. Restore from backup
echo "Restoring from backup..."
shardforge backup restore --backup-path /tmp/backup --target-database recovered

# 4. Validate recovery
echo "Validating recovery..."
shardforge sql --database recovered --command "SELECT COUNT(*) FROM users;"

echo "Disaster recovery test completed."
```

## 6. Operational Procedures

### Rolling Upgrades

```bash
# Rolling upgrade procedure
#!/bin/bash
NODES=("node-001" "node-002" "node-003")

for node in "${NODES[@]}"; do
    echo "Upgrading $node..."

    # Drain connections from node
    shardforge node drain --node-id $node --timeout 300s

    # Wait for drain to complete
    while ! shardforge node is-drained --node-id $node; do
        sleep 10
    done

    # Stop node
    shardforge node stop --node-id $node --graceful

    # Upgrade binary
    scp new-shardforge-binary $node:/usr/local/bin/shardforge

    # Start node
    ssh $node "shardforge start --config /etc/shardforge/config.toml"

    # Wait for node to be healthy
    while ! shardforge node health --node-id $node; do
        sleep 10
    done

    # Allow connections back
    shardforge node undrain --node-id $node

    echo "$node upgrade completed."
done

echo "Rolling upgrade completed successfully."
```

### Capacity Planning

```rust
#[derive(Debug)]
pub struct CapacityPlanner {
    pub metrics_store: Arc<MetricsStore>,
    pub forecasting_model: Arc<ForecastingModel>,
}

impl CapacityPlanner {
    pub async fn plan_capacity(&self, timeframe_days: u32) -> Result<CapacityPlan, Error> {
        // Analyze current usage patterns
        let current_usage = self.metrics_store.get_current_usage().await?;
        let growth_trends = self.analyze_growth_trends().await?;
        let seasonal_patterns = self.detect_seasonal_patterns().await?;

        // Forecast future requirements
        let forecasted_load = self.forecasting_model.predict_load(
            current_usage,
            growth_trends,
            seasonal_patterns,
            timeframe_days
        ).await?;

        // Generate capacity recommendations
        let recommendations = self.generate_recommendations(forecasted_load).await?;

        Ok(CapacityPlan {
            current_capacity: current_usage,
            forecasted_load,
            recommendations,
            implementation_plan: self.create_implementation_plan(recommendations).await?,
        })
    }

    pub async fn generate_recommendations(&self, forecast: LoadForecast) -> Result<Vec<CapacityRecommendation>, Error> {
        let mut recommendations = Vec::new();

        // CPU recommendations
        if forecast.cpu_usage > 0.8 {
            recommendations.push(CapacityRecommendation::ScaleOut {
                resource: "CPU".to_string(),
                current_utilization: forecast.cpu_usage,
                recommended_action: "Add 2 more nodes".to_string(),
                estimated_cost: 500.0,
            });
        }

        // Storage recommendations
        if forecast.storage_usage > 0.85 {
            recommendations.push(CapacityRecommendation::ScaleStorage {
                current_size_gb: forecast.storage_size_gb,
                recommended_size_gb: forecast.storage_size_gb * 1.5,
                estimated_cost: forecast.storage_size_gb as f64 * 0.1,
            });
        }

        Ok(recommendations)
    }
}
```

### Runbooks & Troubleshooting

#### Common Issues & Solutions

##### Issue: Node fails to join cluster

```bash
# Check network connectivity
telnet <seed-node> 5432

# Check firewall rules
sudo iptables -L | grep 5432

# Check node logs
shardforge logs --node-id <node-id> --since 1h

# Verify cluster configuration
shardforge cluster status --detailed
```

##### Issue: High query latency

```bash
# Check system resources
shardforge metrics --format table | grep -E "(cpu|memory|disk)"

# Analyze slow queries
shardforge query profile --slow-threshold 1s --last 1h

# Check query plan
shardforge query explain "SELECT * FROM large_table WHERE condition"

# Monitor cache hit rates
shardforge metrics | grep cache_hit_rate
```

##### Issue: Replication lag

```bash
# Check replication status
shardforge replication status --detailed

# Monitor network latency
shardforge debug network-latency --all-nodes

# Check leader health
shardforge cluster status | grep leader

# Validate data consistency
shardforge debug consistency-check --table important_table
```

This comprehensive deployment guide ensures reliable, scalable, and maintainable ShardForge deployments across various infrastructure platforms.
