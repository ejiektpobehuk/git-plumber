use crossbeam_channel::{Receiver, select, tick, unbounded};
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::thread;
use std::time::Duration;

// Import the model types and message types
use crate::tui::model::AppState;
use crate::tui::view::draw_ui;

// Include helper functions and view logic
mod helpers;
mod message;
pub mod model; // Made public for CLI formatter
mod view;

// FS watcher
mod watcher;

// Include the main view module
mod loose_details;
mod main_view;
mod pack_details;
pub mod widget; // Made public for CLI formatter

// Include the split update modules
mod git_tree;
mod loaders;
mod navigation;
mod pure_loaders;
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

    // Background channel for worker -> UI messages
    let (tx, rx) = unbounded::<crate::tui::message::Message>();
    app.set_tx(tx.clone());

    // Enqueue initial load as a Command; runner will execute it
    app.effects.push(crate::tui::message::Command::LoadInitial);

    // Main event loop
    // Start filesystem watcher for live updates
    if let Ok(w) = crate::tui::watcher::spawn_git_watcher(app.repo_path.clone(), tx.clone()) {
        app.fs_watcher = Some(w);
    } else if let Err(e) = crate::tui::watcher::spawn_git_watcher(app.repo_path.clone(), tx.clone())
    {
        eprintln!("Watcher error: {e}");
    }

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &plumber, rx, tx.clone());

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
    rx: Receiver<crate::tui::message::Message>,
    tx: crossbeam_channel::Sender<crate::tui::message::Message>,
) -> Result<(), String> {
    let ticker = tick(Duration::from_millis(100));

    // Helper to execute commands emitted by update
    let run_commands = |app: &AppState| {
        for cmd in &app.effects {
            match cmd {
                crate::tui::message::Command::LoadInitial => {
                    let repo_path = app.repo_path.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let worker_plumber = crate::GitPlumber::new(&repo_path);
                        let res = crate::tui::pure_loaders::load_git_objects_pure(&worker_plumber);
                        let _ = tx.send(match res {
                            Ok(data) => crate::tui::message::Message::GitObjectsLoaded(data),
                            Err(e) => crate::tui::message::Message::LoadGitObjects(Err(e)),
                        });
                    });
                }
                crate::tui::message::Command::LoadPackObjects { path } => {
                    let path = path.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let res = crate::tui::pure_loaders::load_pack_objects_pure(&path);
                        let _ = tx.send(crate::tui::message::Message::LoadPackObjects {
                            path,
                            result: res,
                        });
                    });
                }
            }
        }
    };

    // Execute any queued effects before entering the loop (e.g., initial LoadInitial)
    run_commands(app);
    app.effects.clear();

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

        // Multiplex background messages, timer, and input
        select! {
            recv(rx) -> msg => {
                if let Ok(msg) = msg {
                    if !app.update(msg, plumber) { return Ok(()); }
                    run_commands(app);
                }
            }
            recv(ticker) -> _ => {
                // Periodic maintenance: expire ephemeral highlights (new + deleted)
                let mut needs_redraw = false;
                if let crate::tui::model::AppView::Main { state } = &mut app.view {
                    if state.prune_timeouts() {
                        state.flatten_tree();
                        needs_redraw = true;
                    }
                }
                if needs_redraw {
                    terminal
                        .draw(|f| draw_ui(f, app))
                        .map_err(|e| format!("Failed to draw: {e}"))?;
                }
            }
            default => {
                // Non-blocking keyboard handling
                if event::poll(Duration::from_millis(0))
                    .map_err(|e| format!("Failed to poll events: {e}"))?
                {
                    if let Event::Key(key) =
                        event::read().map_err(|e| format!("Failed to read event: {e}"))?
                    {
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
                            if !app.update(msg, plumber) { return Ok(()); }
                            run_commands(app);
                        }
                    }
                }
            }
        }
        // Clear executed effects
        app.effects.clear();
    }
}
