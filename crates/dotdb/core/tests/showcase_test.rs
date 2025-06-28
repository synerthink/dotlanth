// Test script for the showcase scenario
use dotdb_core::document::create_in_memory_collection_manager;
use serde_json::json;

#[test]
fn test_showcase_scenario() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DotDB Alpha MVP Showcase ===");

    // Create collection manager
    let manager = create_in_memory_collection_manager()?;

    // Step 1: Insert a user document (equivalent to `dotdb put users '{"name": "Ada", "count": 5}'`)
    println!("\n1. Inserting user document...");
    let user_data = json!({"name": "Ada", "count": 5});
    let user_id = manager.insert_value("users", user_data)?;
    println!("   Document inserted with ID: {}", user_id);

    // Step 2: Read the user
    println!("\n2. Reading user document...");
    let mut user = manager.get_value("users", &user_id)?.unwrap();
    println!("   Current user data: {}", serde_json::to_string_pretty(&user)?);

    // Step 3: Increment count and save back (simulating what a dot program would do)
    println!("\n3. Incrementing count and saving...");
    let current_count = user["count"].as_i64().unwrap();
    user["count"] = json!(current_count + 1);
    manager.update_value("users", &user_id, user)?;
    println!("   Count incremented from {} to {}", current_count, current_count + 1);

    // Step 4: Verify the change (equivalent to `dotdb get users <id>`)
    println!("\n4. Verifying the change...");
    let updated_user = manager.get_value("users", &user_id)?.unwrap();
    println!("   Updated user data: {}", serde_json::to_string_pretty(&updated_user)?);

    // Verify the count is now 6
    assert_eq!(updated_user["count"], 6);
    assert_eq!(updated_user["name"], "Ada");

    println!("\n✅ Showcase scenario completed successfully!");
    println!("   - Document was inserted");
    println!("   - Document was read and modified");
    println!("   - Changes were persisted");
    println!("   - Final count: {}", updated_user["count"]);

    Ok(())
}
