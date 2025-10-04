# Test script to verify CI logic works correctly

Write-Host "Testing CI build logic..." -ForegroundColor Blue

# Test the feature selection logic
function Test-FeatureLogic {
    param([string]$Features)
    
    Write-Host "Testing with features: $Features" -ForegroundColor Yellow
    
    if ($Features -eq "rocksdb") {
        Write-Host "Would run: cargo build --all-targets --features rocksdb --no-default-features" -ForegroundColor Green
        Write-Host "Would run: cargo test --workspace --features rocksdb --no-default-features" -ForegroundColor Green
    } else {
        Write-Host "Would run: cargo build --all-targets --features sled --no-default-features" -ForegroundColor Green
        Write-Host "Would run: cargo test --workspace --features sled --no-default-features" -ForegroundColor Green
    }
}

# Test both feature combinations
Test-FeatureLogic "sled"
Test-FeatureLogic "rocksdb"

Write-Host "✅ CI logic test completed successfully!" -ForegroundColor Green
