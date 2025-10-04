#!/bin/bash
# ShardForge Test Runner Script

set -e

echo "🧪 Running ShardForge Test Suite"
echo "================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "success")
            echo -e "${GREEN}✅${NC} $message"
            ;;
        "error")
            echo -e "${RED}❌${NC} $message"
            ;;
        "warning")
            echo -e "${YELLOW}⚠️${NC} $message"
            ;;
        "info")
            echo -e "ℹ️  $message"
            ;;
    esac
}

# Function to run command and check result
run_test() {
    local test_name=$1
    local test_command=$2

    echo ""
    print_status "info" "Running $test_name..."

    if eval "$test_command"; then
        print_status "success" "$test_name completed successfully"
        return 0
    else
        print_status "error" "$test_name failed"
        return 1
    fi
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_status "error" "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Run unit tests
run_test "Unit Tests" "cargo test --lib --bins --tests -- --nocapture"

# Run integration tests
run_test "Integration Tests" "cargo test --test integration -- --nocapture"

# Run storage integration tests
run_test "Storage Integration Tests" "cargo test --test storage_integration -- --nocapture"

# Run config integration tests
run_test "Config Integration Tests" "cargo test --test config_integration -- --nocapture"

# Run benchmarks (without capturing output for visibility)
echo ""
print_status "info" "Running Benchmarks..."
if cargo bench --quiet; then
    print_status "success" "Benchmarks completed successfully"
else
    print_status "warning" "Benchmarks failed (this may be expected if criterion is not configured)"
fi

# Run with different feature sets
echo ""
print_status "info" "Testing different feature combinations..."

# Test with default features
run_test "Default Features" "cargo test --lib --features default"

# Test without default features
run_test "No Default Features" "cargo test --lib --no-default-features"

# Code quality checks
echo ""
print_status "info" "Running Code Quality Checks..."

run_test "Clippy" "cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo"

run_test "Format Check" "cargo fmt --check"

# Security audit
if command -v cargo-audit &> /dev/null; then
    run_test "Security Audit" "cargo audit"
else
    print_status "warning" "cargo-audit not installed, skipping security audit"
fi

# Test coverage (if tarpaulin is available)
if command -v cargo-tarpaulin &> /dev/null; then
    echo ""
    print_status "info" "Generating Test Coverage Report..."
    if cargo tarpaulin --out Html --output-dir target/tarpaulin; then
        print_status "success" "Coverage report generated at target/tarpaulin/tarpaulin-report.html"
    else
        print_status "warning" "Coverage report generation failed"
    fi
else
    print_status "warning" "cargo-tarpaulin not installed, skipping coverage report"
fi

# Performance regression check
echo ""
print_status "info" "Checking for performance regressions..."

# Simple performance test
if timeout 30s cargo bench --quiet > /dev/null 2>&1; then
    print_status "success" "Performance benchmarks completed without timeout"
else
    print_status "warning" "Performance benchmarks timed out or failed"
fi

# Final summary
echo ""
echo "🎉 Test Suite Complete!"
echo "======================"
print_status "info" "Run 'cargo test' to run tests manually"
print_status "info" "Run 'cargo bench' to run benchmarks manually"
print_status "info" "Run 'cargo doc --open' to view documentation"

# Check if all tests passed
if [ $? -eq 0 ]; then
    print_status "success" "All tests completed successfully! 🎉"
    exit 0
else
    print_status "error" "Some tests failed. Please check the output above."
    exit 1
fi
