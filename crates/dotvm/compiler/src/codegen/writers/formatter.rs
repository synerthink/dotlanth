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

//! Output formatting utilities (e.g. compression, hex dumps)

/// Formatter for bytecode output
pub trait OutputFormatter {
    /// Format raw bytes into the desired output format
    fn format_output(&self, data: &[u8]) -> Vec<u8>;
}

/// Example hex-dump formatter
pub struct HexFormatter;

impl OutputFormatter for HexFormatter {
    fn format_output(&self, data: &[u8]) -> Vec<u8> {
        // Simple uppercase hex
        data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join("").into_bytes()
    }
}
