//! Full-screen TUI for Continuum.
//!
//! Provides a Claude Code-style terminal interface with:
//! - Header bar (app name, model, project, git, mode, tokens)
//! - Main conversation timeline
//! - Right sidebar (agent activity, models, files)
//! - Bottom composer (input, hints, status)
//! - Keyboard navigation (vim-like)
//! - Command palette
//! - Model selector with search

pub mod app;
pub mod commands;
pub mod events;
pub mod render;
pub mod theme;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Instant;
use tokio::sync::broadcast;

use crate::repl::ReplSession;
use app::TuiApp;

/// Run the full-screen TUI.
pub fn run_tui(session: ReplSession) -> Result<(), Box<dyn std::error::Error>> {
    run_tui_with_events(session, None)
}

/// Run the full-screen TUI with a live dashboard event stream.
pub fn run_tui_with_events(
    session: ReplSession,
    event_rx: Option<broadcast::Receiver<continuum_telemetry::DashboardEvent>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = TuiApp::new(session);
    app.load_models();
    if let Some(rx) = event_rx {
        app = app.with_event_stream(rx);
    }

    // Add welcome message
    app.add_system_message("Welcome to Continuum TUI. Type /help or /available for commands.");

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// Run the app event loop.
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::KeyEventKind;
    let mut last_tick = Instant::now();

    loop {
        app.drain_live_events();
        // Render
        terminal.draw(|frame| render::render(frame, app))?;

        if app.should_exit {
            break;
        }

        // Handle events
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Only handle Press events - ignore Release and Repeat
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Check for exit
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
                {
                    break;
                }

                // Check for /exit command
                if app.active_panel == app::ActivePanel::Composer
                    && app.composer.input.trim() == "/exit"
                {
                    break;
                }

                // Handle the event
                events::handle_event(app, Event::Key(key));
            }
        }

        // Update elapsed time using wall-clock seconds.
        let now = Instant::now();
        let delta_secs = now.duration_since(last_tick).as_secs();
        if delta_secs > 0 {
            app.agent_activity.elapsed_secs =
                app.agent_activity.elapsed_secs.saturating_add(delta_secs);
            last_tick = now;
        }
    }

    Ok(())
}

/// Quick TUI for testing (runs and exits).
pub fn run_tui_demo() -> Result<(), Box<dyn std::error::Error>> {
    let session = ReplSession::new();
    run_tui(session)
}
