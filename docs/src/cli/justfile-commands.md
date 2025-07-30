# Justfile Commands Reference

This document provides a comprehensive reference for all available `just` commands in the DotVM/DotDB project.

## Quick Start

```bash
# Show all available commands
just

# Build everything
just build

# Test all CLI tools
just test-cli

# Start the TUI
just tui
```

## Build Commands

### `just build`
Build all workspace components in debug mode.

```bash
just build
```

### `just build-release`
Build all workspace components in release mode with optimizations.

```bash
just build-release
```

### `just build-component <component>`
Build a specific component only.

```bash
just build-component dotlanth-cli
just build-component dotdb-core
```

### `just rebuild`
Clean and rebuild everything from scratch.

```bash
just rebuild
```

## Test Commands

### `just test`
Run all tests across the workspace.

```bash
just test
```

### `just test-component <component>`
Run tests for a specific component.

```bash
just test-component dotlanth-cli
```

### `just test-verbose`
Run tests with verbose output (shows println! statements).

```bash
just test-verbose
```

### `just test-integration`
Run integration tests only.

```bash
just test-integration
```

## CLI Execution Commands

### Built Binary Commands (After `just build`)

#### `just dotlanth [args...]`
Run the DotLanth infrastructure management CLI.

```bash
# Show help
just dotlanth --help

# Start TUI
just dotlanth run

# Show cluster status
just dotlanth status

# List nodes
just dotlanth nodes list

# Add a node
just dotlanth nodes add "192.168.1.100:8080"

# Deploy a dot file
just dotlanth deploy my-dot.dot

# Show configuration
just dotlanth config show
```

#### `just dotdb [args...]`
Run the DotDB database CLI.

```bash
# Show help
just dotdb --help

# List collections
just dotdb collections

# Create a collection
just dotdb create-collection users

# Insert a document
just dotdb put users user1 '{"name": "Alice", "age": 30}'

# Get a document
just dotdb get users user1
```

#### `just dotvm [args...]`
Run the DotVM tools.

```bash
# Show help
just dotvm --help

# Transpile Rust to DotVM bytecode
just dotvm transpile input.rs output.dot

# Run DotVM bytecode
just dotvm run program.dot
```

### Development Commands (Using `cargo run`)

#### `just dev-dotlanth [args...]`
Run DotLanth CLI in development mode.

```bash
just dev-dotlanth run
```

#### `just dev-dotdb [args...]`
Run DotDB CLI in development mode.

```bash
just dev-dotdb collections
```

#### `just dev-dotvm [args...]`
Run DotVM tools in development mode.

```bash
just dev-dotvm --help
```

## TUI Commands

### `just tui`
Start the DotLanth TUI dashboard (after build).

```bash
just tui
```

### `just dev-tui`
Start the TUI in development mode.

```bash
just dev-tui
```

## Quality Assurance Commands

### `just check`
Run all quality checks (format, lint, test).

```bash
just check
```

### `just dev-check`
Quick development checks (format and lint only).

```bash
just dev-check
```

### `just format`
Format all Rust code.

```bash
just format
```

### `just check-format`
Check if code is properly formatted.

```bash
just check-format
```

### `just lint`
Run clippy lints.

```bash
just lint
```

### `just lint-fix`
Automatically fix clippy issues.

```bash
just lint-fix
```

### `just fix`
Format code and fix lint issues.

```bash
just fix
```

## Documentation Commands

### `just docs`
Generate and open Rust documentation.

```bash
just docs
```

### `just docs-book`
Build the mdBook documentation.

```bash
just docs-book
```

### `just docs-serve`
Serve the documentation book locally.

```bash
just docs-serve
```

## Docker Commands

### `just docker-dev`
Build development Docker image.

```bash
just docker-dev
```

### `just docker-prod`
Build production Docker image.

```bash
just docker-prod
```

### `just docker-up`
Start development environment with Docker Compose.

```bash
just docker-up
```

### `just docker-down`
Stop development environment.

```bash
just docker-down
```

### `just docker-logs`
View Docker logs.

```bash
just docker-logs
```

## Benchmarking Commands

### `just bench`
Run all benchmarks.

```bash
just bench
```

### `just bench-component <component>`
Run benchmarks for a specific component.

```bash
just bench-component dotdb-core
```

## Maintenance Commands

### `just clean`
Clean build artifacts.

```bash
just clean
```

### `just install`
Install CLI tools to system.

```bash
just install
```

### `just uninstall`
Uninstall CLI tools from system.

```bash
just uninstall
```

### `just update`
Update dependencies.

```bash
just update
```

### `just audit`
Run security audit.

```bash
just audit
```

### `just outdated`
Check for outdated dependencies.

```bash
just outdated
```

## Development Workflow Commands

### `just setup`
Set up development environment.

```bash
just setup
```

### `just dev`
Quick development workflow (format + lint).

```bash
just dev
```

### `just watch`
Watch for changes and run tests.

```bash
just watch
```

### `just watch-component <component>`
Watch a specific component for changes.

```bash
just watch-component dotlanth-cli
```

## Information Commands

### `just info`
Show workspace and system information.

```bash
just info
```

### `just stats`
Show project statistics.

```bash
just stats
```

### `just profile-build`
Profile build times.

```bash
just profile-build
```

## CI/CD Commands

### `just ci`
Run full CI pipeline locally.

```bash
just ci
```

### `just coverage`
Generate test coverage report.

```bash
just coverage
```

### `just release <version>`
Create a new release.

```bash
just release 1.0.0
```

## Common Workflows

### Development Workflow
```bash
# 1. Set up environment (first time only)
just setup

# 2. Make changes to code

# 3. Quick checks
just dev-check

# 4. Test changes
just test-component dotlanth-cli

# 5. Run the application
just dev-tui
```

### Testing Workflow
```bash
# Build everything
just build

# Test all CLIs
just test-cli

# Test specific functionality
just dotlanth nodes list
just dotdb collections
just dotvm --help
```

### Release Workflow
```bash
# Full quality check
just check

# Build release version
just build-release

# Run benchmarks
just bench

# Create release
just release 1.0.0
```

## Tips and Best Practices

1. **Use built binaries for testing**: After `just build`, use `just dotlanth` instead of `just dev-dotlanth` for faster execution.

2. **Quick development cycle**: Use `just dev-check` for fast feedback during development.

3. **Watch mode for TDD**: Use `just watch-component <component>` when doing test-driven development.

4. **CI simulation**: Run `just ci` before pushing to ensure all checks pass.

5. **Performance testing**: Use `just bench` to ensure changes don't regress performance.

6. **Documentation updates**: Run `just docs-serve` when updating documentation to see changes live.