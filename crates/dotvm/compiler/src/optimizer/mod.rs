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

//! Optimization passes for DotVM bytecode

pub mod constant_folding;
pub mod dead_code;
pub mod peephole;

use crate::transpiler::engine::TranspiledFunction;
use dotvm_core::bytecode::VmArchitecture;

/// Main optimizer that coordinates all optimization passes
pub struct Optimizer {
    target_arch: VmArchitecture,
    peephole: peephole::PeepholeOptimizer,
    dead_code: dead_code::DeadCodeEliminator,
    constant_folder: constant_folding::ConstantFolder,
    optimization_level: u8,
}

impl Optimizer {
    /// Create a new optimizer
    pub fn new(target_arch: VmArchitecture, optimization_level: u8) -> Self {
        Self {
            target_arch,
            peephole: peephole::PeepholeOptimizer::new(target_arch),
            dead_code: dead_code::DeadCodeEliminator::new(),
            constant_folder: constant_folding::ConstantFolder::new(),
            optimization_level,
        }
    }

    /// Optimize a list of transpiled functions
    pub fn optimize(&mut self, functions: Vec<TranspiledFunction>) -> Vec<TranspiledFunction> {
        let mut optimized_functions = functions;

        match self.optimization_level {
            0 => {
                // No optimization
                optimized_functions
            }
            1 => {
                // Basic optimizations
                optimized_functions = self.dead_code.eliminate(optimized_functions);
                optimized_functions
            }
            2 => {
                // Standard optimizations
                optimized_functions = self.constant_folder.fold(optimized_functions);
                optimized_functions = self.dead_code.eliminate(optimized_functions);
                optimized_functions = self.peephole.optimize(optimized_functions);
                optimized_functions
            }
            3 => {
                // Aggressive optimizations (multiple passes)
                for _ in 0..2 {
                    optimized_functions = self.constant_folder.fold(optimized_functions);
                    optimized_functions = self.dead_code.eliminate(optimized_functions);
                    optimized_functions = self.peephole.optimize(optimized_functions);
                }
                optimized_functions
            }
            _ => {
                // Invalid optimization level, use level 2
                self.optimization_level = 2;
                self.optimize(optimized_functions)
            }
        }
    }

    /// Get combined optimization statistics
    pub fn stats(&self) -> OptimizationStats {
        OptimizationStats {
            peephole: *self.peephole.stats(),
            dead_code: *self.dead_code.stats(),
            constant_folding: *self.constant_folder.stats(),
        }
    }

    /// Reset all optimization statistics
    pub fn reset_stats(&mut self) {
        self.peephole.reset_stats();
        self.dead_code.reset_stats();
        self.constant_folder.reset_stats();
    }
}

/// Combined optimization statistics
#[derive(Debug)]
pub struct OptimizationStats {
    pub peephole: peephole::OptimizationStats,
    pub dead_code: dead_code::EliminationStats,
    pub constant_folding: constant_folding::FoldingStats,
}

impl OptimizationStats {
    /// Calculate total optimizations performed
    pub fn total_optimizations(&self) -> usize {
        self.peephole.total_optimizations() + self.dead_code.total_eliminated() + self.constant_folding.total_optimizations()
    }
}
