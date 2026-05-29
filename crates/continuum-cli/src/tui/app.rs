//! Main TUI application for Continuum.
//!
//! Renders the full-screen terminal UI with:
//! - Header bar (app name, model, project, git, mode, tokens)
//! - Main conversation timeline
//! - Right sidebar (agent activity, models, files)
//! - Bottom composer (input, hints, status)

use ratatui::style::Style;
use tokio::sync::broadcast;

use continuum_telemetry::DashboardEvent;

use super::commands::{
    filter_slash_commands, resolve_slash_command, split_command_input, SlashCommand,
};
use super::theme::Theme;
use crate::repl::ReplSession;

/// Active panel for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Composer,
    Timeline,
    Sidebar,
    ModelSelector,
    CommandPalette,
    ApprovalPrompt,
}

/// TUI application state.
pub struct TuiApp {
    pub theme: Theme,
    pub active_panel: ActivePanel,
    pub session: ReplSession,
    pub messages: Vec<ChatMessage>,
    pub agent_activity: AgentActivity,
    pub model_selector: ModelSelector,
    pub composer: ComposerState,
    pub status: AppStatus,
    pub show_help: bool,
    pub scroll_offset: u16,
    pub should_exit: bool,
    pub slash_matches: Vec<SlashCommand>,
    pub slash_selected_index: usize,
    pub event_rx: Option<broadcast::Receiver<DashboardEvent>>,
}

/// A message in the conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// A tool call made by the agent.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub status: ToolStatus,
    pub target: Option<String>,
    pub duration_ms: Option<u64>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Completed,
    Failed,
    WaitingApproval,
}

/// Agent activity tracker.
#[derive(Debug, Clone)]
pub struct AgentActivity {
    pub tasks: Vec<TaskNode>,
    pub current_step: Option<String>,
    pub elapsed_secs: u64,
    pub tokens_used: u64,
}

#[derive(Debug, Clone)]
pub struct TaskNode {
    pub task_id: Option<String>,
    pub label: String,
    pub status: TaskStatus,
    pub depends_on: Vec<String>,
    pub percent: Option<u8>,
    pub children: Vec<TaskNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Warning,
}

impl TaskStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "-",
            Self::Running => ">",
            Self::Completed => "+",
            Self::Failed => "x",
            Self::Warning => "!",
        }
    }

    pub fn style(&self, theme: &Theme) -> Style {
        match self {
            Self::Pending => theme.muted_style(),
            Self::Running => theme.status_running_style(),
            Self::Completed => theme.success_style(),
            Self::Failed => theme.error_style(),
            Self::Warning => theme.warning_style(),
        }
    }
}

/// Model selector state.
#[derive(Debug, Clone)]
pub struct ModelSelector {
    pub models: Vec<ModelEntry>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub search_query: String,
    pub search_active: bool,
}

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub provider: String,
    pub model_id: String,
    pub name: String,
    pub context_window: u64,
    pub input_price: f64,
    pub output_price: f64,
    pub is_current: bool,
}

/// Composer state.
#[derive(Debug, Clone)]
pub struct ComposerState {
    pub input: String,
    pub cursor_pos: usize,
    pub multiline: bool,
    pub lines: Vec<String>,
}

/// Application status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStatus {
    Ready,
    Thinking,
    RunningCommand,
    WaitingApproval,
}

impl AppStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Thinking => "Thinking...",
            Self::RunningCommand => "Running...",
            Self::WaitingApproval => "Waiting approval",
        }
    }

    pub fn style(&self, theme: &Theme) -> Style {
        match self {
            Self::Ready => theme.status_ready_style(),
            Self::Thinking => theme.status_running_style(),
            Self::RunningCommand => theme.status_running_style(),
            Self::WaitingApproval => theme.status_waiting_style(),
        }
    }
}

impl TuiApp {
    /// Create a new TUI app.
    pub fn new(session: ReplSession) -> Self {
        Self {
            theme: Theme::default(),
            active_panel: ActivePanel::Composer,
            session,
            messages: Vec::new(),
            agent_activity: AgentActivity {
                tasks: Vec::new(),
                current_step: None,
                elapsed_secs: 0,
                tokens_used: 0,
            },
            model_selector: ModelSelector {
                models: Vec::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                search_query: String::new(),
                search_active: false,
            },
            composer: ComposerState {
                input: String::new(),
                cursor_pos: 0,
                multiline: false,
                lines: vec![String::new()],
            },
            status: AppStatus::Ready,
            show_help: false,
            scroll_offset: 0,
            should_exit: false,
            slash_matches: Vec::new(),
            slash_selected_index: 0,
            event_rx: None,
        }
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: current_time(),
            tool_calls: Vec::new(),
        });
    }

    /// Add an assistant message.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: current_time(),
            tool_calls: Vec::new(),
        });
    }

    /// Add a system message.
    pub fn add_system_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: content.to_string(),
            timestamp: current_time(),
            tool_calls: Vec::new(),
        });
    }

    /// Add a task to the activity panel.
    pub fn add_task(&mut self, label: &str) {
        self.agent_activity.tasks.push(TaskNode {
            task_id: None,
            label: label.to_string(),
            status: TaskStatus::Pending,
            depends_on: Vec::new(),
            percent: Some(0),
            children: Vec::new(),
        });
    }

    /// Add or update a task using a stable ID.
    pub fn upsert_task(&mut self, task_id: &str, label: &str, status: TaskStatus) {
        if let Some(task) = self
            .agent_activity
            .tasks
            .iter_mut()
            .find(|task| task.task_id.as_deref() == Some(task_id))
        {
            task.label = label.to_string();
            task.status = status;
            return;
        }

        self.agent_activity.tasks.push(TaskNode {
            task_id: Some(task_id.to_string()),
            label: label.to_string(),
            status,
            depends_on: Vec::new(),
            percent: Some(0),
            children: Vec::new(),
        });
    }

    /// Update the current task status.
    pub fn update_task_status(&mut self, index: usize, status: TaskStatus) {
        if let Some(task) = self.agent_activity.tasks.get_mut(index) {
            task.status = status;
        }
    }

    /// Set the current step.
    pub fn set_current_step(&mut self, step: &str) {
        self.agent_activity.current_step = Some(step.to_string());
    }

    /// Clear the current step.
    pub fn clear_current_step(&mut self) {
        self.agent_activity.current_step = None;
    }

    /// Load models for the model selector.
    pub fn load_models(&mut self) {
        use continuum_models_registry::MODELS;

        let current_provider = &self.session.provider;
        let current_model = &self.session.model;

        let mut models: Vec<ModelEntry> = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&format!("{}/", current_provider)))
            .map(|(k, m)| {
                let model_id = k.trim_start_matches(&format!("{}/", current_provider));
                ModelEntry {
                    provider: current_provider.clone(),
                    model_id: model_id.to_string(),
                    name: m.name.to_string(),
                    context_window: m.context_window,
                    input_price: m.input_per_mtok,
                    output_price: m.output_per_mtok,
                    is_current: model_id == current_model,
                }
            })
            .collect();

        models.sort_by(|a, b| a.model_id.cmp(&b.model_id));

        let filtered_indices: Vec<usize> = (0..models.len()).collect();

        self.model_selector = ModelSelector {
            models,
            filtered_indices,
            selected_index: 0,
            search_query: String::new(),
            search_active: false,
        };
    }

    /// Filter models by search query.
    pub fn filter_models(&mut self) {
        let query = self.model_selector.search_query.to_lowercase();
        self.model_selector.filtered_indices = self
            .model_selector
            .models
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                query.is_empty()
                    || m.model_id.to_lowercase().contains(&query)
                    || m.name.to_lowercase().contains(&query)
                    || m.provider.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        self.model_selector.selected_index = 0;
    }

    /// Get the selected model.
    pub fn selected_model(&self) -> Option<&ModelEntry> {
        self.model_selector
            .filtered_indices
            .get(self.model_selector.selected_index)
            .and_then(|&i| self.model_selector.models.get(i))
    }

    /// Switch to the selected model.
    pub fn switch_model(&mut self) {
        if let Some(model) = self.selected_model().cloned() {
            let provider = model.provider.clone();
            let model_id = model.model_id.clone();
            self.session.provider = provider.clone();
            self.session.model = model_id.clone();
            self.add_system_message(&format!("Switched to {provider}/{model_id}"));
        }
    }

    /// Whether the composer currently contains a slash command.
    pub fn slash_mode(&self) -> bool {
        self.composer.input.trim_start().starts_with('/')
    }

    /// Refresh live command palette suggestions from the current composer input.
    pub fn refresh_slash_matches(&mut self) {
        if !self.slash_mode() {
            self.slash_matches.clear();
            self.slash_selected_index = 0;
            return;
        }

        let query = self
            .composer
            .input
            .trim_start()
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("");

        self.slash_matches = filter_slash_commands(query);
        if self.slash_selected_index >= self.slash_matches.len() {
            self.slash_selected_index = 0;
        }
    }

    /// Return the currently selected slash suggestion, if any.
    pub fn selected_slash_match(&self) -> Option<SlashCommand> {
        self.slash_matches.get(self.slash_selected_index).copied()
    }

    /// Cycle the selected slash suggestion by `delta`.
    pub fn move_slash_selection(&mut self, delta: isize) {
        if self.slash_matches.is_empty() {
            self.slash_selected_index = 0;
            return;
        }

        let len = self.slash_matches.len() as isize;
        let current = self.slash_selected_index as isize;
        let next = (current + delta).rem_euclid(len);
        self.slash_selected_index = next as usize;
    }

    /// Apply the currently selected slash suggestion into the composer.
    pub fn apply_slash_completion(&mut self, append_space: bool) -> bool {
        let Some(cmd) = self.selected_slash_match() else {
            return false;
        };

        let input = self.composer.input.trim_start().to_string();
        let prefix = if input.starts_with('/') { "/" } else { "" };
        let (_, rest) = split_command_input(&input);
        let mut completed = format!("{prefix}{}", cmd.name);
        if !rest.is_empty() {
            completed.push(' ');
            completed.push_str(&rest);
        } else if append_space {
            completed.push(' ');
        }

        self.composer.input = completed;
        self.composer.cursor_pos = self.composer.input.len();
        self.refresh_slash_matches();
        true
    }

    /// Check whether the current slash command token resolves exactly.
    pub fn exact_slash_match(&self) -> Option<SlashCommand> {
        let query = self
            .composer
            .input
            .trim_start()
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("");
        resolve_slash_command(query)
    }

    /// Whether the current slash command is ambiguous.
    pub fn slash_is_ambiguous(&self) -> bool {
        self.slash_mode() && !self.slash_matches.is_empty() && self.exact_slash_match().is_none()
    }

    /// Attach a live dashboard event receiver.
    pub fn with_event_stream(mut self, rx: broadcast::Receiver<DashboardEvent>) -> Self {
        self.event_rx = Some(rx);
        self
    }

    /// Drain all pending live events and fold them into the UI state.
    pub fn drain_live_events(&mut self) {
        let mut disconnected = false;
        let mut rx = match self.event_rx.take() {
            Some(rx) => rx,
            None => return,
        };

        loop {
            match rx.try_recv() {
                Ok(event) => self.apply_dashboard_event(event),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Closed) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if !disconnected {
            self.event_rx = Some(rx);
        }
    }

    fn apply_dashboard_event(&mut self, event: DashboardEvent) {
        match event {
            DashboardEvent::PlanLoaded { tasks } => {
                self.agent_activity.tasks.clear();
                for task in tasks {
                    self.agent_activity.tasks.push(TaskNode {
                        task_id: Some(task.task_id.to_string()),
                        label: format!("{} ({:?})", task.label, task.agent),
                        status: TaskStatus::Pending,
                        depends_on: task
                            .depends_on
                            .into_iter()
                            .map(|id| id.to_string())
                            .collect(),
                        percent: Some(0),
                        children: Vec::new(),
                    });
                }
            }
            DashboardEvent::TaskQueued {
                task_id,
                agent,
                label,
                ..
            } => {
                self.upsert_task(&task_id, &format!("{label} ({agent})"), TaskStatus::Pending);
            }
            DashboardEvent::TaskStarted {
                task_id,
                agent,
                label,
                workspace,
            } => {
                self.upsert_task(&task_id, &format!("{label} ({agent})"), TaskStatus::Running);
                self.status = AppStatus::Thinking;
                self.set_current_step(&format!("{agent}: {label}"));
                if let Some(ws) = workspace {
                    self.add_system_message(&format!("workspace: {ws}"));
                }
            }
            DashboardEvent::TaskProgress {
                task_id,
                agent,
                message,
                percent,
            } => {
                if let Some(task) = self
                    .agent_activity
                    .tasks
                    .iter_mut()
                    .find(|task| task.task_id.as_deref() == Some(&task_id))
                {
                    task.percent = percent.or(task.percent);
                }
                let prefix = percent.map(|p| format!("{p}% ")).unwrap_or_default();
                self.add_system_message(&format!("{agent} [{task_id}] {prefix}{message}"));
            }
            DashboardEvent::TaskBlocked {
                task_id,
                agent,
                reason,
            } => {
                self.upsert_task(&task_id, &format!("{agent} blocked"), TaskStatus::Warning);
                self.add_system_message(&format!("{agent} [{task_id}] blocked: {reason}"));
            }
            DashboardEvent::TaskCompleted {
                task_id,
                agent,
                status,
                percent,
            } => {
                let task_status = if status.eq_ignore_ascii_case("done")
                    || status.eq_ignore_ascii_case("success")
                    || status.eq_ignore_ascii_case("passed")
                {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Warning
                };
                self.upsert_task(&task_id, &format!("{agent} done"), task_status);
                if let Some(task) = self
                    .agent_activity
                    .tasks
                    .iter_mut()
                    .find(|task| task.task_id.as_deref() == Some(&task_id))
                {
                    task.percent = percent.or(Some(100));
                }
                self.status = AppStatus::Ready;
                self.clear_current_step();
                self.add_system_message(&format!("{agent} [{task_id}] {status}"));
            }
            DashboardEvent::TaskFailed {
                task_id,
                agent,
                error,
            } => {
                self.upsert_task(&task_id, &format!("{agent} failed"), TaskStatus::Failed);
                self.status = AppStatus::Ready;
                self.clear_current_step();
                self.add_system_message(&format!("{agent} [{task_id}] failed: {error}"));
            }
            DashboardEvent::WorkspacePrepared {
                task_id,
                agent,
                workspace,
                isolated,
                source,
            } => {
                self.add_system_message(&format!(
                    "{agent} [{task_id}] workspace {workspace}{}{}",
                    if isolated { " [isolated]" } else { "" },
                    source.map(|s| format!(" from {s}")).unwrap_or_default()
                ));
            }
            DashboardEvent::Merge {
                task_id,
                workspace,
                merged_files,
                conflict,
            } => {
                let detail = conflict
                    .map(|c| format!(" conflict={c}"))
                    .unwrap_or_default();
                self.add_system_message(&format!(
                    "{task_id} merge {merged_files} file(s) from {workspace}{detail}"
                ));
            }
            DashboardEvent::ValidationUpdate {
                stage,
                passed,
                findings,
            } => {
                self.add_system_message(&format!(
                    "Validation {stage}: {} ({findings} findings)",
                    if passed { "passed" } else { "failed" }
                ));
            }
            DashboardEvent::CostUpdate { usd, tokens } => {
                self.agent_activity.tokens_used = tokens;
                self.add_system_message(&format!("Cost update: ${usd:.4}, {tokens} tokens"));
            }
            DashboardEvent::Log {
                level,
                target,
                message,
            } => {
                self.add_system_message(&format!("[{level}] {target}: {message}"));
            }
            DashboardEvent::Shutdown => {
                self.status = AppStatus::Ready;
                self.should_exit = true;
            }
        }
    }
}

/// Get current time as HH:MM:SS.
fn current_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
