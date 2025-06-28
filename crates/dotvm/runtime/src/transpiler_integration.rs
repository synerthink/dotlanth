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

//! Runtime integration with the transpiler
//!
//! This module provides just-in-time transpilation, bytecode caching,
//! and hot reloading capabilities for development.

use dotvm_compiler::{
    codegen::dotvm_generator::DotVMGenerator,
    transpiler::engine::TranspilationEngine,
    wasm::{ast::WasmModule, parser::WasmParser},
};
use dotvm_core::bytecode::VmArchitecture;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tokio::sync::Mutex;

/// Just-in-time transpiler for runtime bytecode generation
pub struct JitTranspiler {
    /// Target VM architecture
    target_arch: VmArchitecture,
    /// Bytecode cache
    cache: Arc<RwLock<BytecodeCache>>,
    /// Wasm parser
    wasm_parser: WasmParser,
    /// Transpiler engine
    transpiler: Arc<Mutex<TranspilationEngine>>,
    /// Bytecode generator
    generator: Arc<Mutex<DotVMGenerator>>,
}

impl JitTranspiler {
    /// Create a new JIT transpiler
    pub fn new(target_arch: VmArchitecture) -> Self {
        Self {
            target_arch,
            cache: Arc::new(RwLock::new(BytecodeCache::new())),
            wasm_parser: WasmParser::new(),
            transpiler: Arc::new(Mutex::new(TranspilationEngine::with_architecture(target_arch))),
            generator: Arc::new(Mutex::new(DotVMGenerator::with_architecture(target_arch))),
        }
    }

    /// Transpile Wasm bytecode to DotVM bytecode with caching
    pub async fn transpile_wasm(&self, wasm_bytes: &[u8]) -> Result<Vec<u8>, JitError> {
        // Calculate hash for cache key
        let cache_key = self.calculate_hash(wasm_bytes);

        // Check cache first
        if let Some(cached_bytecode) = self.get_cached_bytecode(&cache_key) {
            return Ok(cached_bytecode);
        }

        // Parse Wasm
        let wasm_module = self.wasm_parser.parse(wasm_bytes).map_err(|e| JitError::WasmParsing(format!("Failed to parse Wasm: {:?}", e)))?;

        // Transpile to DotVM
        let bytecode = self.transpile_module(wasm_module).await?;

        // Cache the result
        self.cache_bytecode(cache_key, bytecode.clone());

        Ok(bytecode)
    }

    /// Transpile from Wasm file with caching
    pub async fn transpile_wasm_file<P: AsRef<Path>>(&self, wasm_path: P) -> Result<Vec<u8>, JitError> {
        let wasm_path = wasm_path.as_ref();

        // Check if we have a cached version that's newer than the file
        if let Some(cached_bytecode) = self.get_cached_file_bytecode(wasm_path)? {
            return Ok(cached_bytecode);
        }

        // Read and transpile the file
        let wasm_bytes = fs::read(wasm_path).map_err(|e| JitError::FileSystem(format!("Cannot read Wasm file: {}", e)))?;

        let bytecode = self.transpile_wasm(&wasm_bytes).await?;

        // Cache with file metadata
        self.cache_file_bytecode(wasm_path, bytecode.clone())?;

        Ok(bytecode)
    }

    /// Transpile a Wasm module to DotVM bytecode
    async fn transpile_module(&self, wasm_module: WasmModule) -> Result<Vec<u8>, JitError> {
        // Convert WasmModule to bytes (simplified approach)
        let wasm_bytes = serde_json::to_vec(&wasm_module).map_err(|e| JitError::Transpilation(format!("Failed to serialize Wasm module: {:?}", e)))?;

        // Transpile Wasm to intermediate representation
        let mut transpiler = self.transpiler.lock().await;
        let transpiled_module = transpiler.transpile(&wasm_bytes).map_err(|e| JitError::Transpilation(format!("Transpilation failed: {:?}", e)))?;

        // Generate bytecode
        let mut generator = self.generator.lock().await;
        let generated = generator
            .generate(&transpiled_module)
            .map_err(|e| JitError::BytecodeGeneration(format!("Bytecode generation failed: {:?}", e)))?;

        let bytecode = generated.bytecode;

        Ok(bytecode)
    }

    /// Calculate hash for cache key
    fn calculate_hash(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        (self.target_arch as u8).hash(&mut hasher); // Convert to u8 for hashing
        hasher.finish()
    }

    /// Get cached bytecode
    fn get_cached_bytecode(&self, cache_key: &u64) -> Option<Vec<u8>> {
        let cache = self.cache.read().ok()?;
        cache.get_bytecode(cache_key).cloned()
    }

    /// Cache bytecode
    fn cache_bytecode(&self, cache_key: u64, bytecode: Vec<u8>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert_bytecode(cache_key, bytecode);
        }
    }

    /// Get cached bytecode for a file (if newer than file modification time)
    fn get_cached_file_bytecode(&self, file_path: &Path) -> Result<Option<Vec<u8>>, JitError> {
        let file_metadata = fs::metadata(file_path).map_err(|e| JitError::FileSystem(format!("Cannot read file metadata: {}", e)))?;

        let file_modified = file_metadata.modified().map_err(|e| JitError::FileSystem(format!("Cannot get file modification time: {}", e)))?;

        let cache = self.cache.read().map_err(|_| JitError::CacheLock("Cannot acquire cache read lock".to_string()))?;

        if let Some(cached_entry) = cache.get_file_entry(file_path) {
            if cached_entry.file_modified >= file_modified {
                return Ok(Some(cached_entry.bytecode.clone()));
            }
        }

        Ok(None)
    }

    /// Cache bytecode for a file
    fn cache_file_bytecode(&self, file_path: &Path, bytecode: Vec<u8>) -> Result<(), JitError> {
        let file_metadata = fs::metadata(file_path).map_err(|e| JitError::FileSystem(format!("Cannot read file metadata: {}", e)))?;

        let file_modified = file_metadata.modified().map_err(|e| JitError::FileSystem(format!("Cannot get file modification time: {}", e)))?;

        let mut cache = self.cache.write().map_err(|_| JitError::CacheLock("Cannot acquire cache write lock".to_string()))?;

        cache.insert_file_entry(
            file_path.to_path_buf(),
            CachedFileEntry {
                bytecode,
                file_modified,
                cached_at: SystemTime::now(),
            },
        );

        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.read() { cache.stats() } else { CacheStats::default() }
    }
}

/// Bytecode cache for JIT compilation
#[derive(Debug)]
struct BytecodeCache {
    /// Hash-based cache for raw bytecode
    bytecode_cache: HashMap<u64, Vec<u8>>,
    /// File-based cache with modification time tracking
    file_cache: HashMap<PathBuf, CachedFileEntry>,
    /// Cache statistics
    hits: u64,
    misses: u64,
}

impl BytecodeCache {
    fn new() -> Self {
        Self {
            bytecode_cache: HashMap::new(),
            file_cache: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    fn get_bytecode(&self, key: &u64) -> Option<&Vec<u8>> {
        self.bytecode_cache.get(key)
    }

    fn get_bytecode_mut(&mut self, key: &u64) -> Option<&Vec<u8>> {
        if let Some(bytecode) = self.bytecode_cache.get(key) {
            self.hits += 1;
            Some(bytecode)
        } else {
            self.misses += 1;
            None
        }
    }

    fn insert_bytecode(&mut self, key: u64, bytecode: Vec<u8>) {
        self.bytecode_cache.insert(key, bytecode);
    }

    fn get_file_entry(&self, path: &Path) -> Option<&CachedFileEntry> {
        self.file_cache.get(path)
    }

    fn get_file_entry_mut(&mut self, path: &Path) -> Option<&CachedFileEntry> {
        if let Some(entry) = self.file_cache.get(path) {
            self.hits += 1;
            Some(entry)
        } else {
            self.misses += 1;
            None
        }
    }

    fn insert_file_entry(&mut self, path: PathBuf, entry: CachedFileEntry) {
        self.file_cache.insert(path, entry);
    }

    fn clear(&mut self) {
        self.bytecode_cache.clear();
        self.file_cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            bytecode_entries: self.bytecode_cache.len(),
            file_entries: self.file_cache.len(),
        }
    }
}

/// Cached file entry with metadata
#[derive(Debug, Clone)]
struct CachedFileEntry {
    bytecode: Vec<u8>,
    file_modified: SystemTime,
    cached_at: SystemTime,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub bytecode_entries: usize,
    pub file_entries: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 { 0.0 } else { self.hits as f64 / (self.hits + self.misses) as f64 }
    }
}

/// Hot reloader for development
pub struct HotReloader {
    jit_transpiler: Arc<JitTranspiler>,
    watched_files: Arc<RwLock<HashMap<PathBuf, SystemTime>>>,
}

impl HotReloader {
    /// Create a new hot reloader
    pub fn new(target_arch: VmArchitecture) -> Self {
        Self {
            jit_transpiler: Arc::new(JitTranspiler::new(target_arch)),
            watched_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a file to watch for changes
    pub fn watch_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), JitError> {
        let file_path = file_path.as_ref().to_path_buf();
        let metadata = fs::metadata(&file_path).map_err(|e| JitError::FileSystem(format!("Cannot read file metadata: {}", e)))?;

        let modified = metadata.modified().map_err(|e| JitError::FileSystem(format!("Cannot get file modification time: {}", e)))?;

        if let Ok(mut watched) = self.watched_files.write() {
            watched.insert(file_path, modified);
        }

        Ok(())
    }

    /// Check for file changes and reload if necessary
    pub async fn check_and_reload(&self) -> Result<Vec<PathBuf>, JitError> {
        let mut reloaded_files = Vec::new();

        let watched_files = {
            let watched = self.watched_files.read().map_err(|_| JitError::CacheLock("Cannot acquire watched files read lock".to_string()))?;
            watched.clone()
        };

        for (file_path, last_modified) in watched_files {
            if let Ok(metadata) = fs::metadata(&file_path) {
                if let Ok(current_modified) = metadata.modified() {
                    if current_modified > last_modified {
                        // File has been modified, reload it
                        self.jit_transpiler.transpile_wasm_file(&file_path).await?;
                        reloaded_files.push(file_path.clone());

                        // Update the modification time
                        if let Ok(mut watched) = self.watched_files.write() {
                            watched.insert(file_path, current_modified);
                        }
                    }
                }
            }
        }

        Ok(reloaded_files)
    }

    /// Get the underlying JIT transpiler
    pub fn jit_transpiler(&self) -> &JitTranspiler {
        &self.jit_transpiler
    }
}

/// JIT transpilation errors
#[derive(Debug, thiserror::Error)]
pub enum JitError {
    #[error("Wasm parsing failed: {0}")]
    WasmParsing(String),

    #[error("Transpilation failed: {0}")]
    Transpilation(String),

    #[error("Bytecode generation failed: {0}")]
    BytecodeGeneration(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Cache lock error: {0}")]
    CacheLock(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_jit_transpiler_creation() {
        let transpiler = JitTranspiler::new(VmArchitecture::Arch64);
        assert!(matches!(transpiler.target_arch, VmArchitecture::Arch64));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let transpiler = JitTranspiler::new(VmArchitecture::Arch64);
        let stats = transpiler.cache_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_hot_reloader_creation() {
        let reloader = HotReloader::new(VmArchitecture::Arch128);
        assert!(matches!(reloader.jit_transpiler.target_arch, VmArchitecture::Arch128));
    }

    #[tokio::test]
    async fn test_hot_reloader_watch_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.wasm");
        fs::write(&test_file, b"test content").await.unwrap();

        let reloader = HotReloader::new(VmArchitecture::Arch64);
        let result = reloader.watch_file(&test_file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut cache = BytecodeCache::new();
        cache.hits = 7;
        cache.misses = 3;
        let stats = cache.stats();
        assert_eq!(stats.hit_rate(), 0.7);
    }
}
