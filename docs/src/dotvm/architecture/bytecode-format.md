# Bytecode Format

DotVM bytecode files use a structured binary format that contains the compiled program along with metadata, constants, and optional debug information.

## File Structure Overview

```
+-------------------------------------------------------------+
|                    File Header                              |
+-------------------------------------------------------------+
|                    Code Section                            |
+-------------------------------------------------------------+
|                    Data Section                            |
+-------------------------------------------------------------+
|                   Debug Section                            |
|                   (optional)                               |
+-------------------------------------------------------------+
```

## File Header

The file header contains essential metadata about the bytecode file:

```
+--------+--------+--------+--------+--------+--------+--------+--------+
| Magic Number (4 bytes)           | Version (2 bytes) | Arch | Flags  |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Entry Point (8 bytes)                                                |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Code Section Offset (8 bytes)                                        |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Code Section Size (8 bytes)                                          |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Data Section Offset (8 bytes)                                        |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Data Section Size (8 bytes)                                          |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Debug Section Offset (8 bytes)                                       |
+--------+--------+--------+--------+--------+--------+--------+--------+
| Debug Section Size (8 bytes)                                         |
+--------+--------+--------+--------+--------+--------+--------+--------+
```

### Header Fields

#### Magic Number (4 bytes)
- **Value**: `0x444F5456` ("DOTV" in ASCII)
- **Purpose**: File format identification
- **Validation**: Must match exactly for valid bytecode

#### Version (2 bytes)
- **Format**: Major.Minor (1 byte each)
- **Current**: 0x0001 (version 0.1)
- **Purpose**: Bytecode format version compatibility

#### Architecture (1 byte)
- **Arch32**: 0x00
- **Arch64**: 0x01 (default)
- **Arch128**: 0x02
- **Arch256**: 0x03
- **Arch512**: 0x04

#### Flags (1 byte)
Bitfield containing various flags:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | DEBUG | Debug information present |
| 1 | COMPRESSED | Code section is compressed |
| 2 | ENCRYPTED | Bytecode is encrypted |
| 3 | SIGNED | Digital signature present |
| 4-7 | RESERVED | Reserved for future use |

#### Entry Point (8 bytes)
- **Purpose**: Starting address for program execution
- **Format**: 64-bit unsigned integer
- **Default**: 0x0000000000000000 (start of code section)

#### Section Offsets and Sizes (8 bytes each)
- **Offset**: Byte offset from start of file
- **Size**: Size of section in bytes
- **Purpose**: Allows random access to sections

## Code Section

The code section contains the actual bytecode instructions:

```
+--------+--------+--------+--------+
| Instruction Count (4 bytes)      |
+--------+--------+--------+--------+
| Instruction 1                     |
+--------+--------+--------+--------+
| Instruction 2                     |
+--------+--------+--------+--------+
| ...                               |
+--------+--------+--------+--------+
| Instruction N                     |
+--------+--------+--------+--------+
```

### Instruction Format

Each instruction has a variable-length encoding:

```
+--------+--------+--------+--------+
| Opcode | Operand Count | Operands... |
+--------+--------+--------+--------+
```

#### Opcode (1 byte)
- **Range**: 0x01-0xFF
- **Purpose**: Identifies the instruction type
- **Categories**: See [Instruction Set](instruction-set.md) for details

#### Operand Count (1 byte)
- **Range**: 0-255
- **Purpose**: Number of operands following the opcode
- **Note**: Most instructions have 0-2 operands

#### Operands (variable length)
- **Format**: Depends on instruction type
- **Types**: Immediate values, addresses, constant references

### Operand Types

#### Immediate Values
- **8-bit**: 1 byte
- **16-bit**: 2 bytes (little-endian)
- **32-bit**: 4 bytes (little-endian)
- **64-bit**: 8 bytes (little-endian)

#### Addresses
- **Format**: Architecture-dependent
- **Arch32**: 4 bytes
- **Arch64**: 8 bytes
- **Arch128+**: 8 bytes (logical addressing)

#### Constant References
- **Format**: 4-byte index into data section
- **Purpose**: Reference to constants in data section

## Data Section

The data section contains constants and static data:

```
+--------+--------+--------+--------+
| Constant Count (4 bytes)         |
+--------+--------+--------+--------+
| Constant Table                    |
+--------+--------+--------+--------+
| String Pool                       |
+--------+--------+--------+--------+
| Binary Data                       |
+--------+--------+--------+--------+
```

### Constant Table

The constant table provides metadata for each constant:

```
+--------+--------+--------+--------+
| Type   | Size (4 bytes)           |
+--------+--------+--------+--------+
| Offset (4 bytes)                  |
+--------+--------+--------+--------+
```

#### Constant Types

| Type | Value | Description |
|------|-------|-------------|
| NULL | 0x00 | Null value |
| BOOL | 0x01 | Boolean value |
| INT8 | 0x02 | 8-bit signed integer |
| INT16 | 0x03 | 16-bit signed integer |
| INT32 | 0x04 | 32-bit signed integer |
| INT64 | 0x05 | 64-bit signed integer |
| UINT8 | 0x06 | 8-bit unsigned integer |
| UINT16 | 0x07 | 16-bit unsigned integer |
| UINT32 | 0x08 | 32-bit unsigned integer |
| UINT64 | 0x09 | 64-bit unsigned integer |
| FLOAT32 | 0x0A | 32-bit IEEE 754 float |
| FLOAT64 | 0x0B | 64-bit IEEE 754 float |
| STRING | 0x0C | UTF-8 string |
| BYTES | 0x0D | Binary data |
| BIGINT | 0x0E | Arbitrary precision integer |

### String Pool

Strings are stored in a dedicated pool with length prefixes:

```
+--------+--------+--------+--------+
| Length (4 bytes)                  |
+--------+--------+--------+--------+
| UTF-8 String Data...              |
+--------+--------+--------+--------+
```

### Binary Data

Binary constants are stored with length prefixes:

```
+--------+--------+--------+--------+
| Length (4 bytes)                  |
+--------+--------+--------+--------+
| Binary Data...                    |
+--------+--------+--------+--------+
```

## Debug Section (Optional)

The debug section contains information for debugging and profiling:

```
+--------+--------+--------+--------+
| Debug Info Version (2 bytes)     |
+--------+--------+--------+--------+
| Source Map Table                  |
+--------+--------+--------+--------+
| Symbol Table                      |
+--------+--------+--------+--------+
| Line Number Table                 |
+--------+--------+--------+--------+
```

### Source Map Table

Maps bytecode addresses to source code locations:

```
+--------+--------+--------+--------+
| Entry Count (4 bytes)            |
+--------+--------+--------+--------+
| Bytecode Address (8 bytes)        |
+--------+--------+--------+--------+
| Source File ID (4 bytes)         |
+--------+--------+--------+--------+
| Line Number (4 bytes)            |
+--------+--------+--------+--------+
| Column Number (4 bytes)          |
+--------+--------+--------+--------+
```

### Symbol Table

Contains function and variable names:

```
+--------+--------+--------+--------+
| Symbol Count (4 bytes)           |
+--------+--------+--------+--------+
| Symbol Type | Name Length        |
+--------+--------+--------+--------+
| Name (UTF-8)...                   |
+--------+--------+--------+--------+
| Address (8 bytes)                 |
+--------+--------+--------+--------+
```

## File Format Validation

### Header Validation
1. **Magic Number**: Must be `0x444F5456`
2. **Version**: Must be supported by runtime
3. **Architecture**: Must match runtime architecture
4. **Section Offsets**: Must be within file bounds
5. **Section Sizes**: Must not exceed file size

### Code Section Validation
1. **Instruction Count**: Must match actual instructions
2. **Opcodes**: Must be valid instruction opcodes
3. **Operands**: Must match instruction requirements
4. **Addresses**: Must be within valid ranges

### Data Section Validation
1. **Constant Count**: Must match constant table entries
2. **Constant Types**: Must be valid type identifiers
3. **String Data**: Must be valid UTF-8
4. **Offsets**: Must be within section bounds

## Example Bytecode File

Here's a simple "Hello, World!" program in bytecode format:

### Source Code
```rust
fn main() {
    println!("Hello, World!");
}
```

### Bytecode Hexdump
```
00000000: 444f 5456 0001 0100 0000 0000 0000 0000  DOTV............
00000010: 0000 0000 0000 0040 0000 0000 0000 0020  .......@.......
00000020: 0000 0000 0000 0060 0000 0000 0000 0030  .......`......0
00000030: 0000 0000 0000 0000 0000 0000 0000 0000  ................
00000040: 0000 0004 1001 0000 a001 1100            ............

; Header breakdown:
; 444f5456 - Magic "DOTV"
; 0001 - Version 0.1
; 01 - Arch64
; 00 - No flags
; 0000000000000000 - Entry point 0
; 0000000000000040 - Code section at offset 0x40
; 0000000000000020 - Code section size 32 bytes
; 0000000000000060 - Data section at offset 0x60
; 0000000000000030 - Data section size 48 bytes

; Code section:
; 00000004 - 4 instructions
; 10 01 0000 - PUSH constant 0
; a0 01 - SYSCALL_PRINT
; 11 00 - POP
; 00 - HALT
```

## Performance Considerations

### File Size Optimization
1. **Constant Deduplication**: Reuse identical constants
2. **String Interning**: Share common strings
3. **Instruction Packing**: Use minimal operand sizes
4. **Compression**: Enable compression flag for large files

### Loading Performance
1. **Memory Mapping**: Use memory-mapped files for large bytecode
2. **Lazy Loading**: Load sections on demand
3. **Caching**: Cache parsed bytecode in memory
4. **Validation**: Minimize validation overhead

### Security Considerations
1. **Signature Verification**: Verify digital signatures
2. **Bounds Checking**: Validate all offsets and sizes
3. **Instruction Validation**: Verify instruction sequences
4. **Resource Limits**: Enforce memory and execution limits

## Tools and Utilities

### Bytecode Inspector
```bash
dotvm inspect program.dotvm
```

### Bytecode Disassembler
```bash
dotvm disasm program.dotvm > program.asm
```

### Bytecode Validator
```bash
dotvm validate program.dotvm
```

### Bytecode Optimizer
```bash
dotvm optimize input.dotvm output.dotvm
```

## Version History

### Version 0.1 (Current)
- Initial bytecode format
- Basic instruction set support
- Debug information support
- Architecture-specific encoding

### Future Versions
- **0.2**: Compression support
- **0.3**: Encryption and signing
- **0.4**: Extended instruction set
- **1.0**: Stable format specification

For more information about creating and working with bytecode, see the [Transpilation Guide](../usage/transpilation.md).