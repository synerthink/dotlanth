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

//! DotVM CLI Tool
//!
//! Main entry point for the DotVM command-line interface.

use clap::{Parser, Subcommand};
use dotvm_tools::cli::run::{RunArgs, run_bytecode};
use dotvm_tools::cli::transpile::TranspileArgs;

#[derive(Parser)]
#[command(name = "dotvm")]
#[command(about = "DotVM - Multi-Architecture Virtual Machine")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transpile Rust code to DotVM bytecode
    Transpile(TranspileArgs),
    /// Run DotVM bytecode
    Run(RunArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Transpile(args) => {
            // Create a new TranspileArgs with the parsed arguments
            let transpile_args = TranspileArgs {
                input: args.input,
                output: args.output,
                architecture: args.architecture,
                opt_level: args.opt_level,
                debug: args.debug,
                verbose: args.verbose,
                keep_intermediate: args.keep_intermediate,
                target_dir: args.target_dir,
            };

            let pipeline = dotvm_tools::TranspilationPipeline::new(transpile_args);
            pipeline.execute()?;
        }
        Commands::Run(args) => {
            run_bytecode(args)?;
        }
    }

    Ok(())
}
