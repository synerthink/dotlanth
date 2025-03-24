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

//! Contract processing module

// Sub-modules
pub mod dependencies;
pub mod error;
pub mod lib;
pub mod scheduling;
pub mod splitting;
pub mod validation;

// Public exports
pub use dependencies::{DependencyGraph, DependencyResolver, DependencyType, SegmentDependency};
pub use error::{ProcessingError, ValidationError};
pub use lib::{Contract, ContractProcessor};
pub use scheduling::{ProcessingOrder, SchedulingAlgorithm, SchedulingStrategy};
pub use splitting::{ContractSegment, SegmentCriteria, SegmentExtractor};
pub use validation::{ValidationResult, Validator};
