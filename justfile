# Dotlanth Distributed System Build & Deployment
# Usage: just <command>

# Default recipe - show available commands
default:
    @just --list

# 🔨 BUILD COMMANDS
# Build all workspace components
build:
    @echo "🔨 Building all workspace components..."
    cargo build --workspace

# Build in release mode
build-release:
    @echo "🚀 Building all components in release mode..."
    cargo build --workspace --release

# Build specific component
build-component component:
    @echo "🔨 Building {{component}}..."
    cargo build -p {{component}}

# Build Docker images
build-docker:
    @echo "🐳 Building Docker images..."
    docker build -f Dockerfile.prod -t dotlanth:latest .
    docker build -f Dockerfile.dev -t dotlanth:dev .

# 🚀 DEPLOYMENT COMMANDS
# Deploy development cluster
deploy-dev:
    @echo "🚀 Deploying development cluster..."
    docker-compose -f docker-compose.dev.yml up -d
    @echo "✅ Development cluster deployed!"
    @echo "📊 Grafana: http://localhost:3000 (admin/admin)"
    @echo "📈 Prometheus: http://localhost:9090"
    @echo "📚 Docs: http://localhost:3002"

# Deploy production cluster
deploy-prod:
    @echo "🚀 Deploying production cluster..."
    @just setup-certs
    docker-compose -f docker-compose.prod.yml up -d
    @echo "✅ Production cluster deployed!"
    @just cluster-status

# Scale worker nodes
scale-workers count:
    @echo "📈 Scaling workers to {{count}} instances..."
    docker-compose -f docker-compose.prod.yml up -d --scale dotlanth-worker-1={{count}}

# 🔧 MANAGEMENT COMMANDS
# Show cluster status
cluster-status:
    @echo "📊 Cluster Status:"
    @docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep dotlanth

# View logs for specific service
logs service:
    docker-compose -f docker-compose.prod.yml logs -f {{service}}

# View all cluster logs
logs-all:
    docker-compose -f docker-compose.prod.yml logs -f

# Stop cluster
stop:
    @echo "🛑 Stopping cluster..."
    docker-compose -f docker-compose.prod.yml down
    docker-compose -f docker-compose.dev.yml down

# Restart cluster
restart:
    @just stop
    @just deploy-prod

# 🔐 SECURITY COMMANDS
# Generate certificates
setup-certs:
    @echo "🔐 Setting up certificates..."
    mkdir -p certs
    @just generate-ca
    @just generate-node-certs

# Generate CA certificate
generate-ca:
    #!/bin/bash
    if [ ! -f certs/ca.key ]; then
        openssl genrsa -out certs/ca.key 4096
        openssl req -new -x509 -days 365 -key certs/ca.key -out certs/ca.crt \
            -subj "/C=US/ST=CA/L=SF/O=Synerthink/OU=Dotlanth/CN=Dotlanth-CA"
        echo "✅ CA certificate generated"
    else
        echo "ℹ️  CA certificate already exists"
    fi

# Generate node certificates
generate-node-certs:
    #!/bin/bash
    for node in master-1 master-2 master-3 worker-1 worker-2 storage-1 storage-2 gateway; do
        if [ ! -f certs/${node}.key ]; then
            openssl genrsa -out certs/${node}.key 2048
            openssl req -new -key certs/${node}.key -out certs/${node}.csr \
                -subj "/C=US/ST=CA/L=SF/O=Synerthink/OU=Dotlanth/CN=${node}"
            openssl x509 -req -in certs/${node}.csr -CA certs/ca.crt -CAkey certs/ca.key \
                -CAcreateserial -out certs/${node}.crt -days 365
            rm certs/${node}.csr
            echo "✅ Certificate generated for ${node}"
        fi
    done
    # Create generic node certificate
    if [ ! -f certs/node.crt ]; then
        cp certs/master-1.crt certs/node.crt
        cp certs/master-1.key certs/node.key
    fi

# 📊 MONITORING COMMANDS
# Open Grafana dashboard
grafana:
    @echo "📊 Opening Grafana dashboard..."
    @echo "URL: http://localhost:3000"
    @echo "Login: admin/admin"

# Open Prometheus
prometheus:
    @echo "📈 Opening Prometheus..."
    @echo "URL: http://localhost:9090"

# Show cluster metrics
metrics:
    @echo "📊 Cluster Metrics:"
    @curl -s http://localhost:9090/api/v1/query?query=up | jq '.data.result[] | {instance: .metric.instance, status: .value[1]}'

# 🧪 TESTING COMMANDS
# Run all tests
test:
    @echo "🧪 Running all tests..."
    cargo test --workspace

# Run integration tests
test-integration:
    @echo "🧪 Running integration tests..."
    cargo test --workspace --test '*integration*'

# Test cluster connectivity
test-cluster:
    @echo "🧪 Testing cluster connectivity..."
    @just test-master-nodes
    @just test-worker-nodes
    @just test-storage-nodes

# Test master nodes
test-master-nodes:
    #!/bin/bash
    echo "Testing master nodes..."
    for port in 8080 8081 8082; do
        if curl -s -f http://localhost:${port}/health > /dev/null; then
            echo "✅ Master node on port ${port} is healthy"
        else
            echo "❌ Master node on port ${port} is unhealthy"
        fi
    done

# Test worker nodes
test-worker-nodes:
    #!/bin/bash
    echo "Testing worker nodes..."
    for port in 50061 50062; do
        if nc -z localhost ${port}; then
            echo "✅ Worker node on port ${port} is reachable"
        else
            echo "❌ Worker node on port ${port} is unreachable"
        fi
    done

# Test storage nodes
test-storage-nodes:
    #!/bin/bash
    echo "Testing storage nodes..."
    for port in 5432 5433; do
        if nc -z localhost ${port}; then
            echo "✅ Storage node on port ${port} is reachable"
        else
            echo "❌ Storage node on port ${port} is unreachable"
        fi
    done

# 🔄 BACKUP & RECOVERY
# Backup cluster data
backup:
    @echo "💾 Creating cluster backup..."
    mkdir -p backups/$(date +%Y%m%d-%H%M%S)
    docker-compose -f docker-compose.prod.yml exec dotlanth-storage-1 \
        dotdb-cli backup --output /var/lib/dotlanth/backups/backup-$(date +%Y%m%d-%H%M%S).db

# List backups
list-backups:
    @echo "📋 Available backups:"
    @ls -la backups/

# Restore from backup
restore backup_file:
    @echo "🔄 Restoring from backup: {{backup_file}}"
    docker-compose -f docker-compose.prod.yml exec dotlanth-storage-1 \
        dotdb-cli restore --input {{backup_file}}

# 🧹 CLEANUP COMMANDS
# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Clean Docker resources
clean-docker:
    @echo "🧹 Cleaning Docker resources..."
    docker system prune -f
    docker volume prune -f

# Clean everything
clean-all:
    @just clean
    @just clean-docker
    @echo "✅ All cleaned up!"

# 📦 DEVELOPMENT COMMANDS
# Format code
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt --all

# Run clippy
clippy:
    @echo "📎 Running clippy..."
    cargo clippy --workspace --all-targets -- -D warnings

# Run security audit
audit:
    @echo "🔍 Running security audit..."
    cargo audit

# Watch and rebuild on changes
watch:
    @echo "👀 Watching for changes..."
    cargo watch -x "build --workspace"

# 📈 PERFORMANCE COMMANDS
# Run benchmarks
bench:
    @echo "🏃 Running benchmarks..."
    cargo bench --workspace

# Profile application
profile:
    @echo "📊 Profiling application..."
    cargo build --release
    perf record --call-graph=dwarf target/release/dotvm
    perf report

# Load test cluster
load-test:
    @echo "⚡ Running load test..."
    @echo "Install 'hey' tool first: go install github.com/rakyll/hey@latest"
    hey -n 1000 -c 10 http://localhost:8080/health

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