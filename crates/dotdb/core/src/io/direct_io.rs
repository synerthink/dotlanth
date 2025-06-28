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

use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Seek, SeekFrom, Write};
use std::path::Path;

/// Configuration for Direct I/O operations, including block size, alignment, and buffer size.
#[derive(Debug, Clone)]
pub struct DirectIOConfig {
    pub enabled: bool,
    pub block_size: usize,
    pub alignment: usize,
    pub buffer_size: usize,
}

impl Default for DirectIOConfig {
    /// Returns a default configuration for Direct I/O.
    fn default() -> Self {
        Self {
            enabled: true,
            block_size: 4096,       // 4KB default block size
            alignment: 512,         // 512-byte alignment
            buffer_size: 64 * 1024, // 64KB buffer
        }
    }
}

/// Wrapper for a file handle that supports Direct I/O operations and alignment.
pub struct DirectIOFile {
    file: File,
    config: DirectIOConfig,
    position: u64,
}

impl DirectIOFile {
    /// Opens an existing file with Direct I/O if enabled in the configuration.
    pub fn open<P: AsRef<Path>>(path: P, config: &DirectIOConfig) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);

        // Enable Direct I/O on Linux systems
        #[cfg(target_os = "linux")]
        if config.enabled {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_DIRECT);
        }

        let file = options.open(path)?;

        Ok(Self {
            file,
            config: config.clone(),
            position: 0,
        })
    }

    /// Creates a new file with Direct I/O if enabled in the configuration.
    pub fn create<P: AsRef<Path>>(path: P, config: &DirectIOConfig) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);

        #[cfg(target_os = "linux")]
        if config.enabled {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_DIRECT);
        }

        let file = options.open(path)?;

        Ok(Self {
            file,
            config: config.clone(),
            position: 0,
        })
    }

    /// Writes data to the file, ensuring proper alignment for Direct I/O.
    pub fn write_aligned(&mut self, data: &[u8]) -> Result<usize> {
        if !self.config.enabled {
            return self.file.write(data);
        }

        let aligned_size = self.align_size(data.len());
        let mut aligned_buffer = vec![0u8; aligned_size];
        aligned_buffer[..data.len()].copy_from_slice(data);

        let bytes_written = self.file.write(&aligned_buffer)?;
        self.position += bytes_written as u64;
        Ok(data.len().min(bytes_written))
    }

    /// Reads data from the file, ensuring proper alignment for Direct I/O.
    pub fn read_aligned(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.config.enabled {
            return self.file.read(buf);
        }

        let aligned_size = self.align_size(buf.len());
        let mut aligned_buffer = vec![0u8; aligned_size];

        let bytes_read = self.file.read(&mut aligned_buffer)?;
        let copy_size = buf.len().min(bytes_read);
        buf[..copy_size].copy_from_slice(&aligned_buffer[..copy_size]);

        self.position += bytes_read as u64;
        Ok(copy_size)
    }

    /// Writes data at a specific offset, ensuring alignment.
    pub fn write_at(&mut self, data: &[u8], offset: u64) -> Result<usize> {
        let aligned_offset = self.align_offset(offset);
        self.file.seek(SeekFrom::Start(aligned_offset))?;
        self.position = aligned_offset;
        self.write_aligned(data)
    }

    /// Reads data from a specific offset, ensuring alignment.
    pub fn read_at(&mut self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let aligned_offset = self.align_offset(offset);
        self.file.seek(SeekFrom::Start(aligned_offset))?;
        self.position = aligned_offset;
        self.read_aligned(buf)
    }

    /// Synchronizes file data to disk.
    pub fn sync(&mut self) -> Result<()> {
        self.file.sync_all()
    }

    /// Returns the current file position.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Returns the current file size.
    pub fn size(&self) -> Result<u64> {
        Ok(self.file.metadata()?.len())
    }

    /// Aligns a size to the configured block boundary for Direct I/O.
    fn align_size(&self, size: usize) -> usize {
        let block_size = self.config.block_size;
        size.div_ceil(block_size) * block_size
    }

    /// Aligns an offset to the configured alignment boundary for Direct I/O.
    fn align_offset(&self, offset: u64) -> u64 {
        let alignment = self.config.alignment as u64;
        (offset / alignment) * alignment
    }

    /// Returns true if Direct I/O is enabled for this file.
    pub fn is_direct_io_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Buffered writer for Direct I/O, supporting batch writes and internal buffering.
pub struct BufferedDirectIOWriter {
    file: DirectIOFile,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

impl BufferedDirectIOWriter {
    /// Creates a new buffered writer for the given DirectIOFile.
    pub fn new(file: DirectIOFile) -> Self {
        let buffer_size = file.config.buffer_size;
        Self {
            file,
            buffer: vec![0u8; buffer_size],
            buffer_pos: 0,
        }
    }

    /// Writes data to the internal buffer, flushing to disk as needed.
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut remaining = data;

        while !remaining.is_empty() {
            let space_left = self.buffer.len() - self.buffer_pos;
            let to_copy = remaining.len().min(space_left);

            self.buffer[self.buffer_pos..self.buffer_pos + to_copy].copy_from_slice(&remaining[..to_copy]);

            self.buffer_pos += to_copy;
            remaining = &remaining[to_copy..];

            if self.buffer_pos == self.buffer.len() {
                self.flush()?;
            }
        }

        Ok(())
    }

    /// Flushes the internal buffer to disk, ensuring all data is written.
    pub fn flush(&mut self) -> Result<()> {
        if self.buffer_pos > 0 {
            self.file.write_aligned(&self.buffer[..self.buffer_pos])?;
            self.buffer_pos = 0;
        }
        self.file.sync()
    }
}

/// Ensures any buffered data is flushed to disk when the writer is dropped.
impl Drop for BufferedDirectIOWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_direct_io_config_default() {
        let config = DirectIOConfig::default();
        assert!(config.enabled);
        assert_eq!(config.block_size, 4096);
        assert_eq!(config.alignment, 512);
        assert_eq!(config.buffer_size, 64 * 1024);
    }

    #[test]
    fn test_align_size() {
        let config = DirectIOConfig {
            block_size: 4096,
            ..Default::default()
        };
        let temp_file = NamedTempFile::new().unwrap();
        let dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();

        assert_eq!(dio_file.align_size(100), 4096);
        assert_eq!(dio_file.align_size(4096), 4096);
        assert_eq!(dio_file.align_size(4097), 8192);
    }

    #[test]
    fn test_align_offset() {
        let config = DirectIOConfig { alignment: 512, ..Default::default() };
        let temp_file = NamedTempFile::new().unwrap();
        let dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();

        assert_eq!(dio_file.align_offset(100), 0);
        assert_eq!(dio_file.align_offset(512), 512);
        assert_eq!(dio_file.align_offset(600), 512);
        assert_eq!(dio_file.align_offset(1024), 1024);
    }

    #[test]
    fn test_write_and_read_aligned() {
        let config = DirectIOConfig {
            enabled: false, // Disable for testing to avoid platform-specific issues
            ..Default::default()
        };
        let temp_file = NamedTempFile::new().unwrap();
        let mut dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();

        let test_data = b"Hello, Direct I/O!";
        let bytes_written = dio_file.write_aligned(test_data).unwrap();
        assert_eq!(bytes_written, test_data.len());

        // Dosyayı kapatıp yeniden açalım
        drop(dio_file);
        let mut dio_file = DirectIOFile::open(temp_file.path(), &config).unwrap();

        let mut read_buffer = vec![0u8; test_data.len()];
        let bytes_read = dio_file.read_aligned(&mut read_buffer).unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&read_buffer, test_data);
    }

    #[test]
    fn test_write_at_and_read_at() {
        let config = DirectIOConfig { enabled: false, ..Default::default() };
        let temp_file = NamedTempFile::new().unwrap();
        let mut dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();

        let test_data = b"Test data";
        dio_file.write_at(test_data, 1024).unwrap();

        // Dosyayı kapatıp yeniden açalım
        drop(dio_file);
        let mut dio_file = DirectIOFile::open(temp_file.path(), &config).unwrap();

        let mut read_buffer = vec![0u8; test_data.len()];
        let bytes_read = dio_file.read_at(&mut read_buffer, 1024).unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&read_buffer, test_data);
    }

    #[test]
    fn test_buffered_writer() {
        let config = DirectIOConfig {
            enabled: false,
            buffer_size: 1024,
            ..Default::default()
        };
        let temp_file = NamedTempFile::new().unwrap();
        let dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();
        let mut writer = BufferedDirectIOWriter::new(dio_file);

        let test_data = b"This is a test for buffered writer";
        writer.write(test_data).unwrap();
        writer.flush().unwrap();

        // Verify data was written
        let mut file = File::open(temp_file.path()).unwrap();
        let mut read_buffer = Vec::new();
        file.read_to_end(&mut read_buffer).unwrap();
        assert!(read_buffer.starts_with(test_data));
    }

    #[test]
    fn test_file_size() {
        let config = DirectIOConfig { enabled: false, ..Default::default() };
        let temp_file = NamedTempFile::new().unwrap();
        let mut dio_file = DirectIOFile::create(temp_file.path(), &config).unwrap();

        let test_data = b"Size test data";
        dio_file.write_aligned(test_data).unwrap();
        dio_file.sync().unwrap();

        let size = dio_file.size().unwrap();
        assert!(size >= test_data.len() as u64);
    }

    #[test]
    fn test_is_direct_io_enabled() {
        let config_enabled = DirectIOConfig { enabled: true, ..Default::default() };
        let config_disabled = DirectIOConfig { enabled: false, ..Default::default() };

        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();

        let dio_file1 = DirectIOFile::create(temp_file1.path(), &config_enabled).unwrap();
        let dio_file2 = DirectIOFile::create(temp_file2.path(), &config_disabled).unwrap();

        assert!(dio_file1.is_direct_io_enabled());
        assert!(!dio_file2.is_direct_io_enabled());
    }
}
