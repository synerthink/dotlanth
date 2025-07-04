use super::CommandContext;
use crate::ConfigCommands;
use anyhow::Result;

pub fn handle_config_command(ctx: &CommandContext, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show => show_config(ctx),
        ConfigCommands::Set { key, value } => set_config(ctx, &key, &value),
    }
}

fn show_config(ctx: &CommandContext) -> Result<()> {
    println!("Current Configuration");
    println!("====================");

    println!("Data Directory: {}", ctx.config.data_dir.display());
    println!();

    println!("UI Settings:");
    println!("  Theme: {}", ctx.config.ui.theme);
    println!("  Refresh Rate: {}ms", ctx.config.ui.refresh_rate_ms);
    println!("  Debug Info: {}", ctx.config.ui.show_debug_info);
    println!("  Max Log Lines: {}", ctx.config.ui.max_log_lines);
    println!();

    println!("Mock Data Settings:");
    println!("  Generate Sample Data: {}", ctx.config.mock_data.generate_sample_data);
    println!("  Node Count: {}", ctx.config.mock_data.node_count);
    println!("  Deployment Count: {}", ctx.config.mock_data.deployment_count);
    println!("  Simulate Failures: {}", ctx.config.mock_data.simulate_failures);

    Ok(())
}

fn set_config(_ctx: &CommandContext, key: &str, value: &str) -> Result<()> {
    println!("Setting configuration: {} = {}", key, value);

    match key {
        "ui.theme" => {
            if ["default", "dark", "light"].contains(&value) {
                println!("Theme set to: {}", value);
            } else {
                return Err(anyhow::anyhow!("Invalid theme. Valid options: default, dark, light"));
            }
        }
        "ui.refresh_rate_ms" => {
            if let Ok(rate) = value.parse::<u64>() {
                if rate >= 100 && rate <= 10000 {
                    println!("Refresh rate set to: {}ms", rate);
                } else {
                    return Err(anyhow::anyhow!("Refresh rate must be between 100 and 10000ms"));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid refresh rate: {}", value));
            }
        }
        "ui.show_debug_info" => {
            if let Ok(debug) = value.parse::<bool>() {
                println!("Debug info set to: {}", debug);
            } else {
                return Err(anyhow::anyhow!("Invalid boolean value: {}", value));
            }
        }
        "mock_data.generate_sample_data" => {
            if let Ok(generate) = value.parse::<bool>() {
                println!("Sample data generation set to: {}", generate);
            } else {
                return Err(anyhow::anyhow!("Invalid boolean value: {}", value));
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    }

    println!("Note: Configuration changes are not persisted in this placeholder implementation");
    println!("Restart the application to see changes");

    Ok(())
}
