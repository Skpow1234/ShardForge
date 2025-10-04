# ShardForge Build Guide

This document explains how to build ShardForge on different platforms and handle various build scenarios.

## Quick Start

### Windows
```powershell
# Build with sled (recommended for Windows)
.\scripts\build.ps1 -Test

# Build with rocksdb (requires C++ build tools)
.\scripts\build.ps1 -Features rocksdb -Test
```

### Linux/macOS
```bash
# Build and test
make check

# Build with specific features
cargo build --features sled --no-default-features
cargo test --workspace --features sled --no-default-features
```

## Storage Engine Features

ShardForge supports multiple storage engines:

### Sled (Default, Windows-friendly)
- Pure Rust implementation
- No C++ dependencies
- Good for development and testing
- **Recommended for Windows**

```bash
cargo build --features sled --no-default-features
cargo test --workspace --features sled --no-default-features
```

### RocksDB (Production)
- High-performance C++ storage engine
- Requires C++ build tools
- Better for production workloads
- **Not recommended for Windows without proper setup**

```bash
cargo build --features rocksdb --no-default-features
cargo test --workspace --features rocksdb --no-default-features
```

## Platform-Specific Setup

### Windows

#### Prerequisites for Sled (Recommended)
- Rust toolchain
- No additional dependencies required

#### Prerequisites for RocksDB
- Visual Studio Build Tools 2019 or later
- Windows SDK
- CMake (optional, for advanced configuration)

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022

# Or use chocolatey
choco install visualstudio2022buildtools
```

### Linux (Ubuntu/Debian)

```bash
# For Sled (no additional dependencies)
sudo apt-get update

# For RocksDB
sudo apt-get install -y build-essential libssl-dev pkg-config
```

### macOS

```bash
# For Sled (no additional dependencies)
# Xcode Command Line Tools should be sufficient

# For RocksDB
brew install openssl pkg-config
```

## Build Commands

### Basic Build
```bash
# Default (sled)
cargo build

# Specific engine
cargo build --features sled --no-default-features
cargo build --features rocksdb --no-default-features
```

### Testing
```bash
# All tests
cargo test --workspace

# With specific features
cargo test --workspace --features sled --no-default-features
cargo test --workspace --features rocksdb --no-default-features
```

### Formatting and Linting
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy (warnings allowed)
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo

# Run clippy (strict, fails on warnings)
cargo clippy -- -D warnings
```

### Benchmarks
```bash
# Run benchmarks
cargo bench

# With specific features
cargo bench --features sled --no-default-features
cargo bench --features rocksdb --no-default-features
```

## CI/CD Pipeline

The project includes GitHub Actions workflows that handle:

### Main CI Pipeline (`.github/workflows/ci.yml`)
- Format and lint checks
- Multi-platform testing (Ubuntu, Windows, macOS)
- Multiple feature combinations
- Security auditing
- Documentation generation
- Performance benchmarks

### Warnings Pipeline (`.github/workflows/warnings.yml`)
- Non-blocking warnings check
- Logs warnings without failing the build
- Useful for tracking code quality improvements

## Troubleshooting

### Windows Build Issues

#### "fatal error: 'stddef.h' file not found"
This indicates missing C++ build tools. Solutions:

1. **Use Sled instead (Recommended)**:
   ```powershell
   .\scripts\build.ps1 -Features sled -Test
   ```

2. **Install Visual Studio Build Tools**:
   - Download from Microsoft
   - Install "C++ build tools" workload
   - Restart terminal and try again

#### "cl : Línea de comandos warning D9002"
This is a harmless warning from the C++ compiler. It can be ignored.

### Linux Build Issues

#### "No package 'openssl' found"
```bash
sudo apt-get install -y libssl-dev pkg-config
```

#### "No package 'zlib' found"
```bash
sudo apt-get install -y zlib1g-dev
```

### macOS Build Issues

#### "No package 'openssl' found"
```bash
brew install openssl pkg-config
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

## Performance Considerations

### Development
- Use `sled` for faster builds
- Enable debug symbols for better error messages
- Use `cargo check` for fast syntax checking

### Production
- Use `rocksdb` for better performance
- Enable LTO and optimizations
- Consider using release builds for benchmarking

## Environment Variables

```bash
# Enable backtraces for better error messages
export RUST_BACKTRACE=1

# Enable colored output
export CARGO_TERM_COLOR=always

# For RocksDB on macOS
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

## Scripts Reference

### Windows PowerShell Scripts
- `scripts/build.ps1` - Main build script
- `scripts/dev.ps1` - Development environment setup
- `scripts/diagnose.ps1` - System diagnostics
- `scripts/fix.ps1` - Common issue fixes

### Linux/macOS Scripts
- `scripts/test.sh` - Comprehensive test runner
- `Makefile` - Development commands

### Usage Examples
```powershell
# Windows - Basic build and test
.\scripts\build.ps1 -Test

# Windows - Release build with benchmarks
.\scripts\build.ps1 -Release -Bench

# Windows - Clean and rebuild
.\scripts\build.ps1 -Clean -Test
```

```bash
# Linux/macOS - Run all checks
make check

# Linux/macOS - Run tests
make test

# Linux/macOS - Run comprehensive test suite
./scripts/test.sh
```

## Contributing

When contributing to ShardForge:

1. **Always test with both storage engines**:
   ```bash
   cargo test --workspace --features sled --no-default-features
   cargo test --workspace --features rocksdb --no-default-features
   ```

2. **Check formatting and linting**:
   ```bash
   cargo fmt --check
   cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo
   ```

3. **Run the full test suite**:
   ```bash
   ./scripts/test.sh  # Linux/macOS
   .\scripts\build.ps1 -Test  # Windows
   ```

## Support

If you encounter build issues:

1. Check this guide first
2. Try the diagnostic scripts:
   - Windows: `.\scripts\diagnose.ps1`
   - Linux/macOS: `make diagnose` (if available)
3. Check the GitHub Issues for similar problems
4. Create a new issue with:
   - Your operating system
   - Rust version (`rustc --version`)
   - Full error message
   - Steps to reproduce
