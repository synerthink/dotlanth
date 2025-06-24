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

use crate::fs::{FileMetadata, FileSystemLayout, FileType};
use crate::io::DirectIOFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Result, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadata describing a checkpoint, including files, LSN, and integrity information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub id: u64,
    pub timestamp: u64,
    pub version: u32,
    pub data_files: Vec<String>,
    pub log_sequence_number: u64,
    pub size: u64,
    pub checksum: u64,
}

/// Configuration for checkpointing, including retention and verification options.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    pub max_checkpoints: usize,
    pub auto_checkpoint_interval: std::time::Duration,
    pub compression_enabled: bool,
    pub verification_enabled: bool,
}

impl Default for CheckpointConfig {
    /// Returns a default configuration for checkpointing.
    fn default() -> Self {
        Self {
            max_checkpoints: 10,
            auto_checkpoint_interval: std::time::Duration::from_secs(300), // 5 minutes
            compression_enabled: false,
            verification_enabled: true,
        }
    }
}

/// Manages the creation, restoration, and retention of database checkpoints.
pub struct CheckpointManager {
    layout: FileSystemLayout,
    config: CheckpointConfig,
    current_lsn: u64,
    last_checkpoint_time: SystemTime,
}

impl CheckpointManager {
    /// Creates a new CheckpointManager with the given layout and configuration.
    pub fn new(layout: FileSystemLayout, config: CheckpointConfig) -> Self {
        Self {
            layout,
            config,
            current_lsn: 0,
            last_checkpoint_time: SystemTime::now(),
        }
    }

    /// Creates a new checkpoint, writing metadata and data files, and prunes old checkpoints.
    pub fn create_checkpoint(&mut self) -> Result<CheckpointMetadata> {
        let checkpoint_id = self.layout.next_file_id(FileType::Checkpoint)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Collect all data files to include in checkpoint
        let data_files = self.collect_data_files()?;

        // Create checkpoint metadata
        let mut metadata = CheckpointMetadata {
            id: checkpoint_id,
            timestamp,
            version: 1,
            data_files: data_files.iter().map(|f| f.path.to_string_lossy().to_string()).collect(),
            log_sequence_number: self.current_lsn,
            size: 0,
            checksum: 0,
        };

        // Create checkpoint file
        let checkpoint_path = self.layout.generate_file_path(FileType::Checkpoint, checkpoint_id, metadata.version);

        self.write_checkpoint(&checkpoint_path, &data_files, &mut metadata)?;

        // Update last checkpoint time
        self.last_checkpoint_time = SystemTime::now();

        // Clean up old checkpoints
        self.cleanup_old_checkpoints()?;

        Ok(metadata)
    }

    /// Writes checkpoint metadata and data to the specified file.
    fn write_checkpoint(&self, checkpoint_path: &Path, data_files: &[FileMetadata], metadata: &mut CheckpointMetadata) -> Result<()> {
        let file = File::create(checkpoint_path)?;
        let mut writer = BufWriter::new(file);

        // Write metadata header
        let metadata_json = serde_json::to_string(metadata)?;
        let metadata_size = metadata_json.len() as u32;
        writer.write_all(&metadata_size.to_le_bytes())?;
        writer.write_all(metadata_json.as_bytes())?;

        let mut total_size = metadata_json.len() as u64;
        let mut checksum = self.calculate_checksum(metadata_json.as_bytes());

        // Copy data from all files
        for file_metadata in data_files {
            let mut source_file = File::open(&file_metadata.path)?;
            let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

            loop {
                let bytes_read = source_file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }

                writer.write_all(&buffer[..bytes_read])?;
                total_size += bytes_read as u64;
                checksum = checksum.wrapping_add(self.calculate_checksum(&buffer[..bytes_read]));
            }
        }

        writer.flush()?;

        // Update metadata with final size and checksum
        metadata.size = total_size;
        metadata.checksum = checksum;

        // Rewrite metadata with correct values
        drop(writer);
        let file = File::create(checkpoint_path)?;
        let mut writer = BufWriter::new(file);

        let metadata_json = serde_json::to_string(metadata)?;
        let metadata_size = metadata_json.len() as u32;
        writer.write_all(&metadata_size.to_le_bytes())?;
        writer.write_all(metadata_json.as_bytes())?;

        // Re-copy data files (this is inefficient but ensures correctness)
        for file_metadata in data_files {
            let mut source_file = File::open(&file_metadata.path)?;
            let mut buffer = vec![0u8; 64 * 1024];

            loop {
                let bytes_read = source_file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                writer.write_all(&buffer[..bytes_read])?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// Restores the database state from a given checkpoint, including data and LSN.
    pub fn restore_from_checkpoint(&mut self, checkpoint_id: u64) -> Result<()> {
        let checkpoint_metadata = self.load_checkpoint_metadata(checkpoint_id)?;
        let checkpoint_path = self.layout.generate_file_path(FileType::Checkpoint, checkpoint_id, checkpoint_metadata.version);

        // Verify checkpoint integrity
        if self.config.verification_enabled {
            self.verify_checkpoint(&checkpoint_path, &checkpoint_metadata)?;
        }

        // Restore data files from checkpoint
        self.restore_data_files(&checkpoint_path, &checkpoint_metadata)?;

        // Update current LSN
        self.current_lsn = checkpoint_metadata.log_sequence_number;

        Ok(())
    }

    /// Loads checkpoint metadata for a specific checkpoint ID.
    pub fn load_checkpoint_metadata(&self, checkpoint_id: u64) -> Result<CheckpointMetadata> {
        let checkpoints = self.list_checkpoints()?;

        for checkpoint in checkpoints {
            if checkpoint.id == checkpoint_id {
                return Ok(checkpoint);
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, format!("Checkpoint {} not found", checkpoint_id)))
    }

    /// Lists all available checkpoints, sorted by timestamp (newest first).
    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointMetadata>> {
        let checkpoint_files = self.layout.list_files(FileType::Checkpoint)?;
        let mut checkpoints = Vec::new();

        for file_metadata in checkpoint_files {
            if let Ok(checkpoint_metadata) = self.read_checkpoint_metadata(&file_metadata.path) {
                checkpoints.push(checkpoint_metadata);
            }
        }

        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(checkpoints)
    }

    /// Reads checkpoint metadata from a checkpoint file.
    fn read_checkpoint_metadata(&self, path: &Path) -> Result<CheckpointMetadata> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read metadata size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let metadata_size = u32::from_le_bytes(size_bytes) as usize;

        // Read metadata JSON
        let mut metadata_buffer = vec![0u8; metadata_size];
        reader.read_exact(&mut metadata_buffer)?;
        let metadata_json = String::from_utf8(metadata_buffer).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid metadata encoding"))?;

        let metadata: CheckpointMetadata = serde_json::from_str(&metadata_json)?;
        Ok(metadata)
    }

    /// Verifies the integrity of a checkpoint file by comparing checksums.
    fn verify_checkpoint(&self, path: &Path, metadata: &CheckpointMetadata) -> Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Skip metadata header
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let metadata_size = u32::from_le_bytes(size_bytes) as usize;

        let mut metadata_buffer = vec![0u8; metadata_size];
        reader.read_exact(&mut metadata_buffer)?;

        // Calculate checksum of data portion
        let mut checksum = 0u64;
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    checksum = checksum.wrapping_add(self.calculate_checksum(&buffer[..bytes_read]));
                }
                Err(e) => return Err(e),
            }
        }

        if checksum != metadata.checksum {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Checkpoint checksum verification failed"));
        }

        Ok(())
    }

    /// Restores data files from a checkpoint file to the data directory.
    fn restore_data_files(&self, checkpoint_path: &Path, metadata: &CheckpointMetadata) -> Result<()> {
        let file = File::open(checkpoint_path)?;
        let mut reader = BufReader::new(file);

        // Skip metadata header
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let metadata_size = u32::from_le_bytes(size_bytes) as usize;

        let mut metadata_buffer = vec![0u8; metadata_size];
        reader.read_exact(&mut metadata_buffer)?;

        // For now, we'll assume all data is concatenated in the checkpoint file
        // In a real implementation, you'd have a more sophisticated format
        let data_dir = self.layout.data_dir_path();
        let restore_path = data_dir.join(format!("restored_from_checkpoint_{}.dat", metadata.id));

        let mut output_file = File::create(restore_path)?;
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    output_file.write_all(&buffer[..bytes_read])?;
                }
                Err(e) => return Err(e),
            }
        }

        output_file.flush()?;
        Ok(())
    }

    /// Collects all data files to be included in a checkpoint.
    fn collect_data_files(&self) -> Result<Vec<FileMetadata>> {
        self.layout.list_files(FileType::Data)
    }

    /// Removes old checkpoints based on the retention policy.
    fn cleanup_old_checkpoints(&self) -> Result<()> {
        self.layout.cleanup_old_files(FileType::Checkpoint, self.config.max_checkpoints).map(|_| ())
    }

    /// Returns true if enough time has passed to trigger an automatic checkpoint.
    pub fn should_create_checkpoint(&self) -> bool {
        self.last_checkpoint_time.elapsed().unwrap_or_default() >= self.config.auto_checkpoint_interval
    }

    /// Calculates a simple checksum for the given data.
    fn calculate_checksum(&self, data: &[u8]) -> u64 {
        data.iter().map(|&b| b as u64).sum()
    }

    /// Updates the current log sequence number (LSN).
    pub fn update_lsn(&mut self, lsn: u64) {
        self.current_lsn = lsn;
    }

    /// Returns the current log sequence number (LSN).
    pub fn current_lsn(&self) -> u64 {
        self.current_lsn
    }

    /// Returns the most recent checkpoint, if any.
    pub fn get_latest_checkpoint(&self) -> Result<Option<CheckpointMetadata>> {
        let checkpoints = self.list_checkpoints()?;
        Ok(checkpoints.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FileSystemLayout, LayoutConfig};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_checkpoint_manager() -> (CheckpointManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let layout_config = LayoutConfig {
            base_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let layout = FileSystemLayout::new(layout_config).unwrap();
        let config = CheckpointConfig::default();
        let manager = CheckpointManager::new(layout, config);
        (manager, temp_dir)
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.max_checkpoints, 10);
        assert_eq!(config.auto_checkpoint_interval, std::time::Duration::from_secs(300));
        assert!(!config.compression_enabled);
        assert!(config.verification_enabled);
    }

    #[test]
    fn test_create_checkpoint() {
        let (mut manager, _temp_dir) = create_test_checkpoint_manager();

        // Create some test data files
        let data_path = manager.layout.generate_file_path(FileType::Data, 1, 1);
        let mut data_file = File::create(&data_path).unwrap();
        data_file.write_all(b"test data for checkpoint").unwrap();

        let checkpoint = manager.create_checkpoint().unwrap();
        assert_eq!(checkpoint.id, 1);
        assert_eq!(checkpoint.version, 1);
        assert!(!checkpoint.data_files.is_empty());
    }

    #[test]
    fn test_list_checkpoints() {
        let (mut manager, _temp_dir) = create_test_checkpoint_manager();

        // Create test data
        let data_path = manager.layout.generate_file_path(FileType::Data, 1, 1);
        File::create(&data_path).unwrap().write_all(b"test").unwrap();

        // Create multiple checkpoints
        let checkpoint1 = manager.create_checkpoint().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let checkpoint2 = manager.create_checkpoint().unwrap();

        let checkpoints = manager.list_checkpoints().unwrap();
        assert_eq!(checkpoints.len(), 2);

        // Should be sorted by timestamp (newest first)
        assert!(checkpoints[0].timestamp >= checkpoints[1].timestamp);
    }

    #[test]
    fn test_load_checkpoint_metadata() {
        let (mut manager, _temp_dir) = create_test_checkpoint_manager();

        let data_path = manager.layout.generate_file_path(FileType::Data, 1, 1);
        File::create(&data_path).unwrap().write_all(b"test").unwrap();

        let created_checkpoint = manager.create_checkpoint().unwrap();
        let loaded_checkpoint = manager.load_checkpoint_metadata(created_checkpoint.id).unwrap();

        assert_eq!(created_checkpoint.id, loaded_checkpoint.id);
        assert_eq!(created_checkpoint.timestamp, loaded_checkpoint.timestamp);
    }

    #[test]
    fn test_should_create_checkpoint() {
        let (manager, _temp_dir) = create_test_checkpoint_manager();

        // Initially should not need checkpoint (just created)
        assert!(!manager.should_create_checkpoint());
    }

    #[test]
    fn test_calculate_checksum() {
        let (manager, _temp_dir) = create_test_checkpoint_manager();

        let data1 = b"hello";
        let data2 = b"world";

        let checksum1 = manager.calculate_checksum(data1);
        let checksum2 = manager.calculate_checksum(data2);

        assert_ne!(checksum1, checksum2);
        assert_eq!(checksum1, manager.calculate_checksum(data1)); // Consistent
    }

    #[test]
    fn test_lsn_operations() {
        let (mut manager, _temp_dir) = create_test_checkpoint_manager();

        assert_eq!(manager.current_lsn(), 0);

        manager.update_lsn(100);
        assert_eq!(manager.current_lsn(), 100);

        manager.update_lsn(200);
        assert_eq!(manager.current_lsn(), 200);
    }

    #[test]
    fn test_get_latest_checkpoint() {
        let (mut manager, _temp_dir) = create_test_checkpoint_manager();

        // No checkpoints initially
        assert!(manager.get_latest_checkpoint().unwrap().is_none());

        // Create data and checkpoint
        let data_path = manager.layout.generate_file_path(FileType::Data, 1, 1);
        File::create(&data_path).unwrap().write_all(b"test").unwrap();

        let checkpoint = manager.create_checkpoint().unwrap();
        let latest = manager.get_latest_checkpoint().unwrap().unwrap();

        assert_eq!(checkpoint.id, latest.id);
    }

    #[test]
    fn test_checkpoint_metadata_serialization() {
        let metadata = CheckpointMetadata {
            id: 123,
            timestamp: 1640995200,
            version: 1,
            data_files: vec!["file1.dat".to_string(), "file2.dat".to_string()],
            log_sequence_number: 456,
            size: 1024,
            checksum: 789,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CheckpointMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.timestamp, deserialized.timestamp);
        assert_eq!(metadata.data_files, deserialized.data_files);
    }
}
