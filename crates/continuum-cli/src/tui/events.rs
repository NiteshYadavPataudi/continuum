//! Event handling for the TUI.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::tui::chat::{spawn_streaming_assistant_turn, AssistantTurnRequest};
use crate::tui::errors::startup_notice;

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
                    if let Some(cancel) = app.active_turn_cancel.as_ref() {
                        cancel.cancel();
                    }
                    app.status = AppStatus::Ready;
                    app.clear_current_step();
                    app.active_turn_id = None;
                    app.active_turn_cancel = None;
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
                    if !send_chat_prompt(app, &input) {
                        app.composer.input.clear();
                        app.composer.cursor_pos = 0;
                        app.refresh_slash_matches();
                        return false;
                    }
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
                    let _ = send_chat_prompt(app, &input);
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

fn send_chat_prompt(app: &mut TuiApp, input: &str) -> bool {
    if app.status == AppStatus::Streaming {
        app.add_system_message(
            "Please wait for the current response to finish, or press Ctrl+X to stop it.",
        );
        return false;
    }

    app.add_user_message(input);
    let turn_id = app.begin_assistant_turn(input);
    let history = app.messages.clone();
    let session = app.session.clone();
    let goal = app.goal.clone();
    let event_tx = app.event_tx.clone();
    let cancel = app
        .active_turn_cancel
        .clone()
        .unwrap_or_default();

    if let Some(event_tx) = event_tx {
        spawn_streaming_assistant_turn(AssistantTurnRequest {
            turn_id,
            prompt: input.to_string(),
            goal,
            history,
            session,
            cancel,
            event_tx,
        });
    } else {
        app.fail_assistant_turn(
            turn_id,
            "No event channel available for streaming assistant output.",
        );
    }

    true
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
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd {
        "help" | "h" | "available" | "commands" | "command" | "avaliable" | "commnads"
        | "lgaye" => {
            app.show_help = !app.show_help;
        }
        "model" | "m" => {
            app.load_models();
            app.active_panel = ActivePanel::ModelSelector;
        }
        "goal" | "g" => {
            if args.is_empty() {
                match &app.goal {
                    Some(goal) => app.add_system_message(&format!("Current goal: {goal}")),
                    None => app.add_system_message(
                        "No goal is set yet. Use `/goal <text>` to define the session goal.",
                    ),
                }
            } else {
                app.set_goal(Some(args.to_string()));
                app.add_system_message(&format!("Goal set: {args}"));
            }
        }
        "clear" | "cls" => {
            app.messages.clear();
            app.agent_activity.tasks.clear();
            app.scroll_offset = 0;
            app.add_system_message("Conversation cleared");
        }
        "status" | "s" => {
            let status = format!(
                "Status: {} | Model: {}/{} | Tokens: {} | Goal: {}",
                app.status.label(),
                app.session.provider,
                app.session.model,
                app.agent_activity.tokens_used,
                app.goal.as_deref().unwrap_or("(none)")
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
        "config" | "cfg" => handle_config_command(app, args),
        "exit" | "quit" | "q" => {
            // Will be handled by the event loop
            app.add_system_message("Exiting...");
            app.should_exit = true;
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

fn handle_config_command(app: &mut TuiApp, args: &str) {
    let mut parts = args.split_whitespace();
    match parts.next() {
        None | Some("list") => {
            let config_path = continuum_config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(unavailable)".to_string());
            app.add_system_message(&format!("Config file: {config_path}"));
            app.add_system_message(&format!(
                "Current provider: {}/{}",
                app.session.provider, app.session.model
            ));
            if let Some(hint) = startup_notice(&app.session.provider, &app.session.config) {
                app.add_system_message(&hint);
            } else {
                app.add_system_message("Current provider has a configured API key.");
            }
        }
        Some("providers") => {
            let mut providers: Vec<_> = continuum_models_registry::PROVIDERS.entries().collect();
            providers.sort_by_key(|(k, _)| *k);
            for (pid, meta) in providers {
                let status = if meta.env_var.is_empty()
                    || app.session.config.api_key(pid, meta.env_var).is_some()
                {
                    "configured".to_string()
                } else {
                    startup_notice(pid, &app.session.config)
                        .unwrap_or_else(|| "missing API key".to_string())
                };
                app.add_system_message(&format!("{pid}: {} ({status})", meta.name));
            }
        }
        Some("set") => {
            let key = parts.next().unwrap_or("");
            let value = parts.collect::<Vec<_>>().join(" ");
            if key.is_empty() || value.is_empty() {
                app.add_system_message("Usage: /config set <provider>.<field> <value>");
                return;
            }
            match set_config_value(app, key, &value) {
                Ok(message) => app.add_system_message(&message),
                Err(message) => app.add_system_message(&message),
            }
        }
        Some("get") => {
            let key = parts.next().unwrap_or("");
            if key.is_empty() {
                app.add_system_message("Usage: /config get <provider>.<field>");
                return;
            }
            match get_config_value(&app.session.config, key) {
                Ok(message) => app.add_system_message(&message),
                Err(message) => app.add_system_message(&message),
            }
        }
        Some("unset") => {
            let key = parts.next().unwrap_or("");
            if key.is_empty() {
                app.add_system_message("Usage: /config unset <provider>.<field>");
                return;
            }
            match unset_config_value(app, key) {
                Ok(message) => app.add_system_message(&message),
                Err(message) => app.add_system_message(&message),
            }
        }
        Some(other) => {
            app.add_system_message(&format!("Unknown /config subcommand: {other}"));
        }
    }
}

fn parse_config_key(key: &str) -> Result<(&str, &str), String> {
    let dot = key
        .find('.')
        .ok_or_else(|| format!("invalid key '{key}': expected <provider>.<field>"))?;
    Ok((&key[..dot], &key[dot + 1..]))
}

fn set_config_value(app: &mut TuiApp, key: &str, value: &str) -> Result<String, String> {
    let (provider, field) = parse_config_key(key)?;
    let mut cfg = app.session.config.clone();
    match field {
        "api_key" => cfg.set_api_key(provider, value),
        "base_url" => cfg.set_base_url(provider, value),
        other => {
            return Err(format!(
                "unknown config field '{other}'. Valid fields: api_key, base_url"
            ))
        }
    }
    cfg.save()
        .map_err(|e| format!("failed to save config: {e}"))?;
    app.session.config = cfg;
    Ok(format!("✓ Set {provider}.{field}"))
}

fn get_config_value(cfg: &continuum_config::Config, key: &str) -> Result<String, String> {
    let (provider, field) = parse_config_key(key)?;
    let value = match field {
        "api_key" => {
            let env_hint = continuum_models_registry::PROVIDERS
                .get(provider)
                .map(|p| p.env_var)
                .unwrap_or("");
            cfg.api_key(provider, env_hint)
        }
        "base_url" => cfg.base_url(provider).or_else(|| {
            continuum_models_registry::PROVIDERS
                .get(provider)
                .map(|p| p.api_base_url.to_string())
        }),
        other => return Err(format!("unknown field '{other}'. Valid: api_key, base_url")),
    };

    Ok(match value {
        Some(v) => {
            let display = if field == "api_key" {
                crate::output::mask_key(&v)
            } else {
                v
            };
            format!("{provider}.{field} = {display}")
        }
        None => format!("{provider}.{field} = (not set)"),
    })
}

fn unset_config_value(app: &mut TuiApp, key: &str) -> Result<String, String> {
    let (provider, field) = parse_config_key(key)?;
    let mut cfg = app.session.config.clone();
    cfg.unset(provider, field);
    cfg.save()
        .map_err(|e| format!("failed to save config: {e}"))?;
    app.session.config = cfg;
    Ok(format!("✓ Unset {provider}.{field}"))
}
