# DotVM CLI Reference

The DotVM CLI provides tools for transpiling Rust code to DotVM bytecode and executing bytecode programs.

## Installation

The DotVM CLI is built as part of the workspace. To build and install:

```bash
cargo build --release --bin dotvm
# The binary will be available at target/release/dotvm
```

## Commands Overview

The DotVM CLI provides two main commands:

- `transpile`: Transpile Rust code to DotVM bytecode
- `run`: Execute DotVM bytecode files

## Global Options

```
dotvm [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Transpile Command

Transpile Rust source code to DotVM bytecode through the Rust → WebAssembly → DotVM pipeline.

### Usage

```bash
dotvm transpile [OPTIONS] --input <INPUT> --output <OUTPUT>
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input <INPUT>` | `-i` | Input Rust source file or project directory | Required |
| `--output <OUTPUT>` | `-o` | Output DotVM bytecode file | Required |
| `--architecture <ARCH>` | `-a` | Target VM architecture | `arch64` |
| `--opt-level <LEVEL>` | | Optimization level (0-3) | `2` |
| `--debug` | | Enable debug information | `false` |
| `--verbose` | `-v` | Verbose output | `false` |
| `--keep-intermediate` | | Keep intermediate files (Wasm) | `false` |
| `--target-dir <DIR>` | | Custom target directory for Rust compilation | None |

### Architecture Options

- `arch64`: 64-bit architecture (default)
- `arch128`: 128-bit architecture
- `arch256`: 256-bit architecture
- `arch512`: 512-bit architecture

### Examples

**Basic transpilation:**
```bash
dotvm transpile -i src/main.rs -o program.dotvm
```

**With specific architecture:**
```bash
dotvm transpile -i src/main.rs -o program.dotvm -a arch256
```

**Debug build with verbose output:**
```bash
dotvm transpile -i src/main.rs -o program.dotvm --debug --verbose
```

**Keep intermediate files:**
```bash
dotvm transpile -i src/main.rs -o program.dotvm --keep-intermediate
```

### Transpilation Pipeline

The transpilation process follows these steps:

1. **Rust Compilation**: Compile Rust source to WebAssembly
2. **Wasm Parsing**: Parse WebAssembly module
3. **DotVM Translation**: Translate Wasm instructions to DotVM bytecode
4. **Optimization**: Apply architecture-specific optimizations
5. **Bytecode Generation**: Generate final DotVM bytecode file

## Run Command

Execute DotVM bytecode files with debugging and profiling capabilities.

### Usage

```bash
dotvm run [OPTIONS] <BYTECODE_FILE>
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `<BYTECODE_FILE>` | | Path to the bytecode file to execute | Required |
| `--debug` | `-d` | Enable debug mode (shows instruction execution) | `false` |
| `--step` | `-s` | Enable step mode (pause after each instruction) | `false` |
| `--max-instructions <NUM>` | | Maximum number of instructions to execute | `1000000` |
| `--verbose` | `-v` | Verbose output | `false` |

### Examples

**Basic execution:**
```bash
dotvm run program.dotvm
```

**Debug execution:**
```bash
dotvm run program.dotvm --debug
```

**Step-by-step execution:**
```bash
dotvm run program.dotvm --step
```

**Verbose execution with custom instruction limit:**
```bash
dotvm run program.dotvm --verbose --max-instructions 500000
```

### Debug Mode

When `--debug` is enabled, the executor will:
- Show each instruction as it's executed
- Display stack contents after each operation
- Show memory access patterns
- Report execution statistics

### Step Mode

When `--step` is enabled, the executor will:
- Pause after each instruction
- Wait for user input to continue
- Allow inspection of VM state
- Provide interactive debugging capabilities

### Output Information

The run command provides detailed execution information:

```
Execution completed!
Instructions executed: 1234
Execution time: 15.2ms
Total time: 18.7ms
Final stack size: 1
Final stack contents:
  [0]: 42
Program counter: 1234
Halted: true
```

## Error Handling

The CLI provides detailed error messages for common issues:

### Transpilation Errors

- **File not found**: Check input file path
- **Compilation failed**: Review Rust compilation errors
- **Wasm parsing failed**: Ensure valid Rust code
- **Translation failed**: Check for unsupported features

### Execution Errors

- **Invalid bytecode**: File may be corrupted or wrong format
- **Execution timeout**: Increase `--max-instructions` limit
- **Memory errors**: Check for stack overflow or invalid memory access
- **Architecture mismatch**: Ensure bytecode matches VM architecture

## Integration with DOTDB

The DotVM executor automatically integrates with DOTDB for state management:

- Database operations are available through DB opcodes
- State persistence across executions
- Transaction support for atomic operations
- Query capabilities for complex data operations

## Performance Tips

1. **Use appropriate architecture**: Match architecture to your data size requirements
2. **Optimize compilation**: Use higher optimization levels for production
3. **Profile execution**: Use debug mode to identify bottlenecks
4. **Batch operations**: Group database operations for better performance

## Examples

### Complete Workflow

1. **Create a simple Rust program:**
```rust
// src/main.rs
fn main() {
    let result = 40 + 2;
    println!("The answer is: {}", result);
}
```

2. **Transpile to DotVM bytecode:**
```bash
dotvm transpile -i src/main.rs -o answer.dotvm --verbose
```

3. **Execute the bytecode:**
```bash
dotvm run answer.dotvm --debug
```

### Advanced Example

1. **Transpile with optimizations:**
```bash
dotvm transpile \
  -i complex_program.rs \
  -o optimized.dotvm \
  -a arch256 \
  --opt-level 3 \
  --debug
```

2. **Execute with profiling:**
```bash
dotvm run optimized.dotvm --verbose --max-instructions 10000000
```