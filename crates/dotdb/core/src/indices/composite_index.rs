// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::persistence::IndexPersistence;
use crate::indices::{BPlusTree, CompositeKey, HashIndex, Index, IndexError, IndexKey, IndexMaintenance, IndexResult, IndexStats, IndexType, IndexValue, RangeQuery, create_composite_key};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Field specification for composite index
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldSpec {
    /// Name of the field
    pub name: String,
    /// Position in the composite key (0-based)
    pub position: usize,
    /// Whether this field is used for range queries
    pub range_queryable: bool,
}

impl FieldSpec {
    pub fn new(name: String, position: usize, range_queryable: bool) -> Self {
        Self { name, position, range_queryable }
    }
}

/// Configuration for composite index
#[derive(Debug, Clone)]
pub struct CompositeIndexConfig {
    /// Fields that make up the composite key
    pub fields: Vec<FieldSpec>,
    /// Whether to use B+ tree (for range queries) or hash index (for equality)
    pub use_btree: bool,
    /// Initial capacity for hash-based storage
    pub initial_capacity: Option<usize>,
}

impl CompositeIndexConfig {
    pub fn new(fields: Vec<FieldSpec>, use_btree: bool) -> Self {
        Self {
            fields,
            use_btree,
            initial_capacity: None,
        }
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = Some(capacity);
        self
    }

    pub fn validate(&self) -> IndexResult<()> {
        if self.fields.is_empty() {
            return Err(IndexError::InvalidOperation("Composite index must have at least one field".to_string()));
        }

        // Check for duplicate positions
        let mut positions = std::collections::HashSet::new();
        for field in &self.fields {
            if !positions.insert(field.position) {
                return Err(IndexError::InvalidOperation(format!("Duplicate position {} in composite index", field.position)));
            }
        }

        // Check for gaps in positions
        let max_position = self.fields.iter().map(|f| f.position).max().unwrap();
        if max_position != self.fields.len() - 1 {
            return Err(IndexError::InvalidOperation("Positions in composite index must be contiguous starting from 0".to_string()));
        }

        Ok(())
    }
}

/// Bitmap index for efficient set operations
#[derive(Debug, Clone)]
pub struct BitmapIndex {
    /// Bitmap data
    pub bitmap: Vec<u64>,
    /// Number of bits set
    pub count: usize,
    /// Total capacity in bits
    pub capacity: usize,
}

impl BitmapIndex {
    pub fn new(capacity: usize) -> Self {
        let word_count = (capacity + 63) / 64;
        Self {
            bitmap: vec![0; word_count],
            count: 0,
            capacity,
        }
    }

    pub fn set(&mut self, bit: usize) -> bool {
        if bit >= self.capacity {
            return false;
        }

        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        let mask = 1u64 << bit_idx;

        if self.bitmap[word_idx] & mask == 0 {
            self.bitmap[word_idx] |= mask;
            self.count += 1;
            true
        } else {
            false
        }
    }

    pub fn unset(&mut self, bit: usize) -> bool {
        if bit >= self.capacity {
            return false;
        }

        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        let mask = 1u64 << bit_idx;

        if self.bitmap[word_idx] & mask != 0 {
            self.bitmap[word_idx] &= !mask;
            self.count -= 1;
            true
        } else {
            false
        }
    }

    pub fn is_set(&self, bit: usize) -> bool {
        if bit >= self.capacity {
            return false;
        }

        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        let mask = 1u64 << bit_idx;

        self.bitmap[word_idx] & mask != 0
    }

    pub fn intersect(&self, other: &BitmapIndex) -> BitmapIndex {
        let min_capacity = self.capacity.min(other.capacity);
        let mut result = BitmapIndex::new(min_capacity);

        let min_words = result.bitmap.len();
        for i in 0..min_words {
            result.bitmap[i] = self.bitmap[i] & other.bitmap[i];
            result.count += result.bitmap[i].count_ones() as usize;
        }

        result
    }

    pub fn union(&self, other: &BitmapIndex) -> BitmapIndex {
        let max_capacity = self.capacity.max(other.capacity);
        let mut result = BitmapIndex::new(max_capacity);

        for i in 0..result.bitmap.len() {
            let self_word = self.bitmap.get(i).copied().unwrap_or(0);
            let other_word = other.bitmap.get(i).copied().unwrap_or(0);
            result.bitmap[i] = self_word | other_word;
            result.count += result.bitmap[i].count_ones() as usize;
        }

        result
    }
}

/// Query execution plan for composite index operations
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Fields used in the query
    pub fields: Vec<String>,
    /// Whether to use bitmap intersection
    pub use_bitmap_intersection: bool,
    /// Estimated cost of execution
    pub estimated_cost: f64,
    /// Selectivity of each field
    pub field_selectivity: HashMap<String, f64>,
    /// Recommended execution order
    pub execution_order: Vec<String>,
}

impl QueryPlan {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            use_bitmap_intersection: false,
            estimated_cost: 0.0,
            field_selectivity: HashMap::new(),
            execution_order: Vec::new(),
        }
    }

    pub fn optimize(&mut self, index_stats: &HashMap<String, IndexStats>) {
        // Calculate field selectivity based on statistics
        for field in &self.fields {
            if let Some(stats) = index_stats.get(field) {
                let unique_keys = stats.type_specific.get("unique_keys").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                let selectivity = 1.0 / (unique_keys as f64).max(1.0);
                self.field_selectivity.insert(field.clone(), selectivity);
            }
        }

        // Sort fields by selectivity (most selective first)
        self.execution_order = self.fields.clone();
        self.execution_order.sort_by(|a, b| {
            let sel_a = self.field_selectivity.get(a).unwrap_or(&1.0);
            let sel_b = self.field_selectivity.get(b).unwrap_or(&1.0);
            sel_a.partial_cmp(sel_b).unwrap()
        });

        // Decide whether to use bitmap intersection
        self.use_bitmap_intersection = self.fields.len() > 2;

        // Estimate cost
        self.estimated_cost = self.calculate_cost();
    }

    fn calculate_cost(&self) -> f64 {
        let base_cost = self.fields.len() as f64;
        let selectivity_factor: f64 = self.field_selectivity.values().product();
        base_cost * (1.0 - selectivity_factor)
    }
}

/// Composite index that can index multiple fields together
pub struct CompositeIndex<V>
where
    V: IndexValue,
{
    /// Configuration
    config: CompositeIndexConfig,
    /// Underlying storage - either B+ tree or hash index
    storage: CompositeIndexStorage<V>,
    /// Field name to position mapping
    field_positions: HashMap<String, usize>,
    /// Bitmap indices for each field value
    bitmap_indices: HashMap<String, HashMap<Vec<u8>, BitmapIndex>>,
    /// Covering index data (stores frequently accessed fields)
    covering_index: HashMap<CompositeKey, HashMap<String, Vec<u8>>>,
    /// Field statistics for query optimization
    field_stats: HashMap<String, IndexStats>,
    /// Query execution statistics
    query_stats: HashMap<String, usize>,
}

/// Storage backend for composite index
enum CompositeIndexStorage<V>
where
    V: IndexValue,
{
    BTree(Arc<RwLock<BPlusTree<CompositeKey, V>>>),
    Hash(Arc<RwLock<HashIndex<CompositeKey, V>>>),
}

impl<V> CompositeIndex<V>
where
    V: IndexValue + 'static,
{
    /// Create a new composite index
    pub fn new(config: CompositeIndexConfig) -> IndexResult<Self> {
        config.validate()?;

        let field_positions: HashMap<String, usize> = config.fields.iter().map(|f| (f.name.clone(), f.position)).collect();

        let storage = if config.use_btree {
            CompositeIndexStorage::BTree(Arc::new(RwLock::new(BPlusTree::new())))
        } else {
            let hash_index = if let Some(capacity) = config.initial_capacity {
                HashIndex::with_capacity(capacity)
            } else {
                HashIndex::new()
            };
            CompositeIndexStorage::Hash(Arc::new(RwLock::new(hash_index)))
        };

        Ok(Self {
            config,
            storage,
            field_positions,
            bitmap_indices: HashMap::new(),
            covering_index: HashMap::new(),
            field_stats: HashMap::new(),
            query_stats: HashMap::new(),
        })
    }

    /// Insert a record with field values
    pub fn insert_fields(&mut self, field_values: HashMap<String, Vec<u8>>, value: V) -> IndexResult<()> {
        let composite_key = self.create_composite_key_from_fields(&field_values)?;
        self.insert(composite_key, value)
    }

    /// Get a record by field values
    pub fn get_by_fields(&self, field_values: HashMap<String, Vec<u8>>) -> IndexResult<Option<V>> {
        let composite_key = self.create_composite_key_from_fields(&field_values)?;
        self.get(&composite_key)
    }

    /// Update a record by field values
    pub fn update_by_fields(&mut self, field_values: HashMap<String, Vec<u8>>, value: V) -> IndexResult<()> {
        let composite_key = self.create_composite_key_from_fields(&field_values)?;
        self.update(composite_key, value)
    }

    /// Delete a record by field values
    pub fn delete_by_fields(&mut self, field_values: HashMap<String, Vec<u8>>) -> IndexResult<()> {
        let composite_key = self.create_composite_key_from_fields(&field_values)?;
        self.delete(&composite_key)
    }

    /// Query by partial field values (prefix matching)
    pub fn query_by_partial_fields(&self, partial_fields: HashMap<String, Vec<u8>>) -> IndexResult<Vec<(CompositeKey, V)>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;

                // Create a prefix key for range search
                let prefix_key = self.create_partial_composite_key(&partial_fields)?;

                // Find all keys that start with this prefix
                let all_entries = btree.entries();
                let mut results = Vec::new();

                for (key, value) in all_entries {
                    if self.key_matches_partial(&key, &partial_fields)? {
                        results.push((key, value));
                    }
                }

                Ok(results)
            }
            CompositeIndexStorage::Hash(_) => Err(IndexError::InvalidOperation("Partial field queries are only supported with B+ tree storage".to_string())),
        }
    }

    /// Range query by field values (only for B+ tree storage)
    pub fn range_query_by_fields(&self, start_fields: HashMap<String, Vec<u8>>, end_fields: HashMap<String, Vec<u8>>) -> IndexResult<Vec<(CompositeKey, V)>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;

                let start_key = self.create_composite_key_from_fields(&start_fields)?;
                let end_key = self.create_composite_key_from_fields(&end_fields)?;

                btree.range(&start_key, &end_key)
            }
            CompositeIndexStorage::Hash(_) => Err(IndexError::InvalidOperation("Range queries are only supported with B+ tree storage".to_string())),
        }
    }

    /// Get field specifications
    pub fn field_specs(&self) -> &[FieldSpec] {
        &self.config.fields
    }

    /// Check if index supports range queries
    pub fn supports_range_queries(&self) -> bool {
        matches!(self.storage, CompositeIndexStorage::BTree(_))
    }

    /// Create composite key from field values
    fn create_composite_key_from_fields(&self, field_values: &HashMap<String, Vec<u8>>) -> IndexResult<CompositeKey> {
        // Check that all required fields are present
        for field_spec in &self.config.fields {
            if !field_values.contains_key(field_spec.name.as_str()) {
                return Err(IndexError::InvalidKey(format!("Missing required field: {}", field_spec.name)));
            }
        }

        // Order fields by position
        let mut ordered_values = vec![Vec::new(); self.config.fields.len()];
        for (field_name, field_value) in field_values {
            if let Some(&position) = self.field_positions.get(field_name.as_str()) {
                ordered_values[position] = field_value.clone();
            } else {
                return Err(IndexError::InvalidKey(format!("Unknown field: {}", field_name)));
            }
        }

        Ok(CompositeKey::new(ordered_values))
    }

    /// Create partial composite key for prefix matching
    fn create_partial_composite_key(&self, partial_fields: &HashMap<String, Vec<u8>>) -> IndexResult<CompositeKey> {
        let mut ordered_values = Vec::new();

        // Add fields in order, stopping at the first missing field
        for field_spec in &self.config.fields {
            if let Some(field_value) = partial_fields.get(field_spec.name.as_str()) {
                ordered_values.push(field_value.clone());
            } else {
                break;
            }
        }

        if ordered_values.is_empty() {
            return Err(IndexError::InvalidKey("At least one field must be specified for partial matching".to_string()));
        }

        Ok(CompositeKey::new(ordered_values))
    }

    /// Check if a composite key matches partial field values
    fn key_matches_partial(&self, key: &CompositeKey, partial_fields: &HashMap<String, Vec<u8>>) -> IndexResult<bool> {
        for (field_name, expected_value) in partial_fields {
            if let Some(&position) = self.field_positions.get(field_name.as_str()) {
                if let Some(actual_value) = key.get_field(position) {
                    if actual_value != expected_value {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            } else {
                return Err(IndexError::InvalidKey(format!("Unknown field: {}", field_name)));
            }
        }
        Ok(true)
    }

    /// Create a query execution plan for optimized multi-field queries
    pub fn create_query_plan(&self, fields: Vec<String>) -> IndexResult<QueryPlan> {
        let mut plan = QueryPlan::new();
        plan.fields = fields;
        plan.optimize(&self.field_stats);
        Ok(plan)
    }

    /// Execute a complex query using bitmap intersection
    pub fn query_with_bitmap_intersection(&self, field_conditions: HashMap<String, Vec<u8>>) -> IndexResult<Vec<(CompositeKey, V)>> {
        if field_conditions.len() < 2 {
            return self.query_by_partial_fields(field_conditions);
        }

        let mut result_bitmap: Option<BitmapIndex> = None;

        // Intersect bitmaps for each field condition
        for (field_name, field_value) in &field_conditions {
            if let Some(field_bitmaps) = self.bitmap_indices.get(field_name) {
                if let Some(bitmap) = field_bitmaps.get(field_value) {
                    result_bitmap = Some(match result_bitmap {
                        None => bitmap.clone(),
                        Some(existing) => existing.intersect(bitmap),
                    });
                } else {
                    // Field value not found, return empty result
                    return Ok(Vec::new());
                }
            }
        }

        // Convert bitmap result to actual entries
        if let Some(bitmap) = result_bitmap {
            let mut results = Vec::new();
            let all_entries = self.entries();

            for (i, (key, value)) in all_entries.into_iter().enumerate() {
                if i < bitmap.capacity && bitmap.is_set(i) {
                    results.push((key, value));
                }
            }

            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    /// Add covering index data for frequently accessed fields
    pub fn add_covering_data(&mut self, key: CompositeKey, covering_fields: HashMap<String, Vec<u8>>) -> IndexResult<()> {
        self.covering_index.insert(key, covering_fields);
        Ok(())
    }

    /// Get covering index data
    pub fn get_covering_data(&self, key: &CompositeKey) -> Option<&HashMap<String, Vec<u8>>> {
        self.covering_index.get(key)
    }

    /// Update bitmap indices when inserting new data
    fn update_bitmap_indices(&mut self, key: &CompositeKey, record_id: usize) -> IndexResult<()> {
        for (field_name, &position) in &self.field_positions {
            if let Some(field_value) = key.get_field(position) {
                let field_bitmaps = self.bitmap_indices.entry(field_name.clone()).or_insert_with(HashMap::new);

                let bitmap = field_bitmaps.entry(field_value.clone()).or_insert_with(|| BitmapIndex::new(10000)); // Default capacity

                bitmap.set(record_id);
            }
        }
        Ok(())
    }

    /// Update field statistics for query optimization
    fn update_field_stats(&mut self, field_name: &str, value: &[u8]) {
        let stats = self.field_stats.entry(field_name.to_string()).or_insert_with(|| IndexStats::new(IndexType::Hash));

        stats.entry_count += 1;
        stats.size_bytes += value.len();

        // Update unique keys count (simplified)
        if let Some(field_bitmaps) = self.bitmap_indices.get(field_name) {
            let unique_count = field_bitmaps.len();
            stats.type_specific.insert("unique_keys".to_string(), unique_count.to_string());
        }
    }

    /// Get query statistics
    pub fn get_query_stats(&self) -> &HashMap<String, usize> {
        &self.query_stats
    }

    /// Execute optimized multi-field query
    pub fn optimized_query(&mut self, field_conditions: HashMap<String, Vec<u8>>) -> IndexResult<Vec<(CompositeKey, V)>> {
        // Update query statistics
        let field_names: Vec<String> = field_conditions.keys().cloned().collect();
        let query_key = format!("multi:{}", field_names.join(","));
        *self.query_stats.entry(query_key).or_insert(0) += 1;

        // Create and optimize query plan
        let fields: Vec<String> = field_conditions.keys().cloned().collect();
        let plan = self.create_query_plan(fields)?;

        // Choose execution strategy based on plan
        if plan.use_bitmap_intersection && field_conditions.len() > 2 {
            self.query_with_bitmap_intersection(field_conditions)
        } else {
            self.query_by_partial_fields(field_conditions)
        }
    }

    /// Analyze query patterns and suggest optimizations
    pub fn analyze_query_patterns(&self) -> HashMap<String, String> {
        let mut suggestions = HashMap::new();

        for (query_pattern, count) in &self.query_stats {
            if *count > 100 {
                suggestions.insert(query_pattern.clone(), "Consider adding a specialized index for this query pattern".to_string());
            }
        }

        // Check for covering index opportunities
        let frequent_fields: Vec<String> = self
            .query_stats
            .keys()
            .filter(|pattern| pattern.starts_with("multi:"))
            .filter_map(|pattern| pattern.strip_prefix("multi:"))
            .flat_map(|fields| fields.split(','))
            .map(|s| s.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if frequent_fields.len() > 2 {
            suggestions.insert("covering_index".to_string(), format!("Consider creating covering index for fields: {}", frequent_fields.join(", ")));
        }

        suggestions
    }
}

impl<V> Index<CompositeKey, V> for CompositeIndex<V>
where
    V: IndexValue,
{
    fn insert(&mut self, key: CompositeKey, value: V) -> IndexResult<()> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let mut btree = btree.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on B+ tree".to_string()))?;
                btree.insert(key, value)
            }
            CompositeIndexStorage::Hash(hash) => {
                let mut hash = hash.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on hash index".to_string()))?;
                hash.insert(key, value)
            }
        }
    }

    fn get(&self, key: &CompositeKey) -> IndexResult<Option<V>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;
                btree.get(key)
            }
            CompositeIndexStorage::Hash(hash) => {
                let hash = hash.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on hash index".to_string()))?;
                hash.get(key)
            }
        }
    }

    fn update(&mut self, key: CompositeKey, value: V) -> IndexResult<()> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let mut btree = btree.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on B+ tree".to_string()))?;
                btree.update(key, value)
            }
            CompositeIndexStorage::Hash(hash) => {
                let mut hash = hash.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on hash index".to_string()))?;
                hash.update(key, value)
            }
        }
    }

    fn delete(&mut self, key: &CompositeKey) -> IndexResult<()> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let mut btree = btree.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on B+ tree".to_string()))?;
                btree.delete(key)
            }
            CompositeIndexStorage::Hash(hash) => {
                let mut hash = hash.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on hash index".to_string()))?;
                hash.delete(key)
            }
        }
    }

    fn contains(&self, key: &CompositeKey) -> bool {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    btree.contains(key)
                } else {
                    false
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    hash.contains(key)
                } else {
                    false
                }
            }
        }
    }

    fn len(&self) -> usize {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    btree.len()
                } else {
                    0
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    hash.len()
                } else {
                    0
                }
            }
        }
    }

    fn clear(&mut self) {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(mut btree) = btree.write() {
                    btree.clear();
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(mut hash) = hash.write() {
                    hash.clear();
                }
            }
        }
    }

    fn index_type(&self) -> IndexType {
        IndexType::Composite(self.config.fields.iter().map(|f| f.name.clone()).collect())
    }

    fn keys(&self) -> Vec<CompositeKey> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    btree.keys()
                } else {
                    Vec::new()
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    hash.keys()
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn values(&self) -> Vec<V> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    btree.values()
                } else {
                    Vec::new()
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    hash.values()
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn entries(&self) -> Vec<(CompositeKey, V)> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    btree.entries()
                } else {
                    Vec::new()
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    hash.entries()
                } else {
                    Vec::new()
                }
            }
        }
    }
}

impl<V> RangeQuery<CompositeKey, V> for CompositeIndex<V>
where
    V: IndexValue + 'static,
{
    fn range(&self, start: &CompositeKey, end: &CompositeKey) -> IndexResult<Vec<(CompositeKey, V)>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;
                btree.range(start, end)
            }
            CompositeIndexStorage::Hash(_) => Err(IndexError::InvalidOperation("Range queries are not supported for hash-based composite indices".to_string())),
        }
    }

    fn range_from(&self, start: &CompositeKey) -> IndexResult<Vec<(CompositeKey, V)>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;
                btree.range_from(start)
            }
            CompositeIndexStorage::Hash(_) => Err(IndexError::InvalidOperation("Range queries are not supported for hash-based composite indices".to_string())),
        }
    }

    fn range_to(&self, end: &CompositeKey) -> IndexResult<Vec<(CompositeKey, V)>> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;
                btree.range_to(end)
            }
            CompositeIndexStorage::Hash(_) => Err(IndexError::InvalidOperation("Range queries are not supported for hash-based composite indices".to_string())),
        }
    }

    fn range_iter(&self, _start: &CompositeKey, _end: &CompositeKey) -> Box<dyn crate::indices::IndexIterator<CompositeKey, V>> {
        todo!("Range iterator implementation for composite index")
    }
}

impl<V> IndexMaintenance for CompositeIndex<V>
where
    V: IndexValue,
{
    fn compact(&mut self) -> IndexResult<()> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let mut btree = btree.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on B+ tree".to_string()))?;
                btree.compact()
            }
            CompositeIndexStorage::Hash(hash) => {
                let mut hash = hash.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on hash index".to_string()))?;
                hash.compact()
            }
        }
    }

    fn verify(&self) -> IndexResult<bool> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let btree = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on B+ tree".to_string()))?;
                btree.verify()
            }
            CompositeIndexStorage::Hash(hash) => {
                let hash = hash.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock on hash index".to_string()))?;
                hash.verify()
            }
        }
    }

    fn stats(&self) -> IndexStats {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                if let Ok(btree) = btree.read() {
                    let mut stats = btree.stats();
                    stats.index_type = self.index_type();
                    stats.type_specific.insert("backend".to_string(), "btree".to_string());
                    stats.type_specific.insert("field_count".to_string(), self.config.fields.len().to_string());
                    for (i, field) in self.config.fields.iter().enumerate() {
                        stats.type_specific.insert(format!("field_{}", i), field.name.clone());
                    }
                    stats
                } else {
                    IndexStats::new(self.index_type())
                }
            }
            CompositeIndexStorage::Hash(hash) => {
                if let Ok(hash) = hash.read() {
                    let mut stats = hash.stats();
                    stats.index_type = self.index_type();
                    stats.type_specific.insert("backend".to_string(), "hash".to_string());
                    stats.type_specific.insert("field_count".to_string(), self.config.fields.len().to_string());
                    for (i, field) in self.config.fields.iter().enumerate() {
                        stats.type_specific.insert(format!("field_{}", i), field.name.clone());
                    }
                    stats
                } else {
                    IndexStats::new(self.index_type())
                }
            }
        }
    }

    fn rebuild(&mut self) -> IndexResult<()> {
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                let mut btree = btree.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on B+ tree".to_string()))?;
                btree.rebuild()
            }
            CompositeIndexStorage::Hash(hash) => {
                let mut hash = hash.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock on hash index".to_string()))?;
                hash.rebuild()
            }
        }
    }
}

impl<V> IndexPersistence<CompositeKey, V> for CompositeIndex<V>
where
    V: IndexValue + 'static,
{
    fn serialize(&self) -> IndexResult<Vec<u8>> {
        let mut data = Vec::new();

        // Serialize field configuration as JSON for simplicity
        let fields_json = serde_json::to_string(&self.config.fields).map_err(|e| IndexError::SerializationError(format!("Failed to serialize fields: {}", e)))?;
        let fields_bytes = fields_json.as_bytes();
        data.extend_from_slice(&fields_bytes.len().to_le_bytes());
        data.extend_from_slice(fields_bytes);

        // Serialize config flags
        data.push(if self.config.use_btree { 1 } else { 0 });
        data.extend_from_slice(&self.config.initial_capacity.unwrap_or(0).to_le_bytes());

        // Serialize underlying storage based on type
        match &self.storage {
            CompositeIndexStorage::BTree(btree) => {
                data.push(0); // Type marker for B+ tree
                let btree_guard = btree.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock".to_string()))?;
                let btree_data = btree_guard.serialize()?;
                data.extend_from_slice(&btree_data.len().to_le_bytes());
                data.extend_from_slice(&btree_data);
            }
            CompositeIndexStorage::Hash(hash) => {
                data.push(1); // Type marker for hash
                let hash_guard = hash.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock".to_string()))?;
                let hash_data = hash_guard.serialize()?;
                data.extend_from_slice(&hash_data.len().to_le_bytes());
                data.extend_from_slice(&hash_data);
            }
        }

        Ok(data)
    }

    fn deserialize(&mut self, data: &[u8]) -> IndexResult<()> {
        if data.len() < 8 {
            return Err(IndexError::SerializationError("Insufficient data for fields length".to_string()));
        }

        let mut offset = 0;

        // Read fields JSON
        let fields_len = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        if offset + fields_len > data.len() {
            return Err(IndexError::SerializationError("Insufficient data for fields".to_string()));
        }

        let fields_json = String::from_utf8(data[offset..offset + fields_len].to_vec()).map_err(|_| IndexError::SerializationError("Invalid UTF-8 in fields".to_string()))?;
        offset += fields_len;

        let fields: Vec<FieldSpec> = serde_json::from_str(&fields_json).map_err(|e| IndexError::SerializationError(format!("Failed to deserialize fields: {}", e)))?;

        if offset + 9 > data.len() {
            return Err(IndexError::SerializationError("Insufficient data for config flags".to_string()));
        }

        // Read config flags
        let use_btree = data[offset] != 0;
        offset += 1;

        let initial_capacity = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        if offset >= data.len() {
            return Err(IndexError::SerializationError("Insufficient data for storage type".to_string()));
        }

        // Read storage type
        let storage_type = data[offset];
        offset += 1;

        if offset + 8 > data.len() {
            return Err(IndexError::SerializationError("Insufficient data for storage data length".to_string()));
        }

        let storage_data_len = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        if offset + storage_data_len > data.len() {
            return Err(IndexError::SerializationError("Insufficient data for storage data".to_string()));
        }

        // Recreate the composite index
        let mut config = CompositeIndexConfig::new(fields, use_btree);
        if initial_capacity > 0 {
            config = config.with_capacity(initial_capacity);
        }

        self.config = config;
        self.field_positions = self.config.fields.iter().map(|f| (f.name.clone(), f.position)).collect();

        // Recreate storage
        match storage_type {
            0 => {
                // B+ tree
                let mut btree = BPlusTree::new();
                btree.deserialize(&data[offset..offset + storage_data_len])?;
                self.storage = CompositeIndexStorage::BTree(Arc::new(RwLock::new(btree)));
            }
            1 => {
                // Hash index
                let mut hash = HashIndex::new();
                hash.deserialize(&data[offset..offset + storage_data_len])?;
                self.storage = CompositeIndexStorage::Hash(Arc::new(RwLock::new(hash)));
            }
            _ => {
                return Err(IndexError::SerializationError("Unknown storage type".to_string()));
            }
        }

        Ok(())
    }

    fn save_to_disk<P: AsRef<std::path::Path>>(&self, path: P) -> IndexResult<()> {
        let data = self.serialize()?;
        std::fs::write(path, data).map_err(|e| IndexError::IoError(format!("Failed to write to disk: {}", e)))
    }

    fn load_from_disk<P: AsRef<std::path::Path>>(&mut self, path: P) -> IndexResult<()> {
        let data = std::fs::read(path).map_err(|e| IndexError::IoError(format!("Failed to read from disk: {}", e)))?;
        self.deserialize(&data)
    }

    fn format_version(&self) -> u32 {
        1
    }

    fn supports_incremental_save(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_spec_creation() {
        let field = FieldSpec::new("name".to_string(), 0, true);
        assert_eq!(field.name, "name");
        assert_eq!(field.position, 0);
        assert!(field.range_queryable);
    }

    #[test]
    fn test_composite_index_config_validation() {
        // Valid config
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("first_name".to_string(), 0, true), FieldSpec::new("last_name".to_string(), 1, true)], true);
        assert!(config.validate().is_ok());

        // Empty fields
        let config = CompositeIndexConfig::new(vec![], true);
        assert!(config.validate().is_err());

        // Duplicate positions
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, true), FieldSpec::new("field2".to_string(), 0, true)], true);
        assert!(config.validate().is_err());

        // Gap in positions
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, true), FieldSpec::new("field2".to_string(), 2, true)], true);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_composite_index_btree_operations() {
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("first_name".to_string(), 0, true), FieldSpec::new("last_name".to_string(), 1, true)], true);

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Test insert_fields
        let mut fields = HashMap::new();
        fields.insert("first_name".to_string(), b"John".to_vec());
        fields.insert("last_name".to_string(), b"Doe".to_vec());

        index.insert_fields(fields.clone(), "Employee 1".to_string()).unwrap();

        // Test get_by_fields
        let result = index.get_by_fields(fields.clone()).unwrap();
        assert_eq!(result, Some("Employee 1".to_string()));

        // Test update_by_fields
        index.update_by_fields(fields.clone(), "Updated Employee 1".to_string()).unwrap();
        let result = index.get_by_fields(fields.clone()).unwrap();
        assert_eq!(result, Some("Updated Employee 1".to_string()));

        // Test delete_by_fields
        index.delete_by_fields(fields.clone()).unwrap();
        let result = index.get_by_fields(fields).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_composite_index_hash_operations() {
        let config = CompositeIndexConfig::new(
            vec![FieldSpec::new("user_id".to_string(), 0, false), FieldSpec::new("session_id".to_string(), 1, false)],
            false, // Use hash index
        );

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), b"user123".to_vec());
        fields.insert("session_id".to_string(), b"sess456".to_vec());

        index.insert_fields(fields.clone(), "Session Data".to_string()).unwrap();

        let result = index.get_by_fields(fields.clone()).unwrap();
        assert_eq!(result, Some("Session Data".to_string()));

        assert!(index.contains(&index.create_composite_key_from_fields(&fields).unwrap()));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_composite_index_partial_query() {
        let config = CompositeIndexConfig::new(
            vec![
                FieldSpec::new("department".to_string(), 0, true),
                FieldSpec::new("level".to_string(), 1, true),
                FieldSpec::new("employee_id".to_string(), 2, true),
            ],
            true, // Use B+ tree for range queries
        );

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Insert multiple records
        let records = vec![
            (vec!["Engineering", "Senior", "001"], "Alice"),
            (vec!["Engineering", "Senior", "002"], "Bob"),
            (vec!["Engineering", "Junior", "003"], "Charlie"),
            (vec!["Sales", "Senior", "004"], "David"),
        ];

        for (field_values, name) in records {
            let mut fields = HashMap::new();
            fields.insert("department".to_string(), field_values[0].as_bytes().to_vec());
            fields.insert("level".to_string(), field_values[1].as_bytes().to_vec());
            fields.insert("employee_id".to_string(), field_values[2].as_bytes().to_vec());

            index.insert_fields(fields, name.to_string()).unwrap();
        }

        // Query by department only
        let mut partial_fields = HashMap::new();
        partial_fields.insert("department".to_string(), b"Engineering".to_vec());

        let results = index.query_by_partial_fields(partial_fields).unwrap();
        assert_eq!(results.len(), 3); // Should find Alice, Bob, and Charlie

        // Query by department and level
        let mut partial_fields = HashMap::new();
        partial_fields.insert("department".to_string(), b"Engineering".to_vec());
        partial_fields.insert("level".to_string(), b"Senior".to_vec());

        let results = index.query_by_partial_fields(partial_fields).unwrap();
        assert_eq!(results.len(), 2); // Should find Alice and Bob
    }

    #[test]
    fn test_composite_index_range_query() {
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("date".to_string(), 0, true), FieldSpec::new("priority".to_string(), 1, true)], true);

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Insert records with dates and priorities
        let records = vec![
            (vec!["2024-01-01", "high"], "Task 1"),
            (vec!["2024-01-02", "medium"], "Task 2"),
            (vec!["2024-01-03", "low"], "Task 3"),
            (vec!["2024-01-04", "high"], "Task 4"),
        ];

        for (field_values, task) in records {
            let mut fields = HashMap::new();
            fields.insert("date".to_string(), field_values[0].as_bytes().to_vec());
            fields.insert("priority".to_string(), field_values[1].as_bytes().to_vec());

            index.insert_fields(fields, task.to_string()).unwrap();
        }

        // Range query from 2024-01-02 to 2024-01-03
        let mut start_fields = HashMap::new();
        start_fields.insert("date".to_string(), b"2024-01-02".to_vec());
        start_fields.insert("priority".to_string(), b"low".to_vec());

        let mut end_fields = HashMap::new();
        end_fields.insert("date".to_string(), b"2024-01-03".to_vec());
        end_fields.insert("priority".to_string(), b"medium".to_vec());

        let results = index.range_query_by_fields(start_fields, end_fields).unwrap();
        assert!(results.len() >= 1); // Should find at least Task 2
    }

    #[test]
    fn test_composite_index_invalid_operations() {
        let config = CompositeIndexConfig::new(
            vec![FieldSpec::new("field1".to_string(), 0, false)],
            false, // Hash index
        );

        let index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Partial queries should fail with hash index
        let partial_fields = HashMap::new();
        assert!(index.query_by_partial_fields(partial_fields).is_err());

        // Range queries should fail with hash index
        let fields = HashMap::new();
        assert!(index.range_query_by_fields(fields.clone(), fields).is_err());
    }

    #[test]
    fn test_composite_index_missing_fields() {
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, true), FieldSpec::new("field2".to_string(), 1, true)], true);

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Missing field should cause error
        let mut fields = HashMap::new();
        fields.insert("field1".to_string(), b"value1".to_vec());
        // field2 is missing

        assert!(index.insert_fields(fields, "test".to_string()).is_err());
    }

    #[test]
    fn test_composite_index_maintenance() {
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, true)], true);

        let mut index: CompositeIndex<String> = CompositeIndex::new(config).unwrap();

        // Add some data
        let mut fields = HashMap::new();
        fields.insert("field1".to_string(), b"value1".to_vec());
        index.insert_fields(fields, "test".to_string()).unwrap();

        // Test maintenance operations
        assert!(index.verify().unwrap());
        assert!(index.compact().is_ok());
        assert!(index.rebuild().is_ok());

        let stats = index.stats();
        assert_eq!(stats.entry_count, 1);
        assert!(stats.type_specific.contains_key("backend"));
        assert!(stats.type_specific.contains_key("field_count"));
    }
}
