#!/bin/bash
# Docker Build Performance Benchmarking Script
# Measures build times for different Docker configurations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Results file
RESULTS_FILE="$PROJECT_DIR/docker-build-results-$(date +%Y%m%d-%H%M%S).txt"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

measure_build_time() {
    local dockerfile="$1"
    local tag="$2"
    local description="$3"

    log_info "Building $description ($dockerfile -> $tag)"

    local start_time=$(date +%s.%3N)

    if docker build -f "$dockerfile" -t "$tag" "$PROJECT_DIR" 2>&1; then
        local end_time=$(date +%s.%3N)
        local duration=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "0")

        log_success "$description completed in ${duration}s"

        # Log to results file
        echo "$description: ${duration}s" >> "$RESULTS_FILE"
        echo "  Dockerfile: $dockerfile" >> "$RESULTS_FILE"
        echo "  Tag: $tag" >> "$RESULTS_FILE"
        echo "  Timestamp: $(date)" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"

        return 0
    else
        log_error "$description failed"
        return 1
    fi
}

test_container_startup() {
    local tag="$1"
    local name="$2"
    local port="$3"

    log_info "Testing container startup for $tag"

    # Start container
    if docker run -d --name "$name" -p "$port:5432" "$tag" > /dev/null 2>&1; then
        # Wait for startup
        local max_attempts=30
        local attempt=1

        while [ $attempt -le $max_attempts ]; do
            if docker exec "$name" shardforge --version > /dev/null 2>&1; then
                log_success "Container $tag started successfully"
                docker stop "$name" > /dev/null 2>&1
                docker rm "$name" > /dev/null 2>&1
                return 0
            fi

            sleep 2
            ((attempt++))
        done

        log_error "Container $tag failed to start properly"
        docker logs "$name" 2>/dev/null || true
        docker stop "$name" > /dev/null 2>&1 || true
        docker rm "$name" > /dev/null 2>&1 || true
        return 1
    else
        log_error "Failed to start container $tag"
        return 1
    fi
}

compare_builds() {
    log_info "Comparing build configurations..."

    # Initialize results file
    echo "Docker Build Performance Results" > "$RESULTS_FILE"
    echo "=================================" >> "$RESULTS_FILE"
    echo "Date: $(date)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"

    # Test different configurations
    local failed_builds=0

    # Original Dockerfile
    if measure_build_time "$PROJECT_DIR/Dockerfile" "shardforge:benchmark-original" "Original Dockerfile"; then
        test_container_startup "shardforge:benchmark-original" "test-original" "5432"
    else
        ((failed_builds++))
    fi

    # Fast Dockerfile
    if measure_build_time "$PROJECT_DIR/Dockerfile.fast" "shardforge:benchmark-fast" "Fast Dockerfile"; then
        test_container_startup "shardforge:benchmark-fast" "test-fast" "5433"
    else
        ((failed_builds++))
    fi

    # Clean up test images
    log_info "Cleaning up test images..."
    docker rmi shardforge:benchmark-original shardforge:benchmark-fast 2>/dev/null || true

    # Show results
    echo ""
    log_info "Build Results Summary:"
    echo "======================="
    cat "$RESULTS_FILE"
    echo ""
    log_info "Results saved to: $RESULTS_FILE"

    if [ $failed_builds -gt 0 ]; then
        log_warning "$failed_builds build(s) failed"
        return 1
    else
        log_success "All builds completed successfully"
        return 0
    fi
}

show_usage() {
    cat << EOF
Docker Build Benchmarking Script

Usage: $0 [OPTIONS]

Options:
    -h, --help          Show this help message
    -c, --compare       Run build performance comparison
    -s, --single FILE   Test single Dockerfile
    -t, --tag TAG       Tag for single build test
    -d, --desc DESC     Description for single build test

Examples:
    $0 --compare                    # Compare all Dockerfiles
    $0 --single Dockerfile --tag test --desc "Custom build"

EOF
}

main() {
    case "${1:-}" in
        -h|--help)
            show_usage
            exit 0
            ;;
        -c|--compare)
            compare_builds
            ;;
        -s|--single)
            if [ $# -lt 5 ]; then
                log_error "Single build requires FILE, TAG, and DESC"
                show_usage
                exit 1
            fi
            measure_build_time "$2" "$4" "$5"
            ;;
        *)
            log_info "Running default comparison..."
            compare_builds
            ;;
    esac
}

main "$@"
