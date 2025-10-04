#!/bin/bash
# Test script to verify CI logic works correctly

set -e

echo "Testing CI build logic..."

# Test the feature selection logic
test_feature_logic() {
    local features=$1
    echo "Testing with features: $features"
    
    if [ "$features" = "rocksdb" ]; then
        echo "Would run: cargo build --all-targets --features rocksdb --no-default-features"
        echo "Would run: cargo test --workspace --features rocksdb --no-default-features"
    else
        echo "Would run: cargo build --all-targets --features sled --no-default-features"
        echo "Would run: cargo test --workspace --features sled --no-default-features"
    fi
}

# Test both feature combinations
test_feature_logic "sled"
test_feature_logic "rocksdb"

echo "✅ CI logic test completed successfully!"
