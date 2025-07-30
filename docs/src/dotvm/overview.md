# DotVM Overview

DotVM (Dotlanth Virtual Machine) is a high-performance, multi-architecture virtual machine designed for executing dots and general-purpose programs with advanced parallelization capabilities.

## What is DotVM?

DotVM is a stack-based virtual machine that provides:

- **Multi-architecture support**: 32, 64, 128, 256, and 512-bit architectures
- **Comprehensive instruction set**: Arithmetic, control flow, memory, cryptographic, SIMD, and parallel operations
- **Rust-to-bytecode transpilation**: Complete toolchain from Rust source code to optimized bytecode
- **Advanced execution features**: Debugging, profiling, and step-by-step execution
- **Database integration**: Seamless integration with DotDB for state management
- **Security and isolation**: Memory protection and secure execution environment

## Key Features

### Multi-Architecture Support

DotVM supports five different architectures, each optimized for specific use cases:

| Architecture | Word Size | Best For |
|--------------|-----------|----------|
| Arch32 | 32-bit | IoT, embedded systems |
| Arch64 | 64-bit | General-purpose applications (default) |
| Arch128 | 128-bit | Scientific computing, high precision |
| Arch256 | 256-bit | Cryptography |
| Arch512 | 512-bit | Advanced cryptography, research |

### Comprehensive Instruction Set

DotVM provides a rich set of instructions organized into categories:

- **Stack Operations**: `PUSH`, `POP`, `DUP`, `SWAP`, `ROT`
- **Arithmetic**: `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `NEG`
- **Control Flow**: `JMP`, `JZ`, `JNZ`, `CALL`, `RET`, `HALT`
- **Memory**: `LOAD`, `STORE`, `ALLOC`, `FREE`
- **Cryptographic**: `HASH`, `SIGN`, `VERIFY`, `ENCRYPT`, `DECRYPT`
- **Database**: `DB_GET`, `DB_PUT`, `DB_DELETE`, `DB_QUERY`
- **SIMD**: Vector operations for parallel processing
- **BigInt**: Arbitrary precision integer operations

### Transpilation Pipeline

The complete Rust-to-DotVM pipeline:

```
Rust Source Code
       |
   rustc (to WebAssembly)
       |
   WASM Parser
       |
   DotVM Translator
       |
   Architecture Optimizer
       |
   DotVM Bytecode
```

### Execution Environment

DotVM provides a sophisticated execution environment with:

- **Stack-based execution**: Efficient stack machine with configurable stack size
- **Memory management**: Protected memory with allocation tracking
- **Database bridge**: Direct integration with DotDB for persistent state
- **Debug capabilities**: Instruction-level debugging and profiling
- **Error handling**: Comprehensive error reporting and recovery

## Architecture Overview

### Virtual Machine Components

```
+-------------------------------------------------------------+
|                    DotVM Runtime                            |
+-------------------------------------------------------------+
|  +-------------+  +-------------+  +-------------------+  |
|  |   Executor  |  |   Memory    |  |   Database Bridge |  |
|  |             |  |  Manager    |  |                   |  |
|  +-------------+  +-------------+  +-------------------+  |
+-------------------------------------------------------------+
|  +-------------+  +-------------+  +-------------------+  |
|  | Instruction |  |    Stack    |  |   State Manager   |  |
|  |  Decoder    |  |   Machine   |  |                   |  |
|  +-------------+  +-------------+  +-------------------+  |
+-------------------------------------------------------------+
|                    Bytecode Loader                         |
+-------------------------------------------------------------+
```

### Bytecode Format

DotVM bytecode files have a structured format:

```
+-------------------------------------------------------------+
|                    Bytecode Header                         |
+-------------------------------------------------------------+
| Magic Number | Version | Architecture | Flags | Entry Point |
|   (4 bytes)  |(2 bytes)|   (1 byte)   |(1 byte)|  (8 bytes) |
+-------------------------------------------------------------+
|                    Code Section                            |
+-------------------------------------------------------------+
|                    Data Section                            |
+-------------------------------------------------------------+
|                   Debug Section                            |
|                   (optional)                               |
+-------------------------------------------------------------+
```

## Use Cases

### General Computing
- Web application backends
- Microservices
- Data processing pipelines
- Scientific computing

### Embedded Systems
- IoT devices
- Edge computing
- Real-time systems
- Resource-constrained environments

### Cryptographic Applications
- Cryptocurrency wallets
- Digital signature systems
- Secure communication
- Privacy-preserving protocols

## Performance Characteristics

### Execution Speed
- **Optimized instruction dispatch**: Fast instruction decoding and execution
- **Architecture-specific optimizations**: Tailored performance for each architecture
- **Minimal overhead**: Efficient stack operations and memory management
- **Parallel execution**: Support for concurrent operations (ParaDots)

### Memory Efficiency
- **Compact bytecode**: Efficient instruction encoding
- **Stack-based design**: Minimal register pressure
- **Memory protection**: Safe memory access with bounds checking
- **Garbage collection**: Automatic memory management (optional)

### Scalability
- **Multi-threading**: Parallel execution capabilities
- **State management**: Efficient state persistence and retrieval
- **Resource isolation**: Secure execution boundaries
- **Load balancing**: Distributed execution support

## Security Features

### Memory Safety
- **Bounds checking**: All memory accesses are validated
- **Stack overflow protection**: Configurable stack limits
- **Type safety**: Strong typing throughout execution
- **Memory isolation**: Programs cannot access unauthorized memory

### Execution Safety
- **Instruction validation**: All instructions are verified before execution
- **Resource limits**: Configurable limits on execution time and memory
- **Error handling**: Graceful handling of runtime errors
- **Sandboxing**: Isolated execution environment

### Cryptographic Security
- **Secure random number generation**: Cryptographically secure randomness
- **Hash function support**: Multiple hash algorithms
- **Digital signatures**: Built-in signature verification
- **Encryption support**: Symmetric and asymmetric encryption

## Integration with DotDB

DotVM seamlessly integrates with DotDB for state management:

### Database Operations
- **Direct access**: Native database opcodes for efficient operations
- **Transaction support**: Atomic operations across VM and database
- **Query capabilities**: Complex queries through VM instructions
- **State persistence**: Automatic state saving and restoration

### Performance Optimization
- **Caching**: Intelligent caching of frequently accessed data
- **Batch operations**: Efficient bulk database operations
- **Index utilization**: Automatic use of database indices
- **Connection pooling**: Efficient database connection management

## Development Workflow

### 1. Write Rust Code
```rust
fn main() {
    println!("Hello, DotVM!");
    let result = calculate_fibonacci(10);
    println!("Fibonacci(10) = {}", result);
}

fn calculate_fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),
    }
}
```

### 2. Transpile to Bytecode
```bash
dotvm transpile -i fibonacci.rs -o fibonacci.dotvm -a arch64 --opt-level 2
```

### 3. Execute Bytecode
```bash
dotvm run fibonacci.dotvm --verbose
```

### 4. Debug if Needed
```bash
dotvm run fibonacci.dotvm --debug --step
```

## CLI Tools

DotVM provides comprehensive command-line tools:

### Transpilation Tool
- **Input formats**: Rust source files or projects
- **Output formats**: DotVM bytecode files
- **Architecture selection**: Choose target VM architecture
- **Optimization levels**: Control compilation optimizations
- **Debug information**: Include debugging metadata

### Execution Tool
- **Bytecode execution**: Run DotVM bytecode files
- **Debug mode**: Step-by-step execution with state inspection
- **Profiling**: Performance analysis and timing information
- **Resource monitoring**: Memory and execution time tracking

## Getting Started

1. **Install DotVM**: Follow the [installation guide](../getting-started/installation.md)
2. **Try the quickstart**: Complete the [quickstart tutorial](../getting-started/quickstart.md)
3. **Learn the CLI**: Read the [CLI reference](../cli/dotvm.md)
4. **Explore examples**: Check out example programs in the repository
5. **Read the architecture docs**: Understand [VM architectures](architecture/vm-architectures.md)

## Next Steps

- **Architecture Details**: Learn about [VM architectures](architecture/vm-architectures.md)
- **Instruction Set**: Explore the [instruction set reference](architecture/instruction-set.md)
- **Bytecode Format**: Understand the [bytecode format](architecture/bytecode-format.md)
- **Usage Guide**: Read about [basic operations](usage/basic-operations.md)
- **API Reference**: Check the [Core API documentation](api/core.md)