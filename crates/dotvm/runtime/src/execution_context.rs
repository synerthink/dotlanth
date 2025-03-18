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

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ExecutionContext {
    env_vars: HashMap<String, String>,
    isolated: bool,
}

impl ExecutionContext {
    pub fn new() -> Self {
        ExecutionContext {
            env_vars: HashMap::new(),
            isolated: false,
        }
    }

    pub fn set_env(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    pub fn switch_context(&self) -> Self {
        ExecutionContext {
            env_vars: self.env_vars.clone(),
            isolated: self.isolated,
        }
    }

    pub fn isolate_contract(&self) -> Self {
        let mut new_ctx = self.switch_context();
        new_ctx.isolated = true;
        new_ctx
    }

    pub fn is_isolated(&self) -> bool {
        self.isolated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context_empty_environment() {
        let ctx = ExecutionContext::new();
        assert_eq!(ctx.get_env("nonexistent"), None);
        assert!(!ctx.is_isolated());
    }

    #[test]
    fn test_set_and_get_env() {
        let mut ctx = ExecutionContext::new();
        ctx.set_env("KEY".to_string(), "VALUE".to_string());
        assert_eq!(ctx.get_env("KEY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_context_switching() {
        let mut ctx = ExecutionContext::new();
        ctx.set_env("A".to_string(), "1".to_string());
        let new_ctx = ctx.switch_context();
        assert_eq!(new_ctx.get_env("A"), Some(&"1".to_string()));
    }

    #[test]
    fn test_contract_isolation() {
        let mut ctx = ExecutionContext::new();
        ctx.set_env("X".to_string(), "Y".to_string());
        let isolated_ctx = ctx.isolate_contract();
        assert_eq!(isolated_ctx.get_env("X"), Some(&"Y".to_string()));
        assert!(isolated_ctx.is_isolated());
    }
}
