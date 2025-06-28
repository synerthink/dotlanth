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

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::unix::io::AsRawFd,
    path::Path,
    ptr::{self, NonNull},
    sync::{Arc, Mutex, RwLock},
};

use crate::memory::lib::{align_to, get_page_size};

/// Defines the memory mapping access strategy
/// Controls how the mapped memory can be accessed
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MappingStrategy {
    ReadOnly,    // Only allow reading from the mapped region
    ReadWrite,   // Allow both reading and writing to the mapped region
    WriteOnly,   // Only allow writing to the mapped region
    CopyOnWrite, // Copy-on-write semantics (modifications don't affect the source)
}

/// Memory protection flags for mapped regions
/// Maps to the protection flags used in mmap system calls
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protection {
    Read = libc::PROT_READ as isize,    // Allow reading
    Write = libc::PROT_WRITE as isize,  // Allow writing
    Execute = libc::PROT_EXEC as isize, // Allow execution of code
    None = libc::PROT_NONE as isize,    // No access allowed
}

/// Memory mapping configuration flags
/// Maps to the flags parameter in mmap system calls
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapFlags {
    Shared = libc::MAP_SHARED as isize,       // Share mapping with other processes
    Private = libc::MAP_PRIVATE as isize,     // Private mapping (changes not visible to other processes)
    Anonymous = libc::MAP_ANONYMOUS as isize, // Not backed by a file
    Fixed = libc::MAP_FIXED as isize,         // Map at exact address
    #[cfg(target_os = "linux")]
    Populate = libc::MAP_POPULATE as isize, // Populate page tables eagerly
    #[cfg(target_os = "linux")]
    Locked = libc::MAP_LOCKED as isize, // Lock pages in memory
}

/// Errors that can occur during memory mapping operations
#[derive(Debug)]
pub enum MmapError {
    IoError(io::Error), // Underlying IO error
    InvalidSize,        // Invalid size specified
    InvalidAlignment,   // Invalid alignment specified
    InvalidPath,        // Invalid file path
    MappingFailed,      // mmap system call failed
    UnmappingFailed,    // munmap system call failed
    SyncFailed,         // msync system call failed
    ProtectionFailed,   // mprotect system call failed
    InvalidOffset,      // Invalid offset into file
    FileTooSmall,       // File is smaller than requested mapping
    Unsupported,        // Operation not supported on this platform
}

impl From<io::Error> for MmapError {
    fn from(err: io::Error) -> Self {
        MmapError::IoError(err)
    }
}

/// Memory-mapped file implementation
/// Provides a safe interface to memory-mapped files or anonymous mappings
pub struct MemoryMap {
    ptr: NonNull<u8>,                             // Pointer to the mapped region
    len: usize,                                   // Length of the mapped region
    file: Option<Arc<File>>,                      // The file being mapped, if any
    strategy: MappingStrategy,                    // The mapping strategy
    page_size: usize,                             // System page size
    is_anonymous: bool,                           // Whether this is an anonymous mapping
    protection: i32,                              // Memory protection flags
    flags: i32,                                   // Mapping flags
    sync_policy: Arc<RwLock<SyncPolicy>>,         // Policy for syncing changes to disk
    dirty_pages: Arc<Mutex<Vec<(usize, usize)>>>, // Tracks modified regions for efficient syncing
}

/// Policy for syncing changes to mapped files
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncPolicy {
    Immediate, // msync after every write
    Periodic,  // msync at regular intervals
    Manual,    // msync only when explicitly called
    OnDrop,    // msync when dropping the mapping
}

// Implement Send and Sync for MemoryMap to allow sharing across threads
unsafe impl Send for MemoryMap {}
unsafe impl Sync for MemoryMap {}

impl MemoryMap {
    /// Create a new memory mapping from a file
    ///
    /// # Arguments
    /// * `path` - Path to the file to map
    /// * `strategy` - Mapping strategy to use (read-only, read-write, etc.)
    /// * `offset` - Offset into the file to start mapping
    /// * `length` - Length of the region to map, or None to map the entire file
    ///
    /// # Returns
    /// * `Result<Self, MmapError>` - The memory map or an error
    pub fn from_file<P: AsRef<Path>>(path: P, strategy: MappingStrategy, offset: u64, length: Option<usize>) -> Result<Self, MmapError> {
        // Open the file with appropriate permissions based on the strategy
        let file = match strategy {
            MappingStrategy::ReadOnly => File::open(&path)?,
            MappingStrategy::ReadWrite | MappingStrategy::WriteOnly => OpenOptions::new().read(true).write(true).create(true).open(&path)?,
            MappingStrategy::CopyOnWrite => File::open(&path)?,
        };

        let file_len = file.metadata()?.len();

        // Validate offset
        if offset >= file_len {
            return Err(MmapError::InvalidOffset);
        }

        // Determine the length to map
        let map_len = match length {
            Some(len) => {
                if offset + len as u64 > file_len {
                    return Err(MmapError::FileTooSmall);
                }
                len
            }
            None => (file_len - offset) as usize,
        };

        if map_len == 0 {
            return Err(MmapError::InvalidSize);
        }

        // Align offset and length to page boundaries
        let page_size = get_page_size();
        let aligned_offset = align_to(offset as usize, page_size);
        let offset_diff = offset as usize - aligned_offset;
        let aligned_len = align_to(map_len + offset_diff, page_size);

        // Get appropriate mmap parameters
        let (protection, flags) = Self::get_mmap_params(strategy);

        unsafe {
            // Create the memory mapping using mmap system call
            let ptr = libc::mmap(ptr::null_mut(), aligned_len, protection, flags, file.as_raw_fd(), aligned_offset as libc::off_t);

            if ptr == libc::MAP_FAILED {
                return Err(MmapError::MappingFailed);
            }

            // Adjust pointer to account for alignment
            let adjusted_ptr = (ptr as *mut u8).add(offset_diff);

            Ok(Self {
                ptr: NonNull::new_unchecked(adjusted_ptr),
                len: map_len,
                file: Some(Arc::new(file)),
                strategy,
                page_size,
                is_anonymous: false,
                protection,
                flags,
                sync_policy: Arc::new(RwLock::new(SyncPolicy::Manual)),
                dirty_pages: Arc::new(Mutex::new(Vec::new())),
            })
        }
    }

    /// Create an anonymous memory mapping (not backed by a file)
    ///
    /// # Arguments
    /// * `size` - Size of the mapping to create
    /// * `strategy` - Mapping strategy to use
    ///
    /// # Returns
    /// * `Result<Self, MmapError>` - The memory map or an error
    pub fn anonymous(size: usize, strategy: MappingStrategy) -> Result<Self, MmapError> {
        if size == 0 {
            return Err(MmapError::InvalidSize);
        }

        // Align to page boundaries
        let page_size = get_page_size();
        let aligned_size = align_to(size, page_size);

        // Get appropriate mmap parameters
        let (protection, mut flags) = Self::get_mmap_params(strategy);
        flags |= libc::MAP_ANONYMOUS;

        unsafe {
            // Create the memory mapping using mmap system call
            let ptr = libc::mmap(ptr::null_mut(), aligned_size, protection, flags, -1, 0);

            if ptr == libc::MAP_FAILED {
                return Err(MmapError::MappingFailed);
            }

            Ok(Self {
                ptr: NonNull::new_unchecked(ptr as *mut u8),
                len: size,
                file: None,
                strategy,
                page_size,
                is_anonymous: true,
                protection,
                flags,
                sync_policy: Arc::new(RwLock::new(SyncPolicy::Manual)),
                dirty_pages: Arc::new(Mutex::new(Vec::new())),
            })
        }
    }

    /// Create a memory mapping with specific protection and flags
    /// Provides more fine-grained control than the other creation methods
    ///
    /// # Arguments
    /// * `size` - Size of the mapping to create
    /// * `protection` - Array of protection flags
    /// * `flags` - Array of mapping flags
    ///
    /// # Returns
    /// * `Result<Self, MmapError>` - The memory map or an error
    pub fn with_protection_and_flags(size: usize, protection: &[Protection], flags: &[MapFlags]) -> Result<Self, MmapError> {
        if size == 0 {
            return Err(MmapError::InvalidSize);
        }

        let page_size = get_page_size();
        let aligned_size = align_to(size, page_size);

        // Combine protection and flag values
        let prot = protection.iter().fold(0, |acc, &p| acc | p as i32);
        let map_flags = flags.iter().fold(0, |acc, &f| acc | f as i32);

        unsafe {
            let fd = if map_flags & libc::MAP_ANONYMOUS != 0 { -1 } else { 0 };

            // Create the memory mapping using mmap system call
            let ptr = libc::mmap(ptr::null_mut(), aligned_size, prot, map_flags, fd, 0);

            if ptr == libc::MAP_FAILED {
                return Err(MmapError::MappingFailed);
            }

            // Determine strategy based on protection and flags
            let strategy = if prot & libc::PROT_WRITE != 0 {
                if map_flags & libc::MAP_PRIVATE != 0 {
                    MappingStrategy::CopyOnWrite
                } else {
                    MappingStrategy::ReadWrite
                }
            } else {
                MappingStrategy::ReadOnly
            };

            Ok(Self {
                ptr: NonNull::new_unchecked(ptr as *mut u8),
                len: size,
                file: None,
                strategy,
                page_size,
                is_anonymous: map_flags & libc::MAP_ANONYMOUS != 0,
                protection: prot,
                flags: map_flags,
                sync_policy: Arc::new(RwLock::new(SyncPolicy::Manual)),
                dirty_pages: Arc::new(Mutex::new(Vec::new())),
            })
        }
    }

    /// Get a pointer to the mapped memory
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Get a mutable pointer to the mapped memory
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Get the length of the mapping
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the mapping is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a slice view of the mapped memory
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Get a mutable slice view of the mapped memory
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Read data from the mapping
    pub fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, MmapError> {
        if offset >= self.len {
            return Ok(0);
        }

        let available = self.len - offset;
        let to_read = buf.len().min(available);

        unsafe {
            let src = self.ptr.as_ptr().add(offset);
            ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }

        Ok(to_read)
    }

    /// Write data to the mapping
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<usize, MmapError> {
        if self.strategy == MappingStrategy::ReadOnly {
            return Err(MmapError::ProtectionFailed);
        }

        if offset >= self.len {
            return Ok(0);
        }

        let available = self.len - offset;
        let to_write = data.len().min(available);

        unsafe {
            let dst = self.ptr.as_ptr().add(offset);
            ptr::copy_nonoverlapping(data.as_ptr(), dst, to_write);
        }

        // Track dirty pages
        let page_start = offset / self.page_size;
        let page_end = (offset + to_write).div_ceil(self.page_size);

        let mut dirty_pages = self.dirty_pages.lock().unwrap();
        dirty_pages.push((page_start * self.page_size, (page_end - page_start) * self.page_size));

        // Auto-sync if policy requires it
        let sync_policy = *self.sync_policy.read().unwrap();
        if sync_policy == SyncPolicy::Immediate {
            drop(dirty_pages);
            self.sync_range(offset, to_write)?;
        }

        Ok(to_write)
    }

    /// Synchronize the mapping with the underlying file
    pub fn sync(&self) -> Result<(), MmapError> {
        if self.is_anonymous {
            return Ok(());
        }

        unsafe {
            let result = libc::msync(self.ptr.as_ptr() as *mut libc::c_void, self.len, libc::MS_SYNC);

            if result != 0 {
                return Err(MmapError::SyncFailed);
            }
        }

        // Clear dirty pages tracking
        let mut dirty_pages = self.dirty_pages.lock().unwrap();
        dirty_pages.clear();

        Ok(())
    }

    /// Synchronize a range of the mapping
    pub fn sync_range(&self, offset: usize, length: usize) -> Result<(), MmapError> {
        if self.is_anonymous {
            return Ok(());
        }

        if offset >= self.len {
            return Ok(());
        }

        let sync_len = length.min(self.len - offset);

        unsafe {
            let sync_ptr = self.ptr.as_ptr().add(offset) as *mut libc::c_void;
            let result = libc::msync(sync_ptr, sync_len, libc::MS_SYNC);

            if result != 0 {
                return Err(MmapError::SyncFailed);
            }
        }

        Ok(())
    }

    /// Asynchronously synchronize the mapping
    pub fn sync_async(&self) -> Result<(), MmapError> {
        if self.is_anonymous {
            return Ok(());
        }

        unsafe {
            let result = libc::msync(self.ptr.as_ptr() as *mut libc::c_void, self.len, libc::MS_ASYNC);

            if result != 0 {
                return Err(MmapError::SyncFailed);
            }
        }

        Ok(())
    }

    /// Change the protection of the mapping
    pub fn protect(&mut self, protection: &[Protection]) -> Result<(), MmapError> {
        let prot = protection.iter().fold(0, |acc, &p| acc | p as i32);

        unsafe {
            let result = libc::mprotect(self.ptr.as_ptr() as *mut libc::c_void, self.len, prot);

            if result != 0 {
                return Err(MmapError::ProtectionFailed);
            }
        }

        self.protection = prot;

        // Update strategy based on new protection
        self.strategy = if prot & libc::PROT_WRITE != 0 {
            if self.flags & libc::MAP_PRIVATE != 0 {
                MappingStrategy::CopyOnWrite
            } else {
                MappingStrategy::ReadWrite
            }
        } else {
            MappingStrategy::ReadOnly
        };

        Ok(())
    }

    /// Advise the kernel about memory usage patterns
    pub fn advise(&self, advice: MemoryAdvice) -> Result<(), MmapError> {
        let madvise_advice = match advice {
            MemoryAdvice::Normal => libc::MADV_NORMAL,
            MemoryAdvice::Sequential => libc::MADV_SEQUENTIAL,
            MemoryAdvice::Random => libc::MADV_RANDOM,
            MemoryAdvice::WillNeed => libc::MADV_WILLNEED,
            MemoryAdvice::DontNeed => libc::MADV_DONTNEED,
        };

        unsafe {
            let result = libc::madvise(self.ptr.as_ptr() as *mut libc::c_void, self.len, madvise_advice);

            if result != 0 {
                return Err(MmapError::MappingFailed);
            }
        }

        Ok(())
    }

    /// Lock pages in memory
    pub fn lock(&self) -> Result<(), MmapError> {
        unsafe {
            let result = libc::mlock(self.ptr.as_ptr() as *const libc::c_void, self.len);

            if result != 0 {
                return Err(MmapError::MappingFailed);
            }
        }

        Ok(())
    }

    /// Unlock pages from memory
    pub fn unlock(&self) -> Result<(), MmapError> {
        unsafe {
            let result = libc::munlock(self.ptr.as_ptr() as *const libc::c_void, self.len);

            if result != 0 {
                return Err(MmapError::MappingFailed);
            }
        }

        Ok(())
    }

    /// Set the synchronization policy
    pub fn set_sync_policy(&self, policy: SyncPolicy) {
        let mut sync_policy = self.sync_policy.write().unwrap();
        *sync_policy = policy;
    }

    /// Get the current synchronization policy
    pub fn get_sync_policy(&self) -> SyncPolicy {
        *self.sync_policy.read().unwrap()
    }

    /// Resize the mapping (if possible)
    #[cfg(target_os = "linux")]
    pub fn resize(&mut self, new_size: usize) -> Result<(), MmapError> {
        if new_size == 0 {
            return Err(MmapError::InvalidSize);
        }

        if !self.is_anonymous {
            // For file mappings, we need to extend the file first
            if let Some(ref file) = self.file {
                let mut file_handle = File::options().write(true).open("dummy")?; // Placeholder
                file_handle.set_len(new_size as u64)?;
            }
        }

        let page_size = get_page_size();
        let aligned_size = align_to(new_size, page_size);

        unsafe {
            let new_ptr = libc::mremap(self.ptr.as_ptr() as *mut libc::c_void, self.len, aligned_size, libc::MREMAP_MAYMOVE);

            if new_ptr == libc::MAP_FAILED {
                return Err(MmapError::MappingFailed);
            }

            self.ptr = NonNull::new_unchecked(new_ptr as *mut u8);
            self.len = new_size;
        }

        Ok(())
    }

    /// Resize the mapping (if possible) - Fallback for non-Linux
    #[cfg(not(target_os = "linux"))]
    pub fn resize(&mut self, _new_size: usize) -> Result<(), MmapError> {
        Err(MmapError::Unsupported)
    }

    /// Get information about the mapping
    pub fn info(&self) -> MappingInfo {
        MappingInfo {
            address: self.ptr.as_ptr() as usize,
            size: self.len,
            strategy: self.strategy,
            page_size: self.page_size,
            is_anonymous: self.is_anonymous,
            is_readable: self.protection & libc::PROT_READ != 0,
            is_writable: self.protection & libc::PROT_WRITE != 0,
            is_executable: self.protection & libc::PROT_EXEC != 0,
            is_shared: self.flags & libc::MAP_SHARED != 0,
        }
    }

    /// Flush dirty pages to storage
    pub fn flush_dirty_pages(&self) -> Result<(), MmapError> {
        let dirty_pages = self.dirty_pages.lock().unwrap();

        for &(offset, length) in dirty_pages.iter() {
            self.sync_range(offset, length)?;
        }

        Ok(())
    }

    /// Get the mmap parameters
    fn get_mmap_params(strategy: MappingStrategy) -> (i32, i32) {
        match strategy {
            MappingStrategy::ReadOnly => (libc::PROT_READ, libc::MAP_SHARED),
            MappingStrategy::ReadWrite => (libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED),
            MappingStrategy::WriteOnly => (libc::PROT_WRITE, libc::MAP_SHARED),
            MappingStrategy::CopyOnWrite => (libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE),
        }
    }
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        // Sync if policy requires it
        let sync_policy = *self.sync_policy.read().unwrap();
        if sync_policy == SyncPolicy::OnDrop && !self.is_anonymous {
            let _ = self.sync();
        }

        // Unmap the memory
        unsafe {
            let result = libc::munmap(self.ptr.as_ptr() as *mut libc::c_void, align_to(self.len, self.page_size));

            if result != 0 {
                eprintln!("Warning: Failed to unmap memory at {:p}", self.ptr.as_ptr());
            }
        }
    }
}

/// Memory access pattern hints for the operating system
/// These hints can improve performance by optimizing page handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryAdvice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
}

/// Information about a memory mapping
/// Contains details about the mapping's properties and current state
#[derive(Debug, Clone)]
pub struct MappingInfo {
    pub address: usize,
    pub size: usize,
    pub strategy: MappingStrategy,
    pub page_size: usize,
    pub is_anonymous: bool,
    pub is_readable: bool,
    pub is_writable: bool,
    pub is_executable: bool,
    pub is_shared: bool,
}

/// Builder for creating memory mappings with specific configurations
pub struct MemoryMapBuilder {
    size: Option<usize>,
    strategy: MappingStrategy,
    protection: Vec<Protection>,
    flags: Vec<MapFlags>,
    sync_policy: SyncPolicy,
    advice: Option<MemoryAdvice>,
    lock_pages: bool,
}

impl Default for MemoryMapBuilder {
    fn default() -> Self {
        Self {
            size: None,
            strategy: MappingStrategy::ReadWrite,
            protection: vec![Protection::Read, Protection::Write],
            flags: vec![MapFlags::Private, MapFlags::Anonymous],
            sync_policy: SyncPolicy::Manual,
            advice: None,
            lock_pages: false,
        }
    }
}

impl MemoryMapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn strategy(mut self, strategy: MappingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn protection(mut self, protection: Vec<Protection>) -> Self {
        self.protection = protection;
        self
    }

    pub fn flags(mut self, flags: Vec<MapFlags>) -> Self {
        self.flags = flags;
        self
    }

    pub fn sync_policy(mut self, policy: SyncPolicy) -> Self {
        self.sync_policy = policy;
        self
    }

    pub fn advice(mut self, advice: MemoryAdvice) -> Self {
        self.advice = Some(advice);
        self
    }

    pub fn lock_pages(mut self, lock: bool) -> Self {
        self.lock_pages = lock;
        self
    }

    pub fn build(self) -> Result<MemoryMap, MmapError> {
        let size = self.size.ok_or(MmapError::InvalidSize)?;

        let mmap = MemoryMap::with_protection_and_flags(size, &self.protection, &self.flags)?;

        mmap.set_sync_policy(self.sync_policy);

        if let Some(advice) = self.advice {
            mmap.advise(advice)?;
        }

        if self.lock_pages {
            mmap.lock()?;
        }

        Ok(mmap)
    }

    pub fn build_from_file<P: AsRef<Path>>(self, path: P, offset: u64, length: Option<usize>) -> Result<MemoryMap, MmapError> {
        let mmap = MemoryMap::from_file(path, self.strategy, offset, length)?;

        mmap.set_sync_policy(self.sync_policy);

        if let Some(advice) = self.advice {
            mmap.advise(advice)?;
        }

        if self.lock_pages {
            mmap.lock()?;
        }

        Ok(mmap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_anonymous_mapping() {
        let mmap = MemoryMap::anonymous(4096, MappingStrategy::ReadWrite).unwrap();

        assert_eq!(mmap.len(), 4096);
        assert!(!mmap.is_empty());
        assert!(mmap.info().is_anonymous);
    }

    #[test]
    fn test_file_mapping() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMap::from_file(temp_file.path(), MappingStrategy::ReadOnly, 0, None).unwrap();

        assert_eq!(mmap.len(), test_data.len());

        let slice = mmap.as_slice();
        assert_eq!(slice, test_data);
    }

    #[test]
    fn test_read_write_operations() {
        let mut mmap = MemoryMap::anonymous(1024, MappingStrategy::ReadWrite).unwrap();

        let test_data = b"Test data";
        let written = mmap.write(0, test_data).unwrap();
        assert_eq!(written, test_data.len());

        let mut read_buf = vec![0u8; test_data.len()];
        let read_count = mmap.read(0, &mut read_buf).unwrap();

        assert_eq!(read_count, test_data.len());
        assert_eq!(read_buf, test_data);
    }

    #[test]
    fn test_memory_protection() {
        let mut mmap = MemoryMap::anonymous(4096, MappingStrategy::ReadWrite).unwrap();

        // Change to read-only
        mmap.protect(&[Protection::Read]).unwrap();

        let info = mmap.info();
        assert!(info.is_readable);
        assert!(!info.is_writable);

        // Writing should fail now
        let result = mmap.write(0, b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_advice() {
        let mmap = MemoryMap::anonymous(4096, MappingStrategy::ReadWrite).unwrap();

        // Test different advice types
        assert!(mmap.advise(MemoryAdvice::Sequential).is_ok());
        assert!(mmap.advise(MemoryAdvice::Random).is_ok());
        assert!(mmap.advise(MemoryAdvice::WillNeed).is_ok());
    }

    #[test]
    fn test_sync_operations() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"initial data").unwrap();
        temp_file.flush().unwrap();

        let mut mmap = MemoryMap::from_file(temp_file.path(), MappingStrategy::ReadWrite, 0, None).unwrap();

        // Write some data
        mmap.write(0, b"new data").unwrap();

        // Sync to file
        assert!(mmap.sync().is_ok());
        assert!(mmap.sync_async().is_ok());
    }

    #[test]
    fn test_memory_locking() {
        let mmap = MemoryMap::anonymous(4096, MappingStrategy::ReadWrite).unwrap();

        // Note: These might fail without proper permissions
        let lock_result = mmap.lock();
        if lock_result.is_ok() {
            assert!(mmap.unlock().is_ok());
        }
    }

    #[test]
    fn test_mapping_info() {
        let mmap = MemoryMap::anonymous(8192, MappingStrategy::ReadWrite).unwrap();

        let info = mmap.info();
        assert_eq!(info.size, 8192);
        assert_eq!(info.strategy, MappingStrategy::ReadWrite);
        assert!(info.is_anonymous);
        assert!(info.is_readable);
        assert!(info.is_writable);
        assert!(!info.is_executable);
    }

    #[test]
    fn test_sync_policies() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test").unwrap();
        temp_file.flush().unwrap();

        let mut mmap = MemoryMap::from_file(temp_file.path(), MappingStrategy::ReadWrite, 0, None).unwrap();

        // Test different sync policies
        mmap.set_sync_policy(SyncPolicy::Manual);
        assert_eq!(mmap.get_sync_policy(), SyncPolicy::Manual);

        mmap.set_sync_policy(SyncPolicy::Immediate);
        assert_eq!(mmap.get_sync_policy(), SyncPolicy::Immediate);

        // Write with immediate sync policy
        mmap.write(0, b"sync").unwrap();
    }

    #[test]
    fn test_builder_pattern() {
        let mmap = MemoryMapBuilder::new()
            .size(4096)
            .strategy(MappingStrategy::ReadWrite)
            .sync_policy(SyncPolicy::OnDrop)
            .advice(MemoryAdvice::Sequential)
            .build()
            .unwrap();

        assert_eq!(mmap.len(), 4096);
        assert_eq!(mmap.get_sync_policy(), SyncPolicy::OnDrop);
    }

    #[test]
    fn test_builder_with_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"builder test").unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMapBuilder::new()
            .strategy(MappingStrategy::ReadOnly)
            .advice(MemoryAdvice::Sequential)
            .build_from_file(temp_file.path(), 0, None)
            .unwrap();

        assert_eq!(mmap.len(), 12); // "builder test".len()
        assert_eq!(mmap.as_slice(), b"builder test");
    }

    #[test]
    fn test_custom_protection_and_flags() {
        let mmap = MemoryMap::with_protection_and_flags(4096, &[Protection::Read, Protection::Write], &[MapFlags::Private, MapFlags::Anonymous]).unwrap();

        let info = mmap.info();
        assert!(info.is_readable);
        assert!(info.is_writable);
        assert!(!info.is_shared);
        assert!(info.is_anonymous);
    }

    #[test]
    fn test_boundary_conditions() {
        // Test zero size
        assert!(MemoryMap::anonymous(0, MappingStrategy::ReadWrite).is_err());

        // Test read beyond bounds
        let mmap = MemoryMap::anonymous(100, MappingStrategy::ReadWrite).unwrap();
        let mut buf = [0u8; 10];

        // Read at boundary should return 0
        let read_count = mmap.read(100, &mut buf).unwrap();
        assert_eq!(read_count, 0);

        // Read partially beyond bounds
        let read_count = mmap.read(95, &mut buf).unwrap();
        assert_eq!(read_count, 5);
    }

    #[test]
    fn test_slice_access() {
        let mut mmap = MemoryMap::anonymous(1024, MappingStrategy::ReadWrite).unwrap();

        // Test immutable slice
        let slice = mmap.as_slice();
        assert_eq!(slice.len(), 1024);

        // Test mutable slice
        let mut_slice = mmap.as_mut_slice();
        mut_slice[0] = 42;
        mut_slice[1] = 84;

        let slice = mmap.as_slice();
        assert_eq!(slice[0], 42);
        assert_eq!(slice[1], 84);
    }
}
