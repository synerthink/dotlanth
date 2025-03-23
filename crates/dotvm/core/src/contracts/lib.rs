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

//! Main contract processing pipeline
//!
//! Coordinates splitting, validation, dependency resolution, and scheduling
//!
use super::{ContractSegment, DependencyResolver, ProcessingError, SchedulingAlgorithm, SchedulingStrategy, SegmentExtractor, Validator};

/// Represents a contract that can be split into segments
#[derive(Debug, Clone)]
pub struct Contract {
    pub id: String,
    pub content: String,
}
/// Complete contract processing pipeline
pub struct ContractProcessor {
    splitter: SegmentExtractor,
    resolver: DependencyResolver,
    validator: Validator,
    scheduler: SchedulingAlgorithm,
}

impl ContractProcessor {
    /// Initializes processor with default components:
    /// - Segment extractor
    /// - Dependency resolver
    /// - Validator
    /// - Topological scheduler
    pub fn new() -> Self {
        Self {
            splitter: SegmentExtractor::new(),
            resolver: DependencyResolver::new(),
            validator: Validator::new(),
            scheduler: SchedulingAlgorithm::new(SchedulingStrategy::TopologicalOrder),
        }
    }

    /// Create a new contract processor with custom implementations
    pub fn with_components(splitter: SegmentExtractor, resolver: DependencyResolver, validator: Validator, scheduler: SchedulingAlgorithm) -> Self {
        Self {
            splitter,
            resolver,
            validator,
            scheduler,
        }
    }

    /// Processes contract through full pipeline:
    ///
    /// 1. Splitting → 2. Dependency Resolution →
    /// 3. Validation → 4. Scheduling
    ///
    /// # Returns
    /// - Ok(Vec<ContractSegment>): Ordered segments
    /// - Err(ProcessingError): On any stage failure
    pub fn process(&self, contract: &Contract) -> Result<Vec<ContractSegment>, ProcessingError> {
        let segments = self.splitter.extract_segments(contract)?;
        let dependency_graph = self.resolver.resolve_dependencies(&segments)?;

        for segment in &segments {
            self.validator.validate(segment).map_err(ProcessingError::ValidationFailed)?;
        }

        let processing_order = self.scheduler.schedule(&segments, &dependency_graph)?;
        Ok(processing_order.get_ordered_segments(&segments))
    }
}
