use std::fs;
use std::io::{self, Result};
use std::path::{Path, PathBuf};

/// Configuration for file system layout, including directory names and file size limits.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub base_path: PathBuf,
    pub data_dir: String,
    pub log_dir: String,
    pub index_dir: String,
    pub metadata_dir: String,
    pub checkpoint_dir: String,
    pub max_file_size: u64,
    pub file_prefix: String,
}

impl Default for LayoutConfig {
    /// Returns a default layout configuration for the file system.
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./dotdb"),
            data_dir: "data".to_string(),
            log_dir: "logs".to_string(),
            index_dir: "indexes".to_string(),
            metadata_dir: "metadata".to_string(),
            checkpoint_dir: "checkpoints".to_string(),
            max_file_size: 100 * 1024 * 1024, // 100MB
            file_prefix: "dotdb".to_string(),
        }
    }
}

/// Enum representing the different file types managed by the storage engine.
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Data,
    Log,
    Index,
    Metadata,
    Checkpoint,
}

impl FileType {
    /// Returns the file extension associated with the file type.
    pub fn extension(&self) -> &'static str {
        match self {
            FileType::Data => "dat",
            FileType::Log => "log",
            FileType::Index => "idx",
            FileType::Metadata => "meta",
            FileType::Checkpoint => "ckpt",
        }
    }
}

/// Metadata describing a file managed by the storage engine.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub id: u64,
    pub file_type: FileType,
    pub version: u32,
    pub size: u64,
    pub created_at: std::time::SystemTime,
    pub path: PathBuf,
}

/// Manages the layout and organization of files and directories for the storage engine.
pub struct FileSystemLayout {
    config: LayoutConfig,
}

impl FileSystemLayout {
    /// Creates a new FileSystemLayout and initializes all required directories.
    pub fn new(config: LayoutConfig) -> Result<Self> {
        let layout = Self { config };
        layout.initialize_directories()?;
        Ok(layout)
    }

    /// Ensures all required directories exist, creating them if necessary.
    fn initialize_directories(&self) -> Result<()> {
        let dirs = [
            &self.config.data_dir,
            &self.config.log_dir,
            &self.config.index_dir,
            &self.config.metadata_dir,
            &self.config.checkpoint_dir,
        ];

        for dir in &dirs {
            let path = self.config.base_path.join(dir);
            fs::create_dir_all(&path)?;
        }

        Ok(())
    }

    /// Generates a new file path for the given file type, ID, and version.
    pub fn generate_file_path(&self, file_type: FileType, id: u64, version: u32) -> PathBuf {
        let dir = match file_type {
            FileType::Data => &self.config.data_dir,
            FileType::Log => &self.config.log_dir,
            FileType::Index => &self.config.index_dir,
            FileType::Metadata => &self.config.metadata_dir,
            FileType::Checkpoint => &self.config.checkpoint_dir,
        };

        let filename = format!("{}_{:08}_{:04}.{}", self.config.file_prefix, id, version, file_type.extension());

        self.config.base_path.join(dir).join(filename)
    }

    /// Lists all files of a specific type, returning their metadata.
    pub fn list_files(&self, file_type: FileType) -> Result<Vec<FileMetadata>> {
        let dir = match file_type {
            FileType::Data => &self.config.data_dir,
            FileType::Log => &self.config.log_dir,
            FileType::Index => &self.config.index_dir,
            FileType::Metadata => &self.config.metadata_dir,
            FileType::Checkpoint => &self.config.checkpoint_dir,
        };

        let dir_path = self.config.base_path.join(dir);
        let mut files = Vec::new();

        if !dir_path.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(metadata) = self.parse_file_metadata(&path, file_type.clone())? {
                    files.push(metadata);
                }
            }
        }

        // Sort by ID and version
        files.sort_by(|a, b| a.id.cmp(&b.id).then(a.version.cmp(&b.version)));
        Ok(files)
    }

    /// Parses file metadata from a file path, extracting ID, version, and other attributes.
    fn parse_file_metadata(&self, path: &Path, file_type: FileType) -> Result<Option<FileMetadata>> {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return Ok(None),
        };

        // Expected format: prefix_id_version.extension
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() < 3 {
            return Ok(None);
        }

        let id_str = parts[1];
        let version_ext = parts[2];
        let version_parts: Vec<&str> = version_ext.split('.').collect();

        if version_parts.len() != 2 {
            return Ok(None);
        }

        let id: u64 = id_str.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid file ID"))?;

        let version: u32 = version_parts[0].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid file version"))?;

        let metadata = fs::metadata(path)?;
        let created_at = metadata.created().unwrap_or_else(|_| std::time::SystemTime::now());

        Ok(Some(FileMetadata {
            id,
            file_type,
            version,
            size: metadata.len(),
            created_at,
            path: path.to_path_buf(),
        }))
    }

    /// Returns true if the file exceeds the configured maximum file size and should be compacted.
    pub fn should_compact_file(&self, metadata: &FileMetadata) -> bool {
        metadata.size > self.config.max_file_size
    }

    /// Returns the next available file ID for a given file type.
    pub fn next_file_id(&self, file_type: FileType) -> Result<u64> {
        let files = self.list_files(file_type)?;
        Ok(files.iter().map(|f| f.id).max().unwrap_or(0) + 1)
    }

    /// Returns the path to the data directory.
    pub fn data_dir_path(&self) -> PathBuf {
        self.config.base_path.join(&self.config.data_dir)
    }

    /// Removes old files of a given type, retaining only the specified number of most recent files.
    /// Returns the number of files removed.
    pub fn cleanup_old_files(&self, file_type: FileType, retain_count: usize) -> Result<usize> {
        let mut files = self.list_files(file_type)?;

        if files.len() <= retain_count {
            return Ok(0);
        }

        // Sort by creation time (oldest first)
        files.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let to_remove = files.len() - retain_count;
        let mut removed = 0;

        for file in files.iter().take(to_remove) {
            if fs::remove_file(&file.path).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_layout() -> (FileSystemLayout, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = LayoutConfig {
            base_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let layout = FileSystemLayout::new(config).unwrap();
        (layout, temp_dir)
    }

    #[test]
    fn test_initialize_directories() {
        let (layout, _temp_dir) = create_test_layout();

        let dirs = ["data", "logs", "indexes", "metadata", "checkpoints"];
        for dir in &dirs {
            let path = layout.config.base_path.join(dir);
            assert!(path.exists());
            assert!(path.is_dir());
        }
    }

    #[test]
    fn test_generate_file_path() {
        let (layout, _temp_dir) = create_test_layout();

        let path = layout.generate_file_path(FileType::Data, 123, 1);
        let expected = layout.config.base_path.join("data").join("dotdb_00000123_0001.dat");

        assert_eq!(path, expected);
    }

    #[test]
    fn test_file_type_extension() {
        assert_eq!(FileType::Data.extension(), "dat");
        assert_eq!(FileType::Log.extension(), "log");
        assert_eq!(FileType::Index.extension(), "idx");
        assert_eq!(FileType::Metadata.extension(), "meta");
        assert_eq!(FileType::Checkpoint.extension(), "ckpt");
    }

    #[test]
    fn test_list_files() {
        let (layout, _temp_dir) = create_test_layout();

        // Create some test files
        let file1_path = layout.generate_file_path(FileType::Data, 1, 1);
        let file2_path = layout.generate_file_path(FileType::Data, 2, 1);

        File::create(&file1_path).unwrap().write_all(b"test1").unwrap();
        File::create(&file2_path).unwrap().write_all(b"test2").unwrap();

        let files = layout.list_files(FileType::Data).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 1);
        assert_eq!(files[1].id, 2);
    }

    #[test]
    fn test_next_file_id() {
        let (layout, _temp_dir) = create_test_layout();

        // No files exist, should return 1
        assert_eq!(layout.next_file_id(FileType::Data).unwrap(), 1);

        // Create a file with id 5
        let file_path = layout.generate_file_path(FileType::Data, 5, 1);
        File::create(&file_path).unwrap();

        // Should return 6
        assert_eq!(layout.next_file_id(FileType::Data).unwrap(), 6);
    }

    #[test]
    fn test_should_compact_file() {
        let (layout, _temp_dir) = create_test_layout();

        let small_metadata = FileMetadata {
            id: 1,
            file_type: FileType::Data,
            version: 1,
            size: 1024,
            created_at: std::time::SystemTime::now(),
            path: PathBuf::new(),
        };

        let large_metadata = FileMetadata {
            id: 2,
            file_type: FileType::Data,
            version: 1,
            size: layout.config.max_file_size + 1,
            created_at: std::time::SystemTime::now(),
            path: PathBuf::new(),
        };

        assert!(!layout.should_compact_file(&small_metadata));
        assert!(layout.should_compact_file(&large_metadata));
    }

    #[test]
    fn test_cleanup_old_files() {
        let (layout, _temp_dir) = create_test_layout();

        // Create 5 test files
        for i in 1..=5 {
            let file_path = layout.generate_file_path(FileType::Data, i, 1);
            File::create(&file_path).unwrap();
            // Small delay to ensure different creation times
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Keep only 3 files, should remove 2
        let removed = layout.cleanup_old_files(FileType::Data, 3).unwrap();
        assert_eq!(removed, 2);

        let remaining_files = layout.list_files(FileType::Data).unwrap();
        assert_eq!(remaining_files.len(), 3);
    }
}
