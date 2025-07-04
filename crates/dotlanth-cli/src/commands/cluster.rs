use super::CommandContext;
use crate::ClusterCommands;
use anyhow::Result;

pub fn handle_cluster_command(ctx: &CommandContext, command: ClusterCommands) -> Result<()> {
    match command {
        ClusterCommands::Status => show_status(ctx),
        ClusterCommands::Scale { count } => scale_cluster(ctx, count),
    }
}

pub fn show_status(ctx: &CommandContext) -> Result<()> {
    println!("Cluster Status");
    println!("==============");

    let nodes = ctx.database.list_nodes()?;
    let deployments = ctx.database.list_deployments()?;

    let online_nodes = nodes.iter().filter(|n| matches!(n.status, crate::database::NodeStatus::Online)).count();
    let total_nodes = nodes.len();

    let running_deployments = deployments.iter().filter(|d| matches!(d.status, crate::database::DeploymentStatus::Running)).count();
    let total_deployments = deployments.len();

    println!("Overview:");
    println!("  Nodes: {}/{} online", online_nodes, total_nodes);
    println!("  Deployments: {}/{} running", running_deployments, total_deployments);

    if total_nodes > 0 {
        let health_percentage = (online_nodes * 100) / total_nodes;
        println!("  Cluster Health: {}%", health_percentage);

        if health_percentage >= 80 {
            println!("  Status: Healthy");
        } else if health_percentage >= 50 {
            println!("  Status: Degraded");
        } else {
            println!("  Status: Critical");
        }
    } else {
        println!("  Status: No nodes registered");
    }

    Ok(())
}

fn scale_cluster(ctx: &CommandContext, target_count: u32) -> Result<()> {
    let current_nodes = ctx.database.list_nodes()?.len();

    println!("Scaling cluster from {} to {} nodes", current_nodes, target_count);

    if target_count > current_nodes as u32 {
        println!("Would add {} nodes (placeholder - not implemented)", target_count - current_nodes as u32);
    } else if target_count < current_nodes as u32 {
        println!("Would remove {} nodes (placeholder - not implemented)", current_nodes as u32 - target_count);
    } else {
        println!("Cluster already at target size");
    }

    Ok(())
}
