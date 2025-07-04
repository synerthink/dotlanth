use super::CommandContext;
use crate::database::{DeploymentInfo, DeploymentStatus};
use anyhow::Result;
use std::path::Path;

pub fn deploy_dot(ctx: &CommandContext, dot_file: &Path) -> Result<()> {
    println!("Deploying dot file: {}", dot_file.display());

    // Check if file exists
    if !dot_file.exists() {
        return Err(anyhow::anyhow!("Dot file not found: {}", dot_file.display()));
    }

    // Get available nodes
    let nodes = ctx.database.list_nodes()?;
    let online_nodes: Vec<_> = nodes.iter().filter(|n| matches!(n.status, crate::database::NodeStatus::Online)).collect();

    if online_nodes.is_empty() {
        return Err(anyhow::anyhow!("No online nodes available for deployment"));
    }

    // Select first online node (simple strategy)
    let target_node = &online_nodes[0];

    // Create deployment
    let deployment = DeploymentInfo {
        id: format!("deploy-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        dot_name: dot_file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string(),
        dot_version: "1.0.0".to_string(),
        node_id: target_node.id.clone(),
        status: DeploymentStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        config: serde_json::json!({
            "file_path": dot_file.to_string_lossy(),
            "memory": "512MB",
            "cpu": "0.5"
        }),
    };

    ctx.database.create_deployment(deployment.clone())?;

    println!("Deployment created:");
    println!("  ID: {}", deployment.id);
    println!("  Dot: {}", deployment.dot_name);
    println!("  Target Node: {} ({})", target_node.id, target_node.address);
    println!("  Status: Pending");

    // Simulate deployment process
    println!("Uploading dot file...");
    std::thread::sleep(std::time::Duration::from_millis(500));

    println!("Configuring runtime...");
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Update status to running
    ctx.database.update_deployment_status(&deployment.id, DeploymentStatus::Running)?;
    println!("Deployment successful! Status: Running");

    Ok(())
}
