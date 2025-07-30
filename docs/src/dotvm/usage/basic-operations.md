# Basic Operations

This guide covers the fundamental operations you can perform with DotVM, from basic program execution to common development workflows.

## Getting Started

Before diving into basic operations, ensure you have DotVM installed and configured. See the [Installation Guide](../../getting-started/installation.md) for setup instructions.

### Verify Installation

```bash
# Check DotVM version
dotvm --version

# Check available commands
dotvm --help
```

## Basic Workflow

The typical DotVM workflow involves three main steps:

1. **Write Rust Code**: Create your application in Rust
2. **Transpile to Bytecode**: Convert Rust to DotVM bytecode
3. **Execute Bytecode**: Run the bytecode on DotVM

### Simple Example

**Step 1: Create a Rust program**
```rust
// hello.rs
fn main() {
    println!("Hello, DotVM!");
    let x = 10;
    let y = 5;
    let result = x + y;
    println!("Result: {}", result);
}
```

**Step 2: Transpile to bytecode**
```bash
dotvm transpile -i hello.rs -o hello.dotvm
```

**Step 3: Execute the bytecode**
```bash
dotvm run hello.dotvm
```

**Expected Output:**
```
Hello, DotVM!
Result: 15
Execution completed!
Instructions executed: 42
Execution time: 1.2ms
Total time: 2.1ms
```

## Common Operations

### Working with Different Architectures

DotVM supports multiple architectures. Choose based on your needs:

**64-bit (Default):**
```bash
dotvm transpile -i program.rs -o program.dotvm -a arch64
```

**128-bit (Scientific Computing):**
```bash
dotvm transpile -i scientific.rs -o scientific.dotvm -a arch128
```

### Optimization Levels

Control compilation optimization:

**Development (Fast compilation):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 0
```

**Production (Balanced):**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 2
```

**Maximum Performance:**
```bash
dotvm transpile -i program.rs -o program.dotvm --opt-level 3
```

### Debug Mode

Enable debugging for development:

**Compile with debug info:**
```bash
dotvm transpile -i program.rs -o program.dotvm --debug
```

**Run with debugging:**
```bash
dotvm run program.dotvm --debug
```

**Step-by-step execution:**
```bash
dotvm run program.dotvm --step
```

## Working with Functions

### Function Calls

```rust
// functions.rs
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn main() {
    let x = 10;
    let y = 5;
    
    let sum = add(x, y);
    let product = multiply(x, y);
    
    println!("Sum: {}", sum);
    println!("Product: {}", product);
}
```

### Recursive Functions

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
    for i in 0..10 {
        println!("fibonacci({}) = {}", i, fibonacci(i));
    }
}
```

## Control Flow

### Conditional Statements

```rust
// conditions.rs
fn main() {
    let number = 42;
    
    if number > 0 {
        println!("Positive number");
    } else if number < 0 {
        println!("Negative number");
    } else {
        println!("Zero");
    }
    
    // Match expressions
    match number {
        0 => println!("Zero"),
        1..=10 => println!("Small positive"),
        11..=100 => println!("Medium positive"),
        _ => println!("Large positive"),
    }
}
```

### Loops

```rust
// loops.rs
fn main() {
    // For loop
    for i in 1..=5 {
        println!("Count: {}", i);
    }
    
    // While loop
    let mut counter = 0;
    while counter < 3 {
        println!("Counter: {}", counter);
        counter += 1;
    }
    
    // Loop with break
    let mut value = 0;
    loop {
        value += 1;
        if value > 5 {
            break;
        }
        println!("Value: {}", value);
    }
}
```

## Data Types and Structures

### Basic Data Types

```rust
// data_types.rs
fn main() {
    // Integers
    let small: i32 = 42;
    let large: i64 = 1_000_000;
    
    // Floating point
    let pi: f64 = 3.14159;
    
    // Boolean
    let is_true: bool = true;
    
    // String
    let message: String = "Hello, DotVM!".to_string();
    let slice: &str = "String slice";
    
    println!("Integer: {}", small);
    println!("Float: {}", pi);
    println!("Boolean: {}", is_true);
    println!("String: {}", message);
}
```

### Collections

```rust
// collections.rs
fn main() {
    // Vector
    let mut numbers = vec![1, 2, 3, 4, 5];
    numbers.push(6);
    
    for num in &numbers {
        println!("Number: {}", num);
    }
    
    // Array
    let array: [i32; 5] = [1, 2, 3, 4, 5];
    println!("Array length: {}", array.len());
    
    // Tuple
    let tuple: (i32, f64, &str) = (42, 3.14, "hello");
    println!("Tuple: {:?}", tuple);
}
```

### Structs and Enums

```rust
// structs_enums.rs
#[derive(Debug)]
struct Person {
    name: String,
    age: u32,
}

#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
    RGB(u8, u8, u8),
}

fn main() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    
    let color = Color::RGB(255, 0, 0);
    
    println!("Person: {:?}", person);
    println!("Color: {:?}", color);
}
```

## Error Handling

### Result Type

```rust
// error_handling.rs
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}

fn main() {
    match divide(10.0, 2.0) {
        Ok(result) => println!("Result: {}", result),
        Err(error) => println!("Error: {}", error),
    }
    
    match divide(10.0, 0.0) {
        Ok(result) => println!("Result: {}", result),
        Err(error) => println!("Error: {}", error),
    }
}
```

## Performance Monitoring

### Timing Execution

```bash
# Basic timing
time dotvm run program.dotvm

# Detailed performance info
dotvm run program.dotvm --verbose
```

### Comparing Optimizations

```bash
# No optimization
dotvm transpile -i program.rs -o program_o0.dotvm --opt-level 0
time dotvm run program_o0.dotvm

# Maximum optimization
dotvm transpile -i program.rs -o program_o3.dotvm --opt-level 3
time dotvm run program_o3.dotvm
```

## File I/O Operations

### Reading and Writing Files

```rust
// file_io.rs
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Write to file
    let content = "Hello, DotVM file I/O!";
    fs::write("output.txt", content)?;
    
    // Read from file
    let read_content = fs::read_to_string("output.txt")?;
    println!("File content: {}", read_content);
    
    Ok(())
}
```

## Memory Management

### Understanding Stack Usage

```rust
// memory.rs
fn recursive_function(depth: u32) {
    if depth > 0 {
        println!("Depth: {}", depth);
        recursive_function(depth - 1);
    }
}

fn main() {
    // Monitor stack usage with debug mode
    recursive_function(10);
}
```

**Run with memory monitoring:**
```bash
dotvm run memory.dotvm --debug --verbose
```

## Best Practices

### Code Organization

1. **Keep functions focused**: Write small, single-purpose functions
2. **Use meaningful names**: Choose descriptive variable and function names
3. **Handle errors properly**: Use Result types for error handling
4. **Comment complex logic**: Add comments for non-obvious code

### Performance Tips

1. **Choose appropriate architecture**: Match architecture to your data needs
2. **Use optimization levels**: Enable optimizations for production code
3. **Profile your code**: Use verbose mode to identify bottlenecks
4. **Minimize allocations**: Reuse data structures when possible

### Debugging Workflow

1. **Start with debug mode**: Use `--debug` flag during development
2. **Use step mode**: Step through code with `--step` for detailed debugging
3. **Check verbose output**: Use `--verbose` for detailed execution information
4. **Validate bytecode**: Ensure bytecode is generated correctly

## Common Patterns

### Configuration Pattern

```rust
// config.rs
struct Config {
    debug: bool,
    max_iterations: u32,
    output_file: String,
}

impl Config {
    fn new() -> Self {
        Config {
            debug: false,
            max_iterations: 1000,
            output_file: "output.txt".to_string(),
        }
    }
}

fn main() {
    let config = Config::new();
    println!("Debug mode: {}", config.debug);
    println!("Max iterations: {}", config.max_iterations);
}
```

### Iterator Pattern

```rust
// iterators.rs
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];
    
    // Map and collect
    let doubled: Vec<i32> = numbers.iter().map(|x| x * 2).collect();
    println!("Doubled: {:?}", doubled);
    
    // Filter and sum
    let sum: i32 = numbers.iter().filter(|&&x| x > 2).sum();
    println!("Sum of numbers > 2: {}", sum);
}
```

## Next Steps

After mastering basic operations, explore:

1. **[Transpilation Guide](transpilation.md)**: Advanced transpilation techniques
2. **[Execution Guide](execution.md)**: Advanced execution and debugging
3. **[Advanced Features](advanced-features.md)**: Complex DotVM capabilities
4. **[DotDB Integration](../../dotdb/usage/basic-operations.md)**: Database operations

## Troubleshooting

### Common Issues

**Compilation Errors:**
- Check Rust syntax and ensure code compiles with `rustc`
- Verify all dependencies are available

**Execution Errors:**
- Use `--debug` mode to see detailed execution information
- Check for stack overflow with recursive functions
- Verify bytecode file exists and is not corrupted

**Performance Issues:**
- Try different optimization levels
- Use `--verbose` to identify bottlenecks
- Consider different VM architectures for your use case
