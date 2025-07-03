# DotDB Overview

DotDB (Dotlanth Database) is a high-performance document database designed to work seamlessly with DotVM. It provides efficient state management, document storage, and query capabilities optimized for blockchain and high-performance applications.

## What is DotDB?

DotDB is a document-oriented database that provides:

- **Document-based storage**: JSON document collections with efficient indexing
- **Custom storage engine**: Optimized for high-performance operations
- **Advanced state management**: MVCC (Multi-Version Concurrency Control)
- **Efficient indexing system**: B+ trees, hash indices, and composite indices
- **Memory management**: Advanced allocators and caching systems
- **Query optimization**: Cost-based query planner and optimizer
- **DotVM integration**: Native integration with the virtual machine

## Key Features

### Document Storage Model

DotDB organizes data into collections of JSON documents:

```json
{
  "collection": "users",
  "documents": [
    {
      "id": "user_123",
      "name": "Alice Johnson",
      "email": "alice@example.com",
      "age": 30,
      "preferences": {
        "theme": "dark",
        "notifications": true
      },
      "tags": ["developer", "admin"]
    }
  ]
}
```

### Storage Engine Architecture

```
+-------------------------------------------------------------+
|                    DotDB Core                               |
+-------------------------------------------------------------+
|  +-------------+  +-------------+  +-------------------+  |
|  |  Document   |  |   Index     |  |   Query Engine    |  |
|  | Collections |  |  Manager    |  |                   |  |
|  +-------------+  +-------------+  +-------------------+  |
+-------------------------------------------------------------+
|  +-------------+  +-------------+  +-------------------+  |
|  |   Storage   |  |   Memory    |  |   Transaction     |  |
|  |   Engine    |  |  Manager    |  |     Manager       |  |
|  +-------------+  +-------------+  +-------------------+  |
+-------------------------------------------------------------+
|                    File System Layer                       |
+-------------------------------------------------------------+
```

### Advanced Indexing

DotDB supports multiple index types for optimal query performance:

- **B+ Tree Indices**: Range queries and sorted access
- **Hash Indices**: Fast exact-match lookups
- **Composite Indices**: Multi-field indexing
- **Full-text Indices**: Text search capabilities (planned)

### MVCC (Multi-Version Concurrency Control)

DotDB implements MVCC for concurrent access:

- **Snapshot Isolation**: Consistent read views
- **Non-blocking Reads**: Readers don't block writers
- **Optimistic Concurrency**: Minimal locking overhead
- **Version Management**: Automatic cleanup of old versions

## Core Components

### Document Collections

Collections are containers for related documents:

```rust
// Collection operations
create_collection("users")
drop_collection("users")
list_collections()
collection_stats("users")
```

### Document Operations

CRUD operations on individual documents:

```rust
// Document CRUD
put_document("users", "user_123", document)
get_document("users", "user_123")
update_document("users", "user_123", new_document)
delete_document("users", "user_123")
```

### Query Engine

Advanced query capabilities:

```rust
// Query operations
find_documents("users", {"age": {"$gt": 25}})
count_documents("users", {"role": "admin"})
aggregate_documents("users", pipeline)
```

## Storage Engine Details

### File Layout

DotDB uses a sophisticated file layout for optimal performance:

```
database/
├── collections/
│   ├── users/
│   │   ├── data/
│   │   │   ├── segment_001.db
│   │   │   ├── segment_002.db
│   │   │   └── ...
│   │   ├── indices/
│   │   │   ├── primary.idx
│   │   │   ├── email.idx
│   │   │   └── ...
│   │   └── metadata.json
│   └── ...
├── wal/
│   ├── wal_001.log
│   └── ...
└── system/
    ├── config.json
    └── stats.json
```

### Write-Ahead Logging (WAL)

DotDB uses WAL for durability and crash recovery:

- **Atomic Writes**: All changes are logged before application
- **Crash Recovery**: Automatic recovery from WAL on startup
- **Checkpointing**: Periodic WAL compaction
- **Replication**: WAL-based replication support (planned)

### Memory Management

Advanced memory management for optimal performance:

- **Buffer Pool**: Intelligent page caching
- **Memory Allocators**: Custom allocators for different data types
- **Compression**: Optional data compression
- **Memory Mapping**: Efficient file access

## Performance Characteristics

### Throughput

DotDB is optimized for high throughput:

- **Batch Operations**: Efficient bulk operations
- **Parallel Processing**: Multi-threaded query execution
- **Index Optimization**: Smart index selection
- **Cache Efficiency**: Intelligent caching strategies

### Latency

Low-latency operations:

- **In-Memory Indices**: Fast index lookups
- **Optimized Data Structures**: Efficient internal representations
- **Minimal Serialization**: Reduced overhead
- **Direct Memory Access**: Zero-copy operations where possible

### Scalability

Designed for scalability:

- **Horizontal Scaling**: Sharding support (planned)
- **Vertical Scaling**: Efficient resource utilization
- **Storage Efficiency**: Compact data representation
- **Index Scaling**: Scalable index structures

## Integration with DotVM

DotDB is tightly integrated with DotVM for optimal performance:

### Native Opcodes

DotVM provides native database opcodes:

```assembly
; Store user data
PUSH "users"
PUSH "user_123"
PUSH {"name": "Alice", "age": 30}
DB_PUT

; Retrieve user data
PUSH "users"
PUSH "user_123"
DB_GET
```

### State Management

Seamless state persistence:

- **Automatic Persistence**: VM state automatically persisted
- **Transaction Boundaries**: VM transactions map to DB transactions
- **Rollback Support**: Automatic rollback on VM errors
- **Snapshot Consistency**: Consistent state views

### Performance Optimization

Optimized for VM workloads:

- **Predictable Latency**: Consistent performance for VM operations
- **Memory Sharing**: Shared memory between VM and DB
- **Batch Processing**: Efficient bulk operations from VM
- **Connection Pooling**: Optimized connection management

## Use Cases

### Blockchain Applications

DotDB is ideal for blockchain state management:

- **Account State**: User accounts and balances
- **Smart Contract State**: Contract storage and execution state
- **Transaction History**: Immutable transaction logs
- **Block Data**: Block headers and transaction data

### Web Applications

Traditional web application backends:

- **User Management**: User profiles and authentication
- **Content Management**: Articles, posts, and media
- **Session Storage**: User sessions and temporary data
- **Analytics**: Event tracking and metrics

### IoT and Edge Computing

Optimized for resource-constrained environments:

- **Sensor Data**: Time-series sensor readings
- **Device State**: Device configuration and status
- **Event Logs**: System and application events
- **Local Caching**: Edge data caching

### Scientific Computing

High-performance data processing:

- **Research Data**: Experimental data and results
- **Simulation State**: Computational simulation checkpoints
- **Metadata**: Dataset descriptions and annotations
- **Collaboration**: Shared research data

## CLI Interface

DotDB provides a comprehensive CLI for database operations:

### Collection Management
```bash
# Create and manage collections
dotdb create-collection users
dotdb delete-collection users
dotdb collections
```

### Document Operations
```bash
# CRUD operations
dotdb put users '{"name": "Alice", "age": 30}'
dotdb get users user_123
dotdb update users user_123 '{"name": "Alice", "age": 31}'
dotdb delete users user_123
```

### Query Operations
```bash
# Search and analytics
dotdb find users age 30
dotdb count users
dotdb list users
```

## Configuration

DotDB supports extensive configuration options:

### Storage Configuration
```json
{
  "storage": {
    "data_directory": "/var/lib/dotdb",
    "wal_directory": "/var/lib/dotdb/wal",
    "segment_size": "64MB",
    "compression": "lz4",
    "sync_mode": "normal"
  }
}
```

### Memory Configuration
```json
{
  "memory": {
    "buffer_pool_size": "1GB",
    "cache_size": "256MB",
    "max_connections": 1000,
    "worker_threads": 8
  }
}
```

### Index Configuration
```json
{
  "indexing": {
    "auto_index": true,
    "index_cache_size": "128MB",
    "btree_page_size": "4KB",
    "hash_bucket_count": 1024
  }
}
```

## Security Features

DotDB implements multiple security layers:

### Access Control
- **Authentication**: User authentication and authorization
- **Role-Based Access**: Fine-grained permission system
- **Collection-Level Security**: Per-collection access controls
- **API Security**: Secure API endpoints and protocols

### Data Protection
- **Encryption at Rest**: Optional data encryption
- **Encryption in Transit**: TLS/SSL support
- **Data Integrity**: Checksums and validation
- **Backup Security**: Secure backup and restore

## Getting Started

1. **Installation**: Follow the [installation guide](../getting-started/installation.md)
2. **CLI Tutorial**: Learn the [DotDB CLI](../cli/dotdb.md)
3. **Basic Operations**: Try [basic operations](usage/basic-operations.md)
4. **Integration**: Explore [DotVM integration](../dotvm/overview.md)

## Next Steps

- **Architecture Details**: Learn about the [storage engine](architecture/storage-engine.md)
- **Document Management**: Read about [document operations](usage/document-management.md)
- **Advanced Features**: Explore [advanced capabilities](usage/advanced-features.md)
- **API Reference**: Check the [Core API documentation](api/core.md)