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

//! Function complexity analysis

use crate::wasm::ast::WasmFunction;

/// Function complexity analyzer
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Create a new complexity analyzer
    pub fn new() -> Self {
        Self
    }

    /// Calculate complexity score for a function
    pub fn calculate_complexity(&self, function: &WasmFunction) -> u32 {
        let mut score = 0;

        // Base score from instruction count
        score += (function.body.len() / 10) as u32;

        // Add score for parameters and locals
        score += function.signature.params.len() as u32;
        score += function.locals.len() as u32;

        // Cap at 100
        score.min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_calculation() {
        let analyzer = ComplexityAnalyzer::new();

        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType { params: vec![], results: vec![] },
            locals: vec![],
            body: vec![],
        };

        let complexity = analyzer.calculate_complexity(&function);
        assert_eq!(complexity, 0);
    }
}
