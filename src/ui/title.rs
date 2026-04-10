use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::theme;

/// Title screen menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    NewCampaign,
    ContinueCampaign,
    Settings,
    Quit,
}

pub struct TitleScreen {
    pub selected: usize,
    pub action: Option<TitleAction>,
    items: Vec<(&'static str, TitleAction)>,
    has_save: bool,
    frame_count: u64,
}

impl TitleScreen {
    pub fn new(has_save: bool) -> Self {
        let mut items = vec![];
        items.push(("  New Campaign", TitleAction::NewCampaign));
        if has_save {
            items.push(("  Continue Campaign", TitleAction::ContinueCampaign));
        }
        items.push(("  Settings", TitleAction::Settings));
        items.push(("  Quit", TitleAction::Quit));

        Self {
            selected: 0,
            action: None,
            items,
            has_save,
            frame_count: 0,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<TitleAction, Box<dyn std::error::Error>> {
        loop {
            self.frame_count += 1;
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.selected > 0 {
                                self.selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.selected < self.items.len() - 1 {
                                self.selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            return Ok(self.items[self.selected].1);
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(TitleAction::Quit);
                        }
                        KeyCode::Char('c')
                            if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                        {
                            return Ok(TitleAction::Quit);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // top margin
                Constraint::Length(2),      // title
                Constraint::Length(2),      // subtitle
                Constraint::Length(3),      // spacer
                Constraint::Length(8),      // menu (fixed, not Min)
                Constraint::Min(1),         // fill
                Constraint::Length(2),      // footer
            ])
            .split(area);

        // Title: flag emoji + POLIT
        let title = Paragraph::new(Line::from(vec![
            Span::styled("🇺🇸 ", Style::default()),
            Span::styled("P O L I T", Style::default().fg(theme::FG).bold()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(title, layout[1]);

        // Subtitle
        let subtitle = Paragraph::new(Line::from(vec![Span::styled(
            "The American Politics Simulator",
            Style::default().fg(theme::FG_DIM),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(subtitle, layout[2]);

        // Menu
        self.render_menu(frame, layout[4]);

        // Footer
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑↓ Navigate  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled("Enter Select  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled("Q Quit", Style::default().fg(theme::FG_MUTED)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(footer, layout[5]);
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        // Add extra line for grayed-out Continue if no saves
        let extra = if self.has_save { 0 } else { 1 };
        let menu_area = centered_rect_fixed(34, self.items.len() as u16 + extra as u16 + 2, area);

        let mut lines: Vec<Line> = Vec::new();

        for (i, (label, _)) in self.items.iter().enumerate() {
            if i == self.selected {
                lines.push(Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(theme::ACCENT).bold()),
                    Span::styled(*label, Style::default().fg(theme::FG).bold()),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(*label, Style::default().fg(theme::FG_DIM)),
                ]));
            }

            // After New Campaign, show grayed Continue if no saves
            if !self.has_save && *label == "  New Campaign" {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(
                        "  Continue Campaign (no saves)",
                        Style::default().fg(theme::FG_MUTED),
                    ),
                ]));
            }
        }

        let menu = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .style(Style::default().bg(theme::BG_SUBTLE)),
        );

        frame.render_widget(Clear, menu_area);
        frame.render_widget(menu, menu_area);
    }
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
