# DotDB CLI Reference

The DotDB CLI provides a command-line interface for interacting with the DotDB document database, allowing you to manage collections and documents efficiently.

## Installation

The DotDB CLI is built as part of the workspace. To build and install:

```bash
cargo build --release --bin dotdb
# The binary will be available at target/release/dotdb
```

## Commands Overview

The DotDB CLI provides comprehensive document and collection management:

- `put`: Insert a JSON document into a collection
- `get`: Retrieve a document by ID
- `update`: Update an existing document
- `delete`: Remove a document by ID
- `list`: List all document IDs in a collection
- `collections`: List all collections
- `create-collection`: Create a new collection
- `delete-collection`: Remove a collection and all its documents
- `count`: Count documents in a collection
- `find`: Find documents by field value

## Global Options

```
dotdb [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Document Operations

### Put Command

Insert a JSON document into a collection.

**Usage:**
```bash
dotdb put <COLLECTION> <JSON>
```

**Arguments:**
- `<COLLECTION>`: Collection name
- `<JSON>`: JSON document content

**Examples:**
```bash
# Insert a simple document
dotdb put users '{"name": "Alice", "age": 30, "email": "alice@example.com"}'

# Insert a complex document
dotdb put products '{
  "name": "Laptop",
  "price": 999.99,
  "specs": {
    "cpu": "Intel i7",
    "ram": "16GB",
    "storage": "512GB SSD"
  },
  "tags": ["electronics", "computers"]
}'
```

### Get Command

Retrieve a document by its ID.

**Usage:**
```bash
dotdb get <COLLECTION> <ID>
```

**Arguments:**
- `<COLLECTION>`: Collection name
- `<ID>`: Document ID

**Examples:**
```bash
# Get a document by ID
dotdb get users user_123

# Get a product document
dotdb get products prod_456
```

### Update Command

Update an existing document by ID.

**Usage:**
```bash
dotdb update <COLLECTION> <ID> <JSON>
```

**Arguments:**
- `<COLLECTION>`: Collection name
- `<ID>`: Document ID
- `<JSON>`: New JSON document content

**Examples:**
```bash
# Update user information
dotdb update users user_123 '{"name": "Alice Smith", "age": 31, "email": "alice.smith@example.com"}'

# Update product price
dotdb update products prod_456 '{
  "name": "Laptop",
  "price": 899.99,
  "specs": {
    "cpu": "Intel i7",
    "ram": "16GB", 
    "storage": "512GB SSD"
  }
}'
```

### Delete Command

Remove a document by ID.

**Usage:**
```bash
dotdb delete <COLLECTION> <ID>
```

**Arguments:**
- `<COLLECTION>`: Collection name
- `<ID>`: Document ID

**Examples:**
```bash
# Delete a user
dotdb delete users user_123

# Delete a product
dotdb delete products prod_456
```

### List Command

List all document IDs in a collection.

**Usage:**
```bash
dotdb list <COLLECTION>
```

**Arguments:**
- `<COLLECTION>`: Collection name

**Examples:**
```bash
# List all user IDs
dotdb list users

# List all product IDs
dotdb list products
```

### Find Command

Find documents by field value.

**Usage:**
```bash
dotdb find <COLLECTION> <FIELD> <VALUE>
```

**Arguments:**
- `<COLLECTION>`: Collection name
- `<FIELD>`: Field name to search
- `<VALUE>`: Field value (JSON format)

**Examples:**
```bash
# Find users by age
dotdb find users age 30

# Find products by name
dotdb find products name '"Laptop"'

# Find users by email
dotdb find users email '"alice@example.com"'

# Find products by price range (requires JSON value)
dotdb find products price 999.99
```

## Collection Management

### Collections Command

List all collections in the database.

**Usage:**
```bash
dotdb collections
```

**Example:**
```bash
dotdb collections
# Output:
# Available collections:
# - users
# - products
# - orders
```

### Create Collection Command

Create a new collection.

**Usage:**
```bash
dotdb create-collection <COLLECTION>
```

**Arguments:**
- `<COLLECTION>`: Collection name

**Examples:**
```bash
# Create a users collection
dotdb create-collection users

# Create a products collection
dotdb create-collection products
```

### Delete Collection Command

Delete a collection and all its documents.

**Usage:**
```bash
dotdb delete-collection <COLLECTION>
```

**Arguments:**
- `<COLLECTION>`: Collection name

**Examples:**
```bash
# Delete users collection
dotdb delete-collection users

# Delete products collection
dotdb delete-collection products
```

**Warning:** This operation is irreversible and will delete all documents in the collection.

### Count Command

Count the number of documents in a collection.

**Usage:**
```bash
dotdb count <COLLECTION>
```

**Arguments:**
- `<COLLECTION>`: Collection name

**Examples:**
```bash
# Count users
dotdb count users
# Output: Collection 'users' contains 150 documents

# Count products
dotdb count products
# Output: Collection 'products' contains 45 documents
```

## JSON Document Format

DotDB stores documents in JSON format. Documents can contain:

- **Primitive types**: strings, numbers, booleans, null
- **Objects**: nested JSON objects
- **Arrays**: lists of values
- **Mixed types**: combinations of the above

### Valid JSON Examples

```json
{
  "id": "user_123",
  "name": "Alice",
  "age": 30,
  "active": true,
  "address": {
    "street": "123 Main St",
    "city": "Anytown",
    "zipcode": "12345"
  },
  "hobbies": ["reading", "hiking", "coding"],
  "metadata": null
}
```

## Error Handling

The CLI provides clear error messages for common issues:

### Document Errors
- **Invalid JSON**: Check JSON syntax and formatting
- **Document not found**: Verify collection name and document ID
- **Collection not found**: Ensure collection exists

### Collection Errors
- **Collection already exists**: Use existing collection or choose different name
- **Collection not empty**: Cannot delete non-empty collection without force

### System Errors
- **Permission denied**: Check file system permissions
- **Disk space**: Ensure sufficient storage space
- **Memory errors**: Check available system memory

## Performance Tips

1. **Batch operations**: Group multiple operations when possible
2. **Use appropriate field names**: Short, descriptive field names improve performance
3. **Index frequently queried fields**: Consider field access patterns
4. **Optimize JSON structure**: Avoid deeply nested objects when possible
5. **Regular maintenance**: Periodically clean up unused documents

## Integration with DotVM

DotDB seamlessly integrates with DotVM for state management:

- **State persistence**: VM state is automatically persisted
- **Transaction support**: Atomic operations across VM and database
- **Query capabilities**: Complex queries available through VM opcodes
- **Performance optimization**: Optimized for VM access patterns

## Workflow Examples

### User Management System

1. **Create users collection:**
```bash
dotdb create-collection users
```

2. **Add users:**
```bash
dotdb put users '{"name": "Alice", "email": "alice@example.com", "role": "admin"}'
dotdb put users '{"name": "Bob", "email": "bob@example.com", "role": "user"}'
dotdb put users '{"name": "Charlie", "email": "charlie@example.com", "role": "user"}'
```

3. **List all users:**
```bash
dotdb list users
```

4. **Find admin users:**
```bash
dotdb find users role '"admin"'
```

5. **Update user role:**
```bash
dotdb update users user_456 '{"name": "Bob", "email": "bob@example.com", "role": "admin"}'
```

6. **Count total users:**
```bash
dotdb count users
```

### Product Catalog

1. **Create products collection:**
```bash
dotdb create-collection products
```

2. **Add products:**
```bash
dotdb put products '{
  "name": "Laptop Pro",
  "category": "electronics",
  "price": 1299.99,
  "stock": 50,
  "specs": {
    "cpu": "M2 Pro",
    "ram": "32GB",
    "storage": "1TB SSD"
  }
}'

dotdb put products '{
  "name": "Wireless Mouse",
  "category": "accessories", 
  "price": 29.99,
  "stock": 200
}'
```

3. **Find products by category:**
```bash
dotdb find products category '"electronics"'
```

4. **Update stock levels:**
```bash
dotdb update products prod_123 '{
  "name": "Laptop Pro",
  "category": "electronics",
  "price": 1299.99,
  "stock": 45
}'
```

5. **Check inventory:**
```bash
dotdb count products
```

## Advanced Usage

### Complex Queries

While the CLI provides basic find functionality, complex queries can be performed through the DotVM integration:

```rust
// Example: Find products with price > 100 and stock > 10
// This would be implemented in DotVM bytecode
```

### Bulk Operations

For bulk operations, consider writing DotVM programs that use the database opcodes for better performance than individual CLI commands.

### Backup and Restore

```bash
# Export all collections (conceptual - would need implementation)
dotdb export --output backup.json

# Import collections (conceptual - would need implementation)  
dotdb import --input backup.json
```