@echo off
REM ShardForge Code Formatting Script for Windows
REM This script ensures consistent code formatting across the project

echo 🔧 ShardForge Code Formatting
echo ==============================

REM Check if we're in the project root
if not exist "Cargo.toml" (
    echo ❌ Error: Please run this script from the project root directory
    exit /b 1
)

echo 📋 Checking Rust toolchain...
rustc --version
cargo --version

echo.
echo 🔍 Checking code formatting...
cargo fmt --all -- --check
if %errorlevel% equ 0 (
    echo ✅ Code is already properly formatted
) else (
    echo ⚠️ Code formatting issues found. Fixing...
    cargo fmt --all
    echo ✅ Code formatting applied
)

echo.
echo 🔍 Running clippy lints...
cargo clippy --all-targets --all-features -- -D warnings
if %errorlevel% equ 0 (
    echo ✅ No clippy warnings found
) else (
    echo ⚠️ Clippy warnings found. Please review and fix them.
    echo 💡 Tip: Run 'cargo clippy --fix --allow-dirty' to auto-fix some issues
)

echo.
echo 🎯 Formatting check complete!
echo 💡 To format code manually, run: cargo fmt --all
echo 💡 To check formatting, run: cargo fmt --all -- --check
