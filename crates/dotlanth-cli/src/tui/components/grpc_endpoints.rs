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

//! gRPC Endpoints Testing Component for TUI

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::tui::app::App;
use serde_json;

/// Format JSON string for better readability in TUI
fn format_json_for_display(json_str: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| json_str.to_string()),
        Err(_) => json_str.to_string(),
    }
}

/// Improved JSON formatting for test results with better error handling
fn format_test_result(result: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // First try to parse as JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(result) {
        let pretty_json = serde_json::to_string_pretty(&value).unwrap_or_else(|_| result.to_string());

        for line in pretty_json.lines() {
            let line_owned = line.to_string();

            // Color different JSON elements
            if line.trim_start().starts_with('"') && line.contains(':') {
                // Field names and values
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let key_part = parts[0].trim();
                    let value_part = parts[1].trim();

                    lines.push(Line::from(vec![
                        Span::raw("  ".repeat(line.len() - line.trim_start().len())), // Preserve indentation
                        Span::styled(key_part.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::styled(value_part.to_string(), Style::default().fg(Color::Green)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::White))));
                }
            } else if line.trim() == "{" || line.trim() == "}" || line.trim() == "[" || line.trim() == "]" {
                // Brackets
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
            } else if line.trim().starts_with('"') && line.trim().ends_with(',') {
                // Array elements
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Magenta))));
            } else {
                // Regular content
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::White))));
            }
        }
    } else {
        // Not JSON, format as regular text with highlighting
        for line in result.lines() {
            let line_owned = line.to_string();
            if line.to_lowercase().contains("error") || line.contains("Error:") || line.contains("failed") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))));
            } else if line.to_lowercase().contains("success") || line.contains("OK") || line.contains("ok") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))));
            } else if line.to_lowercase().contains("warning") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
            } else if line.starts_with("grpc.") || line.starts_with("vm_service.") || line.starts_with("runtime.") {
                // Service names
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Cyan))));
            } else {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::White))));
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled("(empty response)", Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC))));
    }

    lines
}

pub fn render_grpc_endpoints_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Endpoints list
            Constraint::Percentage(60), // Test results and details
        ])
        .split(area);

    // Left panel: Endpoints list
    render_endpoints_list(f, app, chunks[0]);

    // Right panel: Test results and details
    render_test_results(f, app, chunks[1]);
}

fn render_endpoints_list(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(6), // Categories
            Constraint::Min(0),    // Endpoints
            Constraint::Length(4), // Controls
        ])
        .split(area);

    // Header
    let header = Paragraph::new("gRPC Endpoints Testing")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Categories
    render_categories(f, app, chunks[1]);

    // Endpoints list
    render_endpoints(f, app, chunks[2]);

    // Controls
    render_endpoint_controls(f, app, chunks[3]);
}

fn render_categories(f: &mut Frame, app: &App, area: Rect) {
    let categories = vec![
        ("VM Service", app.grpc_endpoint_manager.current_category == 0),
        ("Runtime", app.grpc_endpoint_manager.current_category == 1),
        ("Reflection", app.grpc_endpoint_manager.current_category == 2),
        ("Deployment", app.grpc_endpoint_manager.current_category == 3),
        ("Streaming", app.grpc_endpoint_manager.current_category == 4),
    ];

    let category_items: Vec<ListItem> = categories
        .iter()
        .map(|(name, selected)| {
            let style = if *selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(*name, style)))
        })
        .collect();

    let categories_list = List::new(category_items).block(Block::default().title("Categories").borders(Borders::ALL));

    f.render_widget(categories_list, area);
}

fn render_endpoints(f: &mut Frame, app: &App, area: Rect) {
    let endpoints = app.grpc_endpoint_manager.get_endpoints_for_category();

    let endpoint_items: Vec<ListItem> = endpoints
        .iter()
        .enumerate()
        .map(|(i, endpoint)| {
            let selected = i == app.grpc_endpoint_manager.current_endpoint;
            let style = if selected {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let auth_indicator = if endpoint.requires_auth { " [AUTH]" } else { "" };
            let display_text = format!("{}{}", endpoint.method, auth_indicator);

            ListItem::new(Line::from(Span::styled(display_text, style)))
        })
        .collect();

    let endpoints_list = List::new(endpoint_items).block(Block::default().title("Endpoints").borders(Borders::ALL));

    f.render_widget(endpoints_list, area);
}

fn render_endpoint_controls(f: &mut Frame, app: &App, area: Rect) {
    let controls_text = "Enter: Test | ←→: Categories | ↑↓: Endpoints | A: Auth Token";

    let controls = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().title("Controls").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(controls, area);
}

fn render_test_results(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Endpoint details
            Constraint::Length(4), // Request metrics
            Constraint::Min(0),    // Test results
        ])
        .split(area);

    // Endpoint details
    render_endpoint_details(f, app, chunks[0]);

    // Request metrics
    render_request_metrics(f, app, chunks[1]);

    // Test results
    render_test_results_detailed(f, app, chunks[2]);
}

fn render_endpoint_details(f: &mut Frame, app: &App, area: Rect) {
    let endpoint = app.grpc_endpoint_manager.get_selected_endpoint();

    let details_text = if let Some(ep) = endpoint {
        let formatted_request = format_json_for_display(&ep.example_request);
        format!(
            "Service: {}\nMethod: {}\nAuth Required: {}\nExample Request (JSON):\n{}",
            ep.service,
            ep.method,
            if ep.requires_auth { "Yes" } else { "No" },
            formatted_request
        )
    } else {
        "No endpoint selected".to_string()
    };

    let details = Paragraph::new(details_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().title("Endpoint Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(details, area);
}

fn render_request_metrics(f: &mut Frame, app: &App, area: Rect) {
    let metrics = &app.request_metrics;

    let metrics_text = format!(
        "Total: {} | Success: {} | Failed: {} | Success Rate: {:.1}%\nAvg Response: {:.1}ms | Min: {}ms | Max: {}ms",
        metrics.total_requests,
        metrics.successful_requests,
        metrics.failed_requests,
        metrics.success_rate(),
        metrics.avg_response_time,
        if metrics.min_response_time == u64::MAX { 0 } else { metrics.min_response_time },
        metrics.max_response_time
    );

    let metrics_widget = Paragraph::new(metrics_text)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Request Metrics").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(metrics_widget, area);
}

fn render_test_results_detailed(f: &mut Frame, app: &App, area: Rect) {
    if let Some(last_result) = app.grpc_endpoint_manager.test_results.last() {
        // Show the most recent test result with full response
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Result header
                Constraint::Min(0),    // Full response
            ])
            .split(area);

        // Result header
        let status_color = if last_result.success { Color::Green } else { Color::Red };
        let status_icon = if last_result.success { "OK" } else { "FAIL" };
        let header_text = format!(
            "{} {} | {}ms | {}s ago",
            status_icon,
            last_result.endpoint,
            last_result.duration_ms,
            last_result.timestamp.elapsed().as_secs()
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(status_color).add_modifier(Modifier::BOLD))
            .block(Block::default().title("Latest Test Result").borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Full response with improved formatting
        let response_lines = if last_result.response.is_empty() {
            vec![Line::from(Span::styled("No response data", Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)))]
        } else {
            format_test_result(&last_result.response)
        };

        let response = Paragraph::new(response_lines)
            .block(
                Block::default()
                    .title("Full Response (JSON Formatted)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(status_color)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(response, chunks[1]);
    } else {
        // Show test history table when no recent result
        let header = Row::new(vec!["Time", "Endpoint", "Status", "Duration"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = app
            .grpc_endpoint_manager
            .test_results
            .iter()
            .rev()
            .take((area.height as usize).saturating_sub(3))
            .map(|result| {
                let status_style = if result.success { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };

                let status_text = if result.success { "OK" } else { "FAIL" };

                Row::new(vec![
                    format!("{}s ago", result.timestamp.elapsed().as_secs()),
                    result.endpoint.clone(),
                    status_text.to_string(),
                    format!("{}ms", result.duration_ms),
                ])
                .style(status_style)
            })
            .collect();

        let table = Table::new(rows).header(header).block(Block::default().title("Test History").borders(Borders::ALL)).widths(&[
            Constraint::Length(8),
            Constraint::Length(25),
            Constraint::Length(8),
            Constraint::Length(8),
        ]);

        f.render_widget(table, area);
    }
}
