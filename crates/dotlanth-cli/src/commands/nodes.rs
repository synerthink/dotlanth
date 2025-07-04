use super::CommandContext;
use crate::NodeCommands;
use crate::database::{NodeInfo, NodeStatus};
use anyhow::Result;
use serde_json::Value;

pub fn handle_node_command(ctx: &CommandContext, command: NodeCommands) -> Result<()> {
    match command {
        NodeCommands::List => list_nodes(ctx),
        NodeCommands::Add { addr } => add_node(ctx, &addr),
        NodeCommands::Remove { node_id } => remove_node(ctx, &node_id),
    }
}

fn list_nodes(ctx: &CommandContext) -> Result<()> {
    let nodes = ctx.database.list_nodes()?;

    if nodes.is_empty() {
        println!("No nodes registered.");
        return Ok(());
    }

    println!("Registered Nodes:");
    println!("{:<20} {:<30} {:<12} {:<10} {:<20}", "ID", "Address", "Status", "Version", "Last Heartbeat");
    println!("{}", "-".repeat(92));

    for node in nodes {
        let status_str = match node.status {
            NodeStatus::Online => "Online",
            NodeStatus::Offline => "Offline",
            NodeStatus::Maintenance => "Maintenance",
            NodeStatus::Error(_) => "Error",
        };

        println!(
            "{:<20} {:<30} {:<12} {:<10} {:<20}",
            &node.id[..20.min(node.id.len())],
            node.address,
            status_str,
            node.version,
            node.last_heartbeat.format("%Y-%m-%d %H:%M:%S")
        );
    }

    Ok(())
}

fn add_node(ctx: &CommandContext, address: &str) -> Result<()> {
    let node = NodeInfo {
        id: uuid::Uuid::new_v4().to_string(),
        address: address.to_string(),
        status: NodeStatus::Offline,
        last_heartbeat: chrono::Utc::now(),
        version: "1.0.0".to_string(),
        capabilities: vec!["dotvm".to_string(), "dotdb".to_string()],
        metadata: Value::Object(serde_json::Map::new()),
    };

    ctx.database.register_node(node.clone())?;
    println!("Node added successfully:");
    println!("  ID: {}", node.id);
    println!("  Address: {}", address);
    println!("  Status: Offline (will be online once connected)");

    Ok(())
}

fn remove_node(ctx: &CommandContext, node_id: &str) -> Result<()> {
    if ctx.database.get_node(node_id)?.is_some() {
        ctx.database.remove_node(node_id)?;
        println!("Node {} removed successfully.", node_id);
    } else {
        println!("Node {} not found.", node_id);
    }

    Ok(())
}
