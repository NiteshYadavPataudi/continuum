//! Rendering functions for TUI components.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::app::*;

/// Render the full TUI layout.
pub fn render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let command_ahead = app.slash_mode() && !app.slash_matches.is_empty();
    let palette_open = app.active_panel == ActivePanel::CommandPalette;
    let composer_height = if palette_open || command_ahead { 10 } else { 5 };

    // Main layout: header | body | composer
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),               // header
            Constraint::Min(10),                 // body
            Constraint::Length(composer_height), // composer
        ])
        .split(area);

    // Render header
    render_header(frame, app, main_layout[0]);

    // Body layout: timeline | sidebar
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(50),    // timeline
            Constraint::Length(35), // sidebar
        ])
        .split(main_layout[1]);

    // Render timeline
    render_timeline(frame, app, body_layout[0]);

    // Render sidebar
    render_sidebar(frame, app, body_layout[1]);

    // Render composer
    render_composer(frame, app, main_layout[2]);

    // Render overlays
    if app.show_help {
        render_help_overlay(frame, app);
    }

    if app.active_panel == ActivePanel::ModelSelector {
        render_model_selector(frame, app);
    }

    if app.active_panel == ActivePanel::ApprovalPrompt {
        render_approval_prompt(frame, app);
    }

    if app.active_panel == ActivePanel::CommandPalette {
        render_command_palette(frame, app);
    }

    // Render model error tooltip when there's an error
    if app.model_error.is_some() && app.status == AppStatus::Ready {
        render_model_error_tooltip(frame, app);
    }
}

/// Render the top header bar with live telemetry.
fn render_header(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let git_branch = get_git_branch();
    let mode = match app.active_panel {
        ActivePanel::Composer => "Ask",
        ActivePanel::Timeline => "Ask",
        ActivePanel::Sidebar => "Ask",
        ActivePanel::ModelSelector => "Model",
        ActivePanel::CommandPalette => "Command",
        ActivePanel::ApprovalPrompt => "Approval",
    };

    // Live token & cost info
    let token_info = format!("{}t", app.agent_activity.tokens_used);
    let cost_str = if app.total_cost_usd > 0.0 {
        format!("${:.4}", app.total_cost_usd)
    } else {
        String::new()
    };

    // Model status with error indicator
    let model_style = if app.model_error.is_some() {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.accent)
    };
    let model_display = if app.model_error.is_some() {
        format!("⚠ {}", app.session.model)
    } else {
        app.session.model.clone()
    };

    // Provider status
    let provider_label = if app.provider_configured {
        "connected"
    } else {
        "no-key"
    };

    // Validation summary
    let val_str = if app.validation_total > 0 {
        format!(
            " v{}/{}({})",
            app.validation_passed, app.validation_total, app.validation_failed
        )
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled(
            " Continuum ",
            Style::default()
                .bg(theme.accent)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(&model_display, model_style),
        Span::raw("  "),
        Span::styled(
            provider_label,
            Style::default().fg(if app.provider_configured {
                theme.success
            } else {
                theme.warning
            }),
        ),
        Span::raw("  |  "),
        Span::styled(get_project_name(), Style::default().fg(theme.muted)),
        Span::raw("  |  "),
        Span::styled(&git_branch, Style::default().fg(theme.warning)),
        Span::raw("  |  "),
        Span::styled(mode, Style::default().fg(theme.info)),
    ];

    if !cost_str.is_empty() {
        spans.push(Span::raw("  |  "));
        spans.push(Span::styled(&cost_str, Style::default().fg(theme.success)));
    }

    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(&token_info, Style::default().fg(theme.muted)));

    if !val_str.is_empty() {
        spans.push(Span::styled(&val_str, Style::default().fg(theme.info)));
    }

    // Show model error tooltip
    if let Some(ref err) = app.model_error {
        let err_display = format!("ERR: {}", crate::output::truncate_str(err, 30));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(err_display, Style::default().fg(theme.error)));
    }

    let header_line = Line::from(spans);

    let header = Paragraph::new(header_line)
        .style(theme.header_style())
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme.border_style()),
        );

    frame.render_widget(header, area);
}

/// Render the conversation timeline.
fn render_timeline(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::new();

    if app.messages.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No messages yet.",
            Style::default().fg(theme.muted),
        )]));
        lines.push(Line::from(vec![Span::raw("")]));
        lines.push(Line::from(vec![Span::styled(
            "  Type a message or use /help for commands.",
            Style::default().fg(theme.muted),
        )]));
    } else {
        for msg in &app.messages {
            match msg.role {
                MessageRole::User => {
                    lines.push(Line::from(vec![
                        Span::styled(&msg.timestamp, Style::default().fg(theme.muted)),
                        Span::raw(" "),
                        Span::styled(
                            "You",
                            Style::default()
                                .fg(theme.user_msg)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for line in msg.content.lines() {
                        lines.push(Line::from(vec![Span::raw(format!("  {line}"))]));
                    }
                    lines.push(Line::from(vec![Span::raw("")]));
                }
                MessageRole::Assistant => {
                    lines.push(Line::from(vec![
                        Span::styled(&msg.timestamp, Style::default().fg(theme.muted)),
                        Span::raw(" "),
                        Span::styled(
                            "Continuum",
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for line in msg.content.lines() {
                        lines.push(Line::from(vec![Span::raw(format!("  {line}"))]));
                    }
                    lines.push(Line::from(vec![Span::raw("")]));
                }
                MessageRole::System => {
                    lines.push(Line::from(vec![
                        Span::styled(&msg.timestamp, Style::default().fg(theme.muted)),
                        Span::raw(" "),
                        Span::styled("System", Style::default().fg(theme.info)),
                        Span::raw(" "),
                        Span::styled(&msg.content, Style::default().fg(theme.muted)),
                    ]));
                    lines.push(Line::from(vec![Span::raw("")]));
                }
                MessageRole::Tool => {
                    // Tool call card
                    for tc in &msg.tool_calls {
                        let status_icon = match tc.status {
                            ToolStatus::Pending => "-",
                            ToolStatus::Running => ">",
                            ToolStatus::Completed => "+",
                            ToolStatus::Failed => "x",
                            ToolStatus::WaitingApproval => "?",
                        };
                        let status_style = match tc.status {
                            ToolStatus::Pending => theme.muted_style(),
                            ToolStatus::Running => theme.status_running_style(),
                            ToolStatus::Completed => theme.success_style(),
                            ToolStatus::Failed => theme.error_style(),
                            ToolStatus::WaitingApproval => theme.status_waiting_style(),
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{status_icon}] "), status_style),
                            Span::styled(&tc.name, Style::default().fg(theme.accent)),
                            tc.target
                                .as_ref()
                                .map(|t| {
                                    Span::styled(
                                        format!(" → {t}"),
                                        Style::default().fg(theme.muted),
                                    )
                                })
                                .unwrap_or(Span::raw("")),
                        ]));

                        if let Some(output) = &tc.output {
                            for line in output.lines().take(3) {
                                lines.push(Line::from(vec![Span::styled(
                                    format!("      {line}"),
                                    Style::default().fg(theme.muted),
                                )]));
                            }
                        }
                    }
                    lines.push(Line::from(vec![Span::raw("")]));
                }
            }
        }
    }

    // Add activity panel if there are tasks
    if !app.agent_activity.tasks.is_empty() {
        lines.push(Line::from(vec![Span::raw("")]));
        lines.push(Line::from(vec![Span::styled(
            "  -- Agent Activity --",
            Style::default().fg(theme.accent),
        )]));

        for task in &app.agent_activity.tasks {
            let icon = task.status.icon();
            let style = task.status.style(theme);
            let pct = task.percent.map(|p| format!(" {p}%")).unwrap_or_default();
            let label_len = 40usize.saturating_sub(pct.len());
            lines.push(Line::from(vec![
                Span::styled(format!("  {icon} "), style),
                Span::styled(
                    crate::output::truncate_str(&task.label, label_len),
                    Style::default().fg(theme.fg),
                ),
                Span::styled(pct, Style::default().fg(theme.info)),
            ]));

            // Show dependency info for blocked tasks
            if task.status == TaskStatus::Pending && !task.depends_on.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    format!("    waits for: {}", task.depends_on.len()),
                    Style::default().fg(theme.muted),
                )]));
            }

            for child in &task.children {
                let child_icon = child.status.icon();
                let child_style = child.status.style(theme);
                let child_pct = child.percent.map(|p| format!(" {p}%")).unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(format!("    {child_icon} "), child_style),
                    Span::styled(
                        crate::output::truncate_str(&child.label, 36),
                        Style::default().fg(theme.fg),
                    ),
                    Span::styled(child_pct, Style::default().fg(theme.info)),
                ]));
            }
        }
    }

    let timeline = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0))
        .block(
            Block::default()
                .title(" Timeline ")
                .borders(Borders::ALL)
                .border_style(theme.border_style()),
        );

    frame.render_widget(timeline, area);
}

/// Render the right sidebar.
fn render_sidebar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let sidebar_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // agent activity summary
            Constraint::Length(10), // quick models
            Constraint::Min(5),     // recent files / tasks
        ])
        .split(area);

    // Agent activity summary
    render_agent_summary(frame, app, sidebar_layout[0]);

    // Quick model list
    render_quick_models(frame, app, sidebar_layout[1]);

    // Recent files / tasks
    render_recent_files(frame, app, sidebar_layout[2]);
}

/// Render agent activity summary in sidebar with live telemetry.
fn render_agent_summary(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::new();

    // Status with icon
    let status_icon = match app.status {
        AppStatus::Ready => "*",
        AppStatus::Thinking => ">",
        AppStatus::RunningCommand => ">",
        AppStatus::WaitingApproval => "?",
    };
    let status_style = app.status.style(theme);
    lines.push(Line::from(vec![
        Span::styled(format!(" {status_icon} "), status_style),
        Span::styled(app.status.label(), status_style),
    ]));
    lines.push(Line::from(vec![Span::raw("")]));

    // Current step
    if let Some(step) = &app.agent_activity.current_step {
        lines.push(Line::from(vec![
            Span::styled("  Task: ", Style::default().fg(theme.muted)),
            Span::styled(
                crate::output::truncate_str(step, 24),
                Style::default().fg(theme.fg),
            ),
        ]));
    }

    // Elapsed time
    let mins = app.agent_activity.elapsed_secs / 60;
    let secs = app.agent_activity.elapsed_secs % 60;
    lines.push(Line::from(vec![
        Span::styled("  Time: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{mins:02}:{secs:02}"),
            Style::default().fg(theme.fg),
        ),
    ]));

    // Tokens used
    let token_label = app.agent_activity.tokens_used;
    lines.push(Line::from(vec![
        Span::styled("  Tokens: ", Style::default().fg(theme.muted)),
        Span::styled(format!("{token_label}"), Style::default().fg(theme.fg)),
    ]));

    // Cost
    if app.total_cost_usd > 0.0 {
        lines.push(Line::from(vec![
            Span::styled("  Cost: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("${:.4}", app.total_cost_usd),
                Style::default().fg(theme.success),
            ),
        ]));
    }

    // Validation summary
    if app.validation_total > 0 {
        let val_color = if app.validation_failed > 0 {
            theme.error
        } else {
            theme.success
        };
        lines.push(Line::from(vec![
            Span::styled("  Valid: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}/{}", app.validation_passed, app.validation_total),
                Style::default().fg(val_color),
            ),
        ]));
    }

    // Model error
    if let Some(ref err) = app.model_error {
        lines.push(Line::from(vec![
            Span::styled("  Model: ", Style::default().fg(theme.muted)),
            Span::styled(
                crate::output::truncate_str(err, 22),
                Style::default().fg(theme.error),
            ),
        ]));
    }

    let block = Block::default()
        .title(" Agent ")
        .borders(Borders::ALL)
        .border_style(theme.border_style());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Render quick model list in sidebar.
fn render_quick_models(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::new();

    // Show top 8 models
    for (i, &idx) in app
        .model_selector
        .filtered_indices
        .iter()
        .take(8)
        .enumerate()
    {
        if let Some(model) = app.model_selector.models.get(idx) {
            let marker = if model.is_current { "*" } else { "o" };
            let style = if i == app.model_selector.selected_index
                && app.active_panel == ActivePanel::ModelSelector
            {
                theme.selected_style()
            } else if model.is_current {
                Style::default().fg(theme.accent)
            } else {
                theme.muted_style()
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {marker} "), style),
                Span::styled(crate::output::truncate_str(&model.model_id, 28), style),
            ]));
        }
    }

    if app.model_selector.filtered_indices.len() > 8 {
        lines.push(Line::from(vec![Span::styled(
            format!(
                "   ... {} more",
                app.model_selector.filtered_indices.len() - 8
            ),
            Style::default().fg(theme.muted),
        )]));
    }

    let block = Block::default()
        .title(" Models ")
        .borders(Borders::ALL)
        .border_style(theme.border_style());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Render task list in sidebar with progress and status.
fn render_recent_files(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::new();

    if !app.agent_activity.tasks.is_empty() {
        let running = app
            .agent_activity
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed = app
            .agent_activity
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = app
            .agent_activity
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        let total = app.agent_activity.tasks.len();

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ready", total - running - completed),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!(" | {} running", running),
                Style::default().fg(theme.status_running),
            ),
            Span::styled(
                format!(" | {} done", completed),
                Style::default().fg(theme.success),
            ),
            if failed > 0 {
                Span::styled(
                    format!(" | {} failed", failed),
                    Style::default().fg(theme.error),
                )
            } else {
                Span::raw("")
            },
        ]));
        lines.push(Line::from(vec![Span::raw("")]));

        for task in &app.agent_activity.tasks {
            let icon = task.status.icon();
            let style = task.status.style(theme);
            let pct = task.percent.map(|p| format!(" {p}%")).unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), style),
                Span::styled(
                    crate::output::truncate_str(&task.label, 26),
                    Style::default().fg(theme.fg),
                ),
                Span::styled(pct, Style::default().fg(theme.info)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No active tasks",
            Style::default().fg(theme.muted),
        )]));
    }

    let block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(theme.border_style());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Render the bottom composer area.
fn render_composer(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;
    let show_slash_suggestions = app.slash_mode() && !app.slash_matches.is_empty();

    let composer_layout = if show_slash_suggestions {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // input area
                Constraint::Length(1), // hint row
                Constraint::Min(1),    // suggestion box
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // input area
                Constraint::Length(1), // hint row
            ])
            .split(area)
    };

    // Input area
    let input_style = if app.active_panel == ActivePanel::Composer {
        theme.border_focus_style()
    } else {
        theme.border_style()
    };

    let input_text = if app.composer.input.is_empty() {
        Line::from(vec![Span::styled(
            "Type a message or /help...",
            Style::default().fg(theme.muted),
        )])
    } else {
        Line::from(vec![Span::styled(
            &app.composer.input,
            Style::default().fg(theme.composer_fg),
        )])
    };

    let input = Paragraph::new(input_text)
        .style(theme.composer_style())
        .block(
            Block::default()
                .title(" Input ")
                .borders(Borders::ALL)
                .border_style(input_style),
        );

    frame.render_widget(input, composer_layout[0]);

    // Hint row with dynamic model error / status
    let hint_text = if let Some(ref err) = app.model_error {
        format!(
            " ERR: {} | Tab:sidebar  Enter:send  Esc:cancel",
            crate::output::truncate_str(err, 40)
        )
    } else if app.total_cost_usd > 0.0 {
        format!(
            " Cost: ${:.4} | {}t used | Tab:sidebar  Enter:send  Esc:cancel  /:commands",
            app.total_cost_usd, app.agent_activity.tokens_used
        )
    } else {
        " Tab:sidebar/autocomplete  Enter:send  Esc:cancel  Ctrl+T:tasks  Ctrl+L:clear  /:commands"
            .to_string()
    };
    let hint_style = if app.model_error.is_some() {
        Style::default().fg(theme.error)
    } else {
        Style::default().fg(theme.muted)
    };

    let hints = Line::from(vec![Span::styled(&hint_text, hint_style)]);

    let hint_paragraph = Paragraph::new(hints).style(theme.composer_style());

    frame.render_widget(hint_paragraph, composer_layout[1]);

    if show_slash_suggestions {
        render_slash_suggestions(frame, app, composer_layout[2]);
    }
}

/// Render the model error tooltip.
fn render_model_error_tooltip(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let theme = &app.theme;
    let err_text = app.model_error.as_deref().unwrap_or("unknown error");

    let lines = vec![
        Line::from(vec![Span::styled(
            "  Model Error ",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(err_text, Style::default().fg(theme.fg))]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Tips:",
            Style::default().fg(theme.warning),
        )]),
        Line::from(vec![Span::styled(
            "    • Set API key via: continuum login",
            Style::default().fg(theme.muted),
        )]),
        Line::from(vec![Span::styled(
            "    • Check provider: /model or /providers",
            Style::default().fg(theme.muted),
        )]),
        Line::from(vec![Span::styled(
            "    • Run diagnostics: continuum doctor",
            Style::default().fg(theme.muted),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Press any key to dismiss",
            Style::default().fg(theme.accent_dim),
        )]),
    ];

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" API / Model Error ")
                .borders(Borders::ALL)
                .border_style(theme.error_style()),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let popup_area = centered_rect(55, 40, area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Render the help overlay.
fn render_help_overlay(frame: &mut Frame, _app: &TuiApp) {
    let area = frame.area();

    let help_text = vec![
        Line::from(vec![Span::styled(
            "  Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Navigation",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![Span::raw("    Tab        Switch panel")]),
        Line::from(vec![Span::raw("    Ctrl+T     Task panel")]),
        Line::from(vec![Span::raw("    Ctrl+L     Live logs")]),
        Line::from(vec![Span::raw("    Ctrl+A     Agent activity")]),
        Line::from(vec![Span::raw("    Ctrl+R     Retry step")]),
        Line::from(vec![Span::raw("    Ctrl+X     Stop agent")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Commands",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![Span::raw("    /help      Show this help")]),
        Line::from(vec![Span::raw("    /model     Switch model")]),
        Line::from(vec![Span::raw("    /clear     Clear screen")]),
        Line::from(vec![Span::raw("    /diff      Show changes")]),
        Line::from(vec![Span::raw("    /status    Show status")]),
        Line::from(vec![Span::raw("    /available Show commands")]),
        Line::from(vec![Span::raw("    /commands  Show commands")]),
        Line::from(vec![Span::raw("    /exit      Exit")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Press Esc or /help to close",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let popup = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let area = centered_rect(60, 50, area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Render the model selector popup.
fn render_model_selector(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::new();

    // Search bar
    let search_style = if app.model_selector.search_active {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.muted)
    };
    lines.push(Line::from(vec![
        Span::styled(" Search: ", search_style),
        Span::styled(
            &app.model_selector.search_query,
            Style::default().fg(theme.fg),
        ),
        Span::styled("|", search_style),
    ]));
    lines.push(Line::from(vec![Span::raw("")]));

    // Model list
    for (i, &idx) in app.model_selector.filtered_indices.iter().enumerate() {
        if let Some(model) = app.model_selector.models.get(idx) {
            let marker = if model.is_current { "*" } else { "o" };
            let style = if i == app.model_selector.selected_index {
                theme.selected_style()
            } else if model.is_current {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };

            let price = if model.input_price > 0.0 {
                format!("${:.2}/{:.2}", model.input_price, model.output_price)
            } else {
                "free".to_string()
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {marker} "), style),
                Span::styled(crate::output::truncate_str(&model.model_id, 25), style),
                Span::styled(format!("  {price:>10}"), Style::default().fg(theme.muted)),
            ]));
        }
    }

    let block = Block::default()
        .title(" Select Model (up/down navigate, Enter select, / search) ")
        .borders(Borders::ALL)
        .border_style(theme.border_focus_style())
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let popup = Paragraph::new(lines).block(block);

    let area = centered_rect(50, 60, area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Render the approval prompt popup.
fn render_approval_prompt(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let theme = &app.theme;

    let lines = vec![
        Line::from(vec![Span::styled(
            "  Approve Command Execution",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Command:",
            Style::default().fg(theme.muted),
        )]),
        Line::from(vec![Span::styled(
            "    npm test",
            Style::default().fg(theme.fg),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "  Run this command? [y/N]",
            Style::default().fg(theme.fg),
        )]),
    ];

    let block = Block::default()
        .title(" Approval Required ")
        .borders(Borders::ALL)
        .border_style(theme.warning_style())
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let popup = Paragraph::new(lines).block(block);

    let area = centered_rect(50, 30, area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Render the command palette popup.
fn render_command_palette(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let theme = &app.theme;
    let input = app.composer.input.trim_start();
    let query = input
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");
    let matches = if app.slash_matches.is_empty() {
        super::commands::filter_slash_commands(query)
    } else {
        app.slash_matches.clone()
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        " Command Search",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  /{query}"),
        Style::default().fg(theme.muted),
    )]));
    lines.push(Line::from(vec![Span::raw("")]));

    if matches.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No matching commands.",
            Style::default().fg(theme.warning),
        )]));
    } else {
        for (i, cmd) in matches.iter().take(10).enumerate() {
            let selected = i == app.slash_selected_index;
            let row_style = if selected {
                theme.selected_style()
            } else {
                Style::default().fg(theme.fg)
            };
            let aliases = if cmd.aliases.is_empty() {
                String::new()
            } else {
                format!("  ({})", cmd.aliases.join(", "))
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  /{:<14}", cmd.name), row_style),
                Span::styled(cmd.description, Style::default().fg(theme.muted)),
                Span::styled(aliases, Style::default().fg(theme.accent_dim)),
            ]));
        }
    }

    let block = Block::default()
        .title(" Command Palette (Esc to close, Tab to autocomplete) ")
        .borders(Borders::ALL)
        .border_style(theme.border_focus_style())
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let popup = Paragraph::new(lines).block(block);

    let area = centered_rect(50, 60, area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn render_slash_suggestions(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let theme = &app.theme;

    let input = app.composer.input.trim_start();
    let query = input
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");
    let matches = if app.slash_matches.is_empty() {
        super::commands::filter_slash_commands(query)
    } else {
        app.slash_matches.clone()
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" Suggestions ", Style::default().fg(theme.accent)),
        Span::styled(
            "Tab complete, Enter execute when exact",
            Style::default().fg(theme.muted),
        ),
    ]));
    lines.push(Line::from(vec![Span::raw("")]));

    for (i, cmd) in matches.iter().take(4).enumerate() {
        let selected = i == app.slash_selected_index;
        let row_style = if selected {
            theme.selected_style()
        } else {
            Style::default().fg(theme.fg)
        };
        let aliases = if cmd.aliases.is_empty() {
            String::new()
        } else {
            format!("  ({})", cmd.aliases.join(", "))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  /{:<12}", cmd.name), row_style),
            Span::styled(cmd.description, Style::default().fg(theme.muted)),
            Span::styled(aliases, Style::default().fg(theme.accent_dim)),
        ]));
    }

    let block = Block::default()
        .title(" Slash Commands ")
        .borders(Borders::ALL)
        .border_style(theme.border_focus_style());

    let popup = Paragraph::new(lines).block(block);
    frame.render_widget(popup, area);
}

/// Create a centered rectangle.
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

/// Get the current git branch.
fn get_git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "no-git".to_string())
}

/// Get the project name from current directory.
fn get_project_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}
