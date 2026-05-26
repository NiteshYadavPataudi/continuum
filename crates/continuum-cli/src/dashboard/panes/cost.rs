use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Widget};

#[derive(Debug, Default, Clone)]
pub struct CostPane {
    pub usd_spent: f64,
    pub usd_budget: f64,
    pub tokens_used: u64,
    pub retries: u32,
}

impl Widget for CostPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Token / Cost ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3), Constraint::Length(1)])
            .split(inner);

        let budget_pct = if self.usd_budget > 0.0 {
            (self.usd_spent / self.usd_budget).min(1.0)
        } else {
            0.0
        };
        let gauge = Gauge::default()
            .block(Block::default().title(" Budget "))
            .gauge_style(Style::default().fg(if budget_pct > 0.8 { Color::Red } else { Color::Green }))
            .ratio(budget_pct);
        gauge.render(chunks[0], buf);

        let cost_line = Line::from(vec![
            Span::raw(format!(" USD: ${:.4} / ${:.4}", self.usd_spent, self.usd_budget)),
        ]);
        Paragraph::new(cost_line).render(chunks[1], buf);

        let token_line = Line::from(vec![
            Span::raw(format!(" Tokens: {}", self.tokens_used)),
            Span::raw(format!("  Retries: {}", self.retries)),
        ]);
        Paragraph::new(token_line).render(chunks[2], buf);
    }
}
