# Quickstart Guide

This guide will get you up and running with Dotlanth in just a few minutes. You'll learn how to transpile Rust code to DotVM bytecode and execute it, as well as how to use the DotDB document database.

## Prerequisites

Make sure you have completed the [installation](installation.md) process and have both `dotvm` and `dotdb` CLI tools available.

## Your First DotVM Program

Let's create a simple "Hello, World!" program and run it through the complete DotVM pipeline.

### Step 1: Create a Rust Program

Create a new file called `hello.rs`:

```rust
fn main() {
    println!("Hello, DotVM!");
    let result = 40 + 2;
    println!("The answer is: {}", result);
}
```

### Step 2: Transpile to DotVM Bytecode

Use the DotVM CLI to transpile your Rust code to bytecode:

```bash
dotvm transpile -i hello.rs -o hello.dotvm --verbose
```

You should see output similar to:
```
Loading Rust source: hello.rs
Compiling Rust to WebAssembly...
Parsing WebAssembly module...
Translating to DotVM bytecode...
Optimizing for arch64 architecture...
Writing bytecode to: hello.dotvm
Transpilation completed successfully!
```

### Step 3: Execute the Bytecode

Run your DotVM bytecode:

```bash
dotvm run hello.dotvm --verbose
```

Expected output:
```
Loading bytecode from: hello.dotvm
Bytecode loaded in 1.2ms
Starting execution...
Hello, DotVM!
The answer is: 42
Execution completed!
Instructions executed: 156
Execution time: 0.8ms
Total time: 2.0ms
```

Congratulations! You've successfully created, transpiled, and executed your first DotVM program.

## Your First DotDB Operations

Now let's explore the document database capabilities.

### Step 1: Create a Collection

Create a collection to store user data:

```bash
dotdb create-collection users
```

### Step 2: Insert Documents

Add some user documents:

```bash
# Insert first user
dotdb put users '{
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 30,
  "role": "developer"
}'

# Insert second user
dotdb put users '{
  "name": "Bob Smith", 
  "email": "bob@example.com",
  "age": 25,
  "role": "designer"
}'

# Insert third user
dotdb put users '{
  "name": "Charlie Brown",
  "email": "charlie@example.com", 
  "age": 35,
  "role": "manager"
}'
```

### Step 3: Query the Database

List all collections:
```bash
dotdb collections
```

List all user IDs:
```bash
dotdb list users
```

Count users:
```bash
dotdb count users
```

Find users by role:
```bash
dotdb find users role '"developer"'
```

### Step 4: Retrieve and Update Documents

Get a specific user (use an ID from the list command):
```bash
dotdb get users <user_id>
```

Update a user's information:
```bash
dotdb update users <user_id> '{
  "name": "Alice Johnson",
  "email": "alice.johnson@example.com",
  "age": 31,
  "role": "senior_developer"
}'
```

## Advanced Example: Calculator Program

Let's create a more complex program that demonstrates DotVM's capabilities.

### Step 1: Create the Calculator

Create `calculator.rs`:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn factorial(n: i32) -> i32 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn main() {
    println!("DotVM Calculator Demo");
    
    let x = 10;
    let y = 5;
    
    println!("{} + {} = {}", x, y, add(x, y));
    println!("{} * {} = {}", x, y, multiply(x, y));
    println!("{}! = {}", y, factorial(y));
    
    // Demonstrate loops
    println!("Counting to 5:");
    for i in 1..=5 {
        println!("  {}", i);
    }
}
```

### Step 2: Transpile with Different Architectures

Try different VM architectures:

```bash
# 64-bit architecture (default)
dotvm transpile -i calculator.rs -o calculator_64.dotvm -a arch64

# 256-bit architecture
dotvm transpile -i calculator.rs -o calculator_256.dotvm -a arch256

# With debug information
dotvm transpile -i calculator.rs -o calculator_debug.dotvm --debug --verbose
```

### Step 3: Execute with Debug Mode

Run with debug information to see instruction execution:

```bash
dotvm run calculator_debug.dotvm --debug
```

## Working with Both Systems

DotVM and DotDB are designed to work together. Here's an example of how they might be used in a real application:

### Step 1: Create Application Data

Set up collections for an application:

```bash
# Create collections
dotdb create-collection config
dotdb create-collection logs
dotdb create-collection results

# Add configuration
dotdb put config '{
  "app_name": "DotVM Calculator",
  "version": "1.0.0",
  "max_calculations": 1000
}'

# Add some initial data
dotdb put results '{
  "operation": "factorial",
  "input": 5,
  "result": 120,
  "timestamp": "2025-01-01T12:00:00Z"
}'
```

### Step 2: Create a Data-Aware Program

Create `data_app.rs`:

```rust
fn main() {
    println!("Data-aware DotVM application");
    
    // In a real application, this would use DotDB opcodes
    // to read configuration and store results
    
    let calculations = vec![
        ("add", 10, 5, 15),
        ("multiply", 10, 5, 50),
        ("subtract", 10, 5, 5),
    ];
    
    println!("Performing calculations:");
    for (op, a, b, result) in calculations {
        println!("{} {} {} = {}", a, op, b, result);
    }
    
    println!("Results would be stored in DotDB");
}
```

### Step 3: Execute and Verify

```bash
# Transpile and run
dotvm transpile -i data_app.rs -o data_app.dotvm
dotvm run data_app.dotvm

# Check database state
dotdb count results
dotdb list results
```

## Performance Testing

Test the performance characteristics of your programs:

### Step 1: Create a Performance Test

Create `performance_test.rs`:

```rust
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    println!("Performance test starting...");
    
    let test_values = [10, 15, 20, 25];
    
    for &n in &test_values {
        let result = fibonacci(n);
        println!("fibonacci({}) = {}", n, result);
    }
    
    println!("Performance test completed!");
}
```

### Step 2: Run Performance Tests

```bash
# Transpile with maximum optimization
dotvm transpile -i performance_test.rs -o perf_test.dotvm --opt-level 3

# Run with timing information
dotvm run perf_test.dotvm --verbose

# Compare different architectures
dotvm transpile -i performance_test.rs -o perf_test_256.dotvm -a arch256 --opt-level 3
dotvm run perf_test_256.dotvm --verbose
```

## Next Steps

Now that you've completed the quickstart guide, you can:

1. **Explore CLI Features**: Learn more about [DotVM CLI](../cli/dotvm.md) and [DotDB CLI](../cli/dotdb.md)
2. **Understand Architecture**: Read about [VM architectures](../dotvm/architecture/vm-architectures.md) and [instruction sets](../dotvm/architecture/instruction-set.md)
3. **Advanced Usage**: Check out [advanced DotVM features](../dotvm/usage/advanced-features.md)
4. **Development Setup**: Set up a [development environment](development-setup.md) for contributing
5. **API Reference**: Explore the [Core API](../dotvm/api/core.md) documentation

## Common Issues

### Transpilation Fails
- Ensure your Rust code compiles with `rustc`
- Check that all dependencies are available
- Try with `--verbose` flag for more information

### Execution Errors
- Verify the bytecode file exists and is not corrupted
- Check architecture compatibility
- Use `--debug` mode to see detailed execution information

### Database Operations Fail
- Ensure collections exist before inserting documents
- Verify JSON syntax in document content
- Check available disk space and permissions

### Performance Issues
- Try different optimization levels (`--opt-level 0-3`)
- Test different VM architectures
- Use `--verbose` to identify bottlenecks

For more detailed troubleshooting, see the [troubleshooting guide](../guides/troubleshooting.md).
