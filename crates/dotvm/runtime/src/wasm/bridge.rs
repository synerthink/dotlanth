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

//! WASM <-> DotVM Bridge Performance Layer
//!
//! This module provides a performance optimization layer between the WASM
//! runtime and DotVM opcode execution. It implements:
//! - Zero-copy utilities where possible (borrowing from WASM memory)
//! - A light JIT wrapper backed by the runtime JIT transpiler and caches
//! - A caching manager for expensive operations
//! - A batch processor to amortize call overhead across multiple ops
//! - A simple profiler to surface optimization guidance

use crate::wasm::{WasmError, WasmInstance, WasmMemory, WasmResult};
use dotvm_core::bytecode::{BytecodeFile, VmArchitecture};
use dotvm_core::instruction::registry::Opcode as CoreOpcode;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Technical spec compatibility types (minimal, mapped to existing core types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomOpcode(pub CoreOpcode);

#[derive(Debug, Default, Clone)]
pub struct ExecutionTrace {
    pub calls: usize,
    pub instructions: usize,
    pub duration: Duration,
}

#[derive(Debug, Default, Clone)]
pub struct PerformanceProfile {
    pub avg_call_ns: u128,
    pub avg_instructions: usize,
    pub cache_hit_rate: f64,
    pub zero_copy_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub trace: ExecutionTrace,
    pub profile: PerformanceProfile,
}

#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub description: String,
}

/// Compiled sequence cache (opaque stub for now)
#[derive(Debug, Default)]
pub struct CompiledCache {
    entries: Mutex<HashMap<u64, Vec<u8>>>,
    hits: Mutex<u64>,
    misses: Mutex<u64>,
}

impl CompiledCache {
    pub fn get(&self, key: &u64) -> Option<Vec<u8>> {
        let mut hits = self.hits.lock().unwrap();
        let mut misses = self.misses.lock().unwrap();
        let map = self.entries.lock().unwrap();
        if let Some(v) = map.get(key) {
            *hits += 1;
            Some(v.clone())
        } else {
            *misses += 1;
            None
        }
    }

    pub fn insert(&self, key: u64, value: Vec<u8>) {
        let mut map = self.entries.lock().unwrap();
        map.insert(key, value);
    }

    pub fn stats(&self) -> (u64, u64) {
        (*self.hits.lock().unwrap(), *self.misses.lock().unwrap())
    }
}

/// Hot path detector (simple frequency counter)
#[derive(Debug, Default)]
pub struct HotPathDetector {
    freq: Mutex<HashMap<u64, u64>>, // key -> call count
    hot_threshold: u64,
}

impl HotPathDetector {
    pub fn new(hot_threshold: u64) -> Self {
        Self {
            freq: Mutex::new(HashMap::new()),
            hot_threshold,
        }
    }

    pub fn observe(&self, key: u64) -> bool {
        let mut f = self.freq.lock().unwrap();
        let c = f.entry(key).or_insert(0);
        *c += 1;
        *c >= self.hot_threshold
    }
}

/// Sequence compiler (placeholder; would call the real JIT transpiler)
#[derive(Debug)]
pub struct OpcodeSequenceCompiler {
    compiled_cache: Arc<CompiledCache>,
    target_arch: VmArchitecture,
}

impl OpcodeSequenceCompiler {
    pub fn new(compiled_cache: Arc<CompiledCache>, target_arch: VmArchitecture) -> Self {
        Self { compiled_cache, target_arch }
    }

    /// Compile a sequence of core opcodes into a serialized DotVM bytecode blob (header + code)
    pub fn compile_sequence(&self, sequence_key: u64, opcodes: &[CustomOpcode]) -> Vec<u8> {
        if let Some(code) = self.compiled_cache.get(&sequence_key) {
            return code;
        }

        let mut bytecode = BytecodeFile::new(self.target_arch);
        for custom in opcodes {
            match &custom.0 {
                CoreOpcode::Arithmetic(op) => {
                    bytecode.add_instruction(op.as_u8(), &[]);
                }
                // Fast-path only: skip instructions that require operands or external context
                _ => {}
            }
        }
        // Serialize (header + code)
        let mut blob = Vec::with_capacity(dotvm_core::bytecode::BytecodeHeader::size() + bytecode.code.len());
        blob.extend_from_slice(&bytecode.header.to_bytes());
        blob.extend_from_slice(&bytecode.code);
        self.compiled_cache.insert(sequence_key, blob.clone());
        blob
    }
}

#[derive(Debug)]
pub struct JITCompiler {
    pub hot_path_detector: HotPathDetector,
    pub opcode_sequence_compiler: OpcodeSequenceCompiler,
    pub compiled_cache: Arc<CompiledCache>,
    pub target_arch: VmArchitecture,
}

impl JITCompiler {
    pub fn new(hot_threshold: u64) -> Self {
        let cache = Arc::new(CompiledCache::default());
        let target_arch = VmArchitecture::Arch64;
        Self {
            hot_path_detector: HotPathDetector::new(hot_threshold),
            opcode_sequence_compiler: OpcodeSequenceCompiler::new(cache.clone(), target_arch),
            compiled_cache: cache,
            target_arch,
        }
    }

    pub fn with_architecture(hot_threshold: u64, arch: VmArchitecture) -> Self {
        let cache = Arc::new(CompiledCache::default());
        Self {
            hot_path_detector: HotPathDetector::new(hot_threshold),
            opcode_sequence_compiler: OpcodeSequenceCompiler::new(cache.clone(), arch),
            compiled_cache: cache,
            target_arch: arch,
        }
    }
}

#[derive(Debug, Default)]
pub struct CacheManager {
    map: Mutex<HashMap<u64, Vec<u8>>>,
    pub hits: Mutex<u64>,
    pub misses: Mutex<u64>,
}

impl CacheManager {
    pub fn get_or_insert_with<F: FnOnce() -> Vec<u8>>(&self, key: u64, f: F) -> Vec<u8> {
        let mut map = self.map.lock().unwrap();
        if let Some(v) = map.get(&key) {
            *self.hits.lock().unwrap() += 1;
            return v.clone();
        }
        *self.misses.lock().unwrap() += 1;
        let v = f();
        map.insert(key, v.clone());
        v
    }

    pub fn hit_rate(&self) -> f64 {
        let h = *self.hits.lock().unwrap();
        let m = *self.misses.lock().unwrap();
        if h + m == 0 { 0.0 } else { h as f64 / (h + m) as f64 }
    }
}

#[derive(Debug, Default)]
pub struct MemoryOptimizer {
    // Simple reusable buffer pool
    pool: Mutex<Vec<Vec<u8>>>,
    pub allocations: Mutex<u64>,
}

impl MemoryOptimizer {
    pub fn take_buffer(&self, min_len: usize) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap();
        if let Some(mut buf) = pool.pop() {
            if buf.capacity() < min_len {
                buf.reserve(min_len - buf.capacity());
            }
            buf.clear();
            buf
        } else {
            *self.allocations.lock().unwrap() += 1;
            Vec::with_capacity(min_len)
        }
    }

    pub fn give_back(&self, mut buf: Vec<u8>) {
        buf.clear();
        self.pool.lock().unwrap().push(buf);
    }
}

#[derive(Debug, Default)]
pub struct BatchProcessor {
    pub processed_ops: Mutex<u64>,
}

impl BatchProcessor {
    /// Execute a batch of simple arithmetic operations provided in WASM memory
    /// Layout per op: [op:u8][a:i32][b:i32] -> result i32 (stored back-to-back into out buffer)
    pub fn execute_batch_zero_copy(&self, memory: &WasmMemory, ops_ptr: usize, op_count: usize) -> WasmResult<Vec<i32>> {
        let bytes = memory.read_bytes(ops_ptr, op_count * (1 + 4 + 4))?; // zero-copy borrow
        let mut results = Vec::with_capacity(op_count);

        for i in 0..op_count {
            let base = i * 9;
            let op = bytes[base];
            let a = i32::from_le_bytes([bytes[base + 1], bytes[base + 2], bytes[base + 3], bytes[base + 4]]);
            let b = i32::from_le_bytes([bytes[base + 5], bytes[base + 6], bytes[base + 7], bytes[base + 8]]);
            let r = match op {
                0 => a + b,
                1 => a - b,
                2 => a * b,
                3 => {
                    if b != 0 {
                        a / b
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            results.push(r);
        }

        *self.processed_ops.lock().unwrap() += op_count as u64;
        Ok(results)
    }
}

#[derive(Debug)]
pub struct PerformanceProfiler {
    samples: Mutex<Vec<Duration>>,
    overhead_count: Mutex<u64>,
    instruction_counts: Mutex<Vec<usize>>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::new()),
            overhead_count: Mutex::new(0),
            instruction_counts: Mutex::new(Vec::new()),
        }
    }

    pub fn measure<T, F: FnOnce() -> T>(&self, f: F) -> T {
        let t0 = Instant::now();
        let out = f();
        self.samples.lock().unwrap().push(t0.elapsed());
        out
    }

    pub fn add_overhead(&self) {
        *self.overhead_count.lock().unwrap() += 1;
    }

    pub fn add_instruction_count(&self, n: usize) {
        self.instruction_counts.lock().unwrap().push(n);
    }

    pub fn build_profile(&self) -> PerformanceProfile {
        let samples = self.samples.lock().unwrap();
        let total = samples.iter().map(|d| d.as_nanos()).sum::<u128>();
        let avg = if samples.is_empty() { 0 } else { total / samples.len() as u128 };
        let instrs = self.instruction_counts.lock().unwrap();
        let avg_instr = if instrs.is_empty() { 0 } else { instrs.iter().sum::<usize>() / instrs.len() };

        PerformanceProfile {
            avg_call_ns: avg,
            avg_instructions: avg_instr,
            cache_hit_rate: 0.0,
            zero_copy_ratio: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct PerformanceOptimizer {
    pub jit_compiler: JITCompiler,
    pub cache_manager: CacheManager,
    pub profiler: PerformanceProfiler,
    pub memory_optimizer: MemoryOptimizer,
    pub batch_processor: BatchProcessor,
}

impl PerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            jit_compiler: JITCompiler::new(8), // detect hot sequence after 8 hits
            cache_manager: CacheManager::default(),
            profiler: PerformanceProfiler::new(),
            memory_optimizer: MemoryOptimizer::default(),
            batch_processor: BatchProcessor::default(),
        }
    }

    pub fn with_architecture(arch: VmArchitecture) -> Self {
        Self {
            jit_compiler: JITCompiler::with_architecture(8, arch),
            cache_manager: CacheManager::default(),
            profiler: PerformanceProfiler::new(),
            memory_optimizer: MemoryOptimizer::default(),
            batch_processor: BatchProcessor::default(),
        }
    }

    /// Optimize a single opcode call (stubbed behavior)
    pub fn optimize_call(&self, opcode: CustomOpcode, _context: &dotvm_core::vm::executor::ExecutionContext) -> OptimizedCall {
        // Hash sequence key by opcode discriminant
        let key = Self::opcode_key(&opcode);
        let hot = self.jit_compiler.hot_path_detector.observe(key);
        let compiled = if hot {
            Some(self.jit_compiler.opcode_sequence_compiler.compile_sequence(key, &[opcode]))
        } else {
            None
        };
        OptimizedCall { is_hot: hot, compiled_blob: compiled }
    }

    /// Profile an execution trace and return a report
    pub fn profile_execution(&self, execution_trace: &ExecutionTrace) -> PerformanceReport {
        let mut trace = execution_trace.clone();
        // Record into profiler stats as a sample
        self.profiler.instruction_counts.lock().unwrap().push(trace.instructions);
        self.profiler.samples.lock().unwrap().push(trace.duration);
        // Approximate zero-copy ratio using allocation counts (synthetic)
        let allocs = *self.memory_optimizer.allocations.lock().unwrap();
        let zero_copy_ratio = if trace.calls == 0 {
            0.0
        } else {
            (trace.calls as f64).max(1.0) / (trace.calls as f64 + allocs as f64)
        };
        let mut profile = self.profiler.build_profile();
        profile.zero_copy_ratio = zero_copy_ratio;
        profile.cache_hit_rate = self.cache_manager.hit_rate();
        PerformanceReport { trace, profile }
    }

    /// Suggest basic optimizations based on a profile
    pub fn suggest_optimizations(&self, profile: &PerformanceProfile) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        if profile.avg_call_ns > 2000 {
            // >2us
            suggestions.push(OptimizationSuggestion {
                description: "Consider batching small ops to amortize call overhead".to_string(),
            });
        }
        if profile.cache_hit_rate < 0.5 {
            suggestions.push(OptimizationSuggestion {
                description: "Enable caching for repeated expensive operations".to_string(),
            });
        }
        if profile.zero_copy_ratio < 0.6 {
            suggestions.push(OptimizationSuggestion {
                description: "Adopt zero-copy reads from WASM memory where applicable".to_string(),
            });
        }
        suggestions
    }

    fn opcode_key(op: &CustomOpcode) -> u64 {
        // Poor man's hash using discriminant via Debug formatting
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", op).hash(&mut h);
        h.finish()
    }
}

#[derive(Debug, Clone)]
pub struct OptimizedCall {
    pub is_hot: bool,
    pub compiled_blob: Option<Vec<u8>>, // placeholder compiled representation
}

/// Register default bridge host functions on a WASM instance.
/// These functions demonstrate zero-copy and batched operation handling.
pub fn register_default_bridge_host_functions(instance: &mut WasmInstance) {
    // Wire the optimized variant
    let optimizer = Arc::new(PerformanceOptimizer::new());
    wire_instance_batch_with_optimizer(instance, optimizer.clone());

    // Also provide a non-suffixed alias for compatibility: dotvm_bridge.batch_arith
    let fn_name = "dotvm_bridge.batch_arith".to_string();
    let mem_arc = instance.memory.as_ref().cloned();
    instance.host_functions.insert(
        fn_name.clone(),
        Box::new(move |vals: Vec<dotvm_compiler::wasm::ast::WasmValue>| {
            if vals.len() != 2 {
                return Err(WasmError::execution_error("batch_arith expects 2 args (ptr, count)"));
            }
            let ptr = match vals[0] {
                dotvm_compiler::wasm::ast::WasmValue::I32(p) => p as usize,
                _ => 0,
            };
            let count = match vals[1] {
                dotvm_compiler::wasm::ast::WasmValue::I32(c) => c as usize,
                _ => 0,
            };

            let t0 = Instant::now();
            optimizer.profiler.add_overhead();

            let results: Vec<i32> = if let Some(mem) = &mem_arc {
                let mem = mem.read().map_err(|_| WasmError::execution_error("memory poisoned"))?;
                optimizer.batch_processor.execute_batch_zero_copy(&mem, ptr, count)?
            } else {
                vec![]
            };

            let duration = t0.elapsed();
            optimizer.profiler.samples.lock().unwrap().push(duration);

            Ok(results.into_iter().map(|r| dotvm_compiler::wasm::ast::WasmValue::I32(r)).collect())
        }),
    );
}

/// Wire an optimizer-aware batch function using the provided optimizer and instance memory
pub fn wire_instance_batch_with_optimizer(instance: &mut WasmInstance, optimizer: Arc<PerformanceOptimizer>) {
    let fn_name = "dotvm_bridge.batch_arith_optimized".to_string();
    // Capture a weak reference to the instance memory via Arc<RwLock<>> inside the instance
    let mem_arc = instance.memory.as_ref().cloned();

    instance.host_functions.insert(
        fn_name.clone(),
        Box::new(move |vals: Vec<dotvm_compiler::wasm::ast::WasmValue>| {
            if vals.len() != 2 {
                return Err(WasmError::execution_error("batch_arith_optimized expects 2 args (ptr, count)"));
            }
            let ptr = match vals[0] {
                dotvm_compiler::wasm::ast::WasmValue::I32(p) => p as usize,
                _ => 0,
            };
            let count = match vals[1] {
                dotvm_compiler::wasm::ast::WasmValue::I32(c) => c as usize,
                _ => 0,
            };

            let t0 = Instant::now();
            optimizer.profiler.add_overhead();

            let results: Vec<i32> = if let Some(mem) = &mem_arc {
                let mem = mem.read().map_err(|_| WasmError::execution_error("memory poisoned"))?;
                optimizer.batch_processor.execute_batch_zero_copy(&mem, ptr, count)?
            } else {
                vec![]
            };

            let duration = t0.elapsed();
            optimizer.profiler.samples.lock().unwrap().push(duration);

            Ok(results.into_iter().map(|r| dotvm_compiler::wasm::ast::WasmValue::I32(r)).collect())
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::{InstanceState, WasmModule};
    use std::sync::Arc;

    fn make_instance_with_memory(mem_pages: u32) -> WasmInstance {
        let module = WasmModule::new("test".to_string(), vec![0, 0, 0, 0], dotvm_compiler::wasm::WasmModule::default());
        let mut instance = WasmInstance::new(Arc::new(module), crate::wasm::management::SecurityContext::default()).unwrap();
        if mem_pages > 0 {
            instance.memory = Some(Arc::new(RwLock::new(WasmMemory::new(mem_pages, Some(mem_pages * 2)).unwrap())));
        }
        instance.state = InstanceState::Ready;
        instance
    }

    #[test]
    fn test_memory_optimizer_allocation_reuse() {
        let opt = PerformanceOptimizer::new();
        let buf1 = opt.memory_optimizer.take_buffer(1024);
        // Give back to enable reuse
        opt.memory_optimizer.give_back(buf1);
        assert_eq!(*opt.memory_optimizer.allocations.lock().unwrap(), 1);

        // Reuse from pool, should not increase allocations
        let mut buf2 = opt.memory_optimizer.take_buffer(512);
        buf2.extend_from_slice(&[1, 2, 3]);
        opt.memory_optimizer.give_back(buf2);
        let _buf3 = opt.memory_optimizer.take_buffer(256);
        // No new allocation expected due to reuse
        assert_eq!(*opt.memory_optimizer.allocations.lock().unwrap(), 1);
    }

    #[test]
    fn test_cache_manager_hit_rate() {
        let cm = CacheManager::default();
        let k = 42u64;
        // miss then hit
        let _ = cm.get_or_insert_with(k, || vec![1]);
        let _ = cm.get_or_insert_with(k, || vec![2]);
        assert!(cm.hit_rate() > 0.0);
    }

    #[test]
    fn test_jit_hot_path_compilation() {
        let opt = PerformanceOptimizer::new();
        let dummy_ctx = dotvm_core::vm::executor::ExecutionContext::new();
        let opcode = CustomOpcode(CoreOpcode::Arithmetic(dotvm_core::opcode::arithmetic_opcodes::ArithmeticOpcode::Add));
        let mut was_hot = false;
        for _ in 0..9 {
            let res = opt.optimize_call(opcode, &dummy_ctx);
            was_hot |= res.is_hot;
        }
        assert!(was_hot, "hot path should be detected after threshold");
    }

    #[test]
    fn test_batch_processor_zero_copy_and_throughput() {
        let mut instance = make_instance_with_memory(1);
        let opt = Arc::new(PerformanceOptimizer::new());
        wire_instance_batch_with_optimizer(&mut instance, opt.clone());

        // Prepare 100 ops into memory: op=0 (add), a=i, b=i
        let ops_count = 100;
        let total_bytes = ops_count * 9;
        let ptr = 0usize;
        {
            let mut mem = instance.memory.as_ref().unwrap().write().unwrap();
            let raw = mem.raw_data_mut();
            assert!(raw.len() >= total_bytes);
            for i in 0..ops_count {
                let base = i * 9;
                raw[base] = 0; // add
                raw[base + 1..base + 5].copy_from_slice(&(i as i32).to_le_bytes());
                raw[base + 5..base + 9].copy_from_slice(&(i as i32).to_le_bytes());
            }
        }

        // Call host function
        let hf = instance.host_functions.get("dotvm_bridge.batch_arith_optimized").unwrap();
        let res = hf(vec![dotvm_compiler::wasm::ast::WasmValue::I32(ptr as i32), dotvm_compiler::wasm::ast::WasmValue::I32(ops_count as i32)]).unwrap();
        assert_eq!(res.len(), ops_count);
        // Validate some results
        if let dotvm_compiler::wasm::ast::WasmValue::I32(v) = &res[10] {
            assert_eq!(*v, 20);
        } else {
            panic!("bad type");
        }

        // Ensure batch processor counted ops
        assert_eq!(*opt.batch_processor.processed_ops.lock().unwrap(), ops_count as u64);
    }
}
