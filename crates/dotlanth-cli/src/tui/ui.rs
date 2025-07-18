use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
};

use crate::tui::app::{App, TabIndex};

/// Render the UI for the application.
pub fn ui(f: &mut Frame<'_>, app: &mut App) {
    let size = f.size();

    // Responsive layout based on terminal size
    let (header_height, footer_height) = if size.height < 15 {
        (2, 1) // Minimal for very small terminals
    } else if size.height < 25 {
        (3, 2) // Compact for small terminals  
    } else {
        (4, 3) // Full for normal terminals
    };

    // Create responsive main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Header with tabs
            Constraint::Min(0),                // Main content
            Constraint::Length(footer_height), // Footer with status
        ])
        .split(size);

    // Render header with tabs
    render_header(f, app, chunks[0]);

    // Render main content based on current tab
    render_content(f, app, chunks[1]);

    // Render footer with status and help
    render_footer(f, app, chunks[2]);

    // Render help popup if needed
    if app.show_help {
        render_help_popup(f, size);
    }

    // Render debug info if enabled
    if app.show_debug {
        render_debug_popup(f, app, size);
    }
}

fn render_header(f: &mut Frame<'_>, app: &App, area: Rect) {
    let size = f.size();

    // Responsive tab titles based on terminal width
    let tab_titles = if size.width < 80 {
        vec!["Overview", "Nodes", "Deploy", "Metrics", "Logs", "gRPC", "Endpoints"]
    } else {
        vec!["Overview", "Nodes", "Deployments", "Metrics", "Logs", "gRPC Server", "gRPC Endpoints"]
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("DotLanth Infrastructure Management")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD))
        .select(app.current_tab as usize);

    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame<'_>, app: &App, area: Rect) {
    match app.current_tab {
        TabIndex::Overview => render_overview(f, app, area),
        TabIndex::Nodes => render_nodes(f, app, area),
        TabIndex::Deployments => render_deployments(f, app, area),
        TabIndex::Metrics => render_metrics(f, app, area),
        TabIndex::Logs => render_logs(f, app, area),
        TabIndex::GrpcServer => render_grpc_server_tab(f, app, area),
        TabIndex::GrpcEndpoints => crate::tui::components::grpc_endpoints::render_grpc_endpoints_tab(f, app, area),
    }
}

fn render_overview(f: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Stats
            Constraint::Min(0),    // Recent activity
        ])
        .split(area);

    // Stats section
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
        .split(chunks[0]);

    // Node count
    let node_count = app.nodes.len();
    let nodes_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Nodes"))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent((node_count * 20).min(100) as u16)
        .label(format!("{} nodes", node_count))
        .use_unicode(true);
    f.render_widget(nodes_gauge, stats_chunks[0]);

    // Deployment count
    let deployment_count = app.deployments.len();
    let deployments_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Deployments"))
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
        .percent((deployment_count * 15).min(100) as u16)
        .label(format!("{} deployments", deployment_count))
        .use_unicode(true);
    f.render_widget(deployments_gauge, stats_chunks[1]);

    // Online nodes
    let online_nodes = app.nodes.iter().filter(|n| matches!(n.status, crate::database::NodeStatus::Online)).count();
    let online_percentage = if node_count > 0 { (online_nodes * 100) / node_count } else { 0 };
    let online_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Online"))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(online_percentage as u16)
        .label(format!("{}% online", online_percentage))
        .use_unicode(true);
    f.render_widget(online_gauge, stats_chunks[2]);

    // System health
    let health_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Health"))
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .percent(85)
        .label("85% healthy")
        .use_unicode(true);
    f.render_widget(health_gauge, stats_chunks[3]);

    // Recent activity
    let recent_logs: Vec<ListItem> = app
        .logs
        .iter()
        .take(10)
        .map(|log| {
            let style = match log.level.as_str() {
                "ERROR" => Style::default().fg(Color::Red),
                "WARN" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::Gray),
            };
            ListItem::new(Line::from(vec![Span::styled(format!("[{}]", log.level), style), Span::raw(" "), Span::raw(&log.message)]))
        })
        .collect();

    let recent_list = List::new(recent_logs).block(Block::default().borders(Borders::ALL).title("Recent Activity"));
    f.render_widget(recent_list, chunks[1]);
}

fn render_nodes(f: &mut Frame<'_>, app: &App, area: Rect) {
    let header = Row::new(vec!["ID", "Address", "Status", "Version", "Last Heartbeat"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .nodes
        .iter()
        .map(|node| {
            let status_style = match node.status {
                crate::database::NodeStatus::Online => Style::default().fg(Color::Green),
                crate::database::NodeStatus::Offline => Style::default().fg(Color::Red),
                crate::database::NodeStatus::Maintenance => Style::default().fg(Color::Yellow),
                crate::database::NodeStatus::Error(_) => Style::default().fg(Color::Red),
            };

            let status_text = match &node.status {
                crate::database::NodeStatus::Online => "Online",
                crate::database::NodeStatus::Offline => "Offline",
                crate::database::NodeStatus::Maintenance => "Maintenance",
                crate::database::NodeStatus::Error(_) => "Error",
            };

            Row::new(vec![
                node.id.chars().take(12).collect::<String>(),
                node.address.clone(),
                status_text.to_string(),
                node.version.clone(),
                node.last_heartbeat.format("%H:%M:%S").to_string(),
            ])
            .style(status_style)
        })
        .collect();

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Nodes"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .widths(&[Constraint::Length(15), Constraint::Length(25), Constraint::Length(12), Constraint::Length(10), Constraint::Length(10)]);

    f.render_widget(table, area);
}

fn render_deployments(f: &mut Frame<'_>, app: &App, area: Rect) {
    let header = Row::new(vec!["ID", "Dot Name", "Version", "Node", "Status"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .deployments
        .iter()
        .map(|deployment| {
            let status_style = match deployment.status {
                crate::database::DeploymentStatus::Running => Style::default().fg(Color::Green),
                crate::database::DeploymentStatus::Pending => Style::default().fg(Color::Yellow),
                crate::database::DeploymentStatus::Stopped => Style::default().fg(Color::Gray),
                crate::database::DeploymentStatus::Failed(_) => Style::default().fg(Color::Red),
            };

            let status_text = match &deployment.status {
                crate::database::DeploymentStatus::Running => "Running",
                crate::database::DeploymentStatus::Pending => "Pending",
                crate::database::DeploymentStatus::Stopped => "Stopped",
                crate::database::DeploymentStatus::Failed(_) => "Failed",
            };

            Row::new(vec![
                deployment.id.chars().take(12).collect::<String>(),
                deployment.dot_name.clone(),
                deployment.dot_version.clone(),
                deployment.node_id.chars().take(12).collect::<String>(),
                status_text.to_string(),
            ])
            .style(status_style)
        })
        .collect();

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Deployments"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .widths(&[Constraint::Length(15), Constraint::Length(20), Constraint::Length(10), Constraint::Length(15), Constraint::Length(10)]);

    f.render_widget(table, area);
}

fn render_metrics(f: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // CPU/Memory gauges
            Constraint::Min(0),    // Metrics table
        ])
        .split(area);

    // Gauges for latest metrics
    if let Some(latest_metric) = app.metrics.first() {
        let gauge_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
            .split(chunks[0]);

        let cpu_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("CPU Usage"))
            .gauge_style(Style::default().fg(Color::Red).bg(Color::Black))
            .percent(latest_metric.cpu_usage as u16)
            .label(format!("{:.1}%", latest_metric.cpu_usage))
            .use_unicode(true);
        f.render_widget(cpu_gauge, gauge_chunks[0]);

        let memory_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Memory Usage"))
            .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
            .percent(latest_metric.memory_usage as u16)
            .label(format!("{:.1}%", latest_metric.memory_usage))
            .use_unicode(true);
        f.render_widget(memory_gauge, gauge_chunks[1]);

        let disk_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Disk Usage"))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(latest_metric.disk_usage as u16)
            .label(format!("{:.1}%", latest_metric.disk_usage))
            .use_unicode(true);
        f.render_widget(disk_gauge, gauge_chunks[2]);
    }

    // Metrics table
    let header = Row::new(vec!["Node", "CPU %", "Memory %", "Disk %", "Net In", "Net Out"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .metrics
        .iter()
        .take(10)
        .map(|metric| {
            Row::new(vec![
                metric.node_id.chars().take(12).collect::<String>(),
                format!("{:.1}", metric.cpu_usage),
                format!("{:.1}", metric.memory_usage),
                format!("{:.1}", metric.disk_usage),
                format!("{}", metric.network_in),
                format!("{}", metric.network_out),
            ])
        })
        .collect();

    let table = Table::new(rows).header(header).block(Block::default().borders(Borders::ALL).title("Recent Metrics")).widths(&[
        Constraint::Length(15),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
    ]);

    f.render_widget(table, chunks[1]);
}

fn render_grpc_server_tab(f: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(8), // Server Status
            Constraint::Length(8), // Server Controls
            Constraint::Min(0),    // Output
        ])
        .split(area);

    // Header
    let header = Paragraph::new("gRPC Server Management")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Server Status
    let status_color = if app.grpc_server_running { Color::Green } else { Color::Red };
    let status_text = if app.grpc_server_running { "RUNNING" } else { "STOPPED" };

    let status = Paragraph::new(format!(
        "Server Status: {}\nAddress: {}:{}\nReflection: Enabled\nServices: runtime.Runtime, vm_service.VmService",
        status_text,
        app.context.config.grpc.server_host,
        app.context.config.grpc.server_port
    ))
    .style(Style::default().fg(status_color))
    .block(Block::default().title("Server Status").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[1]);

    // Controls
    let controls_text = if app.grpc_server_running {
        "S - Stop Server | R - Restart | T - Test Connection | L - View Logs"
    } else {
        "S - Start Server | B - Build | C - Clean Build | T - Test Connection"
    };

    let controls = Paragraph::new(controls_text).block(Block::default().title("Controls").borders(Borders::ALL)).wrap(Wrap { trim: true });
    f.render_widget(controls, chunks[2]);

    // Output
    let output_text = if app.grpc_server_running {
        "Server is running...\nCheck logs for detailed output."
    } else {
        "Server is stopped.\nPress 'S' to start the server."
    };

    let output = Paragraph::new(output_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().title("Server Output").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(output, chunks[3]);
}

fn render_logs(f: &mut Frame<'_>, app: &App, area: Rect) {
    let logs: Vec<ListItem> = app
        .logs
        .iter()
        .skip(app.scroll_offset)
        .take(area.height as usize - 2)
        .map(|log| {
            let style = match log.level.as_str() {
                "ERROR" => Style::default().fg(Color::Red),
                "WARN" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Green),
                "DEBUG" => Style::default().fg(Color::Cyan),
                _ => Style::default().fg(Color::Gray),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}]", log.timestamp.format("%H:%M:%S")), Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(format!("[{}]", log.level), style),
                Span::raw(" "),
                Span::styled(format!("[{}]", log.node_id.chars().take(8).collect::<String>()), Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::raw(&log.message),
            ]))
        })
        .collect();

    let logs_list = List::new(logs).block(Block::default().borders(Borders::ALL).title("System Logs"));
    f.render_widget(logs_list, area);
}

fn render_footer(f: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(0), Constraint::Length(30)]).split(area);

    // Status message
    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[0]);

    // Help text
    let help_text = "q: Quit | h: Help | r: Refresh | Tab: Next | Up/Down: Scroll";
    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Controls")).alignment(Alignment::Center);
    f.render_widget(help, chunks[1]);
}

fn render_help_popup(f: &mut Frame<'_>, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    let help_text = vec![
        Line::from("DotLanth Infrastructure Management - Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  Tab / Shift+Tab  - Switch between tabs"),
        Line::from("  Up / Down        - Scroll up/down"),
        Line::from("  Enter            - Select/Action"),
        Line::from("  Esc              - Close popups"),
        Line::from(""),
        Line::from("Commands:"),
        Line::from("  q                - Quit application"),
        Line::from("  r                - Refresh data"),
        Line::from("  h                - Toggle this help"),
        Line::from("  d                - Toggle debug info"),
        Line::from(""),
        Line::from("Tabs:"),
        Line::from("  Overview         - System summary"),
        Line::from("  Nodes            - Node management"),
        Line::from("  Deployments      - Deployment status"),
        Line::from("  Metrics          - Performance metrics"),
        Line::from("  Logs             - System logs"),
        Line::from(""),
        Line::from("Press 'h' or 'Esc' to close this help."),
    ];

    let help_paragraph = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Help")).wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}

fn render_debug_popup(f: &mut Frame<'_>, app: &App, area: Rect) {
    let popup_area = centered_rect(50, 50, area);

    let debug_text = vec![
        Line::from("Debug Information"),
        Line::from(""),
        Line::from(format!("Current Tab: {:?}", app.current_tab)),
        Line::from(format!("Scroll Offset: {}", app.scroll_offset)),
        Line::from(format!("Nodes: {}", app.nodes.len())),
        Line::from(format!("Deployments: {}", app.deployments.len())),
        Line::from(format!("Metrics: {}", app.metrics.len())),
        Line::from(format!("Logs: {}", app.logs.len())),
        Line::from(format!("Last Update: {:?}", app.last_update)),
        Line::from(""),
        Line::from("Press 'd' or 'Esc' to close."),
    ];

    let debug_paragraph = Paragraph::new(debug_text).block(Block::default().borders(Borders::ALL).title("Debug")).wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(debug_paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
