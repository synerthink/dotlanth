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

//! Architecture compatibility analysis

use super::super::error::{TranspilationError, TranspilationResult};
use dotvm_core::bytecode::VmArchitecture;

/// Architecture compatibility analyzer
pub struct ArchitectureAnalyzer;

impl ArchitectureAnalyzer {
    /// Create a new architecture analyzer
    pub fn new() -> Self {
        Self
    }

    /// Check architecture compatibility
    pub fn check_compatibility(&self, required: VmArchitecture, target: VmArchitecture) -> bool {
        (target as u8) >= (required as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_compatibility() {
        let analyzer = ArchitectureAnalyzer::new();
        assert!(analyzer.check_compatibility(VmArchitecture::Arch64, VmArchitecture::Arch128));
        assert!(!analyzer.check_compatibility(VmArchitecture::Arch128, VmArchitecture::Arch64));
    }
}
