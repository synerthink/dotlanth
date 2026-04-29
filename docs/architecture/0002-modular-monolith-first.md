# Modular Monolith First

**Date**: 2026-04-30
**Status**: Active
**Milestone**: 26.1.0-alpha.1
**Phase**: 0-governance

## Decision

The initial implementation of dotlanth will be a **modular monolith**, not a microservices architecture.

## Context

dotlanth is a local-first OS kernel that must operate with minimal external dependencies. The project is in alpha (26.1.0-alpha.1) with a focus on reusable foundations.

## Why NOT Microservices

### 1. Local-First Constraints
- Microservices typically require network communication, service discovery, and orchestration
- A local-first system should minimize runtime dependencies and network assumptions
- Inter-process communication (IPC) adds complexity without benefit for single-node operation

### 2. Operational Simplicity
- Single deployment unit vs. managing multiple service lifecycles
- No need for service mesh, distributed tracing, or distributed configuration
- Easier debugging and development workflow

### 3. Data Consistency
- Local-first systems benefit from SQLite's ACID transactions across "modules"
- Microservices would require distributed transaction patterns (Saga, 2PC) or eventual consistency
- Single database simplifies the alpha scope

### 4. Alpha Scope Alignment
- The alpha focuses on reusable foundations, not distributed systems
- Modular monolith delivers the same module boundaries with less infrastructure
- Can extract services later if/when multi-node becomes a requirement

## Modular Monolith Structure

The codebase will be organized as independent modules with clear boundaries:

```
dotlanth/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── dotlanth-kernel/          # Core kernel module
│   ├── dotlanth-storage/         # Storage and FTS5 module
│   ├── dotlanth-security/        # RBAC and auth module
│   ├── dotlanth-search/          # Search services module
│   └── dotlanth-ops/             # Operations and health module
```

### Module Communication
- **In-process function calls** for synchronous operations
- **Event bus (in-process)** for asynchronous notifications
- **Well-defined trait boundaries** between modules
- **No circular dependencies** between modules

### Module Boundaries
Each module:
- Owns its data models and business logic
- Exposes a trait-based API for other modules
- May have its own SQLite tables (with table prefix)
- Must not directly access another module's internal state

## Graduation Criteria for Decomposition

The modular monolith should be reconsidered when ANY of these become true:

1. **Multi-node requirement**: When dotlanth needs to run across multiple machines
2. **Independent scaling**: When different modules need different resource allocations
3. **Technology heterogeneity**: When a module needs a different tech stack
4. **Team scaling**: When multiple teams need independent deployment cycles

Until then, the modular monolith provides the right balance of:
- Clear module boundaries (for future extraction)
- Operational simplicity (single deployment)
- Data consistency (single database)
- Developer productivity (easier debugging, testing, deployment)
