# Document Management

This guide covers comprehensive document management in DotDB, including CRUD operations, querying, indexing strategies, and performance optimization.

## Document Basics

### Document Structure

DotDB documents are JSON objects with automatic system fields:

```json
{
  "_id": "user_123",
  "_version": 1,
  "_created_at": "2025-01-01T00:00:00Z",
  "_updated_at": "2025-01-01T00:00:00Z",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 30,
  "preferences": {
    "theme": "dark",
    "notifications": true
  },
  "tags": ["developer", "admin"]
}
```

### System Fields

DotDB automatically manages system fields:

| Field | Type | Description | Editable |
|-------|------|-------------|----------|
| `_id` | String | Unique document identifier | No |
| `_version` | Integer | Document version number | No |
| `_created_at` | Timestamp | Creation timestamp | No |
| `_updated_at` | Timestamp | Last modification timestamp | No |
| `_size` | Integer | Document size in bytes | No |

## CRUD Operations

### Creating Documents

#### Using CLI

**Insert with auto-generated ID:**
```bash
dotdb put users '{
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 30,
  "role": "developer"
}'
```

**Response:**
```
Document inserted successfully
ID: user_507f1f77bcf86cd799439011
Collection: users
```

#### Batch Insert

Insert multiple documents efficiently:

```bash
# Create a JSON file with multiple documents
cat > users_batch.json << EOF
[
  {"name": "Alice", "email": "alice@example.com", "age": 30},
  {"name": "Bob", "email": "bob@example.com", "age": 25},
  {"name": "Charlie", "email": "charlie@example.com", "age": 35}
]
EOF

# Batch insert (conceptual - would need implementation)
dotdb batch-put users users_batch.json
```

### Reading Documents

#### Get by ID

```bash
dotdb get users user_507f1f77bcf86cd799439011
```

**Response:**
```json
{
  "_id": "user_507f1f77bcf86cd799439011",
  "_version": 1,
  "_created_at": "2025-01-01T00:00:00Z",
  "_updated_at": "2025-01-01T00:00:00Z",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 30,
  "role": "developer"
}
```

#### List All Documents

```bash
dotdb list users
```

**Response:**
```
Documents in collection 'users':
- user_507f1f77bcf86cd799439011
- user_507f1f77bcf86cd799439012
- user_507f1f77bcf86cd799439013
Total: 3 documents
```

#### Find Documents by Field

```bash
# Find by exact match
dotdb find users role '"developer"'

# Find by age
dotdb find users age 30

# Find by nested field (conceptual)
dotdb find users preferences.theme '"dark"'
```

### Updating Documents

#### Full Document Update

```bash
dotdb update users user_507f1f77bcf86cd799439011 '{
  "name": "Alice Smith",
  "email": "alice.smith@example.com",
  "age": 31,
  "role": "senior_developer",
  "department": "engineering"
}'
```

**Response:**
```
Document updated successfully
ID: user_507f1f77bcf86cd799439011
Version: 2
Updated fields: name, email, age, role, department
```

#### Partial Updates (Conceptual)

```bash
# Update specific fields only
dotdb patch users user_507f1f77bcf86cd799439011 '{
  "age": 31,
  "role": "senior_developer"
}'
```

### Deleting Documents

#### Delete by ID

```bash
dotdb delete users user_507f1f77bcf86cd799439011
```

**Response:**
```
Document deleted successfully
ID: user_507f1f77bcf86cd799439011
Collection: users
```

#### Bulk Delete (Conceptual)

```bash
# Delete all documents matching criteria
dotdb delete-where users '{"role": "inactive"}'
```

## Advanced Querying

### Query Operators (Conceptual)

DotDB supports MongoDB-style query operators:

#### Comparison Operators

```bash
# Greater than
dotdb query users '{"age": {"$gt": 25}}'

# Less than or equal
dotdb query users '{"age": {"$lte": 30}}'

# In array
dotdb query users '{"role": {"$in": ["developer", "admin"]}}'

# Not equal
dotdb query users '{"status": {"$ne": "inactive"}}'
```

#### Logical Operators

```bash
# AND condition
dotdb query users '{
  "$and": [
    {"age": {"$gt": 25}},
    {"role": "developer"}
  ]
}'

# OR condition
dotdb query users '{
  "$or": [
    {"role": "admin"},
    {"age": {"$gt": 50}}
  ]
}'
```

#### Array Operators

```bash
# Array contains element
dotdb query users '{"tags": {"$elemMatch": "admin"}}'

# Array size
dotdb query users '{"tags": {"$size": 2}}'

# All elements match
dotdb query users '{"tags": {"$all": ["developer", "admin"]}}'
```

### Projection (Conceptual)

Select specific fields to return:

```bash
# Return only name and email
dotdb query users '{}' --fields 'name,email'

# Exclude system fields
dotdb query users '{}' --exclude '_version,_created_at,_updated_at'
```

### Sorting and Limiting

```bash
# Sort by age ascending
dotdb query users '{}' --sort 'age:1'

# Sort by age descending
dotdb query users '{}' --sort 'age:-1'

# Limit results
dotdb query users '{}' --limit 10

# Skip and limit (pagination)
dotdb query users '{}' --skip 20 --limit 10
```

## Indexing for Performance

### Creating Indices

#### Single Field Index

```bash
# Create index on email field
dotdb create-index users email_idx email --unique

# Create index on age field
dotdb create-index users age_idx age
```

#### Compound Index

```bash
# Create compound index on multiple fields
dotdb create-index users name_age_idx name,age

# Create index with custom options
dotdb create-index users location_idx city,state --type btree
```

#### Index Types

**Hash Index (Fast Equality):**
```bash
dotdb create-index users email_hash_idx email --type hash --unique
```
- **Use case**: Exact match queries
- **Performance**: O(1) lookup
- **Limitations**: No range queries

**B-Tree Index (Range Queries):**
```bash
dotdb create-index users age_btree_idx age --type btree
```
- **Use case**: Range queries, sorting
- **Performance**: O(log n) lookup
- **Features**: Supports <, >, <=, >= operations

### Index Management

#### List Indices

```bash
dotdb list-indices users
```

**Response:**
```
Indices for collection 'users':
- primary (btree, unique) on [_id]
- email_idx (hash, unique) on [email]
- age_idx (btree) on [age]
- name_age_idx (btree) on [name, age]
Total: 4 indices
```

#### Index Statistics

```bash
dotdb index-stats users email_idx
```

**Response:**
```
Index Statistics: users.email_idx
Type: Hash
Fields: [email]
Unique: true
Size: 2.1 MB
Entries: 10,000
Usage: 95.2% (queries using this index)
Last used: 2025-01-01T12:00:00Z
```

#### Drop Index

```bash
dotdb drop-index users email_idx
```

### Query Optimization

#### Explain Query Plan

```bash
dotdb explain users '{"email": "alice@example.com"}'
```

**Response:**
```
Query Plan:
Collection: users
Query: {"email": "alice@example.com"}

Execution Plan:
1. Index Scan: email_idx (hash)
   - Index: users.email_idx
   - Type: Hash lookup
   - Estimated cost: 1
   - Estimated rows: 1

Performance:
- Execution time: 0.1ms
- Documents examined: 1
- Documents returned: 1
- Index hit: Yes
```

#### Query Performance Tips

1. **Create appropriate indices** for common query patterns
2. **Use compound indices** for multi-field queries
3. **Avoid full collection scans** by ensuring queries use indices
4. **Monitor query performance** with explain plans
5. **Consider index selectivity** when designing indices

## Data Validation

### Schema Validation (Conceptual)

Define validation rules for documents:

```json
{
  "collection": "users",
  "schema": {
    "type": "object",
    "required": ["name", "email"],
    "properties": {
      "name": {
        "type": "string",
        "minLength": 1,
        "maxLength": 100
      },
      "email": {
        "type": "string",
        "format": "email"
      },
      "age": {
        "type": "integer",
        "minimum": 0,
        "maximum": 150
      }
    }
  }
}
```

### Validation Examples

```bash
# Valid document
dotdb put users '{
  "name": "Alice",
  "email": "alice@example.com",
  "age": 30
}'  # Success

# Invalid document (missing required field)
dotdb put users '{
  "name": "Bob"
}'  # Error: Missing required field 'email'

# Invalid document (invalid email format)
dotdb put users '{
  "name": "Charlie",
  "email": "invalid-email"
}'  # Error: Invalid email format
```

## Document Versioning

### Version Control

DotDB automatically manages document versions:

```bash
# Initial insert (version 1)
dotdb put users '{"name": "Alice", "age": 30}'

# Update (version 2)
dotdb update users user_123 '{"name": "Alice", "age": 31}'

# Another update (version 3)
dotdb update users user_123 '{"name": "Alice Smith", "age": 31}'
```

### Version History (Conceptual)

```bash
# Get document history
dotdb history users user_123

# Get specific version
dotdb get users user_123 --version 2

# Revert to previous version
dotdb revert users user_123 --to-version 2
```

## Aggregation and Analytics

### Basic Aggregation

#### Count Documents

```bash
# Count all documents
dotdb count users

# Count with filter
dotdb count users '{"role": "developer"}'
```

#### Group By (Conceptual)

```bash
# Group by role and count
dotdb aggregate users '[
  {"$group": {"_id": "$role", "count": {"$sum": 1}}}
]'
```

**Response:**
```json
[
  {"_id": "developer", "count": 15},
  {"_id": "admin", "count": 3},
  {"_id": "manager", "count": 7}
]
```

### Statistical Operations (Conceptual)

```bash
# Average age by role
dotdb aggregate users '[
  {"$group": {
    "_id": "$role",
    "avg_age": {"$avg": "$age"},
    "min_age": {"$min": "$age"},
    "max_age": {"$max": "$age"}
  }}
]'
```

## Performance Optimization

### Document Design

#### Optimal Document Structure

```json
{
  "user_id": "user_123",
  "profile": {
    "name": "Alice Johnson",
    "email": "alice@example.com",
    "avatar_url": "https://example.com/avatar.jpg"
  },
  "settings": {
    "theme": "dark",
    "notifications": true,
    "language": "en"
  },
  "metadata": {
    "last_login": "2025-01-01T12:00:00Z",
    "login_count": 42,
    "account_type": "premium"
  }
}
```

**Best Practices:**
- **Group related fields** into nested objects
- **Use consistent field names** across documents
- **Avoid deeply nested structures** (max 3-4 levels)
- **Keep document size reasonable** (< 1MB)

#### Anti-Patterns

**Avoid:**
```json
{
  "user_name": "Alice",
  "user_email": "alice@example.com",
  "user_age": 30,
  "user_role": "developer",
  "user_department": "engineering",
  "user_manager": "Bob",
  "user_start_date": "2020-01-01"
}
```

**Better:**
```json
{
  "user": {
    "name": "Alice",
    "email": "alice@example.com",
    "age": 30,
    "employment": {
      "role": "developer",
      "department": "engineering",
      "manager": "Bob",
      "start_date": "2020-01-01"
    }
  }
}
```

### Batch Operations

#### Bulk Insert

```bash
# Prepare batch data
cat > batch_users.json << EOF
[
  {"name": "User1", "email": "user1@example.com"},
  {"name": "User2", "email": "user2@example.com"},
  {"name": "User3", "email": "user3@example.com"}
]
EOF

# Bulk insert (conceptual)
dotdb bulk-insert users batch_users.json
```

#### Bulk Update

```bash
# Bulk update matching documents
dotdb bulk-update users '{"role": "intern"}' '{"$set": {"role": "junior_developer"}}'
```

### Monitoring and Maintenance

#### Collection Statistics

```bash
dotdb stats users
```

**Response:**
```
Collection Statistics: users
Documents: 10,000
Total size: 15.2 MB
Average document size: 1.52 KB
Indices: 4 (3.1 MB)
Fragmentation: 5.2%
Last compaction: 2025-01-01T00:00:00Z
```

#### Performance Monitoring

```bash
# Monitor slow queries
dotdb slow-queries users --threshold 100ms

# Monitor index usage
dotdb index-usage users --period 24h

# Monitor collection growth
dotdb growth-stats users --period 7d
```

## Real-World Examples

### User Management System

```bash
# Create users collection with indices
dotdb create-collection users
dotdb create-index users email_idx email --unique
dotdb create-index users role_idx role
dotdb create-index users department_idx department

# Insert users
dotdb put users '{
  "name": "Alice Johnson",
  "email": "alice@company.com",
  "role": "developer",
  "department": "engineering",
  "hire_date": "2020-01-15",
  "salary": 75000,
  "skills": ["rust", "python", "javascript"]
}'

dotdb put users '{
  "name": "Bob Smith",
  "email": "bob@company.com",
  "role": "manager",
  "department": "engineering",
  "hire_date": "2018-03-01",
  "salary": 95000,
  "skills": ["leadership", "project_management"]
}'

# Query users
dotdb find users department '"engineering"'
dotdb find users role '"developer"'
dotdb count users '{"salary": {"$gt": 80000}}'
```

### Product Catalog

```bash
# Create products collection
dotdb create-collection products
dotdb create-index products sku_idx sku --unique
dotdb create-index products category_idx category
dotdb create-index products price_idx price

# Insert products
dotdb put products '{
  "sku": "LAPTOP-001",
  "name": "Professional Laptop",
  "category": "electronics",
  "price": 1299.99,
  "stock": 50,
  "specifications": {
    "cpu": "Intel i7",
    "ram": "16GB",
    "storage": "512GB SSD"
  },
  "tags": ["laptop", "professional", "business"]
}'

# Query products
dotdb find products category '"electronics"'
dotdb query products '{"price": {"$lt": 1000}}'
dotdb query products '{"stock": {"$gt": 0}}'
```

### Event Logging

```bash
# Create events collection
dotdb create-collection events
dotdb create-index events timestamp_idx timestamp
dotdb create-index events user_id_idx user_id
dotdb create-index events event_type_idx event_type

# Log events
dotdb put events '{
  "event_type": "user_login",
  "user_id": "user_123",
  "timestamp": "2025-01-01T12:00:00Z",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0...",
  "success": true
}'

# Query events
dotdb find events event_type '"user_login"'
dotdb query events '{"timestamp": {"$gte": "2025-01-01T00:00:00Z"}}'
dotdb count events '{"success": false}'
```

For more information about advanced DotDB features, see the [Advanced Features Guide](advanced-features.md).