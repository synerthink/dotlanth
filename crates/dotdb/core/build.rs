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

/// Build script for dotdb core
///
/// This script executes at build time and defines build configuration settings.
/// It is responsible for setting up external dependencies and linking options
/// for the dotdb core library.
fn main() {
    // Only apply Linux-specific configurations when building for Linux
    if cfg!(target_os = "linux") {
        // Link against the libnuma library, which provides NUMA (Non-Uniform Memory Access) support
        // NUMA is critical for optimizing memory access on multi-processor systems
        // where memory access time depends on the memory location relative to the processor
        println!("cargo:rustc-link-lib=numa");

        // Specify the search path for the native libraries
        // This helps the linker find the required libraries on the system
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    }

    // Additional cargo directives could be added here for:
    // - Rerunning the build script when specific files change
    // - Setting environment variables for the build
    // - Adding compiler flags
}
