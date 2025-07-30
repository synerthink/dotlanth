# Welcome to Dotlanth

Welcome to the comprehensive documentation for Dotlanth, a next-generation virtual machine solution designed for high-performance parallel dot execution and state management.

## Project Overview

Dotlanth consists of two main components that work together to provide a complete virtual machine and database solution:

### DOTVM (Virtual Machine)
DOTVM is a highly efficient virtual machine designed to execute dots with advanced parallelization capabilities. It supports multiple architectures (32/64/128/256/512-bit) and provides robust security features.

**Key Features:**
- **Multi-architecture support**: 32, 64, 128, 256, and 512-bit architectures
- **Paradots system**: Advanced parallel execution capabilities
- **Comprehensive instruction set**: Arithmetic, control flow, memory, crypto, SIMD, and more
- **Rust to DotVM transpilation**: Complete toolchain from Rust source to bytecode
- **Advanced security and isolation**: Memory protection and secure execution
- **Cross-platform compatibility**: Runs on multiple operating systems
- **Integrated state management**: Seamless integration with DOTDB

**CLI Tools:**
- `dotvm transpile`: Transpile Rust code to DotVM bytecode
- `dotvm run`: Execute DotVM bytecode with debugging capabilities

### DOTDB (Document Database)
DOTDB is a custom-built document database designed to work seamlessly with DOTVM. It provides efficient state management and storage capabilities optimized for  and high-performance applications.

**Key Features:**
- **Document-based storage**: JSON document collections with efficient indexing
- **Custom storage engine**: Optimized for high-performance operations
- **Advanced state management**: MVCC (Multi-Version Concurrency Control)
- **Efficient indexing system**: B+ trees, hash indices, and composite indices
- **Memory management**: Advanced allocators and caching systems
- **Query optimization**: Cost-based query planner and optimizer

**CLI Tools:**
- `dotdb put/get/update/delete`: Basic document operations
- `dotdb collections`: Collection management
- `dotdb find`: Query documents by field values

## Architecture Overview

```
+-------------------+    +-------------------+
|   Rust Code       |--->|  DotVM Bytecode   |
+-------------------+    +-------------------+
         |                       |
         v                       v
+-------------------+    +-------------------+
| Transpiler CLI    |    |  Executor CLI     |
+-------------------+    +-------------------+
                               |
                               v
                    +-------------------+
                    |     DOTDB         |
                    | (State Storage)   |
                    +-------------------+
```

## Quick Start

1. **Installation**: [Get started with installation](getting-started/installation.md)
2. **CLI Tools**: Learn to use [DotVM CLI](cli/dotvm.md) and [DotDB CLI](cli/dotdb.md)
3. **First Program**: Follow the [quickstart guide](getting-started/quickstart.md)
4. **Development**: Set up your [development environment](getting-started/development-setup.md)

## Quick Links

- [Installation Guide](getting-started/installation.md)
- [DotVM CLI Reference](cli/dotvm.md)
- [DotDB CLI Reference](cli/dotdb.md)
- [Architecture Overview](dotvm/architecture/core.md)
- [Contributing Guidelines](contributing/guidelines.md)

## Support and Community

- **GitHub Repository**: [synerthink-organization/dotVM](https://github.com/synerthink-organization/dotVM)
- **Issues**: Report bugs and request features on GitHub
- **Documentation**: Comprehensive guides and API references
- **License**: GNU Affero General Public License v3.0