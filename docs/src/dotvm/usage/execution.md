# Execution Guide

This guide covers executing DotVM bytecode, including runtime options, debugging techniques, performance monitoring, and advanced execution features.

## Basic Execution

### Running Bytecode

The simplest way to execute DotVM bytecode:

```bash
dotvm run program.dotvm
```

**Example output:**
```
Execution completed!
Instructions executed: 1234
Execution time: 15.2ms
Total time: 18.7ms
```

### Command Line Options

The `dotvm run` command supports various execution options:

```bash
dotvm run [OPTIONS] <BYTECODE_FILE>
```

**Options:**
- `<BYTECODE_FILE>`: Path to the bytecode file (required)
- `-d, --debug`: Enable debug mode
- `-s, --step`: Enable step-by-step execution
- `--max-instructions <NUM>`: Maximum instruction limit (default: 1,000,000)
- `-v, --verbose`: Verbose output

## Debug Mode

### Enabling Debug Mode

Debug mode provides detailed execution information:

```bash
dotvm run program.dotvm --debug
```

**Debug output includes:**
- Instruction-by-instruction execution
- Stack state after each operation
- Memory access patterns
- Function calls and returns
- Error details

**Example debug output:**
```
[DEBUG] Loading bytecode: program.dotvm
[DEBUG] Architecture: Arch64, Entry point: 0x0000000000000000
[DEBUG] Starting execution...

[0x00000000] PUSH 0x00000001          Stack: [1]
[0x00000004] PUSH 0x00000002          Stack: [1, 2]
[0x00000008] ADD                      Stack: [3]
[0x00000009] SYSCALL_PRINT            Stack: []
Output: 3
[0x0000000A] HALT                     Stack: []

[DEBUG] Execution completed
[DEBUG] Instructions executed: 4
[DEBUG] Final stack size: 0
[DEBUG] Program counter: 0x0000000A
[DEBUG] Halted: true
```

### Debug Information

When bytecode includes debug information (compiled with `--debug`):

```bash
dotvm run debug_program.dotvm --debug
```

**Enhanced debug output:**
```
[DEBUG] Source: main.rs:5:13
[0x00000000] PUSH 0x00000001          // let x = 1;
             Stack: [1]

[DEBUG] Source: main.rs:6:13  
[0x00000004] PUSH 0x00000002          // let y = 2;
             Stack: [1, 2]

[DEBUG] Source: main.rs:7:13
[0x00000008] ADD                      // let result = x + y;
             Stack: [3]
```

## Step Mode

### Interactive Debugging

Step mode allows interactive debugging with manual control:

```bash
dotvm run program.dotvm --step
```

**Interactive session:**
```
[STEP] Press Enter to execute next instruction, 'q' to quit, 'i' for info
[0x00000000] PUSH 0x00000001          Stack: [1]
> [Enter]

[0x00000004] PUSH 0x00000002          Stack: [1, 2]  
> i
Stack: [1, 2]
PC: 0x00000004
Memory usage: 1024 bytes
> [Enter]

[0x00000008] ADD                      Stack: [3]
> q
Execution stopped by user
```

### Step Mode Commands

Available commands in step mode:

| Command | Description |
|---------|-------------|
| `Enter` | Execute next instruction |
| `q` | Quit execution |
| `i` | Show VM state information |
| `s` | Show stack contents |
| `m <addr>` | Show memory at address |
| `c` | Continue execution (exit step mode) |
| `h` | Show help |

## Verbose Mode

### Detailed Execution Information

Verbose mode provides comprehensive execution details:

```bash
dotvm run program.dotvm --verbose
```

**Verbose output:**
```
Loading bytecode from: program.dotvm
Bytecode size: 1024 bytes
Architecture: Arch64
Entry point: 0x0000000000000000
Code section: 512 bytes
Data section: 256 bytes
Debug section: 256 bytes

Bytecode loaded in 1.2ms
Initializing VM...
Stack size: 1MB
Memory pool: 10MB
Database bridge: Connected

Starting execution...
Instruction cache: Enabled
Branch prediction: Enabled
Optimization level: 2

Hello, DotVM!
Result: 42

Execution completed!
Instructions executed: 1,234
Execution time: 15.2ms
Load time: 1.2ms
Total time: 16.4ms

Performance metrics:
- Instructions per second: 81,250
- Memory usage: 2.1MB peak
- Cache hit ratio: 94.2%
- Database operations: 5
```

## Performance Monitoring

### Execution Statistics

DotVM provides detailed performance statistics:

```rust
pub struct ExecutionResult {
    pub instructions_executed: u64,
    pub execution_time: Duration,
    pub final_stack: Vec<Value>,
    pub pc: u64,
    pub halted: bool,
    pub memory_usage: MemoryStats,
    pub cache_stats: CacheStats,
}
```

### Timing Analysis

**Measure execution performance:**
```bash
# Basic timing
time dotvm run program.dotvm

# Detailed timing with verbose mode
dotvm run program.dotvm --verbose | grep "time:"
```

**Performance comparison:**
```bash
# Compare different architectures
time dotvm run program_64.dotvm   # Arch64
time dotvm run program_256.dotvm  # Arch256

# Compare optimization levels
time dotvm run program_o0.dotvm   # No optimization
time dotvm run program_o3.dotvm   # Maximum optimization
```

### Memory Monitoring

Monitor memory usage during execution:

```bash
# Run with memory monitoring
dotvm run large_program.dotvm --verbose
```

**Memory statistics:**
```
Memory Statistics:
- Initial allocation: 10MB
- Peak usage: 45MB
- Final usage: 12MB
- Allocations: 1,234
- Deallocations: 1,200
- Fragmentation: 5.2%
```

## Error Handling

### Runtime Errors

DotVM provides detailed error information:

**Stack overflow:**
```
Error: Stack overflow
  at instruction 0x00001234 (PUSH)
  Stack size: 1048576 bytes (limit reached)
  
Suggestion: Increase stack size or check for infinite recursion
```

**Division by zero:**
```
Error: Division by zero
  at instruction 0x00000045 (DIV)
  Stack: [10, 0]
  Source: calculator.rs:15:8
  
Suggestion: Add zero-check before division
```

**Memory access violation:**
```
Error: Invalid memory access
  at instruction 0x00000067 (LOAD)
  Address: 0x00001000 (out of bounds)
  Valid range: 0x00000000 - 0x00000FFF
  
Suggestion: Check array bounds and pointer arithmetic
```

### Error Recovery

Some errors allow for recovery:

```bash
# Continue execution after recoverable errors
dotvm run program.dotvm --continue-on-error
```

**Recoverable errors:**
- Non-fatal arithmetic errors
- Recoverable I/O errors
- Database connection issues

## Advanced Execution Features

### Instruction Limits

Prevent infinite loops with instruction limits:

```bash
# Limit to 100,000 instructions
dotvm run program.dotvm --max-instructions 100000
```

**Timeout behavior:**
```
Error: Instruction limit exceeded
Instructions executed: 100,000
Execution time: 1.2s

Suggestion: Increase limit or optimize algorithm
```

### Custom Stack Size

Configure stack size for memory-intensive programs:

```bash
# Set custom stack size (implementation-dependent)
DOTVM_STACK_SIZE=2MB dotvm run program.dotvm
```

### Database Integration

Execute programs with database operations:

```bash
# Ensure database is available
dotdb create-collection users
dotdb put users '{"name": "Alice", "age": 30}'

# Run program that uses database
dotvm run database_program.dotvm --verbose
```

**Database operation output:**
```
[DB] GET users.alice -> {"name": "Alice", "age": 30}
[DB] PUT users.bob <- {"name": "Bob", "age": 25}
[DB] COUNT users -> 2
```

## Profiling and Optimization

### Performance Profiling

Profile program execution for optimization:

```bash
# Run with profiling enabled
dotvm run program.dotvm --profile
```

**Profiling output:**
```
Profiling Results:
Function                 | Calls | Total Time | Avg Time | % Total
-------------------------|-------|------------|----------|--------
main                     |     1 |    15.2ms  |  15.2ms  |  100.0%
calculate_fibonacci      |   177 |    12.8ms  |   0.07ms |   84.2%
print_result            |     1 |     2.1ms  |   2.1ms  |   13.8%
add_numbers             |    88 |     0.3ms  |   0.003ms|    2.0%

Hotspots:
1. calculate_fibonacci (84.2% of execution time)
2. print_result (13.8% of execution time)

Optimization suggestions:
- Consider memoization for calculate_fibonacci
- Use iterative approach instead of recursion
```

### Instruction Analysis

Analyze instruction usage patterns:

```bash
dotvm run program.dotvm --analyze-instructions
```

**Instruction analysis:**
```
Instruction Usage:
PUSH:           45.2% (1,234 executions)
ADD:            12.3% (336 executions)
CALL:           8.7% (238 executions)
RET:            8.7% (238 executions)
JZ:             6.1% (167 executions)
...

Performance impact:
- Arithmetic operations: 67.8% of execution time
- Control flow: 18.9% of execution time
- Memory operations: 8.2% of execution time
- Database operations: 5.1% of execution time
```

## Batch Execution

### Running Multiple Programs

Execute multiple bytecode files:

```bash
# Sequential execution
for file in *.dotvm; do
    echo "Running $file..."
    dotvm run "$file"
done

# Parallel execution (if supported)
dotvm run-batch *.dotvm --parallel
```

### Automated Testing

Create test suites for bytecode programs:

```bash
#!/bin/bash
# test_suite.sh

test_cases=(
    "test_arithmetic.dotvm"
    "test_control_flow.dotvm"
    "test_database.dotvm"
    "test_crypto.dotvm"
)

for test in "${test_cases[@]}"; do
    echo "Running test: $test"
    if dotvm run "$test" --quiet; then
        echo "✓ PASS: $test"
    else
        echo "✗ FAIL: $test"
        exit 1
    fi
done

echo "All tests passed!"
```

## Integration Examples

### Web Service Backend

```rust
// web_service.rs
fn main() {
    println!("Starting web service...");
    
    // Initialize database
    setup_database();
    
    // Process requests
    handle_request("GET", "/users");
    handle_request("POST", "/users");
    
    println!("Service ready!");
}

fn setup_database() {
    println!("Setting up database...");
    // Database operations will use DotDB opcodes
}

fn handle_request(method: &str, path: &str) {
    println!("Handling {} {}", method, path);
    // Request processing logic
}
```

**Execute the service:**
```bash
dotvm transpile -i web_service.rs -o web_service.dotvm
dotvm run web_service.dotvm --verbose
```

### Data Processing Pipeline

```rust
// data_pipeline.rs
fn main() {
    let data = load_data();
    let processed = process_data(data);
    save_results(processed);
}

fn load_data() -> Vec<i32> {
    println!("Loading data...");
    vec![1, 2, 3, 4, 5]
}

fn process_data(data: Vec<i32>) -> Vec<i32> {
    println!("Processing data...");
    data.iter().map(|x| x * 2).collect()
}

fn save_results(data: Vec<i32>) {
    println!("Saving results: {:?}", data);
}
```

**Execute with monitoring:**
```bash
dotvm run data_pipeline.dotvm --verbose --profile
```

## Troubleshooting

### Common Execution Issues

**Bytecode not found:**
```bash
Error: No such file or directory: program.dotvm
```
**Solution:** Check file path and ensure bytecode file exists.

**Architecture mismatch:**
```bash
Error: Architecture mismatch
Expected: Arch64, Found: Arch256
```
**Solution:** Use correct bytecode for runtime architecture.

**Insufficient memory:**
```bash
Error: Out of memory
Requested: 100MB, Available: 50MB
```
**Solution:** Increase available memory or optimize program.

### Debugging Techniques

1. **Use debug mode** to trace execution
2. **Enable step mode** for interactive debugging
3. **Check verbose output** for detailed information
4. **Validate bytecode** before execution
5. **Monitor resource usage** during execution

### Performance Issues

1. **Profile execution** to identify bottlenecks
2. **Analyze instruction patterns** for optimization opportunities
3. **Monitor memory usage** for memory leaks
4. **Check database operations** for efficiency
5. **Compare different optimization levels**

For more information about advanced features, see the [Advanced Features Guide](advanced-features.md).