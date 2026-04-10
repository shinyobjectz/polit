use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::theme;

/// Render a floating input bar with red arrow, multi-line support.
/// Returns the inner area for slash menu positioning.
pub fn render(frame: &mut Frame, area: Rect, input: &str) -> Rect {
    let input_lines: Vec<&str> = input.split('\n').collect();

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_BLUE))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    let inner_area = input_block.inner(area);
    frame.render_widget(input_block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, line) in input_lines.iter().enumerate() {
        let mut spans = Vec::new();
        if i == 0 {
            spans.push(Span::styled("▶ ", Style::default().fg(theme::ACCENT)));
        } else {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(theme::FG),
        ));
        if i == input_lines.len() - 1 {
            spans.push(Span::styled(
                "▊",
                Style::default()
                    .fg(theme::FG_DIM)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner_area);

    inner_area
}

/// Calculate the height needed for the input bar based on content.
pub fn height_for(input: &str) -> u16 {
    let line_count = input.split('\n').count() as u16;
    (line_count + 2).max(3).min(10) // border + content, min 3, max 10
}
