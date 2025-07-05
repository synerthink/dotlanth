// Dotlanth
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
