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

//! Formatting of analysis reports

use crate::dependency_analysis::reporting::{AnalysisReport, ReportFormat};

/// Error during formatting
#[derive(Debug)]
pub struct FormatError(pub String);

/// Trait for formatting analysis reports
pub trait ReportFormatter {
    fn format(&self, report: &AnalysisReport) -> Result<String, FormatError>;
    fn supported_formats(&self) -> &[ReportFormat];
}

/// Simple text formatter
pub struct TextFormatter;

impl ReportFormatter for TextFormatter {
    fn format(&self, _report: &AnalysisReport) -> Result<String, FormatError> {
        Ok("Report in text format".to_string())
    }

    fn supported_formats(&self) -> &[ReportFormat] {
        &[ReportFormat::Text]
    }
}

/// Analysis report payload
pub struct AnalysisReport {
    pub summary: String,
}
