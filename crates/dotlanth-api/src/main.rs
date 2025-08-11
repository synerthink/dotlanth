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

use dotlanth_api::{config::Config, server::ApiServer};
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Dotlanth REST API Gateway");

    // Load configuration
    let config = Config::from_env();
    info!("Loaded configuration: bind_address={}", config.bind_address);

    // Create and start the API server
    let server = ApiServer::new(config).await?;
    info!("REST API Gateway started on http://{}", server.bind_address());

    // Start the server
    server.run().await?;

    Ok(())
}
