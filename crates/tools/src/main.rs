mod cli;
mod utils;

use dotvm_common as common;
use dotvm_core as core;
use dotvm_compiler as compiler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error> {
    // Init logging
    tracing_subscriber::fmt::init();

    // Parse CLI arguments and run
    cli::run().await
}