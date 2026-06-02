//! Event handling for the TUI.

use continuum_core::planner::Planner;
use continuum_core::repo::RepoLoader;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use super::app::*;

/// Handle keyboard events. Returns true if the app should exit.
pub fn handle_event(app: &mut TuiApp, event: Event) -> bool {
    match event {
        Event::Key(key) => {
            // Dismiss model error on any key press
            if app.model_error.is_some() {
                app.model_error = None;
            }

            // Global shortcuts (work in any panel)
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => return true,
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => return true,
                (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                    // Toggle task panel
                    return false;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    // Clear screen
                    app.messages.clear();
                    app.agent_activity.tasks.clear();
                    return false;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                    // Agent activity
                    return false;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
                    // Retry step
                    return false;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                    // Stop agent
                    app.status = AppStatus::Ready;
                    app.clear_current_step();
                    return false;
                }
                _ => {}
            }

            // Panel-specific handling
            match app.active_panel {
                ActivePanel::Composer => handle_composer_input(app, key),
                ActivePanel::ModelSelector => handle_model_selector_input(app, key),
                ActivePanel::CommandPalette => handle_command_palette_input(app, key),
                ActivePanel::ApprovalPrompt => handle_approval_input(app, key),
                ActivePanel::Timeline => handle_timeline_input(app, key),
                ActivePanel::Sidebar => handle_sidebar_input(app, key),
            }
        }
        Event::Resize(cols, rows) => {
            // Terminal resized, UI will re-render on next draw
            tracing::debug!("terminal resized to {cols}x{rows}");
            false
        }
        _ => false,
    }
}

/// Handle input in the composer panel.
fn handle_composer_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.active_panel = ActivePanel::Timeline;
            false
        }
        KeyCode::Tab => {
            if app.slash_mode() && !app.slash_matches.is_empty() {
                app.apply_slash_completion(true);
                return false;
            }
            app.active_panel = ActivePanel::Sidebar;
            false
        }
        KeyCode::Enter => {
            let input = app.composer.input.trim().to_string();
            if !input.is_empty() {
                if input.starts_with('/') {
                    if app.slash_is_ambiguous() {
                        app.apply_slash_completion(true);
                        return false;
                    } else {
                        // Handle slash command
                        handle_slash_command(app, &input);
                    }
                } else {
                    // User message
                    app.add_user_message(&input);
                    // Simulate agent response
                    simulate_agent_response(app, &input);
                }
                app.composer.input.clear();
                app.composer.cursor_pos = 0;
                app.refresh_slash_matches();
            }
            false
        }
        KeyCode::Char(c) => {
            app.composer.input.push(c);
            app.composer.cursor_pos += 1;
            app.refresh_slash_matches();
            false
        }
        KeyCode::Backspace => {
            if app.composer.cursor_pos > 0 {
                app.composer.input.pop();
                app.composer.cursor_pos -= 1;
            }
            app.refresh_slash_matches();
            false
        }
        KeyCode::Up => {
            if app.slash_mode() && !app.slash_matches.is_empty() {
                app.move_slash_selection(-1);
                return false;
            }
            // Scroll timeline up
            if app.scroll_offset > 0 {
                app.scroll_offset -= 1;
            }
            false
        }
        KeyCode::Down => {
            if app.slash_mode() && !app.slash_matches.is_empty() {
                app.move_slash_selection(1);
                return false;
            }
            // Scroll timeline down
            app.scroll_offset += 1;
            false
        }
        _ => false,
    }
}

/// Handle input in the model selector.
fn handle_model_selector_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.active_panel = ActivePanel::Composer;
            app.model_selector.search_active = false;
            false
        }
        KeyCode::Char('/') if !app.model_selector.search_active => {
            app.model_selector.search_active = true;
            app.model_selector.search_query.clear();
            false
        }
        KeyCode::Char(c) if app.model_selector.search_active => {
            app.model_selector.search_query.push(c);
            app.filter_models();
            false
        }
        KeyCode::Backspace if app.model_selector.search_active => {
            app.model_selector.search_query.pop();
            app.filter_models();
            false
        }
        KeyCode::Up => {
            if app.model_selector.selected_index > 0 {
                app.model_selector.selected_index -= 1;
            }
            false
        }
        KeyCode::Down => {
            let max = app.model_selector.filtered_indices.len().saturating_sub(1);
            if app.model_selector.selected_index < max {
                app.model_selector.selected_index += 1;
            }
            false
        }
        KeyCode::Enter => {
            app.switch_model();
            app.active_panel = ActivePanel::Composer;
            app.model_selector.search_active = false;
            false
        }
        _ => false,
    }
}

/// Handle input in the command palette.
fn handle_command_palette_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.active_panel = ActivePanel::Composer;
            false
        }
        KeyCode::Tab => {
            if !app.slash_matches.is_empty() {
                app.apply_slash_completion(true);
            }
            false
        }
        KeyCode::Enter => {
            if app.slash_is_ambiguous() {
                app.apply_slash_completion(true);
                return false;
            }
            let input = app.composer.input.trim().to_string();
            if !input.is_empty() {
                let before_panel = app.active_panel;
                if input.starts_with('/') {
                    handle_slash_command(app, &input);
                } else {
                    app.add_user_message(&input);
                    simulate_agent_response(app, &input);
                }
                app.composer.input.clear();
                app.composer.cursor_pos = 0;
                app.refresh_slash_matches();
                if app.active_panel == before_panel {
                    app.active_panel = ActivePanel::Composer;
                }
            }
            false
        }
        KeyCode::Backspace => {
            if app.composer.cursor_pos > 0 {
                app.composer.input.pop();
                app.composer.cursor_pos -= 1;
                app.refresh_slash_matches();
            }
            false
        }
        KeyCode::Char(c) => {
            app.composer.input.push(c);
            app.composer.cursor_pos += 1;
            app.refresh_slash_matches();
            false
        }
        KeyCode::Up => {
            app.move_slash_selection(-1);
            false
        }
        KeyCode::Down => {
            app.move_slash_selection(1);
            false
        }
        _ => false,
    }
}

/// Handle input in the approval prompt.
fn handle_approval_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Approved
            app.status = AppStatus::RunningCommand;
            app.active_panel = ActivePanel::Composer;
            app.add_system_message("Command approved and executed");
            false
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            // Rejected
            app.status = AppStatus::Ready;
            app.active_panel = ActivePanel::Composer;
            app.add_system_message("Command rejected");
            false
        }
        _ => false,
    }
}

/// Handle input in the timeline.
fn handle_timeline_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('i') => {
            app.active_panel = ActivePanel::Composer;
            false
        }
        KeyCode::Tab => {
            app.active_panel = ActivePanel::Sidebar;
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.scroll_offset > 0 {
                app.scroll_offset -= 1;
            }
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_offset += 1;
            false
        }
        KeyCode::Char('/') => {
            app.active_panel = ActivePanel::CommandPalette;
            false
        }
        _ => false,
    }
}

/// Handle input in the sidebar.
fn handle_sidebar_input(app: &mut TuiApp, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('i') => {
            app.active_panel = ActivePanel::Composer;
            false
        }
        KeyCode::Tab => {
            app.active_panel = ActivePanel::Composer;
            false
        }
        KeyCode::Char('m') => {
            app.load_models();
            app.active_panel = ActivePanel::ModelSelector;
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.model_selector.selected_index = app.model_selector.selected_index.saturating_sub(1);
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.model_selector.filtered_indices.len().saturating_sub(1);
            if app.model_selector.selected_index < max {
                app.model_selector.selected_index += 1;
            }
            false
        }
        KeyCode::Enter => {
            app.switch_model();
            false
        }
        _ => false,
    }
}

/// Handle a slash command from the composer.
fn handle_slash_command(app: &mut TuiApp, input: &str) {
    let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
    let cmd = parts[0];

    match cmd {
        "help" | "h" | "available" | "commands" | "command" | "avaliable" | "commnads"
        | "lgaye" => {
            app.show_help = !app.show_help;
        }
        "model" | "m" => {
            app.load_models();
            app.active_panel = ActivePanel::ModelSelector;
        }
        "clear" | "cls" => {
            app.messages.clear();
            app.agent_activity.tasks.clear();
        }
        "status" | "s" => {
            let status = format!(
                "Status: {} | Model: {}/{} | Tokens: {}",
                app.status.label(),
                app.session.provider,
                app.session.model,
                app.agent_activity.tokens_used
            );
            app.add_system_message(&status);
        }
        "cost" | "c" => {
            let cost = format!(
                "Tokens: {} | Model: {}/{}",
                app.agent_activity.tokens_used, app.session.provider, app.session.model,
            );
            app.add_system_message(&cost);
        }
        "exit" | "quit" | "q" => {
            // Will be handled by the event loop
            app.add_system_message("Exiting...");
        }
        "diff" | "d" => {
            app.add_system_message("Diff view not yet implemented in TUI mode");
        }
        "redact" => {
            if let Some(text) = parts.get(1) {
                let (safe, count) = crate::redact::redact(text);
                if count > 0 {
                    app.add_system_message(&format!("Redacted {count} secret(s): {safe}"));
                } else {
                    app.add_system_message("No secrets detected");
                }
            }
        }
        _ => {
            app.add_system_message(&format!(
                "Unknown command: /{cmd}. Try /help or /available."
            ));
        }
    }
    app.refresh_slash_matches();
}

/// Spawn a real scheduler execution and stream events to the TUI.
fn simulate_agent_response(app: &mut TuiApp, input: &str) {
    app.status = AppStatus::Thinking;
    app.add_task("Analyze repository");
    app.add_task("Plan execution");
    app.add_task("Execute agents");
    app.set_current_step("Analyzing code structure");

    // Use the app's own broadcast sender
    let event_tx = match app.event_tx.clone() {
        Some(tx) => tx,
        None => {
            app.add_assistant_message("No event channel available for live execution.");
            app.status = AppStatus::Ready;
            app.clear_current_step();
            return;
        }
    };

    let provider = app.session.provider.clone();
    let config = app.session.config.clone();
    let input_owned = input.to_string();
    let input_msg = input_owned.clone();
    let root = std::env::current_dir().unwrap_or_default();

    tokio::spawn(async move {
        let model_provider = continuum_models::load_from_config(&config, &provider);
        let cancel = continuum_core::CancellationToken::new();
        let scheduler = continuum_runtime::Scheduler::with_models(model_provider, None);
        let session = continuum_runtime::Session::new().with_workspace_root(root.clone());

        if let Ok(docs) = continuum_markdown::load(&root) {
            let loader = continuum_repo::Loader::new(root.clone());
            if let Ok(index) = loader
                .build(
                    &root,
                    continuum_core::repo::IndexOptions {
                        respect_gitignore: true,
                        max_files: 10000,
                    },
                )
                .await
            {
                let engine = continuum_planner::PlanningEngine::new(
                    continuum_models::load_from_config(&config, &provider),
                    continuum_core::ids::ModelId::new("default"),
                );
                let goal = continuum_core::planner::Goal::new(&input_owned);
                if let Ok(analysis) = engine.analyze(index, &docs).await {
                    if let Ok(plan) = engine.plan(goal, &analysis).await {
                        let _ = scheduler
                            .run_with_session_events(&plan, &session, cancel, Some(event_tx))
                            .await;
                    }
                }
            }
        }
    });

    let msg = format!("Executing: {}\n\n[Live execution started]", input_msg);
    app.add_assistant_message(&msg);
    app.status = AppStatus::RunningCommand;
    app.clear_current_step();
}
