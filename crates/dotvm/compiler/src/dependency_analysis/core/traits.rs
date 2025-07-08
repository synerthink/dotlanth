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

//! Analysis traits and interfaces

use crate::dependency_analysis::detection::pattern_matcher::PatternType;

/// Represents different types of analyses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisType {
    ControlFlow,
    DataFlow,
    StateAccess,
    DependencyDetection,
}

/// Trait for dependency analyzers
pub trait DependencyAnalyzer {
    /// Input type for analysis
    type Input;
    /// Output type of the analysis
    type Output;
    /// Error type for analysis failures
    type Error;

    /// Perform analysis on the given input
    fn analyze(&self, input: &Self::Input) -> Result<Self::Output, Self::Error>;

    /// The type of analysis this analyzer performs
    fn analysis_type(&self) -> AnalysisType;

    /// Dependencies on other analysis types
    fn dependencies(&self) -> &[AnalysisType];

    /// Check if this analyzer can analyze the given input
    fn can_analyze(&self, input: &Self::Input) -> bool;
}

/// Trait for pattern matchers
pub trait PatternMatcher {
    /// Pattern descriptor type
    type Pattern;
    /// Context against which patterns are matched
    type Context;

    /// Check if the pattern matches given context
    fn matches(&self, pattern: &Self::Pattern, context: &Self::Context) -> bool;

    /// Confidence score of the matcher
    fn confidence(&self) -> f32;

    /// Extract information when a match is found
    fn extract_info(&self, pattern: &Self::Pattern, context: &Self::Context) -> ();
}

/// Trait for dependency detectors
pub trait DependencyDetector {
    /// Analysis context passed to the detector
    type Context;
    /// Output type of detection results
    type Output;

    /// Detect dependencies given an analysis context
    fn detect_dependencies(&self, context: &Self::Context) -> Self::Output;

    /// Patterns supported by this detector
    fn supported_patterns(&self) -> &[PatternType];

    /// Strategy used for detection (static, dynamic, hybrid, etc.)
    fn detection_strategy(&self) -> ();
}
