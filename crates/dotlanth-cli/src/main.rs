use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI for DotLanth infrastructure management
#[derive(Parser, Debug)]
#[command(name = "dotlanth", about = "DotLanth Infrastructure Management")]
pub struct Cli {
    /// Path to configuration file (TOML)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Data directory location (overrides $DOTLANTH_DATA_DIR)
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for node management
#[derive(Subcommand, Debug)]
#[command(about = "Manage individual nodes (add/remove/list)")]
pub enum NodeCommands {
    /// List all registered nodes
    List,
    /// Add a new node by address
    Add { addr: String },
    /// Remove an existing node by ID
    Remove { node_id: String },
}

/// Subcommands for cluster operations
#[derive(Subcommand, Debug)]
#[command(about = "Cluster-wide operations and scaling")]
pub enum ClusterCommands {
    /// Show cluster status
    Status,
    /// Scale the cluster to a given number of replicas
    Scale { count: u32 },
}

/// Subcommands for backup and restore
#[derive(Subcommand, Debug)]
#[command(about = "Backup and restore infrastructure state")]
pub enum BackupCommands {
    /// Create a new backup with the given name
    Create { name: String },
    /// Restore from a backup by name
    Restore { name: String },
}

/// Subcommands for configuration inspection and update
#[derive(Subcommand, Debug)]
#[command(about = "Inspect or update CLI configuration")]
pub enum ConfigCommands {
    /// Show current effective configuration
    Show,
    /// Update a configuration key to a new value
    Set { key: String, value: String },
}

/// Top-level commands for dotlanth
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the interactive TUI dashboard
    Run,

    /// Display current infrastructure status
    Status,

    /// Deploy a dot file to the cluster
    Deploy {
        /// Path to the .dot file to deploy
        dot_file: PathBuf,
    },

    /// Stream real-time metrics and logs
    Monitor,

    /// View centralized logs from the cluster
    Logs,

    /// Manage individual nodes (add/remove/list)
    Nodes {
        #[command(subcommand)]
        command: NodeCommands,
    },

    /// Perform cluster-wide operations
    Cluster {
        #[command(subcommand)]
        command: ClusterCommands,
    },

    /// Backup and restore operations
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// Inspect or update configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

fn main() {
    let cli = Cli::parse();
    // TODO: Invoke command dispatchers when implemented
    println!("{:#?}", cli);
}
