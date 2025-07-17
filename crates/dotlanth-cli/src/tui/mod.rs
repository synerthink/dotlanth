use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

pub mod app;
pub mod components;
pub mod events;
pub mod ui;

use crate::commands::CommandContext;
use app::App;

pub fn run(ctx: CommandContext) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(ctx);

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        // Poll for events with timeout to avoid blocking
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Global shortcuts
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.previous_tab(),
                        KeyCode::Char('h') => app.toggle_help(),
                        KeyCode::Char('d') => app.toggle_debug(),

                        // Number key shortcuts (1-7)
                        KeyCode::Char('1') => app.current_tab = app::TabIndex::Overview,
                        KeyCode::Char('2') => app.current_tab = app::TabIndex::Nodes,
                        KeyCode::Char('3') => app.current_tab = app::TabIndex::Deployments,
                        KeyCode::Char('4') => app.current_tab = app::TabIndex::Metrics,
                        KeyCode::Char('5') => app.current_tab = app::TabIndex::Logs,
                        KeyCode::Char('6') => app.current_tab = app::TabIndex::GrpcServer,
                        KeyCode::Char('7') => app.current_tab = app::TabIndex::GrpcEndpoints,

                        // gRPC Server controls
                        KeyCode::Char('s') | KeyCode::Char('S') if app.current_tab == app::TabIndex::GrpcServer => {
                            if app.grpc_server_running {
                                let _ = app.stop_grpc_server();
                            } else {
                                let _ = app.start_grpc_server();
                            }
                        }
                        KeyCode::Char('b') | KeyCode::Char('B') if app.current_tab == app::TabIndex::GrpcServer => {
                            let _ = app.build_grpc_server();
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') if app.current_tab == app::TabIndex::GrpcServer => {
                            let _ = app.test_grpc_connection();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') if app.current_tab == app::TabIndex::GrpcServer => {
                            let _ = app.restart_grpc_server();
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') if app.current_tab == app::TabIndex::GrpcServer => {
                            app.toggle_grpc_logs();
                        }

                        // gRPC Endpoints controls
                        KeyCode::Left if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.grpc_endpoint_manager.previous_category();
                        }
                        KeyCode::Right if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.grpc_endpoint_manager.next_category();
                        }
                        KeyCode::Up if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.grpc_endpoint_manager.previous_endpoint();
                        }
                        KeyCode::Down if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.grpc_endpoint_manager.next_endpoint();
                        }
                        KeyCode::Enter if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            if let Some(endpoint) = app.grpc_endpoint_manager.get_selected_endpoint().cloned() {
                                app.status_message = format!("Testing: {}/{} ...", endpoint.service, endpoint.method);
                                if let Err(e) = app.test_endpoint_sync(&endpoint) {
                                    app.status_message = format!("Test error: {}", e);
                                } else {
                                    let last_result = app.grpc_endpoint_manager.test_results.last();
                                    if let Some(result) = last_result {
                                        if result.success {
                                            app.status_message = format!("✅ {}/{} ({}ms)", endpoint.service, endpoint.method, result.duration_ms);
                                        } else {
                                            app.status_message = format!("❌ {}/{} failed", endpoint.service, endpoint.method);
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.auth_token = Some("test_api_key_12345".to_string());
                            app.status_message = "Auth token set".to_string();
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') if app.current_tab == app::TabIndex::GrpcEndpoints => {
                            app.auth_token = None;
                            app.status_message = "Auth token cleared".to_string();
                        }

                        // General navigation
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::PageUp => {
                            for _ in 0..5 {
                                app.scroll_up();
                            }
                        }
                        KeyCode::PageDown => {
                            for _ in 0..5 {
                                app.scroll_down();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update app state
        app.update();
    }
}
