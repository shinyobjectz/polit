use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ui::theme;

/// Render a floating character sheet to the right of the chat column.
/// Shows condensed key-value pairs with truncated long values.
/// Returns true if it was rendered (enough room on screen).
pub fn render(
    frame: &mut Frame,
    chat_area: Rect,
    full_area: Rect,
    summary: &[(String, String, bool)],
) -> bool {
    let filled: Vec<_> = summary.iter().filter(|(_, _, f)| *f).collect();
    if filled.is_empty() {
        return false;
    }

    let filled_count = filled.len() as u16;
    let block_height = (filled_count + 3).min(full_area.height.saturating_sub(4));
    let block_width = 34;

    // Position with gap (4 chars) from chat column
    let block_x = chat_area.right() + 4;
    let block_y = chat_area.y + 2;

    // Only render if there's room to the right
    if block_x + block_width > full_area.width {
        return false;
    }

    let block_area = Rect::new(block_x, block_y, block_width, block_height);

    let lines: Vec<Line> = filled
        .iter()
        .map(|(key, value, _)| {
            // Truncate long values
            let display_val = if value.len() > 18 {
                format!("{}…", &value[..17])
            } else {
                value.clone()
            };
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(theme::SUCCESS)),
                Span::styled(format!("{}: ", key), Style::default().fg(theme::FG_DIM)),
                Span::styled(display_val, Style::default().fg(theme::FG)),
            ])
        })
        .collect();

    let block = Paragraph::new(lines).block(
        Block::default()
            .title(" Character ")
            .title_style(Style::default().fg(theme::FG_DIM))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .style(Style::default().bg(theme::BG_SUBTLE)),
    );

    frame.render_widget(Clear, block_area);
    frame.render_widget(block, block_area);
    true
}
