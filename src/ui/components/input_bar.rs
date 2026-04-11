use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::theme;

/// Calculate the visual height needed for the input bar, accounting for
/// both explicit newlines AND soft wrapping within the available width.
pub fn height_for(input: &str, available_width: u16) -> u16 {
    // Inner width = available_width - 2 (borders) - 2 (▶ prefix)
    let inner_width = available_width.saturating_sub(4).max(10) as usize;

    let mut visual_lines = 0u16;
    for line in input.split('\n') {
        if line.is_empty() {
            visual_lines += 1;
        } else {
            // Each line may wrap across multiple visual lines
            visual_lines += ((line.len() + inner_width - 1) / inner_width).max(1) as u16;
        }
    }

    // +2 for borders, min 3, max 10
    (visual_lines + 2).max(3).min(10)
}

/// Render a floating input bar with red arrow, word-wrapping, and blinking cursor.
/// Returns the inner area (for slash menu positioning).
pub fn render(frame: &mut Frame, area: Rect, input: &str) -> Rect {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_BLUE))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    let inner_area = input_block.inner(area);
    frame.render_widget(input_block, area);

    // Build the text with ▶ prefix and cursor
    let input_lines: Vec<&str> = input.split('\n').collect();
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
        // Cursor on the last line
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

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(widget, inner_area);

    inner_area
}
