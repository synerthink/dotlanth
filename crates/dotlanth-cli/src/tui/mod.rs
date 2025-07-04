pub mod app;
pub mod components;
pub mod events;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{io, time::Duration};

use crate::commands::CommandContext;
use app::App;
use ui::ui;

pub fn run_tui(ctx: CommandContext) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(ctx);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(app.get_refresh_rate()))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('r') => {
                            if let Err(e) = app.refresh_data() {
                                app.status_message = format!("Refresh error: {}", e);
                            }
                        }
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.previous_tab(),
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::Left => app.scroll_up(),
                        KeyCode::Right => app.scroll_down(),
                        KeyCode::Enter => {
                            if let Err(e) = app.handle_enter() {
                                app.status_message = format!("Action error: {}", e);
                            }
                        }
                        KeyCode::Esc => app.handle_escape(),
                        KeyCode::Char('h') => app.toggle_help(),
                        KeyCode::Char('d') => app.toggle_debug(),
                        _ => {}
                    }
                }
            }
        }

        app.tick();
    }
}
