# ShardForge Development Commands
.PHONY: help format check test clean

help: ## Show this help message
	@echo "ShardForge Development Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

format: ## Format code with cargo fmt
	@echo "🔧 Formatting code..."
	cargo fmt
	@echo "✅ Code formatted!"

check: ## Run all checks (format, clippy, test)
	@echo "🔍 Running all checks..."
	cargo fmt --check
	cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo
	cargo test --workspace
	@echo "✅ All checks passed!"

check-warnings: ## Run checks allowing warnings (for CI/CD)
	@echo "🔍 Running checks with warnings allowed..."
	cargo fmt --check
	cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo || echo "⚠️ Clippy warnings found but continuing..."
	cargo test --workspace
	@echo "✅ Checks completed (warnings allowed)!"

test: ## Run tests
	@echo "🧪 Running tests..."
	cargo test --workspace --verbose

test-integration: ## Run integration tests
	@echo "🧪 Running integration tests..."
	cargo test --test integration
	cargo test --test storage_integration
	cargo test --test config_integration

bench: ## Run benchmarks
	@echo "⚡ Running benchmarks..."
	cargo bench --workspace

clean: ## Clean build artifacts
	@echo "🧹 Cleaning..."
	cargo clean

install-dev: ## Install development dependencies
	@echo "📦 Installing development tools..."
	cargo install cargo-audit
	cargo install cargo-tarpaulin

audit: ## Run security audit
	@echo "🔒 Running security audit..."
	cargo audit

coverage: ## Generate coverage report
	@echo "📊 Generating coverage report..."
	cargo tarpaulin --workspace --out Html --output-dir coverage

# Windows PowerShell compatibility
# To use on Windows, you can run: make check
# Or use: cargo make check (if you install cargo-make)
