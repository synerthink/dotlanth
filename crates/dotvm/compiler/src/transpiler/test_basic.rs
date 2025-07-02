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

//! Basic tests for the new transpiler architecture

#[cfg(test)]
mod tests {
    use super::super::{config::TranspilationConfig, engine_new::NewTranspilationEngine};

    #[test]
    fn test_new_engine_creation() {
        let config = TranspilationConfig::default();
        let engine = NewTranspilationEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_presets() {
        assert!(NewTranspilationEngine::debug().is_ok());
        assert!(NewTranspilationEngine::release().is_ok());
        assert!(NewTranspilationEngine::fast().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let config = TranspilationConfig::default();
        assert!(config.validate().is_ok());
    }
}
