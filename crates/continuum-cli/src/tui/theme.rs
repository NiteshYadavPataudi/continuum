//! Dark terminal theme for Continuum TUI.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

/// Theme colors.
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub border_focus: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub sidebar_bg: Color,
    pub sidebar_fg: Color,
    pub composer_bg: Color,
    pub composer_fg: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub muted: Color,
    pub user_msg: Color,
    pub assistant_msg: Color,
    pub tool_bg: Color,
    pub tool_border: Color,
    pub diff_add: Color,
    pub diff_del: Color,
    pub diff_header: Color,
    pub selection: Color,
    pub scrollbar: Color,
    pub status_ready: Color,
    pub status_running: Color,
    pub status_waiting: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(18, 18, 24),
            fg: Color::Rgb(200, 200, 210),
            border: Color::Rgb(60, 60, 75),
            border_focus: Color::Rgb(100, 120, 200),
            header_bg: Color::Rgb(25, 25, 35),
            header_fg: Color::Rgb(180, 180, 200),
            sidebar_bg: Color::Rgb(22, 22, 30),
            sidebar_fg: Color::Rgb(170, 170, 190),
            composer_bg: Color::Rgb(20, 20, 28),
            composer_fg: Color::Rgb(220, 220, 230),
            accent: Color::Rgb(100, 140, 255),
            accent_dim: Color::Rgb(60, 80, 150),
            success: Color::Rgb(80, 200, 120),
            warning: Color::Rgb(240, 200, 60),
            error: Color::Rgb(240, 80, 80),
            info: Color::Rgb(100, 180, 255),
            muted: Color::Rgb(100, 100, 120),
            user_msg: Color::Rgb(130, 170, 255),
            assistant_msg: Color::Rgb(200, 200, 210),
            tool_bg: Color::Rgb(30, 30, 40),
            tool_border: Color::Rgb(70, 70, 90),
            diff_add: Color::Rgb(80, 200, 120),
            diff_del: Color::Rgb(240, 80, 80),
            diff_header: Color::Rgb(100, 140, 255),
            selection: Color::Rgb(50, 60, 100),
            scrollbar: Color::Rgb(60, 60, 80),
            status_ready: Color::Rgb(80, 200, 120),
            status_running: Color::Rgb(240, 200, 60),
            status_waiting: Color::Rgb(240, 140, 60),
        }
    }
}

impl Theme {
    /// Style for the header bar.
    pub fn header_style(&self) -> Style {
        Style::default().bg(self.header_bg).fg(self.header_fg)
    }

    /// Style for the sidebar.
    pub fn sidebar_style(&self) -> Style {
        Style::default().bg(self.sidebar_bg).fg(self.sidebar_fg)
    }

    /// Style for the composer.
    pub fn composer_style(&self) -> Style {
        Style::default().bg(self.composer_bg).fg(self.composer_fg)
    }

    /// Style for borders.
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Style for focused borders.
    pub fn border_focus_style(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    /// Style for accent text.
    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Style for success indicators.
    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for warning indicators.
    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Style for error indicators.
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Style for muted text.
    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for user messages.
    pub fn user_style(&self) -> Style {
        Style::default().fg(self.user_msg)
    }

    /// Style for assistant messages.
    pub fn assistant_style(&self) -> Style {
        Style::default().fg(self.assistant_msg)
    }

    /// Style for tool cards.
    pub fn tool_style(&self) -> Style {
        Style::default().bg(self.tool_bg).fg(self.fg)
    }

    /// Style for tool borders.
    pub fn tool_border_style(&self) -> Style {
        Style::default().fg(self.tool_border)
    }

    /// Style for selected items.
    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.selection)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for status ready.
    pub fn status_ready_style(&self) -> Style {
        Style::default().fg(self.status_ready)
    }

    /// Style for status running.
    pub fn status_running_style(&self) -> Style {
        Style::default()
            .fg(self.status_running)
            .add_modifier(Modifier::SLOW_BLINK)
    }

    /// Style for status waiting.
    pub fn status_waiting_style(&self) -> Style {
        Style::default().fg(self.status_waiting)
    }
}
