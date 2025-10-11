#!/bin/bash

# ShardForge Code Formatting Script
# This script ensures consistent code formatting across the project

set -e

echo "🔧 ShardForge Code Formatting"
echo "=============================="

# Check if we're in the project root
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    exit 1
fi

echo "📋 Checking Rust toolchain..."
rustc --version
cargo --version

echo ""
echo "🔍 Checking code formatting..."
if cargo fmt --all -- --check; then
    echo "✅ Code is already properly formatted"
else
    echo "⚠️ Code formatting issues found. Fixing..."
    cargo fmt --all
    echo "✅ Code formatting applied"
fi

echo ""
echo "🔍 Running clippy lints..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo "✅ No clippy warnings found"
else
    echo "⚠️ Clippy warnings found. Please review and fix them."
    echo "💡 Tip: Run 'cargo clippy --fix --allow-dirty' to auto-fix some issues"
fi

echo ""
echo "🎯 Formatting check complete!"
echo "💡 To format code manually, run: cargo fmt --all"
echo "💡 To check formatting, run: cargo fmt --all -- --check"
