use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

#[derive(Debug, Default, Clone)]
pub struct LogPane {
    pub lines: Vec<LogLine>,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub level: String,
    #[allow(dead_code)]
    pub target: String,
    pub message: String,
}

impl Widget for LogPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Agent Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        let inner = block.inner(area);
        block.render(area, buf);

        let start = self.lines.len().saturating_sub(inner.height as usize);
        let lines: Vec<Line> = self.lines[std::cmp::max(start, 0)..]
            .iter()
            .map(|l| {
                let level_color = match l.level.as_str() {
                    "ERROR" => Color::Red,
                    "WARN" => Color::Yellow,
                    "INFO" => Color::White,
                    _ => Color::DarkGray,
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", l.level), Style::default().fg(level_color)),
                    Span::styled(&l.message, Style::default().fg(Color::White)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
