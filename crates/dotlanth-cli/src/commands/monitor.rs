use super::CommandContext;
use anyhow::Result;
use std::time::Duration;

pub fn start_monitoring(ctx: &CommandContext) -> Result<()> {
    println!("Starting real-time monitoring...");
    println!("Press Ctrl+C to stop");
    println!();

    for i in 0..5 {
        println!("Monitoring cycle {} - {}", i + 1, chrono::Local::now().format("%H:%M:%S"));

        // Get current metrics
        let nodes = ctx.database.list_nodes()?;
        let deployments = ctx.database.list_deployments()?;
        let metrics = ctx.database.get_recent_metrics(None, 5)?;

        println!("  Nodes: {} total", nodes.len());
        println!("  Deployments: {} total", deployments.len());

        if let Some(latest_metric) = metrics.first() {
            println!(
                "  Latest Metrics ({}): CPU {:.1}%, Memory {:.1}%, Disk {:.1}%",
                latest_metric.node_id.chars().take(8).collect::<String>(),
                latest_metric.cpu_usage,
                latest_metric.memory_usage,
                latest_metric.disk_usage
            );
        }

        println!();
        std::thread::sleep(Duration::from_secs(2));
    }

    println!("Monitoring session completed");
    Ok(())
}

pub fn show_logs(ctx: &CommandContext) -> Result<()> {
    println!("Recent System Logs");
    println!("==================");

    let logs = ctx.database.get_recent_logs(None, 20)?;

    if logs.is_empty() {
        println!("No logs available");
        return Ok(());
    }

    for log in logs {
        let level_indicator = match log.level.as_str() {
            "ERROR" => "[E]",
            "WARN" => "[W]",
            "INFO" => "[I]",
            "DEBUG" => "[D]",
            _ => "[?]",
        };

        println!(
            "{} [{}] [{}] [{}] {}",
            level_indicator,
            log.timestamp.format("%H:%M:%S"),
            log.level,
            log.node_id.chars().take(8).collect::<String>(),
            log.message
        );
    }

    Ok(())
}
