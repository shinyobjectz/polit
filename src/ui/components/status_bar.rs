use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::ui::theme;

/// Render the game status bar (used as footer).
pub fn render_game(
    frame: &mut Frame,
    area: Rect,
    week: u32,
    year: u32,
    phase: &str,
    ap_current: i32,
    ap_max: i32,
) {
    let filled = "█".repeat(ap_current.max(0) as usize);
    let empty = "░".repeat((ap_max - ap_current).max(0) as usize);

    let bar = Paragraph::new(Line::from(vec![
        Span::styled("  🇺🇸 ", Style::default()),
        Span::styled("POLIT", Style::default().fg(theme::FG).bold()),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(
            format!("Week {}, {}", week, year),
            Style::default().fg(theme::FG_DIM),
        ),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(phase.to_string(), Style::default().fg(theme::FG)),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(
            format!("AP {}{} ", filled, empty),
            Style::default().fg(theme::FG_DIM),
        ),
        Span::styled(
            format!("{}/{}", ap_current, ap_max),
            Style::default().fg(theme::FG),
        ),
    ]))
    .style(Style::default().bg(theme::BG_SUBTLE));

    frame.render_widget(bar, area);
}

/// Render the character creation status bar (used as footer).
/// Left side: POLIT branding. Right side: subtle depth meter.
pub fn render_creation(
    frame: &mut Frame,
    area: Rect,
    depth: u32,
    depth_label: &str,
    can_start: bool,
) {
    // Smaller bar — 10 chars instead of 20
    let depth_bar_filled = (depth as f32 / 100.0 * 10.0) as usize;
    let depth_bar = format!(
        "{}{}",
        "▪".repeat(depth_bar_filled),
        "·".repeat(10 - depth_bar_filled)
    );

    // Build left side
    let left = vec![
        Span::styled("  🇺🇸 ", Style::default()),
        Span::styled("POLIT", Style::default().fg(theme::FG).bold()),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(
            "Character Creation",
            Style::default().fg(theme::FG_DIM),
        ),
    ];

    // Build right side — muted blue, subtle
    let mut right_parts = vec![
        Span::styled(
            format!("{} ", depth_label),
            Style::default().fg(theme::FG_MUTED),
        ),
        Span::styled(
            depth_bar,
            Style::default().fg(theme::ACCENT_BLUE),
        ),
        Span::styled(
            format!(" {}%", depth),
            Style::default().fg(theme::FG_MUTED),
        ),
    ];

    if can_start {
        right_parts.push(Span::styled(
            "  → ready",
            Style::default().fg(theme::ACCENT_BLUE),
        ));
    }

    // Combine: left-aligned left, right-aligned right
    // Fill middle with spaces to push right side to the edge
    let left_len: usize = left.iter().map(|s| s.width()).sum();
    let right_len: usize = right_parts.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize).saturating_sub(left_len + right_len + 2);

    let mut spans = left;
    spans.push(Span::styled(
        " ".repeat(gap),
        Style::default(),
    ));
    spans.extend(right_parts);
    spans.push(Span::styled("  ", Style::default()));

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::BG_SUBTLE));
    frame.render_widget(bar, area);
}
