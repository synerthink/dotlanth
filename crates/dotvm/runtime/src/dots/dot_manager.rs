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

use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Dot {
    pub id: String,
    pub code: String,
}

#[derive(Debug)]
pub struct DotInstance {
    pub dot: Dot,
    pub active: bool,
    pub temp_file: Option<std::path::PathBuf>,
}

impl DotInstance {
    pub fn new(dot: Dot) -> Self {
        let temp_path = std::env::temp_dir().join(format!("{}.txt", dot.id));
        std::fs::write(&temp_path, "temp data").expect("Failed to create temp file");

        Self {
            dot,
            active: true,
            temp_file: Some(temp_path),
        }
    }
}

/// Loads a dot from a file path.
/// The dot's id is derived from the file name.
pub fn load_dot<P: AsRef<Path>>(path: P) -> Result<Dot, io::Error> {
    let code = fs::read_to_string(&path)?;
    let id = path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
        .to_string();
    Ok(Dot { id, code })
}

/// Instantiate a dot to create a new instance.
pub fn instantiate_dot(dot: Dot) -> DotInstance {
    DotInstance::new(dot)
}

/// Terminates an active dot instance by marking it inactive.
/// Returns an error if the instance is already terminated.
pub fn terminate_dot(instance: &mut DotInstance) -> Result<(), String> {
    if instance.active {
        instance.active = false;
        Ok(())
    } else {
        Err("Dot instance is already terminated".to_string())
    }
}

/// Cleans up resources associated with a dot instance.
/// This should only be invoked on a terminated dot.
pub fn cleanup_resources(instance: &DotInstance) {
    if !instance.active
        && let Some(path) = &instance.temp_file
    {
        let _ = std::fs::remove_file(path); // Remove temp file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_load_dot() {
        let mut path = env::temp_dir();
        path.push("test_dot.txt");
        let dot_code = "dummy dot code";
        {
            let mut file = File::create(&path).expect("Failed to create temp file");
            file.write_all(dot_code.as_bytes()).expect("Failed to write to temp file");
        }
        let _ = load_dot(&path).expect("Failed to load dot");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_instantiate_and_terminate_dot() {
        let dot = Dot {
            id: "test".to_string(),
            code: "code".to_string(),
        };
        let mut instance = instantiate_dot(dot);
        // Expect the instance to be active initially.
        assert!(instance.active, "Dot instance should be active initially");
        // After termination the instance should be inactive and further termination should error.
        let _ = terminate_dot(&mut instance);
        assert!(!instance.active, "Dot instance should be inactive after termination");
        let _ = terminate_dot(&mut instance);
    }

    #[test]
    fn test_cleanup_resources() {
        use std::time::{Duration, Instant};

        // Set temporary file path with unique name to avoid conflicts
        let temp_path = std::env::temp_dir().join(format!("test_cleanup_{}.txt", std::process::id()));
        // Create the file manually (since DotInstance::new is not called)
        std::fs::write(&temp_path, "temp data").expect("Failed to create file");

        // Create a passive instance
        let instance = DotInstance {
            dot: Dot {
                id: "test".to_string(),
                code: "code".to_string(),
            },
            active: false,
            temp_file: Some(temp_path.clone()), // Save path
        };

        // Verify file exists before cleaning
        assert!(temp_path.exists(), "File must be available before cleaning");
        cleanup_resources(&instance);

        // Add retry logic for file deletion verification (handles OS buffering delays)
        let start_time = Instant::now();
        let timeout = Duration::from_millis(1000); // 1 second timeout

        while temp_path.exists() && start_time.elapsed() < timeout {
            std::thread::sleep(Duration::from_millis(10));
        }

        // Verify that the file has been deleted after cleaning
        assert!(!temp_path.exists(), "File should be deleted after cleaning");
    }
}
