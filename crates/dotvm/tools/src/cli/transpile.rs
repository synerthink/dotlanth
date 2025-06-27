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

//! End-to-end transpilation CLI for Rust → Wasm → DotVM pipeline
//!
//! This module provides the complete transpilation pipeline from Rust source code
//! to DotVM bytecode, with architecture selection and optimization options.

use clap::{Parser, ValueEnum};
use dotvm_compiler::{
    codegen::dotvm_generator::DotVMGenerator,
    transpiler::engine::TranspilationEngine,
    wasm::{parser::WasmParser, ast::WasmModule},
};
use dotvm_core::bytecode::VmArchitecture;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// CLI arguments for the transpilation tool
#[derive(Parser)]
#[command(name = "dotvm-transpile")]
#[command(about = "Transpile Rust code to DotVM bytecode")]
#[command(version = "0.1.0")]
pub struct TranspileArgs {
    /// Input Rust source file or project directory
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output DotVM bytecode file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Target VM architecture
    #[arg(short, long, default_value = "arch64")]
    pub architecture: ArchitectureArg,

    /// Optimization level (0-3)
    #[arg(long, default_value = "2")]
    pub opt_level: u8,

    /// Enable debug information
    #[arg(long)]
    pub debug: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Keep intermediate files (Wasm)
    #[arg(long)]
    pub keep_intermediate: bool,

    /// Custom target directory for Rust compilation
    #[arg(long)]
    pub target_dir: Option<PathBuf>,
}

/// Architecture selection for CLI
#[derive(Clone, Debug, ValueEnum)]
pub enum ArchitectureArg {
    Arch64,
    Arch128,
    Arch256,
    Arch512,
}

impl From<ArchitectureArg> for VmArchitecture {
    fn from(arch: ArchitectureArg) -> Self {
        match arch {
            ArchitectureArg::Arch64 => VmArchitecture::Arch64,
            ArchitectureArg::Arch128 => VmArchitecture::Arch128,
            ArchitectureArg::Arch256 => VmArchitecture::Arch256,
            ArchitectureArg::Arch512 => VmArchitecture::Arch512,
        }
    }
}

/// Main transpilation pipeline
pub struct TranspilationPipeline {
    args: TranspileArgs,
}

impl TranspilationPipeline {
    /// Create a new transpilation pipeline
    pub fn new(args: TranspileArgs) -> Self {
        Self { args }
    }

    /// Execute the complete transpilation pipeline
    pub fn execute(&self) -> Result<(), TranspilationError> {
        if self.args.verbose {
            println!("Starting Rust → Wasm → DotVM transpilation pipeline");
            println!("Input: {:?}", self.args.input);
            println!("Output: {:?}", self.args.output);
            println!("Architecture: {:?}", self.args.architecture);
        }

        // Step 1: Compile Rust to Wasm
        let wasm_path = self.compile_rust_to_wasm()?;

        // Step 2: Parse Wasm to AST
        let wasm_module = self.parse_wasm(&wasm_path)?;

        // Step 3: Transpile Wasm to DotVM bytecode
        let bytecode = self.transpile_to_dotvm(wasm_module)?;

        // Step 4: Write output
        self.write_bytecode(&bytecode)?;

        // Step 5: Cleanup
        if !self.args.keep_intermediate {
            self.cleanup_intermediate_files(&wasm_path)?;
        }

        if self.args.verbose {
            println!("Transpilation completed successfully!");
        }

        Ok(())
    }

    /// Compile Rust source to Wasm
    fn compile_rust_to_wasm(&self) -> Result<PathBuf, TranspilationError> {
        if self.args.verbose {
            println!("Step 1: Compiling Rust to Wasm...");
        }

        let input_path = &self.args.input;
        let is_project = input_path.is_dir() && input_path.join("Cargo.toml").exists();

        let wasm_output = if is_project {
            self.compile_rust_project()?
        } else {
            self.compile_rust_file()?
        };

        if self.args.verbose {
            println!("Wasm compilation completed: {:?}", wasm_output);
        }

        Ok(wasm_output)
    }

    /// Compile a Rust project to Wasm
    fn compile_rust_project(&self) -> Result<PathBuf, TranspilationError> {
        let project_dir = &self.args.input;
        let target_dir = self.args.target_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| project_dir.join("target").to_string_lossy().to_string());

        let mut cmd = Command::new("cargo");
        cmd.current_dir(project_dir)
            .args(&[
                "build",
                "--target", "wasm32-unknown-unknown",
                "--target-dir", &target_dir,
            ]);

        // Add optimization level
        match self.args.opt_level {
            0 => {}, // Debug build (default)
            1 => { cmd.arg("--release"); },
            2 => { cmd.args(&["--release"]); },
            3 => { cmd.args(&["--release"]); },
            _ => return Err(TranspilationError::InvalidOptLevel(self.args.opt_level)),
        }

        let output = cmd.output()
            .map_err(|e| TranspilationError::RustCompilation(format!("Failed to run cargo: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TranspilationError::RustCompilation(format!("Cargo build failed: {}", stderr)));
        }

        // Find the generated Wasm file
        let profile = if self.args.opt_level == 0 { "debug" } else { "release" };
        let wasm_dir = Path::new(&target_dir)
            .join("wasm32-unknown-unknown")
            .join(profile);

        // Look for .wasm files in the target directory
        let wasm_files: Vec<_> = fs::read_dir(&wasm_dir)
            .map_err(|e| TranspilationError::FileSystem(format!("Cannot read target directory: {}", e)))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "wasm" {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        if wasm_files.is_empty() {
            return Err(TranspilationError::RustCompilation("No Wasm files found in target directory".to_string()));
        }

        // Use the first Wasm file found (in a real implementation, you might want to be more specific)
        Ok(wasm_files[0].clone())
    }

    /// Compile a single Rust file to Wasm
    fn compile_rust_file(&self) -> Result<PathBuf, TranspilationError> {
        let input_file = &self.args.input;
        let temp_dir = std::env::temp_dir().join("dotvm_transpile");
        fs::create_dir_all(&temp_dir)
            .map_err(|e| TranspilationError::FileSystem(format!("Cannot create temp directory: {}", e)))?;

        let wasm_output = temp_dir.join("output.wasm");

        let mut cmd = Command::new("rustc");
        cmd.arg(input_file)
            .args(&[
                "--target", "wasm32-unknown-unknown",
                "--crate-type", "cdylib",
                "-o", wasm_output.to_str().unwrap(),
            ]);

        // Add optimization flags
        match self.args.opt_level {
            0 => {},
            1 => { cmd.arg("-O"); },
            2 => { cmd.args(&["-O", "-C", "opt-level=2"]); },
            3 => { cmd.args(&["-O", "-C", "opt-level=3"]); },
            _ => return Err(TranspilationError::InvalidOptLevel(self.args.opt_level)),
        }

        let output = cmd.output()
            .map_err(|e| TranspilationError::RustCompilation(format!("Failed to run rustc: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TranspilationError::RustCompilation(format!("rustc failed: {}", stderr)));
        }

        Ok(wasm_output)
    }

    /// Parse Wasm file to AST
    fn parse_wasm(&self, wasm_path: &Path) -> Result<WasmModule, TranspilationError> {
        if self.args.verbose {
            println!("Step 2: Parsing Wasm to AST...");
        }

        let wasm_bytes = fs::read(wasm_path)
            .map_err(|e| TranspilationError::FileSystem(format!("Cannot read Wasm file: {}", e)))?;

        let parser = WasmParser::new();
        let module = parser.parse(&wasm_bytes)
            .map_err(|e| TranspilationError::WasmParsing(format!("Wasm parsing failed: {:?}", e)))?;

        if self.args.verbose {
            println!("Wasm parsing completed. Functions: {}", module.functions.len());
        }

        Ok(module)
    }

    /// Transpile Wasm AST to DotVM bytecode
    fn transpile_to_dotvm(&self, wasm_module: WasmModule) -> Result<Vec<u8>, TranspilationError> {
        if self.args.verbose {
            println!("Step 3: Transpiling Wasm to DotVM bytecode...");
        }

        let target_arch = VmArchitecture::from(self.args.architecture.clone());
        
        let mut transpiler = TranspilationEngine::with_architecture(target_arch);
        let transpiled_module = transpiler.transpile_module(wasm_module)
            .map_err(|e| TranspilationError::Transpilation(format!("Transpilation failed: {:?}", e)))?;

        let mut generator = DotVMGenerator::with_architecture(target_arch);
        let generated_bytecode = generator.generate(&transpiled_module)
            .map_err(|e| TranspilationError::BytecodeGeneration(format!("Bytecode generation failed: {:?}", e)))?;

        if self.args.verbose {
            println!("DotVM bytecode generation completed. Size: {} bytes", generated_bytecode.bytecode.len());
        }

        Ok(generated_bytecode.bytecode)
    }

    /// Write bytecode to output file
    fn write_bytecode(&self, bytecode: &[u8]) -> Result<(), TranspilationError> {
        if self.args.verbose {
            println!("Step 4: Writing bytecode to output file...");
        }

        // Create output directory if it doesn't exist
        if let Some(parent) = self.args.output.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| TranspilationError::FileSystem(format!("Cannot create output directory: {}", e)))?;
        }

        fs::write(&self.args.output, bytecode)
            .map_err(|e| TranspilationError::FileSystem(format!("Cannot write output file: {}", e)))?;

        if self.args.verbose {
            println!("Bytecode written to: {:?}", self.args.output);
        }

        Ok(())
    }

    /// Clean up intermediate files
    fn cleanup_intermediate_files(&self, wasm_path: &Path) -> Result<(), TranspilationError> {
        if self.args.verbose {
            println!("Step 5: Cleaning up intermediate files...");
        }

        // Only remove files we created in temp directories
        if wasm_path.starts_with(std::env::temp_dir()) {
            if let Err(e) = fs::remove_file(wasm_path) {
                eprintln!("Warning: Could not remove intermediate file {:?}: {}", wasm_path, e);
            }
        }

        Ok(())
    }
}

/// Transpilation errors
#[derive(Debug, thiserror::Error)]
pub enum TranspilationError {
    #[error("Rust compilation failed: {0}")]
    RustCompilation(String),

    #[error("Wasm parsing failed: {0}")]
    WasmParsing(String),

    #[error("Transpilation failed: {0}")]
    Transpilation(String),

    #[error("Bytecode generation failed: {0}")]
    BytecodeGeneration(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Invalid optimization level: {0}")]
    InvalidOptLevel(u8),
}

/// Main entry point for the transpilation CLI
pub fn run_transpile_cli() -> Result<(), Box<dyn std::error::Error>> {
    let args = TranspileArgs::parse();
    let pipeline = TranspilationPipeline::new(args);
    pipeline.execute()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_architecture_conversion() {
        assert!(matches!(VmArchitecture::from(ArchitectureArg::Arch64), VmArchitecture::Arch64));
        assert!(matches!(VmArchitecture::from(ArchitectureArg::Arch128), VmArchitecture::Arch128));
        assert!(matches!(VmArchitecture::from(ArchitectureArg::Arch256), VmArchitecture::Arch256));
        assert!(matches!(VmArchitecture::from(ArchitectureArg::Arch512), VmArchitecture::Arch512));
    }

    #[test]
    fn test_pipeline_creation() {
        let temp_dir = TempDir::new().unwrap();
        let args = TranspileArgs {
            input: temp_dir.path().join("input.rs"),
            output: temp_dir.path().join("output.dotvm"),
            architecture: ArchitectureArg::Arch64,
            opt_level: 2,
            debug: false,
            verbose: false,
            keep_intermediate: false,
            target_dir: None,
        };

        let pipeline = TranspilationPipeline::new(args);
        // Just test that we can create the pipeline
        assert_eq!(pipeline.args.opt_level, 2);
    }

    #[test]
    fn test_invalid_opt_level() {
        let error = TranspilationError::InvalidOptLevel(5);
        assert!(error.to_string().contains("Invalid optimization level: 5"));
    }
}