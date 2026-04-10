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
pub fn render_creation(
    frame: &mut Frame,
    area: Rect,
    depth: u32,
    depth_label: &str,
    can_start: bool,
) {
    let depth_bar_filled = (depth as f32 / 100.0 * 20.0) as usize;
    let depth_bar = format!(
        "{}{}",
        "█".repeat(depth_bar_filled),
        "░".repeat(20 - depth_bar_filled)
    );

    let mut spans = vec![
        Span::styled("  🇺🇸 ", Style::default()),
        Span::styled("POLIT", Style::default().fg(theme::FG).bold()),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled("Character Creation", Style::default().fg(theme::FG)),
        Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(
            format!("{} {} {}%", depth_label, depth_bar, depth),
            Style::default().fg(if depth >= 30 {
                theme::SUCCESS
            } else {
                theme::FG_DIM
            }),
        ),
    ];

    if can_start {
        spans.push(Span::styled(
            "  → ready",
            Style::default().fg(theme::SUCCESS),
        ));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::BG_SUBTLE));

    frame.render_widget(bar, area);
}
