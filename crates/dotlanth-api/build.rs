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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto file paths (relative to project root)
    let proto_dir = "../dotvm/runtime/proto";

    // Check if proto directory exists
    if std::path::Path::new(proto_dir).exists() {
        println!("cargo:rerun-if-changed={}", proto_dir);

        // Compile proto files if they exist
        tonic_build::configure()
            .build_server(false) // We only need client
            .compile(&[format!("{}/vm_service.proto", proto_dir), format!("{}/common.proto", proto_dir)], &[proto_dir])
            .unwrap_or_else(|e| {
                println!("cargo:warning=Failed to compile proto files: {}", e);
            });
    } else {
        println!("cargo:warning=Proto directory not found: {}", proto_dir);
    }

    Ok(())
}
