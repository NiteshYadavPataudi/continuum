use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

#[derive(Debug, Default, Clone)]
pub struct ValidationPane {
    pub stages: Vec<StageState>,
}

#[derive(Debug, Clone)]
pub struct StageState {
    pub name: String,
    pub status: StageStatus,
    pub findings: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StageStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

impl Widget for ValidationPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Validation Pipeline ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();
        for stage in &self.stages {
            let (color, icon) = match stage.status {
                StageStatus::Pending => (Color::Gray, "  "),
                StageStatus::Running => (Color::Yellow, "> "),
                StageStatus::Passed => (Color::Green, "✓ "),
                StageStatus::Failed => (Color::Red, "✗ "),
            };
            let style = Style::default().fg(color);
            let findings_str = if stage.findings > 0 {
                format!(" ({} findings)", stage.findings)
            } else {
                String::new()
            };
            let line = Line::from(vec![
                Span::styled(format!(" {icon}"), style),
                Span::styled(&stage.name, style),
                Span::styled(findings_str, Style::default().fg(Color::Yellow)),
            ]);
            lines.push(line);
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
