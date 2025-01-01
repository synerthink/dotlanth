mod cli;
mod utils;

use dotvm_common as common;
use dotvm_compiler as compiler;
use dotvm_core as core;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse CLI arguments and run
    cli::run().await
}
