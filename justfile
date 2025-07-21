# DotVM/DotDB Project Build System
# Usage: just <command>

# Default recipe - show available commands
default:
    @just --list

# Build all workspace components
build:
    @echo "🔨 Building all workspace components..."
    cargo build --workspace

# FreeBSD deployment automation
deploy-freebsd: build-release
    sudo ./scripts/deploy-freebsd.sh

# FreeBSD service management commands
freebsd-start:
    sudo service dotlanth start

freebsd-stop:
    sudo service dotlanth stop

freebsd-restart:
    sudo service dotlanth restart

freebsd-status:
    sudo service dotlanth status

freebsd-logs:
    tail -f /zroot/dotlanth/logs/dotlanth.log

freebsd-test:
    #!/usr/bin/env sh
    echo "=== Testing Dotlanth Service ==="
    echo "Service status:"
    sudo service dotlanth status
    echo "\nListening ports:"
    sockstat -l | grep 8080 || echo "Port 8080 not listening"
    echo "\nRunning processes:"
    ps aux | grep dotvm | grep -v grep || echo "No dotvm processes found"
    echo "\nTesting API:"
    curl -s http://localhost:8080/health || echo "API not responding"

# Setup monitoring stack (Prometheus, Grafana, Node Exporter)
freebsd-monitoring:
    sudo ./scripts/setup-monitoring.sh

# Check monitoring services status
freebsd-monitoring-status:
    #!/usr/bin/env sh
    echo "=== Monitoring Services Status ==="
    echo "Prometheus:"
    sudo service prometheus status
    echo "\nGrafana:"
    sudo service grafana status
    echo "\nNode Exporter:"
    sudo service node_exporter status
    echo "\nListening ports:"
    sockstat -l | grep -E "(9090|3000|9100)" || echo "No monitoring ports found"

# Stop monitoring services
freebsd-monitoring-stop:
    sudo service prometheus stop
    sudo service grafana stop
    sudo service node_exporter stop

# Start monitoring services
freebsd-monitoring-start:
    sudo service prometheus start
    sudo service grafana start
    sudo service node_exporter start

# Restart monitoring services
freebsd-monitoring-restart:
    sudo service prometheus restart
    sudo service grafana restart
    sudo service node_exporter restart

# Complete deployment with monitoring
deploy-freebsd-full: deploy-freebsd freebsd-monitoring
    @echo "=== Complete Deployment Finished! ==="
    @echo "Services running:"
    @echo "  - Dotlanth: http://localhost:8080"
    @echo "  - Prometheus: http://localhost:9090"
    @echo "  - Grafana: http://localhost:3000"

freebsd-clean:
    #!/usr/bin/env sh
    echo "=== Cleaning Dotlanth FreeBSD Installation ==="
    echo "⚠️  This will remove all Dotlanth files and data!"
    echo "Press Ctrl+C to cancel, or Enter to continue..."
    read dummy
    
    # Stop all services
    sudo service dotlanth stop 2>/dev/null || true
    sudo service prometheus stop 2>/dev/null || true
    sudo service grafana stop 2>/dev/null || true
    sudo service node_exporter stop 2>/dev/null || true
    
    # Disable services
    sudo sysrc dotlanth_enable=NO 2>/dev/null || true
    sudo sysrc prometheus_enable=NO 2>/dev/null || true
    sudo sysrc grafana_enable=NO 2>/dev/null || true
    sudo sysrc node_exporter_enable=NO 2>/dev/null || true
    
    # Remove Dotlanth files
    sudo rm -f /usr/local/etc/rc.d/dotlanth
    sudo rm -rf /usr/local/etc/dotlanth
    sudo rm -f /usr/local/bin/dotvm /usr/local/bin/dotvm-runtime /usr/local/bin/dotlanth /usr/local/bin/dotdb
    sudo rm -rf /var/lib/dotlanth /var/log/dotlanth /zroot/dotlanth
    sudo pw userdel dotlanth 2>/dev/null || true
    
    echo "✅ Dotlanth FreeBSD installation cleaned"
    echo "Note: Monitoring packages (prometheus, grafana, node_exporter) are still installed"
    echo "To remove them: pkg remove prometheus grafana9 node_exporter"

# Build in release mode
build-release:
    @echo "🚀 Building all components in release mode..."
    cargo build --workspace --release

# Build specific component
build-component component:
    @echo "🔨 Building {{component}}..."
    cargo build -p {{component}}

# Run all tests
test:
    @echo "🧪 Running all tests..."
    cargo test --workspace

# Run tests for specific component
test-component component:
    @echo "🧪 Testing {{component}}..."
    cargo test -p {{component}}

# TODO: Advanced testing features
# test-verbose:
#     @echo "🧪 Running tests with verbose output..."
#     cargo test --workspace -- --nocapture

# Format code
format:
    @echo "📝 Formatting code..."
    cargo fmt --all

# Run clippy lints
lint:
    @echo "🔍 Running clippy lints..."
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Quick development check
dev-check: format lint
    @echo "✅ Development checks completed!"

# TODO: Advanced quality checks
# check-format:
#     @echo "📝 Checking code formatting..."
#     cargo fmt --all -- --check
#
# lint-fix:
#     @echo "🔧 Fixing clippy issues..."
#     cargo clippy --workspace --all-targets --all-features --fix --allow-dirty
#
# check: check-format lint test
#     @echo "✅ All quality checks passed!"

# TODO: Installation commands
# install:
#     @echo "📦 Installing CLI tools..."
#     cargo install --path crates/dotdb/cli --bin dotdb
#     cargo install --path crates/dotlanth-cli --bin dotlanth
#     cargo install --path crates/dotvm/tools --bin dotvm
#
# uninstall:
#     @echo "🗑️ Uninstalling CLI tools..."
#     cargo uninstall dotdb dotlanth dotvm

# Run DotDB CLI (after build)
dotdb *args:
    @echo "🗄️ Running DotDB CLI..."
    ./target/debug/dotdb {{args}}

# Run DotLanth CLI (after build)
dotlanth *args:
    @echo "🏗️ Running DotLanth CLI..."
    ./target/debug/dotlanth {{args}}

# Run DotVM Tools (after build)
dotvm *args:
    @echo "⚙️ Running DotVM Tools..."
    ./target/debug/dotvm {{args}}

# Start DotLanth TUI (after build)
tui:
    @echo "🖥️ Starting DotLanth TUI..."
    ./target/debug/dotlanth run

# Development versions (using cargo run)
dev-dotdb *args:
    @echo "🗄️ Running DotDB CLI (dev)..."
    cargo run -p dotdb-cli -- {{args}}

# Development DotLanth CLI
dev-dotlanth *args:
    @echo "🏗️ Running DotLanth CLI (dev)..."
    cargo run -p dotlanth-cli -- {{args}}

# Development DotVM Tools
dev-dotvm *args:
    @echo "⚙️ Running DotVM Tools (dev)..."
    cargo run -p dotvm-tools -- {{args}}

# Development TUI
dev-tui:
    @echo "🖥️ Starting DotLanth TUI (dev)..."
    cargo run -p dotlanth-cli -- run

# TODO: Documentation commands
# docs:
#     @echo "📚 Generating documentation..."
#     cargo doc --workspace --no-deps --open
#
# docs-book:
#     @echo "📖 Building documentation book..."
#     cd docs && mdbook build
#
# docs-serve:
#     @echo "🌐 Serving documentation book..."
#     cd docs && mdbook serve --open

# TODO: Docker commands
# docker-dev:
#     @echo "🐳 Building development Docker image..."
#     docker build -f Dockerfile.dev -t dotlanth:dev .
#
# docker-prod:
#     @echo "🐳 Building production Docker image..."
#     docker build -f Dockerfile.prod -t dotlanth:latest .
#
# docker-up:
#     @echo "🐳 Starting development environment..."
#     docker-compose -f docker-compose.dev.yml up -d
#
# docker-down:
#     @echo "🐳 Stopping development environment..."
#     docker-compose -f docker-compose.dev.yml down
#
# docker-logs:
#     @echo "📋 Viewing Docker logs..."
#     docker-compose -f docker-compose.dev.yml logs -f

# TODO: Advanced commands
# bench:
#     @echo "⚡ Running benchmarks..."
#     cargo bench --workspace
#
# bench-component component:
#     @echo "⚡ Running benchmarks for {{component}}..."
#     cargo bench -p {{component}}
#
# audit:
#     @echo "🔒 Running security audit..."
#     cargo audit
#
# update:
#     @echo "📦 Updating dependencies..."
#     cargo update
#
# outdated:
#     @echo "📊 Checking for outdated dependencies..."
#     cargo outdated
#
# release version:
#     @echo "🚀 Creating release {{version}}..."
#     git tag -a v{{version}} -m "Release version {{version}}"
#     cargo build --workspace --release
#     @echo "✅ Release {{version}} created!"

# Quick test of all CLI tools (after build)
test-cli:
    @echo "🧪 Testing all CLI tools..."
    @echo "Testing DotDB CLI..."
    ./target/debug/dotdb --help
    @echo "Testing DotLanth CLI..."
    ./target/debug/dotlanth --help
    @echo "Testing DotVM Tools..."
    ./target/debug/dotvm --help
    @echo "✅ All CLI tools working!"

# Development test of CLI tools
dev-test-cli:
    @echo "🧪 Testing all CLI tools (dev)..."
    @echo "Testing DotDB CLI..."
    cargo run -p dotdb-cli -- --help
    @echo "Testing DotLanth CLI..."
    cargo run -p dotlanth-cli -- --help
    @echo "Testing DotVM Tools..."
    cargo run -p dotvm-tools -- --help
    @echo "✅ All CLI tools working!"

# TODO: Development tools
# setup:
#     @echo "🔧 Setting up development environment..."
#     rustup component add rustfmt clippy
#     cargo install cargo-audit cargo-outdated mdbook
#     @echo "✅ Development environment ready!"
#
# watch:
#     @echo "👀 Watching for changes..."
#     cargo watch -x "test --workspace"
#
# watch-component component:
#     @echo "👀 Watching {{component}} for changes..."
#     cargo watch -x "test -p {{component}}"
#
# profile-build:
#     @echo "⏱️ Profiling build times..."
#     cargo build --workspace --timings

# Show workspace information
info:
    @echo "📊 Workspace Information:"
    @echo "Rust version: $(rustc --version)"
    @echo "Cargo version: $(cargo --version)"
    @echo "Workspace members:"
    @cargo metadata --format-version 1 | jq -r '.workspace_members[]' | sed 's/.*#/  - /'

# Clean and rebuild everything
rebuild: clean build
    @echo "🔄 Full rebuild completed!"

# Run integration tests
test-integration:
    @echo "🔗 Running integration tests..."
    cargo test --workspace --test '*'

# TODO: Coverage reporting
# coverage:
#     @echo "📊 Generating test coverage report..."
#     cargo tarpaulin --workspace --out Html --output-dir target/coverage

# Quick development workflow
dev: format lint
    @echo "🚀 Development workflow completed!"

# TODO: Advanced fix command
# fix: format lint-fix
#     @echo "🔧 Code fixed and formatted!"

# Show project statistics
stats:
    @echo "📈 Project Statistics:"
    @echo "Lines of code:"
    @find . -name "*.rs" -not -path "./target/*" | xargs wc -l | tail -1
    @echo "Number of Rust files:"
    @find . -name "*.rs" -not -path "./target/*" | wc -l
    @echo "Workspace crates:"
    @ls crates/*/Cargo.toml | wc -l
