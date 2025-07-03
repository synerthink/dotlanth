# VM Architectures

DotVM supports multiple virtual machine architectures, each optimized for different use cases and data sizes. The architecture determines the word size, register width, and memory addressing capabilities of the virtual machine.

## Supported Architectures

### Arch32 (32-bit)
- **Word Size**: 32 bits (4 bytes)
- **Address Space**: 4 GB (2^32 bytes)
- **Register Width**: 32 bits
- **Use Cases**: Lightweight applications, embedded systems, memory-constrained environments

**Characteristics:**
- Minimal memory footprint
- Fast execution for simple operations
- Limited address space
- Suitable for IoT and embedded applications

### Arch64 (64-bit) - Default
- **Word Size**: 64 bits (8 bytes)
- **Address Space**: 16 EB (2^64 bytes)
- **Register Width**: 64 bits
- **Use Cases**: General-purpose applications, standard desktop/server workloads

**Characteristics:**
- Balanced performance and memory usage
- Standard architecture for most applications
- Good compatibility with modern systems
- Default choice for most use cases

### Arch128 (128-bit)
- **Word Size**: 128 bits (16 bytes)
- **Address Space**: 2^128 bytes (theoretical)
- **Register Width**: 128 bits
- **Use Cases**: High-precision arithmetic, cryptographic applications, scientific computing

**Characteristics:**
- Enhanced precision for mathematical operations
- Native support for 128-bit integers
- Optimized for cryptographic operations
- Suitable for financial and scientific applications

### Arch256 (256-bit)
- **Word Size**: 256 bits (32 bytes)
- **Address Space**: 2^256 bytes (theoretical)
- **Register Width**: 256 bits
- **Use Cases**: Advanced cryptography, blockchain applications, high-performance computing

**Characteristics:**
- Maximum precision for arithmetic operations
- Native support for 256-bit integers
- Optimized for blockchain and cryptocurrency operations
- Excellent for cryptographic hash functions

### Arch512 (512-bit)
- **Word Size**: 512 bits (64 bytes)
- **Address Space**: 2^512 bytes (theoretical)
- **Register Width**: 512 bits
- **Use Cases**: Specialized cryptographic applications, research, extreme precision requirements

**Characteristics:**
- Highest precision available
- Native support for 512-bit integers
- Specialized for advanced cryptographic research
- Maximum computational precision

## Architecture Selection Guide

### When to Use Each Architecture

| Architecture | Best For | Performance | Memory Usage | Precision |
|--------------|----------|-------------|--------------|-----------|
| Arch32 | IoT, embedded systems | Fast | Low | Standard |
| Arch64 | General applications | Balanced | Moderate | Good |
| Arch128 | Scientific computing | Good | Higher | High |
| Arch256 | Blockchain, crypto | Specialized | High | Very High |
| Arch512 | Research, extreme precision | Specialized | Highest | Maximum |

### Performance Characteristics

```
Execution Speed (relative):
Arch32:  ████████████████████████████████ (fastest)
Arch64:  ████████████████████████████
Arch128: ████████████████████████
Arch256: ████████████████████
Arch512: ████████████████ (specialized)

Memory Usage (relative):
Arch32:  ████████ (lowest)
Arch64:  ████████████████
Arch128: ████████████████████████
Arch256: ████████████████████████████████
Arch512: ████████████████████████████████████████ (highest)
```

## Architecture-Specific Features

### Arithmetic Operations

Each architecture provides native support for operations matching its word size:

**Arch32:**
- 32-bit integer arithmetic
- 32-bit floating-point operations
- Basic cryptographic primitives

**Arch64:**
- 64-bit integer arithmetic
- 64-bit floating-point operations
- Standard cryptographic operations
- Compatibility with most programming languages

**Arch128:**
- 128-bit integer arithmetic
- Extended precision floating-point
- Advanced cryptographic primitives
- UUID and GUID operations

**Arch256:**
- 256-bit integer arithmetic
- Blockchain-optimized operations
- SHA-256 native support
- Elliptic curve cryptography

**Arch512:**
- 512-bit integer arithmetic
- Research-grade cryptographic operations
- Advanced hash functions
- Experimental mathematical operations

### Memory Model

Each architecture has different memory alignment and addressing requirements:

```rust
// Memory layout examples
Arch32:  [32-bit word][32-bit word][32-bit word]...
Arch64:  [64-bit word][64-bit word][64-bit word]...
Arch128: [128-bit word][128-bit word]...
Arch256: [256-bit word][256-bit word]...
Arch512: [512-bit word]...
```

### Instruction Set Variations

While all architectures share the same basic instruction set, some instructions behave differently:

**Stack Operations:**
- `PUSH`: Pushes architecture-sized values
- `POP`: Pops architecture-sized values
- `DUP`: Duplicates top architecture-sized value

**Arithmetic Operations:**
- `ADD`, `SUB`, `MUL`, `DIV`: Operate on architecture-sized integers
- `FADD`, `FSUB`, `FMUL`, `FDIV`: Operate on architecture-sized floats

**Memory Operations:**
- `LOAD`, `STORE`: Transfer architecture-sized words
- `LOAD_BYTE`, `STORE_BYTE`: Always operate on single bytes

## Bytecode Compatibility

Bytecode files are architecture-specific and contain the target architecture in their header:

```
Bytecode Header:
[Magic: "DOTVM"][Version][Architecture][Flags][Entry Point]
```

**Architecture Encoding:**
- Arch32: 0x00
- Arch64: 0x01
- Arch128: 0x02
- Arch256: 0x03
- Arch512: 0x04

## Transpilation Considerations

When transpiling Rust code to different architectures:

### Data Type Mapping

| Rust Type | Arch32 | Arch64 | Arch128 | Arch256 | Arch512 |
|-----------|--------|--------|---------|---------|---------|
| `i32` | Native | Promoted | Promoted | Promoted | Promoted |
| `i64` | Split | Native | Promoted | Promoted | Promoted |
| `i128` | Emulated | Split | Native | Promoted | Promoted |
| `usize` | 32-bit | 64-bit | 128-bit | 256-bit | 512-bit |

### Performance Implications

**Arch32 Considerations:**
- Large integers require multiple operations
- Limited address space may require memory management
- Fastest for simple operations

**Arch64 Considerations:**
- Best balance of performance and compatibility
- Native support for most common data types
- Recommended for general use

**Arch128+ Considerations:**
- Larger memory footprint
- Slower for simple operations
- Faster for operations matching the architecture size

## Runtime Architecture Detection

The VM runtime automatically detects and validates architecture compatibility:

```rust
// Pseudocode for architecture validation
fn validate_bytecode(bytecode: &[u8]) -> Result<(), ArchError> {
    let header = parse_header(bytecode)?;
    let runtime_arch = detect_runtime_architecture();
    
    if header.architecture != runtime_arch {
        return Err(ArchError::Mismatch {
            expected: runtime_arch,
            found: header.architecture,
        });
    }
    
    Ok(())
}
```

## Best Practices

### Architecture Selection

1. **Start with Arch64**: Use the default 64-bit architecture unless you have specific requirements
2. **Consider Data Types**: Choose architecture based on your primary data types
3. **Evaluate Performance**: Profile your application with different architectures
4. **Memory Constraints**: Use smaller architectures for memory-limited environments

### Optimization Tips

1. **Match Operations to Architecture**: Use operations that align with your chosen architecture
2. **Avoid Unnecessary Precision**: Don't use larger architectures unless needed
3. **Consider Cache Effects**: Larger word sizes may impact cache performance
4. **Profile Real Workloads**: Test with realistic data and operations

### Cross-Architecture Development

1. **Abstract Data Types**: Use appropriate abstractions in your Rust code
2. **Test Multiple Architectures**: Validate behavior across different architectures
3. **Document Requirements**: Clearly specify architecture requirements
4. **Consider Portability**: Design for multiple architectures when possible

## Examples

### Selecting Architecture for Different Use Cases

**IoT Sensor Application (Arch32):**
```bash
dotvm transpile -i sensor.rs -o sensor.dotvm -a arch32
```

**Web Application Backend (Arch64):**
```bash
dotvm transpile -i webapp.rs -o webapp.dotvm -a arch64
```

**Cryptocurrency Wallet (Arch256):**
```bash
dotvm transpile -i wallet.rs -o wallet.dotvm -a arch256
```

**Scientific Computing (Arch128):**
```bash
dotvm transpile -i simulation.rs -o simulation.dotvm -a arch128
```

### Performance Comparison

```bash
# Create test program
echo 'fn main() { 
    let mut sum = 0u64; 
    for i in 0..1000000 { 
        sum += i; 
    } 
    println!("Sum: {}", sum); 
}' > perf_test.rs

# Test different architectures
dotvm transpile -i perf_test.rs -o test_32.dotvm -a arch32
dotvm transpile -i perf_test.rs -o test_64.dotvm -a arch64
dotvm transpile -i perf_test.rs -o test_128.dotvm -a arch128

# Compare execution times
time dotvm run test_32.dotvm
time dotvm run test_64.dotvm  
time dotvm run test_128.dotvm
```

## Future Considerations

The architecture system is designed to be extensible:

- **New Architectures**: Additional architectures can be added as needed
- **Specialized Instructions**: Architecture-specific instruction sets
- **Hardware Acceleration**: Native hardware support for specific architectures
- **Dynamic Architecture**: Runtime architecture switching (future feature)

For more information about specific instruction sets, see the [Instruction Set](instruction-set.md) documentation.