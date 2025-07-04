use crate::commands::CommandContext;
use crate::database::{DeploymentInfo, LogEntry, MetricEntry, NodeInfo};
use anyhow::Result;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabIndex {
    Overview = 0,
    Nodes = 1,
    Deployments = 2,
    Metrics = 3,
    Logs = 4,
}

impl TabIndex {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => TabIndex::Overview,
            1 => TabIndex::Nodes,
            2 => TabIndex::Deployments,
            3 => TabIndex::Metrics,
            4 => TabIndex::Logs,
            _ => TabIndex::Overview,
        }
    }

    pub fn next(self) -> Self {
        Self::from_index((self as usize + 1) % 5)
    }

    pub fn previous(self) -> Self {
        Self::from_index((self as usize + 4) % 5)
    }
}

pub struct App {
    pub context: CommandContext,
    pub current_tab: TabIndex,
    pub nodes: Vec<NodeInfo>,
    pub deployments: Vec<DeploymentInfo>,
    pub logs: Vec<LogEntry>,
    pub metrics: Vec<MetricEntry>,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub show_debug: bool,
    pub last_update: Instant,
    pub status_message: String,
}

impl App {
    pub fn new(context: CommandContext) -> Self {
        let mut app = Self {
            context,
            current_tab: TabIndex::Overview,
            nodes: Vec::new(),
            deployments: Vec::new(),
            logs: Vec::new(),
            metrics: Vec::new(),
            scroll_offset: 0,
            show_help: false,
            show_debug: false,
            last_update: Instant::now(),
            status_message: "Welcome to DotLanth Infrastructure Management".to_string(),
        };

        if let Err(e) = app.refresh_data() {
            app.status_message = format!("Error loading data: {}", e);
        }

        app
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        self.nodes = self.context.database.list_nodes()?;
        self.deployments = self.context.database.list_deployments()?;
        self.logs = self.context.database.get_recent_logs(None, self.context.config.ui.max_log_lines)?;
        self.metrics = self.context.database.get_recent_metrics(None, self.context.config.ui.max_log_lines)?;
        self.last_update = Instant::now();
        self.status_message = format!("Data refreshed at {}", chrono::Local::now().format("%H:%M:%S"));
        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.scroll_offset = 0;
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = self.current_tab.previous();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn handle_enter(&mut self) -> Result<()> {
        match self.current_tab {
            TabIndex::Nodes => {
                self.status_message = "Node action placeholder".to_string();
            }
            TabIndex::Deployments => {
                self.status_message = "Deployment action placeholder".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_escape(&mut self) {
        self.show_help = false;
        self.show_debug = false;
        self.scroll_offset = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_debug(&mut self) {
        self.show_debug = !self.show_debug;
    }

    pub fn tick(&mut self) {
        let refresh_interval = Duration::from_millis(self.context.config.ui.refresh_rate_ms);
        if self.last_update.elapsed() >= refresh_interval {
            if let Err(e) = self.refresh_data() {
                self.status_message = format!("Auto-refresh failed: {}", e);
            }
        }
    }

    pub fn get_refresh_rate(&self) -> u64 {
        self.context.config.ui.refresh_rate_ms / 4
    }

    pub fn get_tab_titles(&self) -> Vec<&str> {
        vec!["Overview", "Nodes", "Deployments", "Metrics", "Logs"]
    }
}
