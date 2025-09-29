#!/bin/bash
# ShardForge Demo Script
# This script demonstrates ShardForge CLI functionality using Docker

set -e

echo "🚀 ShardForge Demo - Running with Docker"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✅${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ️${NC}  $1"
}

# Function to run shardforge commands
shardforge_cmd() {
    docker run --rm -v "$(pwd)/demo-data:/app/demo-data" shardforge:latest "$@"
}

echo "Building ShardForge Docker image..."
docker build -t shardforge:latest . >/dev/null 2>&1
print_success "Docker image built successfully"
echo ""

print_step "1. Checking ShardForge version"
shardforge_cmd version
echo ""

print_step "2. Initializing ShardForge database"
shardforge_cmd init --data-dir /app/demo-data --single-node
print_success "Database initialized"
echo ""

print_step "3. Checking database status"
shardforge_cmd status --detailed
echo ""

print_step "4. Executing sample SQL query"
shardforge_cmd sql --query "SELECT 1 as test_column" --database default
echo ""

print_step "5. Showing available commands"
shardforge_cmd --help | head -20
echo ""

print_info "Demo completed! ShardForge is ready for development."
echo ""
print_info "Next steps:"
echo "  • Full server implementation (Phase 2)"
echo "  • SQL parser and execution engine"
echo "  • Distributed consensus with RAFT"
echo "  • Multi-node clustering"
echo ""
print_success "Happy coding with ShardForge! 🎉"
