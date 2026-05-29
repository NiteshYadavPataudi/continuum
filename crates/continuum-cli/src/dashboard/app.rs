use std::io;
use std::time::Duration;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use tokio::sync::broadcast;

use continuum_telemetry::DashboardEvent;

use super::panes::{
    CostPane, LogLine, LogPane, NodeStatus, PlanNodeState, PlanPane, StageState, StageStatus,
    ValidationPane,
};

/// 4-pane live TUI that consumes [`DashboardEvent`]s from a broadcast channel.
pub struct DashboardApp {
    plan: PlanPane,
    validation: ValidationPane,
    log: LogPane,
    cost: CostPane,
    rx: broadcast::Receiver<DashboardEvent>,
}

impl DashboardApp {
    /// Create a new dashboard bound to the given event stream.
    pub fn new(rx: broadcast::Receiver<DashboardEvent>) -> Self {
        Self {
            plan: PlanPane::default(),
            validation: ValidationPane::default(),
            log: LogPane::default(),
            cost: CostPane::default(),
            rx,
        }
    }

    /// Run the TUI event loop until the stream closes or the user presses `q`.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        crossterm::terminal::enable_raw_mode()?;
        let _reset = ResetGuard;

        loop {
            // Drain all available events (non-blocking)
            loop {
                match self.rx.try_recv() {
                    Ok(event) => {
                        self.handle_event(event);
                    }
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Closed) => return Ok(()),
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        tracing::warn!("dashboard lagged by {n} events");
                        break;
                    }
                }
            }

            // Draw
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(40),
                        Constraint::Percentage(25),
                        Constraint::Percentage(20),
                        Constraint::Percentage(15),
                    ])
                    .split(f.area());

                f.render_widget(self.plan.clone(), chunks[0]);
                f.render_widget(self.validation.clone(), chunks[1]);
                f.render_widget(self.log.clone(), chunks[2]);
                f.render_widget(self.cost.clone(), chunks[3]);
            })?;

            // Non-blocking keyboard input
            if crossterm::event::poll(Duration::from_millis(100))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: DashboardEvent) {
        use DashboardEvent::*;
        match event {
            PlanLoaded { tasks } => {
                self.plan.nodes.clear();
                self.plan.nodes.extend(tasks.into_iter().map(|task| {
                    PlanNodeState {
                        task_id: task.task_id.to_string(),
                        label: task.label,
                        agent: format!("{:?}", task.agent),
                        status: NodeStatus::Pending,
                        depends_on: task
                            .depends_on
                            .into_iter()
                            .map(|id| id.to_string())
                            .collect(),
                        percent: Some(0),
                        retries: 0,
                        usd: 0.0,
                    }
                }));
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "scheduler".into(),
                    message: format!("loaded plan with {} task(s)", self.plan.nodes.len()),
                });
            }
            TaskQueued {
                task_id,
                agent,
                label,
                ready_group,
                depends_on,
            } => {
                if self.plan.nodes.iter().all(|n| n.task_id != task_id) {
                    self.plan.nodes.push(PlanNodeState {
                        task_id: task_id.clone(),
                        label: label.clone(),
                        agent: agent.clone(),
                        status: NodeStatus::Pending,
                        depends_on,
                        percent: Some(0),
                        retries: ready_group as u32,
                        usd: 0.0,
                    });
                }
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "scheduler".into(),
                    message: format!("queued {task_id} for {agent}: {label}"),
                });
            }
            TaskStarted {
                task_id,
                agent,
                label,
                workspace,
            } => {
                if let Some(node) = self.plan.nodes.iter_mut().find(|n| n.task_id == task_id) {
                    node.status = NodeStatus::Running;
                    node.percent = Some(5);
                } else {
                    self.plan.nodes.push(PlanNodeState {
                        task_id: task_id.clone(),
                        label: label.clone(),
                        agent: agent.clone(),
                        status: NodeStatus::Running,
                        depends_on: Vec::new(),
                        percent: Some(5),
                        retries: 0,
                        usd: 0.0,
                    });
                }
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "scheduler".into(),
                    message: format!(
                        "{agent}: started {label} ({task_id}){}",
                        workspace
                            .as_deref()
                            .map(|w| format!(" @ {w}"))
                            .unwrap_or_default()
                    ),
                });
            }
            TaskProgress {
                task_id,
                agent,
                message,
                percent,
            } => {
                if let Some(node) = self.plan.nodes.iter_mut().find(|n| n.task_id == task_id) {
                    node.percent = percent.or(node.percent);
                }
                if let Some(p) = percent {
                    self.log.lines.push(LogLine {
                        level: "INFO".into(),
                        target: "scheduler".into(),
                        message: format!("{agent} [{task_id}] {p}% {message}"),
                    });
                } else {
                    self.log.lines.push(LogLine {
                        level: "INFO".into(),
                        target: "scheduler".into(),
                        message: format!("{agent} [{task_id}] {message}"),
                    });
                }
            }
            TaskBlocked {
                task_id,
                agent,
                reason,
            } => {
                if let Some(node) = self.plan.nodes.iter_mut().find(|n| n.task_id == task_id) {
                    node.status = NodeStatus::Blocked;
                    node.percent = Some(node.percent.unwrap_or(0));
                }
                self.log.lines.push(LogLine {
                    level: "WARN".into(),
                    target: "scheduler".into(),
                    message: format!("{agent}: blocked {task_id} ({reason})"),
                });
            }
            TaskCompleted {
                task_id,
                agent,
                status,
                percent,
            } => {
                if let Some(node) = self.plan.nodes.iter_mut().find(|n| n.task_id == task_id) {
                    node.status = match status.as_str() {
                        "done" | "success" | "passed" => NodeStatus::Done,
                        "failed" | "error" => NodeStatus::Failed,
                        _ => NodeStatus::Done,
                    };
                    node.percent = percent.or(Some(100));
                }
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "scheduler".into(),
                    message: format!("{agent}: {status} ({task_id})"),
                });
            }
            TaskFailed {
                task_id,
                agent,
                error,
            } => {
                if let Some(node) = self.plan.nodes.iter_mut().find(|n| n.task_id == task_id) {
                    node.status = NodeStatus::Failed;
                    node.percent = Some(node.percent.unwrap_or(0));
                }
                self.log.lines.push(LogLine {
                    level: "ERROR".into(),
                    target: "scheduler".into(),
                    message: format!("{agent}: {error} ({task_id})"),
                });
            }
            WorkspacePrepared {
                task_id,
                agent,
                workspace,
                isolated,
                source,
            } => {
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "workspace".into(),
                    message: format!(
                        "{agent}: workspace prepared {workspace} for {task_id}{}{}",
                        if isolated { " [isolated]" } else { "" },
                        source.map(|s| format!(" from {s}")).unwrap_or_default()
                    ),
                });
            }
            Merge {
                task_id,
                workspace,
                merged_files,
                conflict,
            } => {
                let level = if conflict.is_some() { "WARN" } else { "INFO" };
                self.log.lines.push(LogLine {
                    level: level.into(),
                    target: "merge".into(),
                    message: format!(
                        "{task_id}: merged {merged_files} file(s) from {workspace}{}",
                        conflict
                            .map(|c| format!(" (conflict: {c})"))
                            .unwrap_or_default()
                    ),
                });
            }
            ValidationUpdate {
                stage,
                passed,
                findings,
            } => {
                self.validation.stages.push(StageState {
                    name: stage.clone(),
                    status: if passed {
                        StageStatus::Passed
                    } else {
                        StageStatus::Failed
                    },
                    findings,
                });
                self.log.lines.push(LogLine {
                    level: if passed { "INFO" } else { "WARN" }.into(),
                    target: "validation".into(),
                    message: format!(
                        "{stage}: {} ({} findings)",
                        if passed { "pass" } else { "fail" },
                        findings
                    ),
                });
            }
            CostUpdate { usd, tokens } => {
                self.cost.usd_spent = usd;
                self.cost.tokens_used = tokens;
            }
            Log {
                level,
                target,
                message,
            } => {
                self.log.lines.push(LogLine {
                    level,
                    target,
                    message,
                });
            }
            Shutdown => {
                self.log.lines.push(LogLine {
                    level: "INFO".into(),
                    target: "system".into(),
                    message: "shutting down".into(),
                });
            }
        }
    }
}

/// Drops terminal raw mode on exit (even on panic).
struct ResetGuard;

impl Drop for ResetGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
        );
    }
}
