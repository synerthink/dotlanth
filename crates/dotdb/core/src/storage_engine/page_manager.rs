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

// Page management module
// This module is responsible for allocating, freeing, and tracking pages in the storage system. It manages free lists, page versions, and supports versioning and compaction.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};

use crate::storage_engine::file_format::{FileFormat, Page, PageId, PageType};
use crate::storage_engine::lib::StorageConfig;
use crate::storage_engine::lib::{Initializable, StorageError, StorageResult, VersionId};
use tempfile::tempdir;

/// Result of a page allocation operation
#[derive(Debug, Clone)]
pub struct PageAllocation {
    /// The allocated page ID
    pub page_id: PageId,
    /// The type of page allocated
    pub page_type: PageType,
    /// Whether this page was newly allocated (true) or reused (false)
    pub is_new: bool,
    /// The version of this allocation
    pub version: VersionId,
}

/// PageManager handles allocation, deallocation, and version tracking of pages, including free list management and version cleanup.
pub struct PageManager {
    /// The file format manager
    file_format: Arc<Mutex<FileFormat>>,
    /// The current working version
    current_version: VersionId,
    /// Free pages by page type
    free_pages: HashMap<PageType, VecDeque<PageId>>,
    /// Recently allocated pages
    allocated_pages: HashSet<PageId>,
    /// Recently freed pages (not yet persisted)
    recently_freed: HashMap<PageType, Vec<PageId>>,
    /// Maximum number of pages to keep in the free list per type
    max_free_list_size: usize,
    /// Whether the page manager is initialized
    initialized: bool,
    /// Maps page IDs to their versions
    page_versions: HashMap<PageId, Vec<VersionId>>,
    /// Maximum number of versions to keep
    max_versions: u32,
}

impl PageManager {
    /// Create a new page manager
    pub fn new(file_format: Arc<Mutex<FileFormat>>) -> Self {
        Self {
            file_format,
            current_version: VersionId(1),
            free_pages: HashMap::new(),
            allocated_pages: HashSet::new(),
            recently_freed: HashMap::new(),
            max_free_list_size: 1000,
            initialized: false,
            page_versions: HashMap::new(),
            max_versions: 10,
        }
    }

    /// Set the current working version
    pub fn set_version(&mut self, version: VersionId) -> StorageResult<()> {
        self.current_version = version;

        // Update the file format's version as well
        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

        file_format.set_current_version(version)?;
        Ok(())
    }

    /// Get the current working version
    pub fn current_version(&self) -> VersionId {
        self.current_version
    }

    /// Clear the free page cache
    pub fn clear_free_pages(&mut self) {
        self.free_pages.clear();
        self.recently_freed.clear();
    }

    /// Allocates a page of the specified type.
    ///
    /// Steps:
    /// 1. Check if a free page of the requested type is available in the free list.
    ///    a. If yes, reuse it and update tracking structures.
    ///    b. If not, allocate a new page from the file format.
    /// 2. Track the allocation in allocated_pages and page_versions.
    /// 3. Return the allocation result.
    pub fn allocate_page(&mut self, page_type: PageType) -> StorageResult<PageAllocation> {
        // Check if we have any free pages of this type
        if let Some(free_list) = self.free_pages.get_mut(&page_type) {
            if let Some(page_id) = free_list.pop_front() {
                // Found a free page of the right type
                let allocation = PageAllocation {
                    page_id,
                    page_type,
                    is_new: false,
                    version: self.current_version,
                };

                // Track this allocation
                self.allocated_pages.insert(page_id);
                self.page_versions.entry(page_id).or_insert_with(Vec::new).push(self.current_version);

                return Ok(allocation);
            }
        }

        // No free pages of the requested type, allocate a new one
        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

        let page = file_format.allocate_page(page_type, self.current_version)?;

        let allocation = PageAllocation {
            page_id: page.id,
            page_type,
            is_new: true,
            version: self.current_version,
        };

        // Track this allocation
        self.allocated_pages.insert(page.id);
        self.page_versions.entry(page.id).or_insert_with(Vec::new).push(self.current_version);

        Ok(allocation)
    }

    /// Frees a page.
    ///
    /// Steps:
    /// 1. If the page was recently allocated, add it directly to the free list for immediate reuse.
    /// 2. Otherwise, add it to the recently_freed list for later batch processing.
    /// 3. If enough pages accumulate in recently_freed, process them in batch.
    /// 4. Used to optimize free page management and reduce I/O.
    pub fn free_page(&mut self, page_id: PageId, page_type: PageType) -> StorageResult<()> {
        // Check if this page was recently allocated
        if self.allocated_pages.remove(&page_id) {
            // Page was allocated in this session, can be reused immediately
            self.add_to_free_list(page_type, page_id);
            return Ok(());
        }

        // Add to the recently freed list for later processing
        let freed_list = self.recently_freed.entry(page_type).or_insert_with(Vec::new);

        freed_list.push(page_id);

        // Process freed pages if we have accumulated enough
        if freed_list.len() >= 100 {
            self.process_freed_pages(page_type)?;
        }

        Ok(())
    }

    /// Processes recently freed pages in batches.
    ///
    /// Steps:
    /// 1. Take all pages of the given type from the recently_freed list.
    /// 2. For each page, mark it as free in the file format (may require I/O).
    /// 3. After processing, add the pages to the free list for future allocation.
    /// 4. Used to amortize the cost of freeing pages and avoid frequent disk writes.
    fn process_freed_pages(&mut self, page_type: PageType) -> StorageResult<()> {
        // First, check if there are any pages to process
        let pages_to_process = match self.recently_freed.get_mut(&page_type) {
            Some(freed_list) if !freed_list.is_empty() => {
                // Take the pages out of the list, leaving an empty list
                let pages = std::mem::take(freed_list);
                Some(pages)
            }
            _ => None,
        };

        // If there are no pages to process, return early
        let pages_to_process = match pages_to_process {
            Some(pages) => pages,
            None => return Ok(()),
        };

        // Process pages in batches
        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

        for &page_id in &pages_to_process {
            // Mark as free in the file format
            file_format.free_page(page_id)?;
        }

        // Release the lock before adding to free list
        drop(file_format);

        // Add processed pages to the free list
        for page_id in pages_to_process {
            self.add_to_free_list(page_type, page_id);
        }

        Ok(())
    }

    /// Add a page to the free list if it's not full
    fn add_to_free_list(&mut self, page_type: PageType, page_id: PageId) {
        let free_list = self.free_pages.entry(page_type).or_insert_with(VecDeque::new);

        if free_list.len() < self.max_free_list_size {
            free_list.push_back(page_id);
        }
    }

    /// Process all freed pages
    pub fn process_all_freed_pages(&mut self) -> StorageResult<()> {
        // Get all page types with freed pages
        let page_types: Vec<PageType> = self.recently_freed.keys().cloned().collect();

        // Process each type
        for page_type in page_types {
            self.process_freed_pages(page_type)?;
        }

        Ok(())
    }

    /// Get the number of free pages by type
    pub fn free_pages_count(&self) -> HashMap<PageType, usize> {
        let mut counts = HashMap::new();

        for (page_type, free_list) in &self.free_pages {
            counts.insert(*page_type, free_list.len());
        }

        counts
    }

    /// Get the number of recently freed pages by type
    pub fn recently_freed_count(&self) -> HashMap<PageType, usize> {
        let mut counts = HashMap::new();

        for (page_type, freed_list) in &self.recently_freed {
            counts.insert(*page_type, freed_list.len());
        }

        counts
    }

    /// Scan for free pages in the storage file
    pub fn scan_for_free_pages(&mut self) -> StorageResult<usize> {
        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

        let total_pages = file_format.total_pages();
        let mut free_count = 0;
        let mut free_pages = Vec::new();

        // Skip page ID 0 which is the header
        for page_id in 1..total_pages {
            let page = file_format.read_page(PageId(page_id))?;

            if page.header.page_type == PageType::Free {
                // Collect free pages to add after the loop
                free_pages.push(PageId(page_id));
                free_count += 1;
            }
        }

        // Release the lock on file_format before modifying self
        drop(file_format);

        // Now add all free pages to our free list
        for page_id in free_pages {
            self.add_to_free_list(PageType::Free, page_id);
        }

        Ok(free_count)
    }

    /// Get a specific version of a page
    pub fn get_page_version(&self, page_id: PageId, version: VersionId) -> StorageResult<Option<VersionId>> {
        if let Some(versions) = self.page_versions.get(&page_id) {
            // Find the closest version less than or equal to requested version
            let mut suitable_version = None;

            for &v in versions.iter().rev() {
                if v <= version {
                    suitable_version = Some(v);
                    break;
                }
            }

            Ok(suitable_version)
        } else {
            Ok(None)
        }
    }

    /// Cleans up old page versions based on the max_versions policy.
    ///
    /// Steps:
    /// 1. Determine the minimum version ID to keep.
    /// 2. For each page, remove versions older than this threshold.
    /// 3. Update the page_versions map, removing empty entries.
    /// 4. Used to limit memory usage and support versioned storage.
    pub fn cleanup_old_versions(&mut self) -> StorageResult<()> {
        if self.max_versions == 0 {
            return Ok(());
        }

        let min_version_id_to_keep = if self.current_version.0 >= self.max_versions as u64 {
            VersionId(self.current_version.0 - self.max_versions as u64 + 1)
        } else {
            VersionId(1) // Keep versions from ID 1 up to current_version
        };

        // Remove old versions from tracking
        self.page_versions.retain(|_, versions_list| {
            versions_list.retain(|v| v.0 >= min_version_id_to_keep.0);
            !versions_list.is_empty()
        });

        Ok(())
    }

    /// Start a new version
    pub fn start_new_version(&mut self) -> StorageResult<VersionId> {
        let new_version = VersionId(self.current_version.0 + 1);
        self.current_version = new_version;
        self.cleanup_old_versions()?;
        Ok(new_version)
    }

    /// Get the latest version of a page
    pub fn get_latest_version(&self, page_id: PageId) -> StorageResult<Option<VersionId>> {
        if let Some(versions) = self.page_versions.get(&page_id) {
            Ok(versions.last().copied())
        } else {
            Ok(None)
        }
    }

    /// Prefetch a page of the specified type
    pub fn prefetch_page(&mut self, page_type: PageType) -> StorageResult<PageAllocation> {
        self.allocate_page(page_type)
    }

    /// Compacts page versions by removing all but the latest version for each page.
    ///
    /// Steps:
    /// 1. Identify pages with multiple versions.
    /// 2. For each, retain only the most recent version and logically delete old versions.
    /// 3. Update page_versions map accordingly.
    /// 4. Returns the number of compacted (removed) versions.
    pub fn compact(&mut self) -> StorageResult<usize> {
        let mut compacted = 0;
        let candidates: Vec<(PageId, Vec<VersionId>)> = self
            .page_versions
            .iter()
            .filter(|(_, versions)| versions.len() > 1)
            .map(|(&id, versions)| (id, versions.clone()))
            .collect();

        for (page_id, versions) in candidates {
            if let Some(&latest_version) = versions.iter().max() {
                // Add all old versions to the free list
                // Note: Physical free is not performed here because versions share the same page_id.
                // If they had physically different page_ids, free_page could be called.
                // Still, we are logically deleting old versions.
                // Leave only the most recent version
                for &old_version in versions.iter().filter(|&&v| v != latest_version) {
                    compacted += 1;
                }
                // Sadece en güncel versiyonu bırak
                self.page_versions.insert(page_id, vec![latest_version]);
            }
        }
        Ok(compacted)
    }
}

impl Initializable for PageManager {
    fn init(&mut self) -> StorageResult<()> {
        // Initialize by scanning for free pages
        self.scan_for_free_pages()?;

        // Don't update the version from file format, keep our initial version
        self.initialized = true;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// ConcurrentPageManager provides a thread-safe wrapper around PageManager for concurrent operations.
pub struct ConcurrentPageManager {
    /// The inner page manager
    inner: Arc<RwLock<PageManager>>,
}

impl ConcurrentPageManager {
    /// Create a new concurrent page manager
    pub fn new(file_format: Arc<Mutex<FileFormat>>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PageManager::new(file_format))),
        }
    }

    /// Initialize the page manager
    pub fn init(&self) -> StorageResult<()> {
        let mut inner = self.inner.write().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        inner.init()
    }

    /// Set the current working version
    pub fn set_version(&self, version: VersionId) -> StorageResult<()> {
        let mut inner = self.inner.write().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        inner.set_version(version)
    }

    /// Get the current working version
    pub fn current_version(&self) -> StorageResult<VersionId> {
        let inner = self.inner.read().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        Ok(inner.current_version())
    }

    /// Allocate a page of the specified type
    pub fn allocate_page(&self, page_type: PageType) -> StorageResult<PageAllocation> {
        let mut inner = self.inner.write().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        inner.allocate_page(page_type)
    }

    /// Free a page
    pub fn free_page(&self, page_id: PageId, page_type: PageType) -> StorageResult<()> {
        let mut inner = self.inner.write().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        inner.free_page(page_id, page_type)
    }

    /// Process all freed pages
    pub fn process_all_freed_pages(&self) -> StorageResult<()> {
        let mut inner = self.inner.write().map_err(|_| StorageError::Corruption("Failed to lock page manager".to_string()))?;

        inner.process_all_freed_pages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_engine::file_format::FileFormat;
    use crate::storage_engine::lib::StorageConfig;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    fn create_test_file_format() -> Arc<Mutex<FileFormat>> {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            path: file_path,
            page_size: 4096,
            buffer_pool_size: 100,
            direct_io: false,
            wal_size: 1024 * 1024,
            flush_interval_ms: 100,
            max_dirty_pages: 10,
            writer_threads: 1,
        };
        let mut file_format = FileFormat::new(config);
        file_format.init().unwrap();
        Arc::new(Mutex::new(file_format))
    }

    #[test]
    fn test_page_manager_init() {
        let file_format = create_test_file_format();
        let mut page_manager = PageManager::new(file_format);
        assert!(!page_manager.initialized);
        page_manager.init().unwrap();
        assert!(page_manager.initialized);
    }

    #[test]
    fn test_page_allocation_and_free() {
        let file_format = create_test_file_format();
        let mut page_manager = PageManager::new(file_format);
        page_manager.init().unwrap();

        // Allocate a page
        let allocation = page_manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(allocation.page_type, PageType::Data);
        assert!(allocation.is_new);
        assert_eq!(allocation.version, page_manager.current_version);

        // Free the page
        page_manager.free_page(allocation.page_id, PageType::Data).unwrap();

        // Allocate again - should reuse the freed page
        let allocation2 = page_manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(allocation2.page_id, allocation.page_id);
        assert!(!allocation2.is_new);
    }

    #[test]
    fn test_versioning() {
        let file_format = create_test_file_format();
        let mut page_manager = PageManager::new(file_format);
        page_manager.init().unwrap();

        // Allocate a page in version 1
        let allocation1 = page_manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(allocation1.version, VersionId(1));

        // Start a new version
        let version2 = page_manager.start_new_version().unwrap();
        assert_eq!(version2, VersionId(2));

        // Allocate another page in version 2
        let allocation2 = page_manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(allocation2.version, VersionId(2));

        // Get page versions
        let page1_version = page_manager.get_page_version(allocation1.page_id, VersionId(1)).unwrap();
        assert_eq!(page1_version, Some(VersionId(1)));

        let page2_version = page_manager.get_page_version(allocation2.page_id, VersionId(2)).unwrap();
        assert_eq!(page2_version, Some(VersionId(2)));

        // Test version cleanup
        page_manager.max_versions = 1;
        page_manager.cleanup_old_versions().unwrap();

        let page1_version_after_cleanup = page_manager.get_page_version(allocation1.page_id, VersionId(1)).unwrap();
        assert_eq!(page1_version_after_cleanup, None);
    }

    #[test]
    fn test_concurrent_page_manager() {
        let file_format = create_test_file_format();
        let page_manager = ConcurrentPageManager::new(file_format);
        page_manager.init().unwrap();

        // Test concurrent operations
        let version = page_manager.current_version().unwrap();
        assert_eq!(version, VersionId(1));

        let allocation = page_manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(allocation.page_type, PageType::Data);
        assert!(allocation.is_new);

        page_manager.free_page(allocation.page_id, PageType::Data).unwrap();
    }

    #[test]
    fn test_compaction() {
        let file_format = create_test_file_format();
        let mut page_manager = PageManager::new(file_format);
        page_manager.init().unwrap();

        // Version 1: Allocate a page
        let alloc1 = page_manager.allocate_page(PageType::Data).unwrap();

        // Version 2: Start a new version, allocate a new page
        page_manager.start_new_version().unwrap();
        let alloc2 = page_manager.allocate_page(PageType::Data).unwrap();

        // Version 3: Start a new version, free the first page
        page_manager.start_new_version().unwrap();
        page_manager.free_page(alloc1.page_id, PageType::Data).unwrap();

        // Version 4: Start a new version
        page_manager.start_new_version().unwrap();
        // Run cleanup
        page_manager.max_versions = 1;
        page_manager.cleanup_old_versions().unwrap();

        // Only the most recent version should be kept, old pages and versions should be deleted
        let alloc1_version = page_manager.get_page_version(alloc1.page_id, alloc1.version).unwrap();
        assert_eq!(alloc1_version, None);

        let alloc2_version = page_manager.get_page_version(alloc2.page_id, alloc2.version).unwrap();
        assert_eq!(alloc2_version, None);
    }

    #[test]
    fn test_prefetch() {
        let file_format = create_test_file_format();
        let mut page_manager = PageManager::new(file_format);
        page_manager.init().unwrap();

        // Prefetch a page
        let allocation = page_manager.prefetch_page(PageType::Data).unwrap();
        assert_eq!(allocation.page_type, PageType::Data);
        assert!(allocation.is_new);
    }
}
