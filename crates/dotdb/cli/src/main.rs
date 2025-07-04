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

//! DotDB CLI Tool
//!
//! Command-line interface for interacting with the DotDB document database.

use clap::{Parser, Subcommand};
use dotdb_core::document::{DocumentId, create_persistent_collection_manager};
use serde_json::Value;
use std::path::PathBuf;
use std::process;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "dotdb")]
#[command(about = "DotDB - Document Database CLI")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
struct GlobalArgs {
    /// Data directory for persistent storage (defaults to XDG data directory)
    #[arg(long, short = 'd', global = true)]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Insert a JSON document into a collection
    Put {
        /// Collection name
        collection: String,
        /// JSON document content
        json: String,
    },
    /// Get a document by ID from a collection
    Get {
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
    /// Update a document by ID
    Update {
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
        /// New JSON document content
        json: String,
    },
    /// Delete a document by ID
    Delete {
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
    /// List all document IDs in a collection
    List {
        /// Collection name
        collection: String,
    },
    /// List all collections
    Collections,
    /// Create a collection
    CreateCollection {
        /// Collection name
        collection: String,
    },
    /// Delete a collection and all its documents
    DeleteCollection {
        /// Collection name
        collection: String,
    },
    /// Count documents in a collection
    Count {
        /// Collection name
        collection: String,
    },
    /// Find documents by field value
    Find {
        /// Collection name
        collection: String,
        /// Field name
        field: String,
        /// Field value (JSON)
        value: String,
    },
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // For now, use default data directory since we can't easily parse global args with subcommands
    let data_dir = get_data_directory(None);

    // Ensure data directory exists
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        error!("Failed to create data directory {}: {}", data_dir.display(), e);
        process::exit(1);
    }

    // Create collection manager with persistent storage
    let manager = match create_persistent_collection_manager(&data_dir, None) {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to create collection manager: {}", e);
            process::exit(1);
        }
    };

    let result = match cli.command {
        Commands::Put { collection, json } => handle_put(&manager, &collection, &json),
        Commands::Get { collection, id } => handle_get(&manager, &collection, &id),
        Commands::Update { collection, id, json } => handle_update(&manager, &collection, &id, &json),
        Commands::Delete { collection, id } => handle_delete(&manager, &collection, &id),
        Commands::List { collection } => handle_list(&manager, &collection),
        Commands::Collections => handle_list_collections(&manager),
        Commands::CreateCollection { collection } => handle_create_collection(&manager, &collection),
        Commands::DeleteCollection { collection } => handle_delete_collection(&manager, &collection),
        Commands::Count { collection } => handle_count(&manager, &collection),
        Commands::Find { collection, field, value } => handle_find(&manager, &collection, &field, &value),
    };

    if let Err(e) = result {
        error!("Command failed: {}", e);
        process::exit(1);
    }
}

/// Get the data directory for persistent storage with XDG compliance
fn get_data_directory(custom_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = custom_dir {
        return dir;
    }

    // Use XDG data directory if available, otherwise fall back to a sensible default
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data_home).join("dotdb")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local").join("share").join("dotdb")
    } else {
        // Fallback for systems without HOME (like some CI environments)
        PathBuf::from(".dotdb")
    }
}

fn handle_put(manager: &dotdb_core::document::CollectionManager, collection: &str, json: &str) -> anyhow::Result<()> {
    // Validate JSON
    let _: Value = serde_json::from_str(json)?;

    let id = manager.insert_json(collection, json)?;
    println!("Document inserted with ID: {id}");
    info!("Inserted document {} into collection {}", id, collection);
    Ok(())
}

fn handle_get(manager: &dotdb_core::document::CollectionManager, collection: &str, id_str: &str) -> anyhow::Result<()> {
    let id = DocumentId::from_string(id_str)?;

    match manager.get_json(collection, &id)? {
        Some(json) => {
            println!("{json}");
            info!("Retrieved document {} from collection {}", id, collection);
        }
        None => {
            println!("Document not found");
            info!("Document {} not found in collection {}", id, collection);
        }
    }
    Ok(())
}

fn handle_update(manager: &dotdb_core::document::CollectionManager, collection: &str, id_str: &str, json: &str) -> anyhow::Result<()> {
    let id = DocumentId::from_string(id_str)?;

    // Validate JSON
    let _: Value = serde_json::from_str(json)?;

    manager.update_json(collection, &id, json)?;
    println!("Document updated: {id}");
    info!("Updated document {} in collection {}", id, collection);
    Ok(())
}

fn handle_delete(manager: &dotdb_core::document::CollectionManager, collection: &str, id_str: &str) -> anyhow::Result<()> {
    let id = DocumentId::from_string(id_str)?;

    let deleted = manager.delete(collection, &id)?;
    if deleted {
        println!("Document deleted: {id}");
        info!("Deleted document {} from collection {}", id, collection);
    } else {
        println!("Document not found: {id}");
        info!("Document {} not found in collection {}", id, collection);
    }
    Ok(())
}

fn handle_list(manager: &dotdb_core::document::CollectionManager, collection: &str) -> anyhow::Result<()> {
    let doc_ids = manager.list_document_ids(collection)?;
    let count = doc_ids.len();

    if doc_ids.is_empty() {
        println!("No documents found in collection '{collection}'");
    } else {
        println!("Documents in collection '{collection}':");
        for id in doc_ids {
            println!("  {id}");
        }
    }

    info!("Listed {} documents in collection {}", count, collection);
    Ok(())
}

fn handle_list_collections(manager: &dotdb_core::document::CollectionManager) -> anyhow::Result<()> {
    let collections = manager.list_collections()?;
    let count = collections.len();

    if collections.is_empty() {
        println!("No collections found");
    } else {
        println!("Collections:");
        for collection in collections {
            println!("  {collection}");
        }
    }

    info!("Listed {} collections", count);
    Ok(())
}

fn handle_create_collection(manager: &dotdb_core::document::CollectionManager, collection: &str) -> anyhow::Result<()> {
    manager.create_collection(collection)?;
    println!("Collection created: {collection}");
    info!("Created collection {}", collection);
    Ok(())
}

fn handle_delete_collection(manager: &dotdb_core::document::CollectionManager, collection: &str) -> anyhow::Result<()> {
    let deleted = manager.delete_collection(collection)?;
    if deleted {
        println!("Collection deleted: {collection}");
        info!("Deleted collection {}", collection);
    } else {
        println!("Collection not found: {collection}");
        info!("Collection {} not found", collection);
    }
    Ok(())
}

fn handle_count(manager: &dotdb_core::document::CollectionManager, collection: &str) -> anyhow::Result<()> {
    let count = manager.count(collection)?;
    println!("Documents in collection '{collection}': {count}");
    info!("Counted {} documents in collection {}", count, collection);
    Ok(())
}

fn handle_find(manager: &dotdb_core::document::CollectionManager, collection: &str, field: &str, value_str: &str) -> anyhow::Result<()> {
    let value: Value = serde_json::from_str(value_str)?;

    let matching_docs = manager.find_by_field(collection, field, &value)?;
    let count = matching_docs.len();

    if matching_docs.is_empty() {
        println!("No documents found matching {field}={value}");
    } else {
        println!("Found {count} documents matching {field}={value}:");
        for (id, doc) in matching_docs {
            println!("  {}: {}", id, serde_json::to_string(&doc)?);
        }
    }

    info!("Found {} documents in collection {} matching {}={}", count, collection, field, value);
    Ok(())
}
