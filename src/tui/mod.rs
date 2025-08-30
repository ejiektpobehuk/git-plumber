use crossbeam_channel::{Receiver, select, tick, unbounded};
pub fn run_tui_with_options(plumber: crate::GitPlumber, opts: RunOptions) -> Result<(), String> {
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
    app.reduced_motion = opts.reduced_motion;

    // Background channel for worker -> UI messages
    let (tx, rx) = unbounded::<crate::tui::message::Message>();
    app.set_tx(tx.clone());

    // Enqueue initial load as a Command; runner will execute it
    app.effects.push(crate::tui::message::Command::LoadInitial);

    // Start filesystem watcher for live updates
    if let Ok(w) = crate::tui::watcher::spawn_git_watcher(app.repo_path.clone(), tx.clone()) {
        app.fs_watcher = Some(w);
    } else if let Err(e) = crate::tui::watcher::spawn_git_watcher(app.repo_path.clone(), tx.clone())
    {
        eprintln!("Watcher error: {e}");
    }

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &plumber, rx, tx);

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

pub fn run_tui(plumber: crate::GitPlumber) -> Result<(), String> {
    run_tui_with_options(
        plumber,
        RunOptions {
            reduced_motion: false,
        },
    )
}

pub struct RunOptions {
    pub reduced_motion: bool,
}

use crossterm::ExecutableCommand;
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
pub mod main_view;
mod pack_details;
pub mod widget; // Made public for CLI formatter

// Include the split update modules
mod git_tree;
mod loaders;
mod navigation;
mod pure_loaders;
mod scrolling;
mod update;

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    plumber: &crate::GitPlumber,
    rx: Receiver<crate::tui::message::Message>,
    tx: crossbeam_channel::Sender<crate::tui::message::Message>,
) -> Result<(), String> {
    // Dynamic timer thread - only active when animations are running
    let mut timer_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut timer_stop_tx: Option<crossbeam_channel::Sender<()>> = None;

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

    // Start event listener thread for all terminal events (resize, keyboard)
    let tx_events = tx.clone();
    let _event_thread = std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(crossterm::event::Event::Resize(width, height)) => {
                    if tx_events
                        .send(crate::tui::message::Message::TerminalResize(width, height))
                        .is_err()
                    {
                        break; // Main thread closed
                    }
                }
                Ok(crossterm::event::Event::Key(key)) => {
                    if tx_events
                        .send(crate::tui::message::Message::KeyEvent(key))
                        .is_err()
                    {
                        break; // Main thread closed
                    }
                }
                Ok(_) => {
                    // Ignore other events (mouse, focus, etc.)
                    continue;
                }
                Err(_) => {
                    // Error reading events, exit thread
                    break;
                }
            }
        }
    });

    loop {
        // Manage timer thread lifecycle based on animation state
        let needs_animations = app.has_active_animations();
        if needs_animations && timer_handle.is_none() {
            // Start timer thread when animations begin
            let tx_timer = tx.clone();
            let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
            timer_stop_tx = Some(stop_tx);
            timer_handle = Some(std::thread::spawn(move || {
                let timer = tick(Duration::from_millis(100));
                loop {
                    // Check for stop signal first (non-blocking)
                    if stop_rx.try_recv().is_ok() {
                        break; // Stop signal received
                    }

                    // Then check for timer tick
                    if timer.recv().is_ok() {
                        if tx_timer
                            .send(crate::tui::message::Message::TimerTick)
                            .is_err()
                        {
                            break; // Main thread has closed
                        }
                    } else {
                        break; // Timer channel closed
                    }
                }
            }));
        } else if !needs_animations && timer_handle.is_some() {
            // Stop timer thread when animations end
            if let Some(stop_tx) = timer_stop_tx.take() {
                let _ = stop_tx.send(()); // Signal timer thread to stop
            }
            if let Some(handle) = timer_handle.take() {
                let _ = handle.join(); // Wait for thread to finish
            }
        }

        // Single unified select! loop - all events come through message channel
        select! {
            recv(rx) -> msg => {
                if let Ok(msg) = msg {
                    if !app.update(msg, plumber) {
                        // Clean up timer thread on exit
                        if let Some(stop_tx) = timer_stop_tx.take() {
                            let _ = stop_tx.send(());
                        }
                        if let Some(handle) = timer_handle.take() {
                            let _ = handle.join();
                        }
                        return Ok(());
                    }
                    run_commands(app);
                }
            }
        }

        // Clear executed effects
        app.effects.clear();

        // Redraw after any event
        terminal
            .draw(|f| draw_ui(f, app))
            .map_err(|e| format!("Failed to draw: {e}"))?;
    }
}
