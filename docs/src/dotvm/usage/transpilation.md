# Transpilation Guide

This guide covers the complete process of transpiling Rust source code to DotVM bytecode, including optimization strategies, debugging techniques, and advanced features.

## Transpilation Overview

The DotVM transpilation process converts Rust source code to DotVM bytecode through a multi-stage pipeline:

```
Rust Source Code
       ↓
   Rust Compiler (rustc)
       ↓
   WebAssembly (WASM)
       ↓
   WASM Parser
       ↓
   DotVM Translator
       ↓
   Architecture Optimizer
       ↓
   DotVM Bytecode
```

## Basic Transpilation

### Simple Example

Let's start with a basic Rust program:

```rust
// hello.rs
fn main() {
    println!("Hello, DotVM!");
    let result = add_numbers(10, 5);
    println!("Result: {}", result);
}

fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}
```

**Transpile to bytecode:**
```bash
dotvm transpile -i hello.rs -o hello.dotvm
```

**Execute the bytecode:**
```bash
dotvm run hello.dotvm
```

### Command Line Options

The `dotvm transpile` command supports various options:

```bash
dotvm transpile [OPTIONS] --input <INPUT> --output <OUTPUT>
```

**Required Options:**
- `-i, --input <INPUT>`: Input Rust source file or project directory
- `-o, --output <OUTPUT>`: Output DotVM bytecode file

**Optional Parameters:**
- `-a, --architecture <ARCH>`: Target architecture (arch32, arch64, arch128, arch256, arch512)
- `--opt-level <LEVEL>`: Optimization level (0-3)
- `--debug`: Include debug information
- `-v, --verbose`: Verbose output
- `--keep-intermediate`: Keep intermediate WASM files
- `--target-dir <DIR>`: Custom target directory for Rust compilation

## Architecture Selection

### Choosing the Right Architecture

Select the architecture based on your application's requirements:

**Arch64 (Default):**
```bash
dotvm transpile -i program.rs -o program.dotvm -a arch64
```
- **Best for**: General-purpose applications
- **Word size**: 64 bits
- **Memory**: Up to 16 EB addressable
- **Performance**: Balanced

**Arch128 (Scientific):**
```bash
dotvm transpile -i simulation.rs -o simulation.dotvm -a arch128
```
- **Best for**: Scientific computing, high-precision calculations
- **Word size**: 128 bits
- **Features**: Extended precision arithmetic
- **Use cases**: Mathematical simulations, financial calculations

### Architecture-Specific Considerations

**Data Type Mapping:**

| Rust Type | Arch32 | Arch64 | Arch128 | Arch256 | Arch512 |
|-----------|--------|--------|---------|---------|---------|
| `i32` | Native | Promoted | Promoted | Promoted | Promoted |
| `i64` | Emulated | Native | Promoted | Promoted | Promoted |
| `i128` | Emulated | Emulated | Native | Promoted | Promoted |
| `usize` | 32-bit | 64-bit | 128-bit | 256-bit | 512-bit |

## Optimization Levels

### Optimization Strategies

DotVM supports four optimization levels:

**Level 0 (No Optimization):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 0
```
- **Purpose**: Fastest compilation, debugging
- **Features**: No optimizations applied
- **Use cases**: Development, debugging

**Level 1 (Basic Optimization):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 1
```
- **Purpose**: Basic optimizations with fast compilation
- **Features**: Dead code elimination, basic constant folding
- **Use cases**: Development builds with some optimization

**Level 2 (Standard Optimization - Default):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 2
```
- **Purpose**: Good balance of compilation time and performance
- **Features**: Advanced optimizations, inlining, loop optimizations
- **Use cases**: Production builds, general use

**Level 3 (Maximum Optimization):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 3
```
- **Purpose**: Maximum performance
- **Features**: Aggressive optimizations, vectorization
- **Use cases**: Performance-critical applications

### Optimization Examples

**Performance Comparison:**
```rust
// fibonacci.rs
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let result = fibonacci(30);
    println!("Fibonacci(30) = {}", result);
}
```

**Compile with different optimization levels:**
```bash
# No optimization
dotvm transpile -i fibonacci.rs -o fib_o0.dotvm --opt-level 0
time dotvm run fib_o0.dotvm

# Maximum optimization
dotvm transpile -i fibonacci.rs -o fib_o3.dotvm --opt-level 3
time dotvm run fib_o3.dotvm
```

## Debug Information

### Including Debug Information

Enable debug information for better debugging experience:

```bash
dotvm transpile -i program.rs -o program.dotvm --debug --verbose
```

**Debug information includes:**
- Source code mapping
- Function names and locations
- Variable names and scopes
- Line number information

### Debugging Transpiled Code

**Step-by-step execution:**
```bash
dotvm run program.dotvm --debug --step
```

**Debug output example:**
```
[DEBUG] Loading bytecode: program.dotvm
[DEBUG] Architecture: Arch64
[DEBUG] Entry point: 0x0000000000000000
[DEBUG] Executing instruction at 0x00000000: PUSH 0x00000001
[DEBUG] Stack: [1]
[DEBUG] Executing instruction at 0x00000004: PUSH 0x00000002
[DEBUG] Stack: [1, 2]
[DEBUG] Executing instruction at 0x00000008: ADD
[DEBUG] Stack: [3]
```

## Advanced Transpilation Features

### Project Transpilation

Transpile entire Rust projects:

```bash
# Transpile a Cargo project
dotvm transpile -i ./my_project -o my_project.dotvm --target-dir ./target
```

**Project structure:**
```
my_project/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── modules/
│       ├── mod.rs
│       └── utils.rs
└── target/          # Build artifacts
```

### Custom Target Directory

Use a custom target directory for Rust compilation:

```bash
dotvm transpile -i project.rs -o project.dotvm --target-dir /tmp/dotvm_build
```

### Keeping Intermediate Files

Keep intermediate WASM files for analysis:

```bash
dotvm transpile -i program.rs -o program.dotvm --keep-intermediate
```

**Generated files:**
```
program.rs          # Original source
program.wasm        # Intermediate WASM
program.dotvm       # Final bytecode
```

## Error Handling and Troubleshooting

### Common Transpilation Errors

**Rust Compilation Errors:**
```bash
error[E0425]: cannot find value `undefined_variable` in this scope
 --> src/main.rs:3:13
  |
3 |     println!("{}", undefined_variable);
  |             ^^^^^^^^^^^^^^^^^^^ not found in this scope
```
**Solution:** Fix Rust compilation errors first.

**WASM Parsing Errors:**
```
Error: Failed to parse WebAssembly module
Caused by: Invalid WASM magic number
```
**Solution:** Ensure Rust code compiles to valid WebAssembly.

**Translation Errors:**
```
Error: Unsupported WASM instruction: unreachable
```
**Solution:** Avoid unsupported Rust features or WASM instructions.

### Debugging Transpilation Issues

**Verbose Output:**
```bash
dotvm transpile -i program.rs -o program.dotvm --verbose
```

**Check Intermediate WASM:**
```bash
dotvm transpile -i program.rs -o program.dotvm --keep-intermediate
wasm-objdump -d program.wasm  # Inspect WASM
```

**Validate Bytecode:**
```bash
dotvm validate program.dotvm
```

## Performance Optimization

### Rust Code Optimization

**Optimize Rust source for better transpilation:**

```rust
// Use appropriate data types
fn efficient_calculation(data: &[i32]) -> i32 {
    // Use iterators for better optimization
    data.iter().sum()
}

// Avoid unnecessary allocations
fn process_string(s: &str) -> String {
    // Use string slices when possible
    s.to_uppercase()
}

// Use const for compile-time constants
const BUFFER_SIZE: usize = 1024;

fn main() {
    let data = vec![1, 2, 3, 4, 5];
    let result = efficient_calculation(&data);
    println!("Result: {}", result);
}
```

### Architecture-Specific Optimization

**Optimize for target architecture:**

```rust
// For Arch256 (high-security applications)
fn hash_calculation(data: &[u8]) -> [u8; 32] {
    // Use 256-bit operations
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

// For Arch128 (scientific computing)
fn high_precision_calculation(a: f64, b: f64) -> f64 {
    // Use high-precision arithmetic
    (a * b).sqrt()
}
```

### Compilation Flags

**Optimize Rust compilation:**

```bash
# Create optimized WASM
export RUSTFLAGS="-C target-cpu=native -C opt-level=3"
dotvm transpile -i program.rs -o program.dotvm --opt-level 3
```

## Integration Examples

### Database Integration

```rust
// database_app.rs
fn main() {
    // This will use DotDB opcodes in the generated bytecode
    store_user_data("alice", 30);
    let age = get_user_age("alice");
    println!("Alice's age: {}", age);
}

fn store_user_data(name: &str, age: u32) {
    // In the transpiled bytecode, this becomes DB_PUT operations
    println!("Storing user: {} age: {}", name, age);
}

fn get_user_age(name: &str) -> u32 {
    // In the transpiled bytecode, this becomes DB_GET operations
    println!("Getting age for user: {}", name);
    30 // Placeholder
}
```

**Transpile and run:**
```bash
dotvm transpile -i database_app.rs -o database_app.dotvm
dotvm run database_app.dotvm --verbose
```

### Cryptographic Operations

```rust
// crypto_app.rs
use sha2::{Sha256, Digest};

fn main() {
    let data = b"Hello, DotVM!";
    let hash = calculate_hash(data);
    println!("Hash: {:?}", hash);
}

fn calculate_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
```

**Transpile for high-security architecture:**
```bash
dotvm transpile -i crypto_app.rs -o crypto_app.dotvm -a arch256 --opt-level 3
```

## Best Practices

### Code Organization

1. **Modular Design**: Organize code into logical modules
2. **Function Size**: Keep functions reasonably sized for better optimization
3. **Data Structures**: Use appropriate data structures for your use case
4. **Error Handling**: Implement proper error handling

### Performance Guidelines

1. **Choose Appropriate Architecture**: Match architecture to your data requirements
2. **Optimize Hot Paths**: Focus optimization on frequently executed code
3. **Profile Performance**: Use timing and profiling tools
4. **Test Different Optimization Levels**: Compare performance across optimization levels

### Development Workflow

1. **Start with Arch64**: Use default architecture for development
2. **Enable Debug Information**: Use `--debug` during development
3. **Incremental Testing**: Test transpilation frequently during development
4. **Performance Testing**: Benchmark critical code paths

## Automation and Scripting

### Build Scripts

Create build scripts for automated transpilation:

```bash
#!/bin/bash
# build.sh

set -e

echo "Building DotVM bytecode..."

# Clean previous builds
rm -f *.dotvm

# Transpile for different architectures
dotvm transpile -i src/main.rs -o app_64.dotvm -a arch64 --opt-level 2
dotvm transpile -i src/main.rs -o app_256.dotvm -a arch256 --opt-level 2

# Validate bytecode
dotvm validate app_64.dotvm
dotvm validate app_256.dotvm

echo "Build complete!"
```

### Makefile Example

```makefile
# Makefile for DotVM projects

RUST_SRC = src/main.rs
BYTECODE_64 = app_64.dotvm
BYTECODE_256 = app_256.dotvm

.PHONY: all clean test

all: $(BYTECODE_64) $(BYTECODE_256)

$(BYTECODE_64): $(RUST_SRC)
	dotvm transpile -i $(RUST_SRC) -o $(BYTECODE_64) -a arch64 --opt-level 2

$(BYTECODE_256): $(RUST_SRC)
	dotvm transpile -i $(RUST_SRC) -o $(BYTECODE_256) -a arch256 --opt-level 2

test: $(BYTECODE_64)
	dotvm run $(BYTECODE_64) --verbose

clean:
	rm -f *.dotvm *.wasm

debug: $(RUST_SRC)
	dotvm transpile -i $(RUST_SRC) -o debug.dotvm --debug --opt-level 0
	dotvm run debug.dotvm --debug --step
```

For more information about executing bytecode, see the [Execution Guide](execution.md).