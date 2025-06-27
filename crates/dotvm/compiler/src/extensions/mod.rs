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

//! DotVM-specific extensions for the transpiler
//!
//! This module provides advanced transpilation features that go beyond standard
//! WebAssembly capabilities, including BigInt arithmetic, SIMD operations,
//! and vector processing for high-performance computing.

pub mod detector;
pub mod math_ops;
pub mod simd;
pub mod vector;

pub use detector::{ExtensionDetector, ExtensionRequirement, ExtensionType};
pub use math_ops::{MathExtension, MathOperation};
pub use simd::{SimdExtension, SimdOperation};
pub use vector::{VectorExtension, VectorOperation};