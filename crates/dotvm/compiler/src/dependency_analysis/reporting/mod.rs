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

//! Reporting system for dependency analysis results
//!
//! This module provides comprehensive reporting capabilities for dependency analysis
//! results. It includes formatters for different output formats, metrics collection
//! and analysis, and visualization tools for understanding complex dependency
//! relationships and analysis results.
//!
//! ## Core Reporting Components
//!
//! ### Report Formatting (`formatter`)
//! - **Purpose**: Converts analysis results into various output formats
//! - **Formats**: JSON, XML, HTML, Markdown, plain text, CSV
//! - **Customization**: Template-based formatting with custom styling
//! - **Integration**: Export formats compatible with external tools and dashboards
//! - **Features**: Syntax highlighting, interactive elements, responsive design
//!
//! ### Metrics Collection (`metrics`)
//! - **Purpose**: Collects and analyzes performance and quality metrics
//! - **Categories**: Performance metrics, quality metrics, complexity metrics
//! - **Analysis**: Statistical analysis, trend detection, comparative analysis
//! - **Visualization**: Charts, graphs, and statistical summaries
//! - **Historical**: Tracking metrics over time for trend analysis
//!
//! ### Dependency Visualization (`visualization`)
//! - **Purpose**: Creates visual representations of dependency relationships
//! - **Graph Types**: Directed graphs, hierarchical trees, network diagrams
//! - **Interactive Features**: Zoom, pan, filter, search, highlight
//! - **Export Options**: SVG, PNG, PDF, interactive HTML
//! - **Layouts**: Force-directed, hierarchical, circular, custom layouts
//!
//! ## Report Types and Formats
//!
//! ### Summary Reports
//! - **Overview**: High-level summary of analysis results
//! - **Key Metrics**: Most important metrics and findings
//! - **Recommendations**: Actionable recommendations based on analysis
//! - **Executive Summary**: Business-focused summary for stakeholders
//!
//! ### Detailed Analysis Reports
//! - **Complete Results**: Full analysis results with all details
//! - **Technical Details**: In-depth technical information for developers
//! - **Code Examples**: Specific code locations and examples
//! - **Cross-References**: Links between related findings and dependencies
//!
//! ### Security Reports
//! - **Vulnerability Assessment**: Security vulnerabilities and risks
//! - **Threat Analysis**: Potential security threats and attack vectors
//! - **Compliance**: Compliance with security standards and best practices
//! - **Mitigation**: Specific mitigation strategies and recommendations
//!
//! ### Performance Reports
//! - **Optimization Opportunities**: Performance optimization suggestions
//! - **Resource Usage**: Memory, CPU, and storage usage analysis
//! - **Bottleneck Analysis**: Performance bottlenecks and hot spots
//! - **Benchmarking**: Comparison with performance benchmarks
//!
//! ## Visualization Capabilities
//!
//! ### Dependency Graphs
//! - **Node Types**: Modules, functions, variables, state variables
//! - **Edge Types**: Dependencies, data flow, control flow, state access
//! - **Visual Encoding**: Colors, shapes, sizes to represent different attributes
//! - **Clustering**: Grouping related components for better organization
//!
//! ### Flow Diagrams
//! - **Control Flow**: Visual representation of program control flow
//! - **Data Flow**: How data moves through the program
//! - **State Flow**: State transitions and access patterns
//! - **Call Graphs**: Function call relationships and hierarchies
//!
//! ### Metrics Dashboards
//! - **Real-time Metrics**: Live updating metrics and statistics
//! - **Historical Trends**: Metrics over time with trend analysis
//! - **Comparative Analysis**: Side-by-side comparison of different analyses
//! - **Drill-down**: Ability to explore detailed information from summaries
//!
//! ## Integration and Export
//!
//! ### External Tool Integration
//! - **IDE Plugins**: Integration with development environments
//! - **CI/CD Pipelines**: Automated reporting in continuous integration
//! - **Monitoring Systems**: Integration with monitoring and alerting systems
//! - **Documentation**: Automatic documentation generation from analysis
//!
//! ### Export Formats
//! - **Structured Data**: JSON, XML, YAML for programmatic access
//! - **Documents**: PDF, Word, HTML for human consumption
//! - **Presentations**: PowerPoint, reveal.js for stakeholder presentations
//! - **Data Analysis**: CSV, Excel for further statistical analysis
//!
//! ## Customization and Extensibility
//!
//! ### Template System
//! - **Custom Templates**: User-defined report templates
//! - **Styling**: CSS and theme customization for branded reports
//! - **Conditional Content**: Dynamic content based on analysis results
//! - **Internationalization**: Multi-language support for global teams
//!
//! ### Plugin Architecture
//! - **Custom Formatters**: User-defined output formatters
//! - **Visualization Plugins**: Custom visualization components
//! - **Metric Calculators**: Custom metric calculation algorithms
//! - **Export Handlers**: Custom export format implementations
//!
//! ## Performance and Scalability
//!
//! - **Streaming**: Streaming report generation for large datasets
//! - **Pagination**: Paginated reports for better performance
//! - **Compression**: Compressed output formats for large reports
//! - **Caching**: Cached report generation for repeated requests
//! - **Parallel Processing**: Parallel report generation for multiple formats

pub mod formatter;
pub mod metrics;
pub mod visualization;

pub use formatter::{AnalysisReport, FormatError, ReportFormat, ReportFormatter};
pub use metrics::AnalysisMetrics;
pub use visualization::DependencyVisualizer;
