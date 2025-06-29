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

// Buffer management module
// This module provides in-memory caching of pages, coordinates I/O operations, and implements buffer replacement policies. It manages the buffer pool, page pinning, flushing, and background writing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread;
use std::time::{Duration, Instant};

use crate::storage_engine::file_format::{FileFormat, Page, PageId, PageType};
use crate::storage_engine::lib::{Flushable, Initializable, StorageError, StorageResult, VersionId, generate_timestamp};

/// Buffer pool statistics
#[derive(Debug)]
pub struct BufferStats {
    /// Number of reads
    pub reads: AtomicU64,
    /// Number of writes
    pub writes: AtomicU64,
    /// Number of hits
    pub hits: AtomicU64,
    /// Number of misses
    pub misses: AtomicU64,
    /// Number of evictions
    pub evictions: AtomicU64,
}

impl Default for BufferStats {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferStats {
    pub fn new() -> Self {
        Self {
            reads: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    pub fn inc_reads(&self) {
        self.reads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_writes(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_hits(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_misses(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_evictions(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let total = hits + self.misses.load(Ordering::Relaxed);
        if total == 0 { 0.0 } else { hits as f64 / total as f64 }
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, f64) {
        (
            self.reads.load(Ordering::Relaxed),
            self.writes.load(Ordering::Relaxed),
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
            self.get_hit_ratio(),
        )
    }
}

/// Replacement policy enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplacementPolicy {
    /// Least recently used
    LRU,
    /// Clock-sweep (approximated LRU)
    Clock,
    /// Most recently used
    MRU,
    /// First in, first out
    FIFO,
}

/// Represents a buffer that holds a cached page in memory
#[derive(Debug)]
pub struct Buffer {
    /// The cached page
    page: Page,
    /// Last time this buffer was accessed
    last_accessed: Instant,
    /// Whether the buffer is dirty (modified)
    is_dirty: bool,
    /// Whether the buffer is pinned (cannot be evicted)
    pin_count: usize,
    /// Clock bit for Clock replacement algorithm
    clock_bit: AtomicBool,
    /// Insertion timestamp for FIFO policy
    insertion_time: u64,
}

impl Buffer {
    /// Create a new buffer
    fn new(page: Page) -> Self {
        Self {
            page,
            last_accessed: Instant::now(),
            is_dirty: false,
            pin_count: 0,
            clock_bit: AtomicBool::new(false),
            insertion_time: generate_timestamp(),
        }
    }

    /// Get a reference to the page
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Get a mutable reference to the page
    pub fn page_mut(&mut self) -> &mut Page {
        self.is_dirty = true;
        &mut self.page
    }

    /// Mark the buffer as accessed
    fn mark_accessed(&mut self) {
        self.last_accessed = Instant::now();
        self.clock_bit.store(true, Ordering::SeqCst);
    }

    /// Reset the clock bit (for Clock replacement policy)
    fn reset_clock_bit(&self) -> bool {
        self.clock_bit.swap(false, Ordering::SeqCst)
    }

    /// Check if the buffer can be evicted
    fn can_evict(&self) -> bool {
        self.pin_count == 0
    }

    /// Pin the buffer (prevent eviction)
    fn pin(&mut self) {
        self.pin_count += 1;
    }

    /// Unpin the buffer
    fn unpin(&mut self) {
        if self.pin_count > 0 {
            self.pin_count -= 1;
        }
    }

    /// Check if the buffer is dirty
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Mark the buffer as clean
    fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Get insertion time (for FIFO policy)
    fn get_insertion_time(&self) -> u64 {
        self.insertion_time
    }
}

/// BufferPool manages a collection of in-memory buffers for pages, handling caching, eviction, and replacement policies.
pub struct BufferPool {
    /// The file format manager
    file_format: Arc<Mutex<FileFormat>>,
    /// Cached pages by ID
    buffers: HashMap<PageId, Buffer>,
    /// LRU queue for eviction
    lru_queue: VecDeque<PageId>,
    /// Maximum number of buffers
    capacity: usize,
    /// Statistics
    stats: BufferStats,
    /// Set of pages that are currently being read/written
    pending_io: HashSet<PageId>,
    /// Replacement policy to use
    policy: ReplacementPolicy,
    /// Clock hand position (for Clock replacement policy)
    clock_hand: usize,
    /// Maximum number of dirty pages before forced flush
    max_dirty_pages: usize,
    /// Background writer thread running flag
    bg_writer_running: AtomicBool,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(file_format: Arc<Mutex<FileFormat>>, config: &crate::storage_engine::lib::StorageConfig) -> Self {
        Self {
            file_format,
            buffers: HashMap::with_capacity(config.buffer_pool_size),
            lru_queue: VecDeque::with_capacity(config.buffer_pool_size),
            capacity: config.buffer_pool_size,
            stats: BufferStats::new(),
            pending_io: HashSet::new(),
            policy: ReplacementPolicy::LRU, // Default to LRU
            clock_hand: 0,
            max_dirty_pages: config.max_dirty_pages,
            bg_writer_running: AtomicBool::new(false),
        }
    }

    /// Set the replacement policy
    pub fn set_policy(&mut self, policy: ReplacementPolicy) {
        self.policy = policy;
    }

    /// Get the number of buffers in the pool
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Check if the buffer pool is empty
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Get the buffer hit rate
    pub fn hit_rate(&self) -> f64 {
        self.stats.get_hit_ratio()
    }

    /// Get a page from the buffer pool, reading from disk if necessary
    pub fn get_page(&mut self, page_id: PageId) -> StorageResult<&Buffer> {
        // Check if the page is already in the buffer pool
        if self.buffers.contains_key(&page_id) {
            // Update LRU position or clock bit based on policy
            match self.policy {
                ReplacementPolicy::LRU => {
                    self.lru_queue.retain(|&id| id != page_id);
                    self.lru_queue.push_back(page_id);
                }
                ReplacementPolicy::Clock => {
                    // Just set the clock bit, no need to update queue
                    if let Some(buffer) = self.buffers.get_mut(&page_id) {
                        buffer.clock_bit.store(true, Ordering::SeqCst);
                    }
                }
                ReplacementPolicy::MRU => {
                    // For MRU, move to front of queue
                    self.lru_queue.retain(|&id| id != page_id);
                    self.lru_queue.push_front(page_id);
                }
                ReplacementPolicy::FIFO => {
                    // In FIFO we don't change the queue position on access
                }
            }

            // Mark as accessed
            if let Some(buffer) = self.buffers.get_mut(&page_id) {
                buffer.mark_accessed();
            }

            self.stats.inc_hits();
            self.stats.inc_reads();

            return Ok(&self.buffers[&page_id]);
        }

        self.stats.inc_misses();
        self.stats.inc_reads();

        // Need to load the page from disk
        if self.pending_io.contains(&page_id) {
            return Err(StorageError::Io(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Page is already being read or written")));
        }

        // Make room if we're at capacity
        if self.buffers.len() >= self.capacity {
            self.evict_one()?;
        }

        // Read the page from disk
        self.pending_io.insert(page_id);
        let page = {
            let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

            file_format.read_page(page_id)
        };
        self.pending_io.remove(&page_id);

        // Add the page to the buffer pool
        let page = page?;
        let buffer = Buffer::new(page);

        self.buffers.insert(page_id, buffer);

        // Add to appropriate data structure based on policy
        match self.policy {
            ReplacementPolicy::LRU | ReplacementPolicy::Clock | ReplacementPolicy::FIFO => {
                self.lru_queue.push_back(page_id);
            }
            ReplacementPolicy::MRU => {
                self.lru_queue.push_front(page_id);
            }
        }

        Ok(&self.buffers[&page_id])
    }

    /// Get a mutable reference to a page buffer
    pub fn get_page_mut(&mut self, page_id: PageId) -> StorageResult<&mut Buffer> {
        // Try to get the page first
        self.get_page(page_id)?;

        // Get a mutable reference
        Ok(self.buffers.get_mut(&page_id).unwrap())
    }

    /// Pin a page in memory
    pub fn pin_page(&mut self, page_id: PageId) -> StorageResult<()> {
        // Try to get the page first
        self.get_page(page_id)?;

        // Pin the page
        if let Some(buffer) = self.buffers.get_mut(&page_id) {
            buffer.pin();
        }

        Ok(())
    }

    /// Unpin a page in memory
    pub fn unpin_page(&mut self, page_id: PageId) -> StorageResult<()> {
        if let Some(buffer) = self.buffers.get_mut(&page_id) {
            buffer.unpin();
            Ok(())
        } else {
            Err(StorageError::PageNotFound(page_id.0))
        }
    }

    /// Flush a dirty page to disk
    pub fn flush_page(&mut self, page_id: PageId) -> StorageResult<()> {
        if let Some(buffer) = self.buffers.get_mut(&page_id) {
            if buffer.is_dirty() {
                // Write the page to disk
                let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

                self.stats.inc_writes();

                self.pending_io.insert(page_id);
                let mut page_copy = buffer.page.clone();
                // Update the checksum
                page_copy.update_checksum();

                let result = file_format.write_page(&mut page_copy);
                self.pending_io.remove(&page_id);

                // Mark the buffer as clean
                if result.is_ok() {
                    buffer.mark_clean();
                }

                result
            } else {
                Ok(())
            }
        } else {
            Err(StorageError::PageNotFound(page_id.0))
        }
    }

    /// Flushes all dirty pages to disk.
    ///
    /// Steps:
    /// 1. Collect all dirty page IDs.
    /// 2. For each dirty page:
    ///    a. Attempt to flush the page to disk.
    ///    b. If successful, increment the counter.
    ///    c. If an error occurs, collect the error.
    /// 3. If any errors occurred, return the first error; otherwise, return the number of flushed pages.
    pub fn flush_all(&mut self) -> StorageResult<usize> {
        let mut count = 0;
        let mut errors = Vec::new();

        // Get a list of all dirty pages
        let dirty_page_ids: Vec<PageId> = self.buffers.iter().filter(|(_, buffer)| buffer.is_dirty()).map(|(&page_id, _)| page_id).collect();

        // Flush each dirty page
        for page_id in dirty_page_ids {
            match self.flush_page(page_id) {
                Ok(()) => count += 1,
                Err(e) => errors.push((page_id, e)),
            }
        }

        // If there were any errors, return the first one
        if let Some((page_id, error)) = errors.first() {
            Err(StorageError::Io(std::io::Error::other(format!("Failed to flush page {}: {:?}", page_id.0, error))))
        } else {
            Ok(count)
        }
    }

    /// Allocates a new page in the file and adds it to the buffer pool.
    ///
    /// Steps:
    /// 1. If at capacity, evict a page first.
    /// 2. Allocate a new page from the file format.
    /// 3. Insert the new page into the buffer pool and update replacement structures.
    /// 4. Return the new page ID.
    pub fn allocate_page(&mut self, page_type: PageType, version: VersionId) -> StorageResult<PageId> {
        // If we're at capacity, evict a page first
        if self.buffers.len() >= self.capacity {
            self.evict_one()?;
        }

        // Now allocate the page
        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

        let page = file_format.allocate_page(page_type, version)?;
        let page_id = page.id;

        // Add the page to the buffer pool
        self.buffers.insert(page_id, Buffer::new(page));

        // Add to appropriate data structure based on policy
        match self.policy {
            ReplacementPolicy::LRU | ReplacementPolicy::Clock | ReplacementPolicy::FIFO => {
                self.lru_queue.push_back(page_id);
            }
            ReplacementPolicy::MRU => {
                self.lru_queue.push_front(page_id);
            }
        }

        Ok(page_id)
    }

    /// Evicts a page from the buffer pool based on the selected policy.
    ///
    /// Steps:
    /// 1. Depending on the replacement policy, call the appropriate eviction method (LRU, Clock, MRU, FIFO).
    /// 2. Each policy attempts to find a non-pinned page to evict, flushes if dirty, and removes it from the pool.
    /// 3. If all pages are pinned, returns BufferPoolFull error.
    pub fn evict_one(&mut self) -> StorageResult<()> {
        match self.policy {
            ReplacementPolicy::LRU => self.evict_lru(),
            ReplacementPolicy::Clock => self.evict_clock(),
            ReplacementPolicy::MRU => self.evict_mru(),
            ReplacementPolicy::FIFO => self.evict_fifo(),
        }
    }

    /// Evict a page using the LRU policy
    fn evict_lru(&mut self) -> StorageResult<()> {
        // Find the least recently used page that is not pinned
        while let Some(page_id) = self.lru_queue.pop_front() {
            if let Some(buffer) = self.buffers.get(&page_id) {
                if buffer.can_evict() {
                    // If the page is dirty, flush it to disk
                    if buffer.is_dirty() {
                        let mut page_copy = buffer.page.clone();
                        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

                        self.stats.inc_writes();
                        file_format.write_page(&mut page_copy)?;
                    }

                    // Remove the page from the buffer pool
                    self.buffers.remove(&page_id);
                    self.stats.inc_evictions();
                    return Ok(());
                } else {
                    // Page is pinned, put it back at the end of the queue
                    self.lru_queue.push_back(page_id);
                }
            }
        }

        // If we get here, all pages are pinned
        Err(StorageError::BufferPoolFull)
    }

    /// Evict a page using the Clock policy
    fn evict_clock(&mut self) -> StorageResult<()> {
        // If there are no pages, nothing to evict
        if self.lru_queue.is_empty() {
            return Ok(());
        }

        // Clock hand algorithm - scan through pages in a circular fashion
        let queue_size = self.lru_queue.len();
        let starting_position = self.clock_hand % queue_size;

        for i in 0..queue_size {
            let check_pos = (starting_position + i) % queue_size;
            let page_id = self.lru_queue[check_pos];

            if let Some(buffer) = self.buffers.get(&page_id)
                && buffer.can_evict()
            {
                // Check if the clock bit is reset
                if !buffer.reset_clock_bit() {
                    // Clock bit is 0, we can evict this page
                    if buffer.is_dirty() {
                        // Flush to disk first
                        let mut page_copy = buffer.page.clone();
                        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

                        self.stats.inc_writes();
                        file_format.write_page(&mut page_copy)?;
                    }

                    // Remove the page from queue and buffers
                    self.lru_queue.remove(check_pos);
                    self.buffers.remove(&page_id);
                    self.stats.inc_evictions();

                    // Update clock hand
                    self.clock_hand = (check_pos + 1) % queue_size;

                    return Ok(());
                }
                // Otherwise, clock bit is 1, so reset it and continue
            }

            // Move clock hand to next position
            self.clock_hand = (check_pos + 1) % queue_size;
        }

        // If we've gone through all pages and couldn't evict any, try LRU as fallback
        self.evict_lru()
    }

    /// Evict a page using the MRU policy
    fn evict_mru(&mut self) -> StorageResult<()> {
        // Find the most recently used page that is not pinned
        while let Some(page_id) = self.lru_queue.pop_front() {
            if let Some(buffer) = self.buffers.get(&page_id) {
                if buffer.can_evict() {
                    // If the page is dirty, flush it to disk
                    if buffer.is_dirty() {
                        let mut page_copy = buffer.page.clone();
                        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

                        self.stats.inc_writes();
                        file_format.write_page(&mut page_copy)?;
                    }

                    // Remove the page from the buffer pool
                    self.buffers.remove(&page_id);
                    self.stats.inc_evictions();
                    return Ok(());
                } else {
                    // Page is pinned, put it back at the back of the queue
                    self.lru_queue.push_back(page_id);
                }
            }
        }

        // If we get here, all pages are pinned
        Err(StorageError::BufferPoolFull)
    }

    /// Evict a page using the FIFO policy
    fn evict_fifo(&mut self) -> StorageResult<()> {
        // In FIFO, we just evict the page that was added first, regardless of access
        while let Some(page_id) = self.lru_queue.pop_front() {
            if let Some(buffer) = self.buffers.get(&page_id) {
                if buffer.can_evict() {
                    // If the page is dirty, flush it to disk
                    if buffer.is_dirty() {
                        let mut page_copy = buffer.page.clone();
                        let mut file_format = self.file_format.lock().map_err(|_| StorageError::Corruption("Failed to lock file format".to_string()))?;

                        self.stats.inc_writes();
                        file_format.write_page(&mut page_copy)?;
                    }

                    // Remove the page from the buffer pool
                    self.buffers.remove(&page_id);
                    self.stats.inc_evictions();
                    return Ok(());
                } else {
                    // Page is pinned, put it back at the end of the queue
                    self.lru_queue.push_back(page_id);
                }
            }
        }

        // If we get here, all pages are pinned
        Err(StorageError::BufferPoolFull)
    }

    /// Clear the buffer pool
    pub fn clear(&mut self) -> StorageResult<()> {
        // Flush all dirty pages
        self.flush_all()?;

        // Clear the buffer pool
        self.buffers.clear();
        self.lru_queue.clear();
        self.pending_io.clear();

        Ok(())
    }

    /// Get buffer pool statistics
    pub fn get_stats(&self) -> (&BufferStats, usize, usize) {
        (&self.stats, self.buffers.len(), self.capacity)
    }

    /// Check if the buffer pool contains a specific page (for testing)
    pub fn contains_page(&self, page_id: PageId) -> bool {
        self.buffers.contains_key(&page_id)
    }

    /// Get the capacity of the buffer pool
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Flushable for BufferPool {
    fn flush(&mut self) -> StorageResult<()> {
        self.flush_all().map(|_| ())
    }
}

/// BufferManager provides a thread-safe interface to the buffer pool, background flushing, and statistics.
pub struct BufferManager {
    /// The buffer pool
    pool: Arc<RwLock<BufferPool>>,
    /// Background flusher thread handle
    _flusher_handle: Option<thread::JoinHandle<()>>,
    /// Signal to stop the flusher thread
    stop_flusher: Arc<Mutex<bool>>,
    /// Stats for the buffer manager
    stats: Arc<BufferStats>,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new(file_format: Arc<Mutex<FileFormat>>, config: &crate::storage_engine::lib::StorageConfig) -> Self {
        let buffer_pool = BufferPool::new(file_format, config);
        let stats = Arc::new(BufferStats::new());

        let pool = Arc::new(RwLock::new(buffer_pool));
        let stop_flusher = Arc::new(Mutex::new(false));

        let mut manager = Self {
            pool,
            _flusher_handle: None,
            stop_flusher,
            stats,
        };

        // Start the background flusher thread if requested
        if config.flush_interval_ms > 0 {
            manager
                .start_flusher(Duration::from_millis(config.flush_interval_ms))
                .expect("Failed to start background flusher thread");
        }

        manager
    }

    /// Starts a background thread to periodically flush dirty pages.
    ///
    /// Steps:
    /// 1. Spawns a thread that loops, sleeping for the given interval.
    /// 2. On each iteration, checks if it should stop, then flushes all dirty pages.
    /// 3. Handles errors and thread termination gracefully.
    pub fn start_flusher(&mut self, interval: Duration) -> StorageResult<()> {
        let pool_clone = self.pool.clone();
        let stop_flusher = self.stop_flusher.clone();

        let handle = thread::Builder::new()
            .name("buffer-flusher".into())
            .spawn(move || {
                loop {
                    // Check if we should stop
                    {
                        let stop = stop_flusher.lock().unwrap();
                        if *stop {
                            break;
                        }
                    }

                    // Sleep for the interval
                    thread::sleep(interval);

                    // Flush dirty pages
                    if let Ok(mut pool) = pool_clone.write()
                        && let Err(e) = pool.flush_all()
                    {
                        eprintln!("Error flushing buffer pool: {e:?}");
                    }
                }
            })
            .map_err(|e| StorageError::Io(std::io::Error::other(format!("Failed to spawn flusher thread: {e}"))))?;

        self._flusher_handle = Some(handle);

        Ok(())
    }

    /// Stop the background flusher thread
    pub fn stop_flusher(&mut self) -> StorageResult<()> {
        if let Some(handle) = self._flusher_handle.take() {
            // Signal the thread to stop
            {
                let mut stop = self.stop_flusher.lock().unwrap();
                *stop = true;
            }

            // Wait for the thread to finish
            if handle.join().is_err() {
                return Err(StorageError::Io(std::io::Error::other("Failed to join flusher thread")));
            }
        }

        Ok(())
    }

    /// Gets a page from the buffer pool, reading from disk if necessary.
    ///
    /// Steps:
    /// 1. If the page is in the buffer pool, update its position/clock bit and return it.
    /// 2. If not, check for pending I/O and make room if at capacity (evict if needed).
    /// 3. Read the page from disk, insert into the buffer pool, and update replacement structures.
    /// 4. Return a reference to the buffer.
    pub fn get_page(&self, page_id: PageId) -> StorageResult<Arc<Page>> {
        let mut pool = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        // Get the page from the buffer pool
        let buffer = pool.get_page(page_id)?;

        // Create an Arc to the page
        Ok(Arc::new(buffer.page.clone()))
    }

    /// Get a page for update (returns a PageGuard)
    pub fn get_page_for_update(&self, page_id: PageId) -> StorageResult<PageGuard> {
        let mut pool = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        // Get the page from the buffer pool
        let buffer = pool.get_page_mut(page_id)?;

        // Pin the page to prevent eviction
        buffer.pin();

        // Create a PageGuard
        let page = buffer.page.clone();

        Ok(PageGuard {
            page_id,
            page,
            pool: Arc::downgrade(&self.pool),
        })
    }

    /// Allocate a new page
    pub fn allocate_page(&self, page_type: PageType, version: VersionId) -> StorageResult<PageId> {
        let mut pool = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        pool.allocate_page(page_type, version)
    }

    /// Flush a specific page
    pub fn flush_page(&self, page_id: PageId) -> StorageResult<()> {
        let mut pool = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        pool.flush_page(page_id)
    }

    /// Flush all dirty pages
    pub fn flush_all(&self) -> StorageResult<usize> {
        let mut pool = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        pool.flush_all()
    }

    /// Get buffer pool statistics
    pub fn stats(&self) -> StorageResult<BufferStats> {
        let pool = self.pool.read().map_err(|_| StorageError::Corruption("Failed to acquire read lock on buffer pool".to_string()))?;

        let (stats, _, _) = pool.get_stats();

        Ok(BufferStats {
            reads: AtomicU64::new(stats.reads.load(Ordering::Relaxed)),
            writes: AtomicU64::new(stats.writes.load(Ordering::Relaxed)),
            hits: AtomicU64::new(stats.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(stats.misses.load(Ordering::Relaxed)),
            evictions: AtomicU64::new(stats.evictions.load(Ordering::Relaxed)),
        })
    }

    /// Get direct access to the buffer pool for testing
    #[cfg(test)]
    pub fn get_buffer_pool_for_testing(&self) -> StorageResult<BufferPoolGuard<'_>> {
        let lock = self.pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        Ok(BufferPoolGuard { lock })
    }
}

impl Flushable for BufferManager {
    fn flush(&mut self) -> StorageResult<()> {
        self.flush_all().map(|_| ())
    }
}

impl Initializable for BufferManager {
    fn init(&mut self) -> StorageResult<()> {
        // Nothing specific to initialize
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        true // Always considered initialized
    }
}

/// PageGuard ensures safe updates to a page and manages pinning/unpinning in the buffer pool.
pub struct PageGuard {
    /// The page ID
    page_id: PageId,
    /// The page content
    page: Page,
    /// Weak reference to the buffer pool
    pool: Weak<RwLock<BufferPool>>,
}

impl PageGuard {
    /// Get a reference to the page
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Update the page with new data
    pub fn update(self, new_data: Vec<u8>) -> StorageResult<()> {
        // Get the buffer pool
        let pool = self.pool.upgrade().ok_or_else(|| StorageError::Corruption("Buffer pool has been deallocated".to_string()))?;

        // Acquire write lock on the buffer pool
        let mut pool = pool.write().map_err(|_| StorageError::Corruption("Failed to acquire write lock on buffer pool".to_string()))?;

        // Get the buffer
        let buffer = pool.get_page_mut(self.page_id)?;

        // Update the page data
        let page = buffer.page_mut();
        page.data.clear();
        page.data.extend_from_slice(&new_data);

        // Update the checksum
        page.update_checksum();

        Ok(())
    }
}

impl Drop for PageGuard {
    fn drop(&mut self) {
        // Unpin the page when the guard is dropped
        if let Some(pool) = self.pool.upgrade()
            && let Ok(mut pool) = pool.write()
            && let Err(e) = pool.unpin_page(self.page_id)
        {
            eprintln!("Error unpinning page {}: {:?}", self.page_id.0, e);
        }
    }
}

/// A dereferencing wrapper for the buffer pool
#[cfg(test)]
pub struct BufferPoolGuard<'a> {
    lock: std::sync::RwLockWriteGuard<'a, BufferPool>,
}

#[cfg(test)]
impl<'a> Deref for BufferPoolGuard<'a> {
    type Target = BufferPool;

    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}

#[cfg(test)]
impl<'a> DerefMut for BufferPoolGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::storage_engine::file_format::{FileFormat, Page, PageId, PageType};
    use crate::storage_engine::lib::{Flushable, Initializable, StorageError, StorageResult, VersionId, calculate_checksum, generate_timestamp};

    fn create_test_file_format() -> Arc<Mutex<FileFormat>> {
        let storage_dir = tempfile::tempdir().unwrap();
        let storage_path = storage_dir.path().join("test_bf.db");
        let mut config = crate::storage_engine::lib::StorageConfig::default();
        config.path = storage_path;
        let mut file_format = FileFormat::new(config);
        file_format.init().unwrap();
        Arc::new(Mutex::new(file_format))
    }

    #[test]
    fn test_buffer_pool_basic() {
        let file_format = create_test_file_format();
        let config = crate::storage_engine::lib::StorageConfig::default();
        let mut pool = BufferPool::new(file_format.clone(), &config);

        // Test basic operations
        let page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();
        let page = pool.get_page(page_id).unwrap();
        assert_eq!(page.page().id, page_id);

        // Test pin/unpin
        pool.pin_page(page_id).unwrap();
        pool.unpin_page(page_id).unwrap();

        // Test flush
        pool.flush_all().unwrap();
    }

    #[test]
    fn test_buffer_manager() {
        let file_format = create_test_file_format();
        let config = crate::storage_engine::lib::StorageConfig::default();
        let mut manager = BufferManager::new(file_format.clone(), &config);

        // Test basic operations
        let page_id = manager.allocate_page(PageType::Data, VersionId(1)).unwrap();
        let page = manager.get_page(page_id).unwrap();
        assert_eq!(page.id, page_id);

        // Test page guard
        let guard = manager.get_page_for_update(page_id).unwrap();
        let new_data = vec![1, 2, 3, 4];
        guard.update(new_data.clone()).unwrap();

        // Verify the update
        let page = manager.get_page(page_id).unwrap();
        assert_eq!(page.data, new_data);
    }

    #[test]
    fn test_lru_replacement_policy() {
        let file_format = create_test_file_format();
        let mut config = crate::storage_engine::lib::StorageConfig::default();
        config.buffer_pool_size = 3;
        let mut pool = BufferPool::new(file_format.clone(), &config);
        pool.set_policy(ReplacementPolicy::LRU);

        // Fill the buffer
        let page_ids: Vec<PageId> = (0..3).map(|_| pool.allocate_page(PageType::Data, VersionId(1)).unwrap()).collect();

        // Access second page to update LRU
        pool.get_page(page_ids[1]).unwrap();

        // Allocate new page to trigger eviction
        let new_page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();

        // Second page should still be present (most recently used)
        assert!(pool.contains_page(page_ids[1]));
        // First page should be evicted (least recently used)
        assert!(!pool.contains_page(page_ids[0]));
    }

    #[test]
    fn test_clock_replacement_policy() {
        let file_format = create_test_file_format();
        let mut config = crate::storage_engine::lib::StorageConfig::default();
        config.buffer_pool_size = 3;
        let mut pool = BufferPool::new(file_format.clone(), &config);
        pool.set_policy(ReplacementPolicy::Clock);

        // Fill the buffer
        let page_ids: Vec<PageId> = (0..3).map(|_| pool.allocate_page(PageType::Data, VersionId(1)).unwrap()).collect();

        // Access second page to set clock bit
        pool.get_page(page_ids[1]).unwrap();

        // Allocate new page to trigger eviction
        let new_page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();

        // Second page should still be present (clock bit is set)
        assert!(pool.contains_page(page_ids[1]));
        // First page should be evicted (clock bit is not set)
        assert!(!pool.contains_page(page_ids[0]));
    }

    #[test]
    fn test_mru_replacement_policy() {
        let file_format = create_test_file_format();
        let mut config = crate::storage_engine::lib::StorageConfig::default();
        config.buffer_pool_size = 3;
        let mut pool = BufferPool::new(file_format.clone(), &config);
        pool.set_policy(ReplacementPolicy::MRU);

        // Fill the buffer
        let page_ids: Vec<PageId> = (0..3).map(|_| pool.allocate_page(PageType::Data, VersionId(1)).unwrap()).collect();

        // Access second page to make it most recent
        pool.get_page(page_ids[1]).unwrap();

        // Allocate new page to trigger eviction
        let new_page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();

        // Second page should be evicted (most recently used)
        assert!(!pool.contains_page(page_ids[1]));
        // First page should still be present (less recently used)
        assert!(pool.contains_page(page_ids[0]));
    }

    #[test]
    fn test_fifo_replacement_policy() {
        let file_format = create_test_file_format();
        let mut config = crate::storage_engine::lib::StorageConfig::default();
        config.buffer_pool_size = 3;
        let mut pool = BufferPool::new(file_format.clone(), &config);
        pool.set_policy(ReplacementPolicy::FIFO);

        // Fill the buffer
        let page_ids: Vec<PageId> = (0..3).map(|_| pool.allocate_page(PageType::Data, VersionId(1)).unwrap()).collect();

        // Access second page (should not affect FIFO order)
        pool.get_page(page_ids[1]).unwrap();

        // Allocate new page to trigger eviction
        let new_page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();

        // First page should be evicted (first in)
        assert!(!pool.contains_page(page_ids[0]));
        // Second page should still be present
        assert!(pool.contains_page(page_ids[1]));
    }

    #[test]
    fn test_concurrent_access() {
        let file_format = create_test_file_format();
        let config = crate::storage_engine::lib::StorageConfig::default();
        let manager = Arc::new(BufferManager::new(file_format.clone(), &config));

        let mut handles = vec![];
        for i in 0..4 {
            let manager_clone = manager.clone();
            handles.push(thread::spawn(move || {
                let page_id = manager_clone.allocate_page(PageType::Data, VersionId(1)).unwrap();
                let page = manager_clone.get_page(page_id).unwrap();
                assert_eq!(page.id, page_id);

                let guard = manager_clone.get_page_for_update(page_id).unwrap();
                let data = vec![i as u8; 1024];
                guard.update(data).unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_buffer_stats() {
        let file_format = create_test_file_format();
        let config = crate::storage_engine::lib::StorageConfig::default();
        let mut pool = BufferPool::new(file_format.clone(), &config);

        // Perform some operations
        let page_id = pool.allocate_page(PageType::Data, VersionId(1)).unwrap();
        pool.get_page(page_id).unwrap();
        pool.get_page(page_id).unwrap(); // Should be a hit
        // Create a new page ID for miss test
        let miss_page_id = PageId(page_id.0 + 1);
        pool.get_page(miss_page_id).unwrap_err(); // Should be a miss

        let (stats, dirty_count, total_count) = pool.get_stats();
        assert_eq!(stats.reads.load(Ordering::Relaxed), 3);
        assert_eq!(stats.hits.load(Ordering::Relaxed), 2);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 1);
    }
}
