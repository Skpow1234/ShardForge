# ShardForge Development Script for Windows PowerShell
# Usage: .\scripts\dev.ps1 <command>

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("format", "check", "test", "clean", "help")]
    [string]$Command
)

function Write-Step {
    param([string]$Message)
    Write-Host "🔧 $Message..." -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message!" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "❌ $Message!" -ForegroundColor Red
}

switch ($Command) {
    "format" {
        Write-Step "Formatting code"
        cargo fmt
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Code formatted"
        } else {
            Write-Error "Failed to format code"
            exit 1
        }
    }

    "check" {
        Write-Step "Checking code formatting"
        cargo fmt --check
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Code is not properly formatted. Run '.\scripts\dev.ps1 format'"
            exit 1
        }

        Write-Step "Running clippy"
        cargo clippy -- -D warnings
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Clippy found issues"
            exit 1
        }

        Write-Step "Running tests"
        cargo test --workspace
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Tests failed"
            exit 1
        }

        Write-Success "All checks passed"
    }

    "test" {
        Write-Step "Running tests"
        cargo test --workspace --verbose
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Tests completed"
        } else {
            Write-Error "Tests failed"
            exit 1
        }
    }

    "clean" {
        Write-Step "Cleaning build artifacts"
        cargo clean
        Write-Success "Cleaned"
    }

    "help" {
        Write-Host "ShardForge Development Commands:" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "format     - Format code with cargo fmt"
        Write-Host "check      - Run all checks (format, clippy, test)"
        Write-Host "test       - Run tests"
        Write-Host "clean      - Clean build artifacts"
        Write-Host "help       - Show this help message"
        Write-Host ""
        Write-Host "Examples:"
        Write-Host "  .\scripts\dev.ps1 check"
        Write-Host "  .\scripts\dev.ps1 format"
    }
}
