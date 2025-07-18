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
        Ok(value) => {
            // Use custom pretty printing with 2-space indentation
            match serde_json::to_string_pretty(&value) {
                Ok(pretty) => {
                    // Replace 4-space indentation with 2-space for better TUI display
                    pretty
                        .lines()
                        .map(|line| {
                            let leading_spaces = line.len() - line.trim_start().len();
                            let new_indent = " ".repeat(leading_spaces / 2);
                            format!("{}{}", new_indent, line.trim_start())
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                Err(_) => json_str.to_string(),
            }
        }
        Err(_) => json_str.to_string(),
    }
}

/// Format test result for better display
fn format_test_result(result: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Try to parse as JSON first
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(result) {
        let pretty_json = serde_json::to_string_pretty(&value).unwrap_or_else(|_| result.to_string());

        for line in pretty_json.lines() {
            let line_owned = line.to_string();
            if line.trim().starts_with('"') && line.contains(':') {
                // Field name and value
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    lines.push(Line::from(vec![
                        Span::styled(parts[0].trim().to_string(), Style::default().fg(Color::Cyan)),
                        Span::raw(": "),
                        Span::styled(parts[1].trim().to_string(), Style::default().fg(Color::Green)),
                    ]));
                } else {
                    lines.push(Line::from(line_owned));
                }
            } else if line.trim() == "{" || line.trim() == "}" || line.trim() == "[" || line.trim() == "]" {
                // Brackets
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Yellow))));
            } else {
                // Regular line
                lines.push(Line::from(line_owned));
            }
        }
    } else {
        // Not JSON, format as regular text with some highlighting
        for line in result.lines() {
            let line_owned = line.to_string();
            if line.contains("Error:") || line.contains("error") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Red))));
            } else if line.contains("Success") || line.contains("OK") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Green))));
            } else if line.contains("Warning") {
                lines.push(Line::from(Span::styled(line_owned, Style::default().fg(Color::Yellow))));
            } else {
                lines.push(Line::from(line_owned));
            }
        }
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
        ("Advanced", app.grpc_endpoint_manager.current_category == 3),
        ("Week 3 Features", app.grpc_endpoint_manager.current_category == 4),
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
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if endpoint.requires_auth {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let auth_indicator = if endpoint.requires_auth { "🔒 " } else { "   " };
            let method_text = format!("{}{}", auth_indicator, endpoint.method);

            ListItem::new(Line::from(Span::styled(method_text, style)))
        })
        .collect();

    let endpoints_list = List::new(endpoint_items).block(Block::default().title("Endpoints").borders(Borders::ALL));

    f.render_widget(endpoints_list, area);
}

fn render_endpoint_controls(f: &mut Frame, app: &App, area: Rect) {
    let auth_status = if app.auth_token.is_some() { "🔑 Auth: Set" } else { "🔓 Auth: None" };

    let controls_text = format!("↑↓: Select | ←→: Category | Enter: Test | A: Set Auth | X: Clear Auth\n{}", auth_status);

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
            Constraint::Length(8), // Current endpoint details
            Constraint::Length(4), // Request metrics (smaller)
            Constraint::Min(0),    // Test results and responses (larger)
        ])
        .split(area);

    // Current endpoint details
    render_endpoint_details(f, app, chunks[0]);

    // Request metrics
    render_request_metrics(f, app, chunks[1]);

    // Test results and full responses
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
        let status_icon = if last_result.success { "✅" } else { "❌" };
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
            vec![Line::from(Span::styled("No response data", Style::default().fg(Color::Gray)))]
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

                let status_text = if result.success { "✅ OK" } else { "❌ FAIL" };

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
