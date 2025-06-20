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

use super::lib::{IndexError, IndexKey, IndexResult, IndexType, IndexValue};
use crate::memory::mmap::{MappingStrategy, MemoryMap, MmapError};
use crate::storage_engine::page_manager::PageManager;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Trait for index persistence operations
pub trait IndexPersistence<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    /// Serialize the index to bytes
    fn serialize(&self) -> IndexResult<Vec<u8>>;

    /// Deserialize the index from bytes
    fn deserialize(&mut self, data: &[u8]) -> IndexResult<()>;

    /// Save the index to disk
    fn save_to_disk<P: AsRef<Path>>(&self, path: P) -> IndexResult<()>;

    /// Load the index from disk
    fn load_from_disk<P: AsRef<Path>>(&mut self, path: P) -> IndexResult<()>;

    /// Get the index format version for compatibility
    fn format_version(&self) -> u32;

    /// Check if the index can be incremental saved (delta changes)
    fn supports_incremental_save(&self) -> bool {
        false
    }

    /// Save only the changes since last save (if supported)
    fn save_incremental<P: AsRef<Path>>(&self, _path: P) -> IndexResult<()> {
        Err(IndexError::InvalidOperation("Incremental save not supported".to_string()))
    }
}

/// Persistence manager for indices with storage engine integration
pub struct IndexPersistenceManager {
    /// Root directory for index files
    root_path: PathBuf,
    /// Page manager for storage engine integration  
    page_manager: Option<Arc<RwLock<crate::storage_engine::page_manager::PageManager>>>,
    /// Memory-mapped index files
    mmapped_indices: HashMap<String, Arc<RwLock<MemoryMap>>>,
    /// Index metadata
    metadata: HashMap<String, IndexMetadata>,
    /// Enable compression for serialized indices
    enable_compression: bool,
    /// Index file format version
    format_version: u32,
}

/// Metadata for persisted indices
#[derive(Debug, Clone)]
pub struct IndexMetadata {
    /// Index name
    pub name: String,
    /// Index type
    pub index_type: IndexType,
    /// File path on disk
    pub file_path: PathBuf,
    /// Size of the index on disk
    pub disk_size: u64,
    /// Number of entries in the index
    pub entry_count: usize,
    /// Last modification timestamp
    pub last_modified: std::time::SystemTime,
    /// Format version used to save this index
    pub format_version: u32,
    /// Whether this index is currently memory-mapped
    pub is_mmap: bool,
    /// Checksum for integrity verification
    pub checksum: u64,
}

impl IndexPersistenceManager {
    /// Create a new persistence manager
    pub fn new<P: AsRef<Path>>(root_path: P) -> IndexResult<Self> {
        let root_path = root_path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&root_path).map_err(|e| IndexError::IoError(format!("Failed to create directory: {}", e)))?;

        Ok(Self {
            root_path,
            page_manager: None,
            mmapped_indices: HashMap::new(),
            metadata: HashMap::new(),
            enable_compression: true,
            format_version: 1,
        })
    }

    /// Set the page manager for storage engine integration
    pub fn set_page_manager(&mut self, page_manager: Arc<RwLock<crate::storage_engine::page_manager::PageManager>>) {
        self.page_manager = Some(page_manager);
    }

    /// Enable or disable compression
    pub fn set_compression(&mut self, enable: bool) {
        self.enable_compression = enable;
    }

    /// Register an index for persistence
    pub fn register_index(&mut self, name: String, index_type: IndexType) -> IndexResult<()> {
        let file_path = self.root_path.join(format!("{}.idx", name));

        let metadata = IndexMetadata {
            name: name.clone(),
            index_type,
            file_path,
            disk_size: 0,
            entry_count: 0,
            last_modified: std::time::SystemTime::now(),
            format_version: self.format_version,
            is_mmap: false,
            checksum: 0,
        };

        self.metadata.insert(name, metadata);
        Ok(())
    }

    /// Save an index to disk with optional memory mapping
    pub fn save_index<K, V, I>(&mut self, name: &str, index: &I, use_mmap: bool) -> IndexResult<()>
    where
        K: IndexKey,
        V: IndexValue + 'static,
        I: IndexPersistence<K, V>,
    {
        // Serialize the index first
        let mut data = index.serialize()?;

        // Apply compression if enabled
        if self.enable_compression {
            data = self.compress_data(&data)?;
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&data);

        // Extract file path before mutable borrow
        let file_path = {
            let metadata = self.metadata.get(name).ok_or_else(|| IndexError::InvalidOperation(format!("Index {} not registered", name)))?;
            metadata.file_path.clone()
        };

        if use_mmap {
            // Create memory-mapped file
            let mmap = self.create_mmap_file(&file_path, data.len())?;

            // Write data to memory-mapped region
            {
                let mut mmap_guard = mmap.write().map_err(|_| IndexError::Corruption("Failed to acquire write lock".to_string()))?;

                mmap_guard.write(0, &data).map_err(|e| IndexError::IoError(format!("Failed to write to mmap: {:?}", e)))?;

                mmap_guard.sync().map_err(|e| IndexError::IoError(format!("Failed to sync mmap: {:?}", e)))?;
            }

            self.mmapped_indices.insert(name.to_string(), mmap);

            // Now update metadata
            let metadata = self.metadata.get_mut(name).unwrap();
            metadata.is_mmap = true;
        } else {
            // Write to regular file
            std::fs::write(&file_path, &data).map_err(|e| IndexError::IoError(format!("Failed to write file: {}", e)))?;

            // Now update metadata
            let metadata = self.metadata.get_mut(name).unwrap();
            metadata.is_mmap = false;
        }

        // Update metadata
        let metadata = self.metadata.get_mut(name).unwrap();
        metadata.disk_size = data.len() as u64;
        metadata.last_modified = std::time::SystemTime::now();
        metadata.checksum = checksum;

        Ok(())
    }

    /// Load an index from disk
    pub fn load_index<K, V, I>(&mut self, name: &str, index: &mut I) -> IndexResult<()>
    where
        K: IndexKey,
        V: IndexValue + 'static,
        I: IndexPersistence<K, V>,
    {
        let metadata = self.metadata.get(name).ok_or_else(|| IndexError::InvalidOperation(format!("Index {} not registered", name)))?;

        let expected_checksum = metadata.checksum;
        let is_mmap = metadata.is_mmap;
        let file_path = metadata.file_path.clone();

        let data = if is_mmap {
            // Read from memory-mapped file
            let mmap = self.mmapped_indices.get(name).ok_or_else(|| IndexError::InvalidOperation("Memory map not found".to_string()))?;

            let mmap_guard = mmap.read().map_err(|_| IndexError::Corruption("Failed to acquire read lock".to_string()))?;

            mmap_guard.as_slice().to_vec()
        } else {
            // Read from regular file
            std::fs::read(&file_path).map_err(|e| IndexError::IoError(format!("Failed to read file: {}", e)))?
        };

        // Verify checksum
        let checksum = self.calculate_checksum(&data);
        if checksum != expected_checksum {
            return Err(IndexError::Corruption(format!("Checksum mismatch for index {}", name)));
        }

        // Decompress if needed
        let final_data = if self.enable_compression { self.decompress_data(&data)? } else { data };

        // Deserialize the index
        index.deserialize(&final_data)?;

        Ok(())
    }

    /// Create a memory-mapped file for an index
    fn create_mmap_file(&self, path: &Path, size: usize) -> IndexResult<Arc<RwLock<MemoryMap>>> {
        // Ensure file exists and has the right size
        {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|e| IndexError::IoError(format!("Failed to create file: {}", e)))?;

            file.set_len(size as u64).map_err(|e| IndexError::IoError(format!("Failed to set file size: {}", e)))?;
        }

        // Create memory map
        let mmap = MemoryMap::from_file(path, MappingStrategy::ReadWrite, 0, Some(size)).map_err(|e| IndexError::IoError(format!("Failed to create mmap: {:?}", e)))?;

        Ok(Arc::new(RwLock::new(mmap)))
    }

    /// Compress data using a simple compression algorithm
    fn compress_data(&self, data: &[u8]) -> IndexResult<Vec<u8>> {
        // For now, we'll use a simple run-length encoding
        // In production, you'd use a proper compression library like zstd or lz4
        Ok(self.simple_rle_compress(data))
    }

    /// Decompress data
    fn decompress_data(&self, data: &[u8]) -> IndexResult<Vec<u8>> {
        Ok(self.simple_rle_decompress(data)?)
    }

    /// Simple run-length encoding compression
    fn simple_rle_compress(&self, data: &[u8]) -> Vec<u8> {
        let mut compressed = Vec::new();
        if data.is_empty() {
            return compressed;
        }

        let mut current_byte = data[0];
        let mut count = 1u8;

        for &byte in &data[1..] {
            if byte == current_byte && count < 255 {
                count += 1;
            } else {
                compressed.push(count);
                compressed.push(current_byte);
                current_byte = byte;
                count = 1;
            }
        }

        // Add the last run
        compressed.push(count);
        compressed.push(current_byte);

        compressed
    }

    /// Simple run-length encoding decompression
    fn simple_rle_decompress(&self, data: &[u8]) -> IndexResult<Vec<u8>> {
        let mut decompressed = Vec::new();

        if data.len() % 2 != 0 {
            return Err(IndexError::SerializationError("Invalid RLE data length".to_string()));
        }

        for chunk in data.chunks(2) {
            let count = chunk[0];
            let byte = chunk[1];

            for _ in 0..count {
                decompressed.push(byte);
            }
        }

        Ok(decompressed)
    }

    /// Calculate a simple checksum
    fn calculate_checksum(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }

    /// Get metadata for an index
    pub fn get_metadata(&self, name: &str) -> Option<&IndexMetadata> {
        self.metadata.get(name)
    }

    /// List all registered indices
    pub fn list_indices(&self) -> Vec<String> {
        self.metadata.keys().cloned().collect()
    }

    /// Remove an index from disk
    pub fn remove_index(&mut self, name: &str) -> IndexResult<()> {
        if let Some(metadata) = self.metadata.remove(name) {
            // Remove memory mapping if it exists
            self.mmapped_indices.remove(name);

            // Remove file from disk
            if metadata.file_path.exists() {
                std::fs::remove_file(&metadata.file_path).map_err(|e| IndexError::IoError(format!("Failed to remove file: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Compact all indices (removes unused space)
    pub fn compact_all(&mut self) -> IndexResult<()> {
        for name in self.list_indices() {
            self.compact_index(&name)?;
        }
        Ok(())
    }

    /// Compact a specific index
    pub fn compact_index(&mut self, _name: &str) -> IndexResult<()> {
        // Implementation would depend on the specific index type
        // For now, we'll just mark it as a no-op
        Ok(())
    }

    /// Get total disk usage
    pub fn total_disk_usage(&self) -> u64 {
        self.metadata.values().map(|m| m.disk_size).sum()
    }

    /// Verify integrity of all indices
    pub fn verify_all(&self) -> IndexResult<HashMap<String, bool>> {
        let mut results = HashMap::new();

        for (name, metadata) in &self.metadata {
            let is_valid = self.verify_index_integrity(metadata).unwrap_or(false);
            results.insert(name.clone(), is_valid);
        }

        Ok(results)
    }

    /// Verify integrity of a specific index
    fn verify_index_integrity(&self, metadata: &IndexMetadata) -> IndexResult<bool> {
        if !metadata.file_path.exists() {
            return Ok(false);
        }

        let data = std::fs::read(&metadata.file_path).map_err(|e| IndexError::IoError(format!("Failed to read file: {}", e)))?;

        let checksum = self.calculate_checksum(&data);
        Ok(checksum == metadata.checksum)
    }
}

/// Serialization format for indices
#[derive(Debug)]
pub struct IndexSerializationFormat {
    /// Magic number to identify the format
    pub magic: u32,
    /// Format version
    pub version: u32,
    /// Index type
    pub index_type: IndexType,
    /// Metadata length
    pub metadata_len: u32,
    /// Data length
    pub data_len: u64,
}

impl IndexSerializationFormat {
    pub const MAGIC: u32 = 0x49444458; // "IDDX" in hex

    pub fn new(index_type: IndexType, metadata_len: u32, data_len: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            index_type,
            metadata_len,
            data_len,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.magic.to_le_bytes());
        data.extend_from_slice(&self.version.to_le_bytes());

        // Serialize index type
        let type_bytes = match &self.index_type {
            IndexType::BPlusTree => vec![0u8],
            IndexType::Hash => vec![1u8],
            IndexType::Composite(fields) => {
                let mut bytes = vec![2u8];
                bytes.extend_from_slice(&(fields.len() as u32).to_le_bytes());
                for field in fields {
                    let field_bytes = field.as_bytes();
                    bytes.extend_from_slice(&(field_bytes.len() as u32).to_le_bytes());
                    bytes.extend_from_slice(field_bytes);
                }
                bytes
            }
        };

        data.extend_from_slice(&(type_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&type_bytes);
        data.extend_from_slice(&self.metadata_len.to_le_bytes());
        data.extend_from_slice(&self.data_len.to_le_bytes());

        data
    }

    pub fn deserialize(data: &[u8]) -> IndexResult<(Self, usize)> {
        if data.len() < 20 {
            return Err(IndexError::SerializationError("Insufficient data for header".to_string()));
        }

        let mut offset = 0;

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        offset += 4;

        if magic != Self::MAGIC {
            return Err(IndexError::SerializationError("Invalid magic number".to_string()));
        }

        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        offset += 4;

        let type_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        offset += 4;

        if data.len() < offset + type_len + 12 {
            return Err(IndexError::SerializationError("Insufficient data for type and remaining fields".to_string()));
        }

        let type_bytes = &data[offset..offset + type_len];
        offset += type_len;

        let index_type = match type_bytes[0] {
            0 => IndexType::BPlusTree,
            1 => IndexType::Hash,
            2 => {
                if type_bytes.len() < 5 {
                    return Err(IndexError::SerializationError("Invalid composite type data".to_string()));
                }

                let field_count = u32::from_le_bytes([type_bytes[1], type_bytes[2], type_bytes[3], type_bytes[4]]) as usize;

                let mut fields = Vec::new();
                let mut field_offset = 5;

                for _ in 0..field_count {
                    if field_offset + 4 > type_bytes.len() {
                        return Err(IndexError::SerializationError("Invalid field data".to_string()));
                    }

                    let field_len = u32::from_le_bytes([type_bytes[field_offset], type_bytes[field_offset + 1], type_bytes[field_offset + 2], type_bytes[field_offset + 3]]) as usize;
                    field_offset += 4;

                    if field_offset + field_len > type_bytes.len() {
                        return Err(IndexError::SerializationError("Invalid field length".to_string()));
                    }

                    let field_name =
                        String::from_utf8(type_bytes[field_offset..field_offset + field_len].to_vec()).map_err(|_| IndexError::SerializationError("Invalid field name encoding".to_string()))?;

                    fields.push(field_name);
                    field_offset += field_len;
                }

                IndexType::Composite(fields)
            }
            _ => return Err(IndexError::SerializationError("Unknown index type".to_string())),
        };

        let metadata_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        let data_len = u64::from_le_bytes([
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

        Ok((
            Self {
                magic,
                version,
                index_type,
                metadata_len,
                data_len,
            },
            offset,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_persistence_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IndexPersistenceManager::new(temp_dir.path()).unwrap();

        assert_eq!(manager.root_path, temp_dir.path());
        assert!(manager.metadata.is_empty());
    }

    #[test]
    fn test_index_registration() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = IndexPersistenceManager::new(temp_dir.path()).unwrap();

        manager.register_index("test_index".to_string(), IndexType::BPlusTree).unwrap();

        assert!(manager.metadata.contains_key("test_index"));
        let metadata = manager.get_metadata("test_index").unwrap();
        assert_eq!(metadata.name, "test_index");
        assert_eq!(metadata.index_type, IndexType::BPlusTree);
    }

    #[test]
    fn test_serialization_format() {
        let format = IndexSerializationFormat::new(IndexType::BPlusTree, 100, 1000);

        let serialized = format.serialize();
        let (deserialized, _) = IndexSerializationFormat::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.magic, IndexSerializationFormat::MAGIC);
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.index_type, IndexType::BPlusTree);
        assert_eq!(deserialized.metadata_len, 100);
        assert_eq!(deserialized.data_len, 1000);
    }

    #[test]
    fn test_rle_compression() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IndexPersistenceManager::new(temp_dir.path()).unwrap();

        let original = vec![1, 1, 1, 2, 2, 3, 3, 3, 3];
        let compressed = manager.simple_rle_compress(&original);
        let decompressed = manager.simple_rle_decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_checksum_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IndexPersistenceManager::new(temp_dir.path()).unwrap();

        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 5];
        let data3 = vec![1, 2, 3, 4, 6];

        let checksum1 = manager.calculate_checksum(&data1);
        let checksum2 = manager.calculate_checksum(&data2);
        let checksum3 = manager.calculate_checksum(&data3);

        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }
}
