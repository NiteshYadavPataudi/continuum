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
    pub label: String,
    pub agent: String,
    pub status: NodeStatus,
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
}

impl Widget for PlanPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Plan DAG ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();
        for (i, node) in self.nodes.iter().enumerate() {
            let (color, icon) = match node.status {
                NodeStatus::Pending => (Color::Gray, "[ ]"),
                NodeStatus::Running => (Color::Yellow, "[>]"),
                NodeStatus::Done => (Color::Green, "[✓]"),
                NodeStatus::Failed => (Color::Red, "[✗]"),
            };
            let style = if Some(i) == self.selected {
                Style::default().fg(color).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(color)
            };
            let line = Line::from(vec![
                Span::styled(format!(" {icon} "), style),
                Span::styled(&node.label, style),
                Span::styled(format!(" ({})", node.agent), Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(line);
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
