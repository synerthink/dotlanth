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

//! Pipeline orchestration for code generation stages

use crate::codegen::core::context::GenerationContext;
/// Pipeline coordinating various generation steps
use crate::codegen::error::{BytecodeGenerationError, BytecodeResult};
use crate::codegen::sections::traits::{SectionGenerator, SectionType};
use std::collections::{HashMap, VecDeque};

/// Pipeline coordinating section generators based on dependencies
pub struct GenerationPipeline;

impl GenerationPipeline {
    /// Create a new pipeline
    pub fn new() -> Self {
        GenerationPipeline
    }

    /// Run all section generators in dependency order.
    ///
    /// Returns a Vec of (SectionType, bytes) in generation order.
    pub fn run(&self, mut generators: Vec<Box<dyn SectionGenerator>>, context: &mut GenerationContext) -> BytecodeResult<Vec<(SectionType, Vec<u8>)>> {
        // Map section type to its generator instance
        let mut gen_map: HashMap<SectionType, Box<dyn SectionGenerator>> = HashMap::new();
        for generator in generators.drain(..) {
            let st = generator.section_type();
            if gen_map.contains_key(&st) {
                return Err(BytecodeGenerationError::ConfigurationError(format!("Duplicate generator for section {:?}", st)));
            }
            gen_map.insert(st, generator);
        }

        // Build dependency graph
        let mut in_degree: HashMap<SectionType, usize> = HashMap::new();
        let mut adj: HashMap<SectionType, Vec<SectionType>> = HashMap::new();
        for &st in gen_map.keys() {
            in_degree.insert(st, 0);
            adj.insert(st, Vec::new());
        }
        for (&st, generator) in &gen_map {
            for &dep in generator.dependencies() {
                if !gen_map.contains_key(&dep) {
                    return Err(BytecodeGenerationError::ConfigurationError(format!("Missing generator for dependency section {:?}", dep)));
                }
                adj.get_mut(&dep).unwrap().push(st);
                *in_degree.get_mut(&st).unwrap() += 1;
            }
        }

        // Topological sort (Kahn's algorithm)
        let mut queue: VecDeque<SectionType> = in_degree.iter().filter_map(|(&st, &deg)| if deg == 0 { Some(st) } else { None }).collect();
        let mut order: Vec<SectionType> = Vec::new();
        while let Some(st) = queue.pop_front() {
            order.push(st);
            for &next in &adj[&st] {
                let deg = in_degree.get_mut(&next).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(next);
                }
            }
        }
        if order.len() != gen_map.len() {
            return Err(BytecodeGenerationError::ConfigurationError("Section dependency cycle detected".into()));
        }

        // Execute generators in resolved order
        let mut sections = Vec::with_capacity(order.len());
        for st in order {
            let generator = gen_map.remove(&st).unwrap();
            let bytes = generator.generate(context)?;
            sections.push((st, bytes));
        }
        Ok(sections)
    }
}
