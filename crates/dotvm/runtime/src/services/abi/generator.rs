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

//! ABI generator - generates ABIs from dot source code

use thiserror::Error;
use tracing::{error, info, instrument};

use crate::proto::vm_service::{AbiField, AbiType, DotAbi, GenerateAbiRequest, GenerateAbiResponse, ParaDotDependency, PermissionConfig, UiHints};

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
    #[error("Type inference failed: {0}")]
    TypeInferenceFailed(String),
}

/// ABI generator creates ABIs from dot source code
pub struct AbiGenerator {
    // TODO: Add parser, type analyzer, etc.
}

impl AbiGenerator {
    pub fn new() -> Self {
        Self {}
    }

    #[instrument(skip(self, request))]
    pub async fn generate_from_source(&self, request: GenerateAbiRequest) -> Result<GenerateAbiResponse, GeneratorError> {
        info!("Generating ABI from source ({} chars)", request.dot_source.len());

        // TODO: Parse dot source code
        let parsed_dot = self.parse_dot_source(&request.dot_source)?;

        // TODO: Extract types and generate ABI
        let abi = self.extract_abi_from_parsed_dot(&parsed_dot, &request.options)?;

        Ok(GenerateAbiResponse {
            success: true,
            abi: Some(abi),
            error_message: String::new(),
            warnings: vec![], // TODO: Add warnings from parsing
        })
    }

    // Private methods
    fn parse_dot_source(&self, source: &str) -> Result<ParsedDot, GeneratorError> {
        info!("Parsing dot source");

        // TODO: Implement actual dot parsing
        // For now, return a mock parsed dot

        if source.trim().is_empty() {
            return Err(GeneratorError::ParseError("Empty source".to_string()));
        }

        Ok(ParsedDot {
            name: self.extract_dot_name(source),
            inputs: self.extract_inputs(source)?,
            outputs: self.extract_outputs(source)?,
            paradots: self.extract_paradots(source)?,
            permissions: self.extract_permissions(source)?,
        })
    }

    fn extract_abi_from_parsed_dot(&self, parsed_dot: &ParsedDot, options: &Option<crate::proto::vm_service::AbiGenerationOptions>) -> Result<DotAbi, GeneratorError> {
        info!("Extracting ABI from parsed dot");

        let mut abi = DotAbi {
            dot_name: parsed_dot.name.clone(),
            version: "1.0.0".to_string(),
            description: format!("Auto-generated ABI for {}", parsed_dot.name),
            inputs: parsed_dot.inputs.clone(),
            outputs: parsed_dot.outputs.clone(),
            paradots: parsed_dot.paradots.clone(),
            ui_hints: None,
            permissions: parsed_dot.permissions.clone(),
        };

        // Generate UI hints if requested
        if let Some(opts) = options {
            if opts.include_ui_hints {
                abi.ui_hints = Some(self.generate_ui_hints(&parsed_dot, opts)?);
            }
        }

        Ok(abi)
    }

    fn extract_dot_name(&self, source: &str) -> String {
        // TODO: Parse actual dot name from source
        // For now, extract from "dot DotName {" pattern

        if let Some(start) = source.find("dot ") {
            let after_dot = &source[start + 4..];
            if let Some(end) = after_dot.find(" {") {
                return after_dot[..end].trim().to_string();
            }
        }

        "UnknownDot".to_string()
    }

    fn extract_inputs(&self, source: &str) -> Result<Vec<AbiField>, GeneratorError> {
        // TODO: Parse actual input fields from source
        // For now, return mock inputs

        Ok(vec![AbiField {
            name: "input1".to_string(),
            field_type: Some(AbiType {
                type_name: "String".to_string(),
                generic_params: vec![],
                attributes: std::collections::HashMap::new(),
            }),
            description: "Mock input field".to_string(),
            constraints: None,
            required: true,
            default_value: vec![],
        }])
    }

    fn extract_outputs(&self, source: &str) -> Result<Vec<AbiField>, GeneratorError> {
        // TODO: Parse actual output fields from source
        // For now, return mock outputs

        Ok(vec![AbiField {
            name: "output1".to_string(),
            field_type: Some(AbiType {
                type_name: "String".to_string(),
                generic_params: vec![],
                attributes: std::collections::HashMap::new(),
            }),
            description: "Mock output field".to_string(),
            constraints: None,
            required: true,
            default_value: vec![],
        }])
    }

    fn extract_paradots(&self, source: &str) -> Result<Vec<ParaDotDependency>, GeneratorError> {
        // TODO: Parse actual paradot dependencies from source
        Ok(vec![])
    }

    fn extract_permissions(&self, source: &str) -> Result<Option<PermissionConfig>, GeneratorError> {
        // TODO: Parse actual permissions from source
        Ok(None)
    }

    fn generate_ui_hints(&self, parsed_dot: &ParsedDot, options: &crate::proto::vm_service::AbiGenerationOptions) -> Result<UiHints, GeneratorError> {
        // TODO: Generate actual UI hints
        Ok(UiHints {
            layout: "form".to_string(),
            theme: if options.ui_theme.is_empty() { "default".to_string() } else { options.ui_theme.clone() },
            responsive: true,
            input_groups: vec![],
            output_sections: vec![],
        })
    }
}

// Helper structs for parsing
#[derive(Debug, Clone)]
struct ParsedDot {
    name: String,
    inputs: Vec<AbiField>,
    outputs: Vec<AbiField>,
    paradots: Vec<ParaDotDependency>,
    permissions: Option<PermissionConfig>,
}
