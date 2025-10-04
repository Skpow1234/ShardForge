# ShardForge Build Script for Windows
# This script handles building with appropriate feature flags for Windows

param(
    [string]$Features = "sled",
    [switch]$Release = $false,
    [switch]$Test = $false,
    [switch]$Bench = $false,
    [switch]$Clean = $false,
    [switch]$Help = $false
)

# Colors for output
$Red = "`e[31m"
$Green = "`e[32m"
$Yellow = "`e[33m"
$Blue = "`e[34m"
$Reset = "`e[0m"

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = $Reset
    )
    Write-Host "${Color}${Message}${Reset}"
}

function Show-Help {
    Write-ColorOutput "ShardForge Build Script for Windows" $Blue
    Write-ColorOutput "=====================================" $Blue
    Write-Host ""
    Write-Host "Usage: .\scripts\build.ps1 [options]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Features <sled|rocksdb>  Storage engine to use (default: sled)"
    Write-Host "  -Release                   Build in release mode"
    Write-Host "  -Test                      Run tests after build"
    Write-Host "  -Bench                     Run benchmarks after build"
    Write-Host "  -Clean                     Clean build artifacts first"
    Write-Host "  -Help                      Show this help message"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\scripts\build.ps1 -Test                    # Build and test with sled"
    Write-Host "  .\scripts\build.ps1 -Features rocksdb -Test   # Build and test with rocksdb"
    Write-Host "  .\scripts\build.ps1 -Release -Bench          # Release build with benchmarks"
}

if ($Help) {
    Show-Help
    exit 0
}

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-ColorOutput "❌ Cargo.toml not found. Please run this script from the project root." $Red
    exit 1
}

Write-ColorOutput "🔧 ShardForge Build Script for Windows" $Blue
Write-ColorOutput "=====================================" $Blue

# Clean if requested
if ($Clean) {
    Write-ColorOutput "🧹 Cleaning build artifacts..." $Yellow
    cargo clean
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "❌ Clean failed" $Red
        exit 1
    }
    Write-ColorOutput "✅ Clean completed" $Green
}

# Validate features
$ValidFeatures = @("sled", "rocksdb")
if ($Features -notin $ValidFeatures) {
    Write-ColorOutput "❌ Invalid features: $Features. Valid options: $($ValidFeatures -join ', ')" $Red
    exit 1
}

# Check for Windows-specific issues with rocksdb
if ($Features -eq "rocksdb") {
    Write-ColorOutput "⚠️  Warning: RocksDB requires C++ build tools on Windows" $Yellow
    Write-ColorOutput "   Make sure you have Visual Studio Build Tools installed" $Yellow
    Write-ColorOutput "   If build fails, try using -Features sled instead" $Yellow
    Write-Host ""
}

# Build command
$BuildArgs = @("build", "--all-targets")
if ($Release) {
    $BuildArgs += "--release"
}
$BuildArgs += "--features", $Features, "--no-default-features"

Write-ColorOutput "🔨 Building with features: $Features" $Blue
Write-ColorOutput "Command: cargo $($BuildArgs -join ' ')" $Blue

# Run build
cargo @BuildArgs
if ($LASTEXITCODE -ne 0) {
    Write-ColorOutput "❌ Build failed" $Red
    if ($Features -eq "rocksdb") {
        Write-ColorOutput "💡 Try using -Features sled for Windows compatibility" $Yellow
    }
    exit 1
}
Write-ColorOutput "✅ Build completed successfully" $Green

# Run tests if requested
if ($Test) {
    Write-ColorOutput "🧪 Running tests..." $Blue
    $TestArgs = @("test", "--workspace", "--features", $Features, "--no-default-features")
    cargo @TestArgs
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "❌ Tests failed" $Red
        exit 1
    }
    Write-ColorOutput "✅ Tests passed" $Green
}

# Run benchmarks if requested
if ($Bench) {
    Write-ColorOutput "⚡ Running benchmarks..." $Blue
    $BenchArgs = @("bench", "--features", $Features, "--no-default-features")
    cargo @BenchArgs
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "⚠️  Benchmarks failed (this may be expected)" $Yellow
    } else {
        Write-ColorOutput "✅ Benchmarks completed" $Green
    }
}

Write-ColorOutput "🎉 Build process completed successfully!" $Green
Write-ColorOutput "Features used: $Features" $Blue
if ($Release) {
    Write-ColorOutput "Build mode: Release" $Blue
} else {
    Write-ColorOutput "Build mode: Debug" $Blue
}
