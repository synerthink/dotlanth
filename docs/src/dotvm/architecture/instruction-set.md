# Instruction Set Reference

DotVM provides a comprehensive instruction set organized into categories. Each instruction operates on the virtual machine's stack and can access memory, perform computations, and interact with the database.

## Instruction Categories

The DotVM instruction set is organized into the following categories:

- [Stack Operations](#stack-operations) - Stack manipulation
- [Arithmetic Operations](#arithmetic-operations) - Mathematical computations
- [Control Flow](#control-flow) - Program flow control
- [Memory Operations](#memory-operations) - Memory access and management
- [Cryptographic Operations](#cryptographic-operations) - Cryptographic functions
- [Database Operations](#database-operations) - Database interaction

## Stack Operations

Stack operations manipulate the execution stack, which is the primary data structure for DotVM execution.

### PUSH (0x10)
Push a constant value onto the stack.

**Format:** `PUSH <constant_id>`
**Stack Effect:** `[] -> [value]`
**Description:** Pushes the constant identified by `constant_id` onto the stack.

**Example:**
```
PUSH 42    ; Push integer 42 onto stack
PUSH 3.14  ; Push float 3.14 onto stack
```

### POP (0x11)
Remove the top value from the stack.

**Format:** `POP`
**Stack Effect:** `[value] -> []`
**Description:** Removes and discards the top value from the stack.

### DUP (0x12)
Duplicate the top value on the stack.

**Format:** `DUP`
**Stack Effect:** `[value] -> [value, value]`
**Description:** Duplicates the top stack value.

### SWAP (0x13)
Swap the top two values on the stack.

**Format:** `SWAP`
**Stack Effect:** `[a, b] -> [b, a]`
**Description:** Exchanges the positions of the top two stack values.

### PUSH_NULL (0x14)
Push a null value onto the stack.

**Format:** `PUSH_NULL`
**Stack Effect:** `[] -> [null]`

### PUSH_TRUE (0x15)
Push boolean true onto the stack.

**Format:** `PUSH_TRUE`
**Stack Effect:** `[] -> [true]`

### PUSH_FALSE (0x16)
Push boolean false onto the stack.

**Format:** `PUSH_FALSE`
**Stack Effect:** `[] -> [false]`

## Arithmetic Operations

Arithmetic operations perform mathematical computations on stack values.

### ADD (0x01)
Add two values.

**Format:** `ADD`
**Stack Effect:** `[a, b] -> [a + b]`
**Description:** Pops two values, adds them, and pushes the result.

**Example:**
```
PUSH 10
PUSH 5
ADD        ; Stack: [15]
```

### SUB (0x02)
Subtract two values.

**Format:** `SUB`
**Stack Effect:** `[a, b] -> [a - b]`
**Description:** Pops two values, subtracts the top from the second, and pushes the result.

### MUL (0x03)
Multiply two values.

**Format:** `MUL`
**Stack Effect:** `[a, b] -> [a * b]`

### DIV (0x04)
Divide two values.

**Format:** `DIV`
**Stack Effect:** `[a, b] -> [a / b]`
**Description:** Performs division. Throws error on division by zero.

### MOD (0x05)
Modulus operation.

**Format:** `MOD`
**Stack Effect:** `[a, b] -> [a % b]`
**Description:** Returns the remainder of division.

## Control Flow

Control flow instructions manage program execution flow.

### IFELSE (0x10)
Conditional execution.

**Format:** `IFELSE <then_addr> <else_addr>`
**Stack Effect:** `[condition] -> []`
**Description:** Pops a condition. If true, jumps to `then_addr`, otherwise to `else_addr`.

### FORLOOP (0x11)
For loop construct.

**Format:** `FORLOOP <start> <end> <body_addr>`
**Description:** Executes a counted loop from start to end.

### WHILELOOP (0x12)
While loop construct.

**Format:** `WHILELOOP <condition_addr> <body_addr>`
**Description:** Executes loop while condition is true.

### DOWHILELOOP (0x13)
Do-while loop construct.

**Format:** `DOWHILELOOP <body_addr> <condition_addr>`
**Description:** Executes loop body at least once, then while condition is true.

### JUMP (0x14)
Unconditional jump.

**Format:** `JUMP <address>`
**Description:** Jumps to the specified address.

## Memory Operations

Memory operations provide access to the virtual machine's memory space.

### LOAD (0x20)
Load value from memory.

**Format:** `LOAD <address>`
**Stack Effect:** `[] -> [value]`
**Description:** Loads a value from the specified memory address and pushes it onto the stack.

### STORE (0x21)
Store value to memory.

**Format:** `STORE <address>`
**Stack Effect:** `[value] -> []`
**Description:** Pops a value from the stack and stores it at the specified memory address.

### ALLOCATE (0x22)
Allocate memory.

**Format:** `ALLOCATE <size>`
**Stack Effect:** `[] -> [address]`
**Description:** Allocates a block of memory and pushes the address onto the stack.

### DEALLOCATE (0x23)
Deallocate memory.

**Format:** `DEALLOCATE`
**Stack Effect:** `[address] -> []`
**Description:** Deallocates the memory block at the specified address.

### POINTEROPERATION (0x24)
Pointer arithmetic and operations.

**Format:** `POINTEROPERATION <operation>`
**Description:** Performs various pointer operations like dereferencing, arithmetic, etc.

## Cryptographic Operations

Cryptographic operations provide security and hashing functions.

### CRYPTO_HASH (0x40)
Compute cryptographic hash.

**Format:** `CRYPTO_HASH <algorithm>`
**Stack Effect:** `[data] -> [hash]`
**Description:** Computes hash of the data using the specified algorithm (SHA-256, Blake3, etc.).

**Example:**
```
PUSH "hello world"
CRYPTO_HASH SHA256    ; Stack: [hash_value]
```

### CRYPTO_ENCRYPT (0x41)
Encrypt data.

**Format:** `CRYPTO_ENCRYPT <algorithm>`
**Stack Effect:** `[data, key] -> [encrypted_data]`
**Description:** Encrypts data using the specified algorithm and key.

### CRYPTO_DECRYPT (0x42)
Decrypt data.

**Format:** `CRYPTO_DECRYPT <algorithm>`
**Stack Effect:** `[encrypted_data, key] -> [data]`
**Description:** Decrypts data using the specified algorithm and key.

### CRYPTO_SIGN (0x43)
Create digital signature.

**Format:** `CRYPTO_SIGN <algorithm>`
**Stack Effect:** `[data, private_key] -> [signature]`
**Description:** Creates a digital signature for the data.

### CRYPTO_VERIFY_SIGNATURE (0x44)
Verify digital signature.

**Format:** `CRYPTO_VERIFY_SIGNATURE <algorithm>`
**Stack Effect:** `[data, signature, public_key] -> [valid]`
**Description:** Verifies a digital signature, pushes boolean result.

## Database Operations

Database operations provide integration with DotDB for persistent state management.

### DB_GET (0x50)
Retrieve value from database.

**Format:** `DB_GET`
**Stack Effect:** `[collection, key] -> [value]`
**Description:** Retrieves a value from the specified collection and key.

**Example:**
```
PUSH "users"
PUSH "user_123"
DB_GET             ; Stack: [user_data]
```

### DB_PUT (0x51)
Store value in database.

**Format:** `DB_PUT`
**Stack Effect:** `[collection, key, value] -> []`
**Description:** Stores a value in the database with the specified collection and key.

**Example:**
```
PUSH "users"
PUSH "user_123"
PUSH {"name": "Alice", "age": 30}
DB_PUT             ; Stores user data
```

### DB_DELETE (0x52)
Delete value from database.

**Format:** `DB_DELETE`
**Stack Effect:** `[collection, key] -> []`
**Description:** Deletes the value associated with the collection and key.

### DB_QUERY (0x53)
Query database.

**Format:** `DB_QUERY`
**Stack Effect:** `[collection, query] -> [results]`
**Description:** Executes a query against the database and returns results.

### DB_COUNT (0x54)
Count documents in collection.

**Format:** `DB_COUNT`
**Stack Effect:** `[collection] -> [count]`
**Description:** Returns the number of documents in the specified collection.

## Instruction Encoding

Instructions are encoded in bytecode format with the following structure:

```
+--------+--------+--------+--------+
| Opcode | Operand 1 (optional)     |
+--------+--------+--------+--------+
| Operand 2 (optional)             |
+--------+--------+--------+--------+
```

### Opcode Ranges

| Range | Category |
|-------|----------|
| 0x01-0x0F | Arithmetic |
| 0x10-0x1F | Stack Operations |
| 0x20-0x2F | Memory Operations |
| 0x30-0x3F | Control Flow |
| 0x40-0x4F | Cryptographic |
| 0x50-0x5F | Database |
| 0x60-0x6F | SIMD |
| 0x70-0x7F | BigInt |
| 0x80-0x8F | Vector |
| 0x90-0x9F | Parallel |
| 0xA0-0xAF | System Calls |
| 0xB0-0xBF | Architecture-Specific |

## Examples

### Simple Arithmetic
```assembly
PUSH 10        ; Stack: [10]
PUSH 5         ; Stack: [10, 5]
ADD            ; Stack: [15]
PUSH 2         ; Stack: [15, 2]
MUL            ; Stack: [30]
```

### Conditional Execution
```assembly
PUSH 1
PUSH 2
SUB            ; Stack: [-1]
IFELSE label_negative label_positive

label_negative:
    PUSH "Negative"
    SYSCALL_PRINT
    JUMP end

label_positive:
    PUSH "Positive"
    SYSCALL_PRINT

end:
    HALT
```

### Database Operations
```assembly
; Store user data
PUSH "users"
PUSH "alice"
PUSH {"name": "Alice", "age": 30}
DB_PUT

; Retrieve user data
PUSH "users"
PUSH "alice"
DB_GET         ; Stack: [user_data]
```

For more detailed information about specific instruction categories, see the related documentation sections.