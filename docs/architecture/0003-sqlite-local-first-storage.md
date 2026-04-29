# SQLite Local-First Storage

**Date**: 2026-04-30
**Status**: Active
**Milestone**: 26.1.0-alpha.1
**Phase**: 0-governance

## Decision

dotlanth will use **SQLite** as the primary local storage engine for the alpha release (26.1.0-alpha.1).

## Context

dotlanth is a local-first OS kernel. All core functionality must work offline without cloud dependencies. The alpha focuses on reusable foundations, not distributed systems.

## Why SQLite

### 1. Local-First Alignment
- **Zero network dependencies**: SQLite is an embedded database
- **Single file storage**: Easy to backup, restore, and transfer
- **Cross-platform**: Works on all targets dotlanth supports
- **Transactional**: Full ACID compliance for data integrity

### 2. Developer Experience
- **No server to manage**: The database is just a file
- **Standard tooling**: Any SQLite client can inspect the data
- **FTS5 support**: Built-in full-text search for the search module
- **WAL mode**: Good concurrent read performance

### 3. Alpha Scope Fit
- Sufficient for single-node operation (the alpha focus)
- Supports the modular monolith architecture (single database, multiple table prefixes)
- Can be replaced later without changing the application interface (trait-based storage)

## Local-First Architecture Principles

### Data Sovereignty
- All data lives on the user's device
- Users can inspect, backup, and restore their data using standard tools
- No data leaves the device unless explicitly configured (post-alpha feature)

### Offline Capability
- All core operations work without network connectivity
- The system degrades gracefully when offline (which is the default state)
- Sync/export features (post-alpha) are additive, not required

### Minimal Dependencies
- SQLite is embedded, not a separate service
- No external database server to install, configure, or secure
- Single binary + database file = complete system

## Storage Interface

The storage module will expose a trait-based interface:

```rust
trait Storage {
    fn query(&self, sql: &str) -> Result<Rows>;
    fn execute(&self, sql: &str) -> Result<usize>;
    fn transaction(&self) -> Result<Transaction>;
}
```

This allows replacing SQLite with another engine (Postgres, etc.) by implementing the trait.

## Storage Graduation Criteria

SQLite is the starting point. The following graduation criteria indicate when to add/replace storage engines:

### PostgreSQL
**When**: Multi-node deployment becomes a requirement
- Rationale: Need a shared database for distributed operation
- Migration path: Implement `Storage` trait for PostgreSQL

### Object Storage (S3-compatible, local filesystem)
**When**: Large binary objects (blobs) exceed SQLite's comfort zone (>1GB blobs)
- Rationale: SQLite is not optimized for large binary storage
- Migration path: Blob storage trait + SQLite metadata references

### Event Streaming (Kafka, NATS, etc.)
**When**: Inter-node event propagation needed for distributed operation
- Rationale: SQLite cannot serve as an event log for multiple nodes
- Migration path: Event streaming module + local SQLite read models

### Analytical Storage (ClickHouse, DuckDB, etc.)
**When**: OLAP queries slow down the transactional database
- Rationale: Separate analytical workloads from OLTP
- Migration path: ETL pipeline from SQLite to analytical store

### Specialized Search (Elasticsearch, Meilisearch, etc.)
**When**: FTS5 capabilities are insufficient (complex rankings, faceted search, etc.)
- Rationale: FTS5 covers basic search; specialized engines for advanced features
- Migration path: Search module implements adapter for specialized engine

## Migration Principles

1. **Trait-based abstraction**: Storage interface remains stable
2. **Gradual migration**: Can run multiple storage engines side-by-side
3. **No forced upgrades**: Users can stay on SQLite as long as it meets their needs
4. **Backwards compatibility**: Migration tools preserve data integrity
