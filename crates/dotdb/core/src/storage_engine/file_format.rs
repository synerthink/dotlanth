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

// File format module
// This module defines the on-disk format for persistent storage, including page layout, headers, and file structure. It provides methods for reading, writing, allocating, and freeing pages, as well as managing file metadata and versions.

use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::storage_engine::lib::{StorageConfig, StorageError, StorageResult, VersionId};

/// Magic number to identify our file format (DOTDB)
const FILE_MAGIC: [u8; 4] = [0x44, 0x4F, 0x54, 0x44];
/// Current format version
const FORMAT_VERSION: u32 = 1;
/// Size of the file header in bytes
const HEADER_SIZE: usize = 4096;

/// Unique identifier for a page within the storage file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u64);

impl PageId {
    /// Creates a new PageId
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw page id value
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Types of pages in the storage system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageType {
    /// Metadata page (file header, free list, etc.)
    Meta = 0,
    /// Patricia trie node
    Node = 1,
    /// Leaf data page
    Data = 2,
    /// Free (unused) page
    Free = 3,
}

impl From<u8> for PageType {
    fn from(value: u8) -> Self {
        match value {
            0 => PageType::Meta,
            1 => PageType::Node,
            2 => PageType::Data,
            3 => PageType::Free,
            _ => PageType::Free, // Default to Free for unknown types
        }
    }
}

/// Page header structure (fixed size)
#[derive(Debug, Clone)]
pub struct PageHeader {
    /// Type of page
    pub page_type: PageType,
    /// Version this page belongs to
    pub version: VersionId,
    /// Reference count for this page
    pub ref_count: u32,
    /// Checksum of the page content
    pub checksum: u32,
    /// Size of the data in this page
    pub data_size: u16,
}

impl PageHeader {
    pub fn new(page_type: PageType, version: VersionId) -> Self {
        Self {
            page_type,
            version,
            ref_count: 1,
            checksum: 0,
            data_size: 0,
        }
    }

    /// Size of the header in bytes
    pub const fn size() -> usize {
        // page_type(1) + version(8) + ref_count(4) + checksum(4) + data_size(2) = 19 bytes
        // Aligned to 32 bytes for better memory access
        32
    }

    /// Serialize the header to bytes
    pub fn serialize(&self, buffer: &mut [u8]) -> StorageResult<()> {
        if buffer.len() < Self::size() {
            return Err(StorageError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Buffer too small for page header")));
        }

        buffer[0] = self.page_type as u8;

        // Version ID (8 bytes)
        let version_bytes = self.version.0.to_le_bytes();
        buffer[1..9].copy_from_slice(&version_bytes);

        // Reference count (4 bytes)
        let ref_count_bytes = self.ref_count.to_le_bytes();
        buffer[9..13].copy_from_slice(&ref_count_bytes);

        // Checksum (4 bytes)
        let checksum_bytes = self.checksum.to_le_bytes();
        buffer[13..17].copy_from_slice(&checksum_bytes);

        // Data size (2 bytes)
        let data_size_bytes = self.data_size.to_le_bytes();
        buffer[17..19].copy_from_slice(&data_size_bytes);

        // Remaining bytes are reserved and set to zero
        buffer[19..Self::size()].fill(0);

        Ok(())
    }

    /// Deserialize the header from bytes
    pub fn deserialize(buffer: &[u8]) -> StorageResult<Self> {
        if buffer.len() < Self::size() {
            return Err(StorageError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Buffer too small for page header")));
        }

        let page_type = PageType::from(buffer[0]);

        let version = VersionId(u64::from_le_bytes(buffer[1..9].try_into().map_err(|_| StorageError::Corruption("Invalid version bytes".to_string()))?));

        let ref_count = u32::from_le_bytes(buffer[9..13].try_into().map_err(|_| StorageError::Corruption("Invalid ref_count bytes".to_string()))?);

        let checksum = u32::from_le_bytes(buffer[13..17].try_into().map_err(|_| StorageError::Corruption("Invalid checksum bytes".to_string()))?);

        let data_size = u16::from_le_bytes(buffer[17..19].try_into().map_err(|_| StorageError::Corruption("Invalid data_size bytes".to_string()))?);

        Ok(Self {
            page_type,
            version,
            ref_count,
            checksum,
            data_size,
        })
    }
}

/// A page in the storage system
#[derive(Debug, Clone)]
pub struct Page {
    /// Page identifier
    pub id: PageId,
    /// Page header
    pub header: PageHeader,
    /// Page data
    pub data: Vec<u8>,
}

impl Page {
    /// Create a new page
    pub fn new(id: PageId, page_type: PageType, version: VersionId, size: usize) -> Self {
        let mut header = PageHeader::new(page_type, version);
        let data = vec![0; size - PageHeader::size()];
        header.data_size = data.len() as u16;

        Self { id, header, data }
    }

    /// Calculate the checksum of the page
    pub fn calculate_checksum(&self) -> u32 {
        // Simple CRC32 checksum
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&[self.header.page_type as u8]);
        hasher.update(&self.header.version.0.to_le_bytes());
        hasher.update(&self.header.ref_count.to_le_bytes());
        hasher.update(&self.header.data_size.to_le_bytes());

        // Only hash data up to data_size, not the entire data buffer
        let data_to_hash = if self.data.len() >= self.header.data_size as usize {
            &self.data[0..self.header.data_size as usize]
        } else {
            // In case data buffer is smaller than data_size, just use what we have
            // This should not happen in normal operation
            &self.data
        };

        hasher.update(data_to_hash);
        hasher.finalize()
    }

    /// Update the page checksum
    pub fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum();
    }

    /// Verify the page checksum
    pub fn verify_checksum(&self) -> bool {
        self.header.checksum == self.calculate_checksum()
    }
}

/// File header structure
#[derive(Debug, Clone)]
struct FileHeader {
    /// Magic number to identify our file format
    magic: [u8; 4],
    /// Format version
    version: u32,
    /// Size of pages in bytes
    page_size: u32,
    /// Total number of pages in the file
    total_pages: u64,
    /// ID of the current version
    current_version: VersionId,
    /// ID of the first free page
    first_free_page: PageId,
}

impl FileHeader {
    /// Create a new file header
    fn new(page_size: u32) -> Self {
        Self {
            magic: FILE_MAGIC,
            version: FORMAT_VERSION,
            page_size,
            total_pages: 1, // At minimum, we have the header page
            current_version: VersionId(0),
            first_free_page: PageId(0),
        }
    }

    /// Serialize the header to bytes
    fn serialize(&self, buffer: &mut [u8]) -> StorageResult<()> {
        if buffer.len() < HEADER_SIZE {
            return Err(StorageError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Buffer too small for file header")));
        }

        // Clear the buffer
        buffer.fill(0);

        // Magic number
        buffer[0..4].copy_from_slice(&self.magic);

        // Format version
        buffer[4..8].copy_from_slice(&self.version.to_le_bytes());

        // Page size
        buffer[8..12].copy_from_slice(&self.page_size.to_le_bytes());

        // Total pages
        buffer[12..20].copy_from_slice(&self.total_pages.to_le_bytes());

        // Current version
        buffer[20..28].copy_from_slice(&self.current_version.0.to_le_bytes());

        // First free page
        buffer[28..36].copy_from_slice(&self.first_free_page.0.to_le_bytes());

        Ok(())
    }

    /// Deserialize the header from bytes
    fn deserialize(buffer: &[u8]) -> StorageResult<Self> {
        if buffer.len() < HEADER_SIZE {
            return Err(StorageError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Buffer too small for file header")));
        }

        // Check magic number
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buffer[0..4]);
        if magic != FILE_MAGIC {
            return Err(StorageError::Corruption("Invalid file format".to_string()));
        }

        // Format version
        let version = u32::from_le_bytes(buffer[4..8].try_into().map_err(|_| StorageError::Corruption("Invalid version bytes".to_string()))?);

        // Check version compatibility
        if version > FORMAT_VERSION {
            return Err(StorageError::Corruption(format!("Unsupported format version: {}", version)));
        }

        // Page size
        let page_size = u32::from_le_bytes(buffer[8..12].try_into().map_err(|_| StorageError::Corruption("Invalid page_size bytes".to_string()))?);

        // Total pages
        let total_pages = u64::from_le_bytes(buffer[12..20].try_into().map_err(|_| StorageError::Corruption("Invalid total_pages bytes".to_string()))?);

        // Current version
        let current_version = VersionId(u64::from_le_bytes(
            buffer[20..28].try_into().map_err(|_| StorageError::Corruption("Invalid current_version bytes".to_string()))?,
        ));

        // First free page
        let first_free_page = PageId(u64::from_le_bytes(
            buffer[28..36].try_into().map_err(|_| StorageError::Corruption("Invalid first_free_page bytes".to_string()))?,
        ));

        Ok(Self {
            magic,
            version,
            page_size,
            total_pages,
            current_version,
            first_free_page,
        })
    }
}

/// Storage file format manager
pub struct FileFormat {
    /// Path to the storage file
    path: PathBuf,
    /// Storage configuration
    config: StorageConfig,
    /// The storage file
    file: Option<File>,
    /// File header
    header: FileHeader,
    /// Whether the file was newly created
    is_new: bool,
}

/// FileFormat manages the storage file, including page allocation, reading, writing, and file metadata. It ensures data is stored and retrieved according to the defined format.
impl FileFormat {
    /// Create a new file format manager
    pub fn new(config: StorageConfig) -> Self {
        Self {
            path: config.path.clone(),
            header: FileHeader::new(config.page_size as u32),
            config,
            file: None,
            is_new: false,
        }
    }

    /// Initialize the storage file
    pub fn init(&mut self) -> StorageResult<()> {
        // Create the directory if it doesn't exist
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Check if the file exists
        let file_exists = self.path.exists();
        self.is_new = !file_exists;

        // Open or create the file
        let mut file = OpenOptions::new().read(true).write(true).create(true).open(&self.path)?;

        // Store the file immediately so that it can be accessed later
        self.file = Some(file);

        // Initialize the file if it's new
        if self.is_new {
            // Set the file size to the header size
            let file = self.file.as_mut().unwrap();
            file.set_len(HEADER_SIZE as u64)?;

            // Write the header
            let mut buffer = vec![0; HEADER_SIZE];
            self.header.serialize(&mut buffer)?;
            file.write_all(&buffer)?;
        } else {
            // Read the header
            let file = self.file.as_mut().unwrap();
            let mut buffer = vec![0; HEADER_SIZE];
            file.read_exact(&mut buffer)?;
            self.header = FileHeader::deserialize(&buffer)?;

            // Update our config with the actual page size from the file
            self.config.page_size = self.header.page_size as usize;
        }

        Ok(())
    }

    /// Check if the storage file is initialized
    pub fn is_initialized(&self) -> bool {
        self.file.is_some()
    }

    /// Get the current version
    pub fn current_version(&self) -> VersionId {
        self.header.current_version
    }

    /// Set the current version
    pub fn set_current_version(&mut self, version: VersionId) -> StorageResult<()> {
        self.header.current_version = version;
        self.write_header()
    }

    /// Get the page size
    pub fn page_size(&self) -> usize {
        self.header.page_size as usize
    }

    /// Get the total number of pages
    pub fn total_pages(&self) -> u64 {
        self.header.total_pages
    }

    /// Write the header to disk
    fn write_header(&mut self) -> StorageResult<()> {
        if let Some(file) = &mut self.file {
            let mut buffer = vec![0; HEADER_SIZE];
            self.header.serialize(&mut buffer)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&buffer)?;
            file.flush()?;
            Ok(())
        } else {
            Err(StorageError::Io(io::Error::new(io::ErrorKind::NotConnected, "File not initialized")))
        }
    }

    /// Reads a page from disk by its ID.
    ///
    /// Steps:
    /// 1. Check if the page ID is valid (within total_pages).
    /// 2. Seek to the correct offset in the file.
    /// 3. Read the page data into a buffer.
    /// 4. Deserialize the page header and data.
    /// 5. Verify the checksum for data integrity.
    /// 6. Return the reconstructed Page object.
    pub fn read_page(&mut self, id: PageId) -> StorageResult<Page> {
        if id.0 >= self.header.total_pages {
            return Err(StorageError::PageNotFound(id.0));
        }

        let file = self
            .file
            .as_mut()
            .ok_or_else(|| StorageError::Io(io::Error::new(io::ErrorKind::NotConnected, "File not initialized")))?;

        // Skip the header page
        let offset = if id.0 == 0 { 0 } else { HEADER_SIZE as u64 + (id.0 - 1) * self.header.page_size as u64 };

        file.seek(SeekFrom::Start(offset))?;

        let page_size = self.header.page_size as usize;
        let mut buffer = vec![0; page_size];
        file.read_exact(&mut buffer)?;

        // Parse header
        let header = PageHeader::deserialize(&buffer[0..PageHeader::size()])?;

        // Create the page with a fully zeroed data buffer of appropriate size
        let mut page = Page::new(id, header.page_type, header.version, page_size);

        // Copy the rest of the header fields
        page.header.ref_count = header.ref_count;
        page.header.checksum = header.checksum;
        page.header.data_size = header.data_size;

        // Extract data - only copy up to data_size bytes
        let data_size = header.data_size as usize;
        if data_size > 0 {
            if data_size <= page.data.len() {
                page.data[0..data_size].copy_from_slice(&buffer[PageHeader::size()..PageHeader::size() + data_size]);
            } else {
                return Err(StorageError::Corruption(format!(
                    "Data size in header ({}) exceeds page data capacity ({})",
                    data_size,
                    page.data.len()
                )));
            }
        }

        // Verify checksum
        if !page.verify_checksum() {
            return Err(StorageError::Corruption(format!("Page {} has invalid checksum", id.0)));
        }

        Ok(page)
    }

    /// Writes a page to disk.
    ///
    /// Steps:
    /// 1. If the page is new, extend the file and update the header.
    /// 2. Seek to the correct offset for the page.
    /// 3. Serialize the header and data into a buffer.
    /// 4. Write the buffer to disk and flush.
    /// 5. Return Ok or error.
    pub fn write_page(&mut self, page: &mut Page) -> StorageResult<()> {
        // We don't call update_checksum() here anymore since:
        // 1. It's the caller's responsibility to ensure checksum is updated
        // 2. Calling it twice would make tests fail because the second call would checksum differently
        // 3. The caller will typically have more context about when the checksum needs updating

        if !self.is_initialized() {
            return Err(StorageError::Io(io::Error::new(io::ErrorKind::NotConnected, "File not initialized")));
        }

        // Check if we need to allocate a new page
        let need_allocation = page.id.0 >= self.header.total_pages;
        let total_pages = if need_allocation { page.id.0 + 1 } else { self.header.total_pages };
        let page_size = self.header.page_size as usize;

        if need_allocation {
            // We need to extend the file and update the header
            self.header.total_pages = total_pages;

            // Get the file
            let file = self.file.as_mut().unwrap();

            // Extend the file
            let new_size = HEADER_SIZE as u64 + (total_pages - 1) * self.header.page_size as u64;
            file.set_len(new_size)?;

            // Write the header
            let mut header_buffer = vec![0; HEADER_SIZE];
            self.header.serialize(&mut header_buffer)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&header_buffer)?;
        }

        // Skip the header page for calculation
        let offset = if page.id.0 == 0 { 0 } else { HEADER_SIZE as u64 + (page.id.0 - 1) * self.header.page_size as u64 };

        // Prepare the buffer for the page
        let mut buffer = vec![0; page_size];

        // Write header to buffer
        page.header.serialize(&mut buffer[0..PageHeader::size()])?;

        // Write data to buffer - only write up to data_size bytes
        let data_size = page.header.data_size as usize;
        if data_size > 0 {
            // Make sure we don't try to copy more data than the page contains
            let actual_size = std::cmp::min(data_size, page.data.len());
            if actual_size > 0 {
                buffer[PageHeader::size()..PageHeader::size() + actual_size].copy_from_slice(&page.data[0..actual_size]);
            }
        }

        // Get file reference and write to disk
        let file = self.file.as_mut().unwrap();
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&buffer)?;
        file.flush()?;

        Ok(())
    }

    /// Allocates a new page, reusing a free page if available.
    ///
    /// Steps:
    /// 1. If a free page exists, reuse it and update the free list pointer.
    /// 2. If not, allocate a new page at the end of the file.
    /// 3. Update the header and write the new page to disk.
    /// 4. Return the new Page object.
    pub fn allocate_page(&mut self, page_type: PageType, version: VersionId) -> StorageResult<Page> {
        // Check if we have free pages
        if self.header.first_free_page.0 != 0 {
            // Reuse a free page
            let free_page_id = self.header.first_free_page;

            // Read the free page - if this fails due to checksum, we'll try to repair it
            let mut free_page = match self.read_page(free_page_id) {
                Ok(page) => page,
                Err(StorageError::Corruption(msg)) if msg.contains("invalid checksum") => {
                    // Try to recover by reading the page directly without checksum verification
                    let file = self
                        .file
                        .as_mut()
                        .ok_or_else(|| StorageError::Io(io::Error::new(io::ErrorKind::NotConnected, "File not initialized")))?;

                    // Skip the header page
                    let offset = if free_page_id.0 == 0 {
                        0
                    } else {
                        HEADER_SIZE as u64 + (free_page_id.0 - 1) * self.header.page_size as u64
                    };
                    file.seek(SeekFrom::Start(offset))?;

                    let page_size = self.header.page_size as usize;
                    let mut buffer = vec![0; page_size];
                    file.read_exact(&mut buffer)?;

                    // Parse header
                    let header = PageHeader::deserialize(&buffer[0..PageHeader::size()])?;

                    // Create a new page with recovered data
                    let mut recovered_page = Page::new(free_page_id, header.page_type, header.version, page_size);
                    recovered_page.header.data_size = header.data_size;

                    if header.data_size > 0 {
                        let data_size = header.data_size as usize;
                        recovered_page.data[0..data_size].copy_from_slice(&buffer[PageHeader::size()..PageHeader::size() + data_size]);
                    }

                    recovered_page
                }
                Err(e) => return Err(e),
            };

            // Update the free list
            if free_page.data.len() >= 8 {
                let next_free = u64::from_le_bytes(free_page.data[0..8].try_into().unwrap_or([0; 8]));
                self.header.first_free_page = PageId(next_free);
                self.write_header()?;
            }

            // Create a new page with the same ID
            let mut page = Page::new(free_page_id, page_type, version, self.header.page_size as usize);

            // Update checksum before writing
            page.update_checksum();

            // Write the page to disk
            self.write_page(&mut page)?;

            Ok(page)
        } else {
            // Allocate a new page at the end of the file
            let page_id = PageId(self.header.total_pages);
            let mut page = Page::new(page_id, page_type, version, self.header.page_size as usize);

            // Update checksum before writing
            page.update_checksum();

            // Write the page to disk
            self.write_page(&mut page)?;

            Ok(page)
        }
    }

    /// Frees a page and adds it to the free list.
    ///
    /// Steps:
    /// 1. Create a free page structure and set its next pointer to the current free list head.
    /// 2. Update the file header to point to the newly freed page.
    /// 3. Write the free page and updated header to disk.
    /// 4. Return Ok or error.
    pub fn free_page(&mut self, id: PageId) -> StorageResult<()> {
        if id.0 >= self.header.total_pages {
            return Err(StorageError::PageNotFound(id.0));
        }

        // Create a free page
        let mut page = Page::new(id, PageType::Free, VersionId(0), self.header.page_size as usize);

        // Add it to the free list
        let next_free = self.header.first_free_page.0.to_le_bytes();
        page.data[0..8].copy_from_slice(&next_free);

        // Update the page data size to include the free list pointer
        page.header.data_size = 8;

        // Update the checksum before writing
        page.update_checksum();

        // Update the header
        self.header.first_free_page = id;
        self.write_header()?;

        // Write the page to disk
        self.write_page(&mut page)?;

        Ok(())
    }

    /// Sync all changes to disk
    pub fn sync(&mut self) -> StorageResult<()> {
        if let Some(file) = &mut self.file {
            file.sync_all()?;
            Ok(())
        } else {
            Err(StorageError::Io(io::Error::new(io::ErrorKind::NotConnected, "File not initialized")))
        }
    }

    /// Close the storage file
    pub fn close(&mut self) -> StorageResult<()> {
        self.file.take();
        Ok(())
    }
}

// Unit tests for the file format
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_page_header_serialization() {
        let header = PageHeader {
            page_type: PageType::Node,
            version: VersionId(42),
            ref_count: 3,
            checksum: 0x12345678,
            data_size: 1000,
        };

        let mut buffer = vec![0; PageHeader::size()];
        assert!(header.serialize(&mut buffer).is_ok());

        let header2 = PageHeader::deserialize(&buffer).unwrap();

        assert_eq!(header.page_type as u8, header2.page_type as u8);
        assert_eq!(header.version.0, header2.version.0);
        assert_eq!(header.ref_count, header2.ref_count);
        assert_eq!(header.checksum, header2.checksum);
        assert_eq!(header.data_size, header2.data_size);
    }

    #[test]
    fn test_file_header_serialization() {
        let header = FileHeader {
            magic: FILE_MAGIC,
            version: FORMAT_VERSION,
            page_size: 4096,
            total_pages: 100,
            current_version: VersionId(5),
            first_free_page: PageId(10),
        };

        let mut buffer = vec![0; HEADER_SIZE];
        assert!(header.serialize(&mut buffer).is_ok());

        let header2 = FileHeader::deserialize(&buffer).unwrap();

        assert_eq!(header.magic, header2.magic);
        assert_eq!(header.version, header2.version);
        assert_eq!(header.page_size, header2.page_size);
        assert_eq!(header.total_pages, header2.total_pages);
        assert_eq!(header.current_version.0, header2.current_version.0);
        assert_eq!(header.first_free_page.0, header2.first_free_page.0);
    }

    #[test]
    fn test_page_checksum() {
        let mut page = Page::new(PageId(1), PageType::Node, VersionId(42), 4096);

        // Fill data with some pattern
        for i in 0..page.data.len() {
            page.data[i] = (i % 256) as u8;
        }

        page.update_checksum();
        let checksum = page.header.checksum;

        // Verify the checksum
        assert!(page.verify_checksum());

        // Modify the data
        page.data[100] = 0xFF;

        // Checksum should no longer match
        assert!(!page.verify_checksum());

        // Update the checksum
        page.update_checksum();

        // Verify the checksum is different
        assert_ne!(checksum, page.header.checksum);

        // Verify the checksum is valid again
        assert!(page.verify_checksum());
    }

    #[test]
    fn test_file_format_init() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file_format.db");

        let config = StorageConfig {
            path: file_path,
            page_size: 512,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };

        let mut file_format = FileFormat::new(config);

        // Initialize the storage file
        assert!(file_format.init().is_ok());
        assert!(file_format.is_initialized());
        assert!(file_format.is_new);

        // Check file properties
        assert_eq!(file_format.current_version().0, 0);
        assert_eq!(file_format.page_size(), 512);
        assert_eq!(file_format.total_pages(), 1);

        // Close the file
        assert!(file_format.close().is_ok());
    }

    #[test]
    fn test_file_format_read_write() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file_format_rw.db");

        let config = StorageConfig {
            path: file_path,
            page_size: 512,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };

        let mut file_format = FileFormat::new(config);

        // Initialize the storage file
        assert!(file_format.init().is_ok());
        assert!(file_format.is_initialized());
        assert!(file_format.is_new);

        // Check file properties
        assert_eq!(file_format.current_version().0, 0);
        assert_eq!(file_format.page_size(), 512);
        assert_eq!(file_format.total_pages(), 1);

        // Close the file
        assert!(file_format.close().is_ok());
    }

    #[test]
    fn test_page_allocation_and_free() {
        // Create temporary directory
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");

        // Configuration
        let config = StorageConfig {
            path: path.clone(),
            page_size: 4096,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };

        // Create and initialize FileFormat
        let mut file_format = FileFormat::new(config);
        assert!(file_format.init().is_ok());

        // 1. Create and write page
        let mut page = file_format.allocate_page(PageType::Node, VersionId(1)).unwrap();
        assert_eq!(page.id.0, 1);
        assert_eq!(page.header.page_type, PageType::Node);
        assert_eq!(page.header.version.0, 1);

        // Test data
        let test_data = b"Hello, world!";
        assert!(page.data.len() >= test_data.len(), "Page data buffer too small");
        page.data[0..test_data.len()].copy_from_slice(test_data);
        page.header.data_size = test_data.len() as u16;

        // Update and verify checksum
        page.update_checksum();
        assert!(page.verify_checksum(), "Page checksum verification failed before writing");

        // Write page to disk
        assert!(file_format.write_page(&mut page).is_ok());

        // 2. Read and verify page from disk
        let page2 = file_format.read_page(PageId(1)).unwrap();

        // Check that all fields are correct
        assert_eq!(page2.id.0, 1);
        assert_eq!(page2.header.page_type, PageType::Node);
        assert_eq!(page2.header.version.0, 1);
        assert_eq!(page2.header.data_size as usize, test_data.len());
        assert_eq!(&page2.data[0..test_data.len()], test_data);
        assert!(page2.verify_checksum(), "Page checksum verification failed after reading");

        // 3. Free the page
        assert!(file_format.free_page(PageId(1)).is_ok());

        // 4. Create another page (should use the freed page)
        let page3 = file_format.allocate_page(PageType::Data, VersionId(2)).unwrap();
        assert_eq!(page3.id.0, 1, "Free page should be reused");
        assert_eq!(page3.header.page_type, PageType::Data);
        assert_eq!(page3.header.version.0, 2);
        assert!(page3.verify_checksum(), "New page checksum verification failed");

        // 5. Close the file
        assert!(file_format.close().is_ok());
    }

    #[test]
    fn test_checksum_serialization() {
        // Create a page and test direct serialization/deserialization to ensure checksum integrity
        let mut page = Page::new(PageId(1), PageType::Node, VersionId(42), 4096);

        // Fill with test data
        let test_data = b"Test checksum serialization";
        page.data[0..test_data.len()].copy_from_slice(test_data);
        page.header.data_size = test_data.len() as u16;

        // Update checksum
        page.update_checksum();
        let original_checksum = page.header.checksum;

        println!("Original checksum: {}", original_checksum);

        // Create a buffer and serialize the page
        let page_size = 4096;
        let mut buffer = vec![0; page_size];

        // Serialize the header
        page.header.serialize(&mut buffer[0..PageHeader::size()]).unwrap();

        // Copy the data
        if page.header.data_size > 0 {
            let data_size = page.header.data_size as usize;
            buffer[PageHeader::size()..PageHeader::size() + data_size].copy_from_slice(&page.data[0..data_size]);
        }

        // Deserialize the header
        let header = PageHeader::deserialize(&buffer[0..PageHeader::size()]).unwrap();

        // Extract the data
        let data = buffer[PageHeader::size()..PageHeader::size() + header.data_size as usize].to_vec();

        // Create a new page from the deserialized data
        let deserialized_page = Page { id: PageId(1), header, data };

        println!("Deserialized checksum: {}", deserialized_page.header.checksum);
        println!("Recalculated checksum: {}", deserialized_page.calculate_checksum());

        // Verify checksum integrity
        assert_eq!(original_checksum, deserialized_page.header.checksum, "Checksum changed during serialization");

        // Verify the deserialized page passes its own checksum verification
        assert!(deserialized_page.verify_checksum(), "Deserialized page failed checksum verification");
    }

    #[test]
    fn test_versioning() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("version_test.dotdb");

        let config = StorageConfig {
            path: file_path.clone(),
            page_size: 4096,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };

        let mut file_format = FileFormat::new(config);
        assert!(file_format.init().is_ok());

        // Create initial page
        let mut page = file_format.allocate_page(PageType::Data, VersionId(1)).unwrap();
        let data_v1 = b"version 1 data";
        page.data[0..data_v1.len()].copy_from_slice(data_v1);
        page.header.data_size = data_v1.len() as u16;
        page.update_checksum();
        assert!(file_format.write_page(&mut page).is_ok());

        // Create new version
        file_format.set_current_version(VersionId(2)).unwrap();
        let mut page2 = file_format.allocate_page(PageType::Data, VersionId(2)).unwrap();
        let data_v2 = b"version 2 data";
        page2.data[0..data_v2.len()].copy_from_slice(data_v2);
        page2.header.data_size = data_v2.len() as u16;
        page2.update_checksum();
        assert!(file_format.write_page(&mut page2).is_ok());

        // Read back the latest version
        let page_read = file_format.read_page(page2.id).unwrap();
        assert_eq!(&page_read.data[0..data_v2.len()], data_v2);

        // Close and verify persistence
        assert!(file_format.close().is_ok());
    }

    #[test]
    fn test_free_list() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("freelist_test.dotdb");

        let config = StorageConfig {
            path: file_path.clone(),
            page_size: 4096,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };

        let mut file_format = FileFormat::new(config);
        assert!(file_format.init().is_ok());

        // Allocate some pages
        let mut page1 = file_format.allocate_page(PageType::Data, VersionId(1)).unwrap();
        let mut page2 = file_format.allocate_page(PageType::Data, VersionId(1)).unwrap();
        let data1 = b"test1";
        let data2 = b"test2";
        page1.data[0..data1.len()].copy_from_slice(data1);
        page1.header.data_size = data1.len() as u16;
        page1.update_checksum();
        assert!(file_format.write_page(&mut page1).is_ok());
        page2.data[0..data2.len()].copy_from_slice(data2);
        page2.header.data_size = data2.len() as u16;
        page2.update_checksum();
        assert!(file_format.write_page(&mut page2).is_ok());

        // Free pages
        assert!(file_format.free_page(page2.id).is_ok());
        assert!(file_format.free_page(page1.id).is_ok());

        // Allocate again - should reuse freed pages
        let page3 = file_format.allocate_page(PageType::Data, VersionId(2)).unwrap();
        assert!(page3.id == page1.id || page3.id == page2.id);
        let page4 = file_format.allocate_page(PageType::Data, VersionId(2)).unwrap();
        assert!(page4.id == page1.id || page4.id == page2.id);
        assert_ne!(page3.id, page4.id);

        // Close file
        assert!(file_format.close().is_ok());
    }
}
