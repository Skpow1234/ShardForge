# Comprehensive Testing Strategy

## Testing Philosophy

ShardForge employs a multi-layered testing approach that ensures correctness, performance, and reliability across all system components. Our testing strategy follows the principle of "shift-left testing" with comprehensive automation and continuous validation.

### Quality Gates

- **Code Coverage**: Minimum 85% line coverage, 90% branch coverage
- **Performance Regression**: <5% performance degradation allowed
- **Security**: Automated vulnerability scanning and penetration testing
- **Reliability**: Chaos engineering and fault injection testing

## 1. Unit Testing

### Core Testing Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_raft_leader_election() {
        let config = RaftConfig::default();
        let raft = RaftNode::new(config).await.unwrap();

        // Test initial state
        assert_eq!(raft.get_current_term().await, 0);
        assert!(raft.get_voted_for().await.is_none());

        // Simulate election timeout
        raft.trigger_election_timeout().await;

        // Verify leader election occurred
        let leader = raft.get_current_leader().await;
        assert!(leader.is_some());
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let mut tx = Transaction::new(TransactionConfig::default());
        let key = b"test_key";
        let value = b"test_value";

        // Begin transaction
        tx.begin().await.unwrap();

        // Perform operation
        tx.put(key, value).await.unwrap();

        // Commit transaction
        let result = tx.commit().await.unwrap();
        assert!(result.success);

        // Verify durability
        let stored_value = tx.get(key).await.unwrap();
        assert_eq!(stored_value.as_deref(), Some(value.as_slice()));
    }
}
```

## 2. Integration Testing

### Multi-Node Cluster Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use testcontainers::clients::Cli;
    use testcontainers::images::generic::GenericImage;
    use testcontainers::RunnableImage;

    #[tokio::test]
    async fn test_multi_node_cluster() {
        let docker = Cli::default();
        let mut nodes = vec![];

        // Start multiple ShardForge nodes in containers
        for i in 1..=3 {
            let image = RunnableImage::from(
                GenericImage::new("shardforge", "latest")
                    .with_env_var("NODE_ID", format!("node-{}", i))
                    .with_env_var("CLUSTER_SIZE", "3")
            );

            let container = docker.run(image);
            nodes.push(container);
        }

        // Wait for cluster to form
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Test cluster operations
        let client = ClusterClient::connect("localhost:5432").await.unwrap();

        // Create table
        client.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await.unwrap();

        // Insert data
        for i in 1..=100 {
            client.execute(&format!("INSERT INTO test VALUES ({}, 'name_{}')", i, i)).await.unwrap();
        }

        // Query data
        let result = client.execute("SELECT COUNT(*) FROM test").await.unwrap();
        assert_eq!(result.rows[0][0], "100");

        // Verify data consistency across nodes
        for node in &nodes {
            let node_client = ClusterClient::connect(&format!("localhost:{}", node.get_host_port_ipv4(5432))).await.unwrap();
            let node_result = node_client.execute("SELECT COUNT(*) FROM test").await.unwrap();
            assert_eq!(node_result.rows[0][0], "100");
        }
    }
}
```

### Network Partition Testing

```rust
#[cfg(test)]
mod network_tests {
    use super::*;
    use tokio_test::block_on;

    #[tokio::test]
    async fn test_network_partition_tolerance() {
        let cluster = TestCluster::new(5).await;

        // Split cluster into two partitions
        cluster.partition(vec![0, 1], vec![2, 3, 4]).await;

        // Wait for partitions to detect split
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Verify minority partition becomes read-only
        let minority_node = &cluster.nodes[0];
        let result = minority_node.execute("INSERT INTO test VALUES (1, 'test')").await;
        assert!(result.is_err()); // Should fail in read-only mode

        // Verify majority partition continues to work
        let majority_node = &cluster.nodes[2];
        majority_node.execute("INSERT INTO test VALUES (1, 'test')").await.unwrap();

        // Heal partition
        cluster.heal().await;

        // Wait for convergence
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Verify data consistency
        for node in &cluster.nodes {
            let count = node.execute("SELECT COUNT(*) FROM test").await.unwrap();
            assert_eq!(count.rows[0][0], "1");
        }
    }
}
```

## 3. Chaos Engineering

### Jepsen-Style Testing Framework

```rust
#[cfg(test)]
mod chaos_tests {
    use super::*;
    use jepsen_rs::{test, checker, generator, nemesis};

    #[test]
    fn test_raft_consensus_under_chaos() {
        let test = Test::new()
            .with_nodes(5)
            .with_workload(raft_workload())
            .with_nemesis(partition_nemesis())
            .with_checker(linearizability_checker())
            .with_time_limit(Duration::from_secs(300));

        let result = test.run().await;
        assert!(result.valid, "Consensus violated: {:?}", result.errors);
    }
}
```

### Automated Chaos Scenarios

```rust
#[cfg(test)]
mod automated_chaos {
    use super::*;

    #[derive(Debug, Clone)]
    enum ChaosEvent {
        KillNode(String),
        NetworkPartition(Vec<String>, Vec<String>),
        DiskFull(String),
        HighLatency(String, Duration),
        ClockSkew(String, Duration),
        CpuSpike(String, f64),
        MemoryLeak(String),
    }

    #[tokio::test]
    async fn test_chaos_scenario() {
        let cluster = TestCluster::new(5).await;
        let chaos = ChaosEngine::new(cluster.clone());

        // Define chaos scenario
        let scenario = vec![
            (Duration::from_secs(10), ChaosEvent::KillNode("node-1".to_string())),
            (Duration::from_secs(20), ChaosEvent::NetworkPartition(
                vec!["node-2".to_string()], vec!["node-3".to_string(), "node-4".to_string()]
            )),
            (Duration::from_secs(30), ChaosEvent::DiskFull("node-5".to_string())),
        ];

        // Run scenario
        for (delay, event) in scenario {
            tokio::time::sleep(delay).await;
            chaos.inject_failure(event).await;
        }

        // Wait for system to stabilize
        tokio::time::sleep(Duration::from_secs(60)).await;

        // Verify system recovers
        let health = cluster.check_health().await;
        assert!(health.all_nodes_healthy, "System did not recover from chaos: {:?}", health);

        // Verify data consistency
        let consistency = cluster.check_data_consistency().await;
        assert!(consistency.is_consistent, "Data inconsistency after chaos: {:?}", consistency);
    }
}
```

## 4. Continuous Testing Infrastructure

### CI/CD Pipeline Configuration

```yaml
# .github/workflows/test.yml
name: Comprehensive Testing

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --lib --bins --tests --benches --all-features -- --nocapture

  integration-tests:
    runs-on: ubuntu-latest
    services:
      etcd:
        image: quay.io/coreos/etcd:v3.5.9
        ports:
          - 2379:2379
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --test integration -- --nocapture

  chaos-tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --test chaos -- --nocapture

  performance-tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo bench
    - run: cargo test --test performance -- --nocapture

  security-tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Run cargo-audit
      uses: actions-rs/cargo@v1
      with:
        command: audit
    - name: Run cargo-deny
      uses: EmbarkStudios/cargo-deny-action@v1

  coverage:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo install cargo-tarpaulin
    - run: cargo tarpaulin --out Html
    - uses: codecov/codecov-action@v3
      with:
        file: ./cobertura.xml
```

This comprehensive testing strategy ensures ShardForge maintains high quality, performance, and reliability throughout its development lifecycle.
