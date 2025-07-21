use crossterm::ExecutableCommand;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

// Import the model types and message types
use crate::tui::model::AppState;
use crate::tui::view::draw_ui;

// Include helper functions and view logic
mod helpers;
mod message;
pub mod model; // Made public for CLI formatter
mod view;

// Include the main view module
mod loose_details;
mod main_view;
mod pack_details;
pub mod widget; // Made public for CLI formatter

// Include the split update modules
mod loaders;
mod navigation;
mod scrolling;
mod update;

pub fn run_tui(plumber: crate::GitPlumber) -> Result<(), String> {
    // Terminal initialization
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .map_err(|e| format!("Failed to enter alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {e}"))?;

    // Initialize application state
    let mut app = AppState::new(plumber.get_repo_path().to_path_buf());

    // Initial loading of pack files
    let initial_msg = app.load_git_objects(&plumber);
    app.update(initial_msg, &plumber);

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &plumber);

    // Clean up
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {e}"))?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map_err(|e| format!("Failed to leave alternate screen: {e}"))?;
    terminal
        .show_cursor()
        .map_err(|e| format!("Failed to show cursor: {e}"))?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    plumber: &crate::GitPlumber,
) -> Result<(), String> {
    loop {
        // Update layout dimensions based on current terminal size
        let terminal_size = terminal
            .size()
            .map_err(|e| format!("Failed to get terminal size: {e}"))?;
        app.update_layout_dimensions(terminal_size);

        // Render the UI
        terminal
            .draw(|f| draw_ui(f, app))
            .map_err(|e| format!("Failed to draw: {e}"))?;

        // Handle input events
        if event::poll(Duration::from_millis(100))
            .map_err(|e| format!("Failed to poll events: {e}"))?
        {
            if let Event::Key(key) =
                event::read().map_err(|e| format!("Failed to read event: {e}"))?
            {
                // Convert key events to messages using the main_view key handler
                if let Some(msg) = match app.view {
                    model::AppView::Main { .. } => {
                        crate::tui::main_view::handle_key_event(key, app)
                    }
                    model::AppView::PackObjectDetail { .. } => {
                        crate::tui::pack_details::handle_key_event(key, app)
                    }
                    model::AppView::LooseObjectDetail { .. } => {
                        crate::tui::loose_details::handle_key_event(key, app)
                    }
                } {
                    // Update the application state
                    if !app.update(msg, plumber) {
                        return Ok(());
                    }
                }
            }
        }
    }
}
