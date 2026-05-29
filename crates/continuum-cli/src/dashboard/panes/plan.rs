#![allow(clippy::too_many_arguments)]

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

#[derive(Debug, Default, Clone)]
pub struct PlanPane {
    pub nodes: Vec<PlanNodeState>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PlanNodeState {
    pub task_id: String,
    pub label: String,
    pub agent: String,
    pub status: NodeStatus,
    pub depends_on: Vec<String>,
    pub percent: Option<u8>,
    #[allow(dead_code)]
    pub retries: u32,
    #[allow(dead_code)]
    pub usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum NodeStatus {
    Pending,
    Running,
    Done,
    Failed,
    Blocked,
}

impl Widget for PlanPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Plan DAG ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        let lines = render_tree(&self.nodes, self.selected);

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

fn render_tree(nodes: &[PlanNodeState], selected: Option<usize>) -> Vec<Line<'_>> {
    use std::collections::{BTreeMap, HashMap};

    let index_by_id: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.task_id.clone(), i))
        .collect();
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots = Vec::new();

    for node in nodes {
        if node.depends_on.is_empty() {
            roots.push(node.task_id.clone());
        }
        for dep in &node.depends_on {
            children
                .entry(dep.clone())
                .or_default()
                .push(node.task_id.clone());
        }
    }

    let mut lines = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for root in roots {
        render_node(
            nodes,
            &index_by_id,
            &children,
            &mut visited,
            &mut lines,
            &root,
            0,
            selected,
        );
    }

    for node in nodes {
        if !visited.contains(&node.task_id) {
            render_node(
                nodes,
                &index_by_id,
                &children,
                &mut visited,
                &mut lines,
                &node.task_id,
                0,
                selected,
            );
        }
    }

    lines
}

fn render_node(
    nodes: &[PlanNodeState],
    index_by_id: &std::collections::HashMap<String, usize>,
    children: &std::collections::BTreeMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    lines: &mut Vec<Line<'static>>,
    task_id: &str,
    depth: usize,
    selected: Option<usize>,
) {
    if !visited.insert(task_id.to_string()) {
        return;
    }
    let Some(&idx) = index_by_id.get(task_id) else {
        return;
    };
    let node = &nodes[idx];
    let (color, icon) = match node.status {
        NodeStatus::Pending => (Color::Gray, "[ ]"),
        NodeStatus::Running => (Color::Yellow, "[>]"),
        NodeStatus::Done => (Color::Green, "[✓]"),
        NodeStatus::Failed => (Color::Red, "[✗]"),
        NodeStatus::Blocked => (Color::Magenta, "[!]"),
    };
    let style = if Some(idx) == selected {
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(color)
    };
    let percent = node.percent.map(|p| format!(" {p}%")).unwrap_or_default();
    let indent = "  ".repeat(depth);
    lines.push(Line::from(vec![
        Span::styled(format!("{indent}{icon} "), style),
        Span::styled(node.label.clone(), style),
        Span::styled(percent, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!(" ({})", node.agent),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    if let Some(kids) = children.get(task_id) {
        for child in kids {
            render_node(
                nodes,
                index_by_id,
                children,
                visited,
                lines,
                child,
                depth + 1,
                selected,
            );
        }
    }
}
