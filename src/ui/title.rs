use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};

/// Title screen menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    NewCampaign,
    ContinueCampaign,
    Demo,
    Settings,
    Quit,
}

pub struct TitleScreen {
    pub selected: usize,
    pub action: Option<TitleAction>,
    items: Vec<(&'static str, TitleAction)>,
    frame_count: u64,
}

impl TitleScreen {
    pub fn new(has_save: bool) -> Self {
        let mut items = vec![];
        if has_save {
            items.push(("  Continue Campaign", TitleAction::ContinueCampaign));
        }
        items.push(("  New Campaign", TitleAction::NewCampaign));
        items.push(("  Demo Walkthrough", TitleAction::Demo));
        items.push(("  Settings", TitleAction::Settings));
        items.push(("  Quit", TitleAction::Quit));

        Self {
            selected: 0,
            action: None,
            items,
            frame_count: 0,
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<TitleAction, Box<dyn std::error::Error>> {
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
                        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
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

        // Dark background
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(8, 8, 16))),
            area,
        );

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),      // top margin
                Constraint::Length(12),     // flag + title
                Constraint::Length(2),      // subtitle
                Constraint::Length(1),      // spacer
                Constraint::Min(8),        // menu
                Constraint::Length(2),      // footer
            ])
            .split(area);

        // Flag + Title
        self.render_flag_and_title(frame, layout[1]);

        // Subtitle
        let subtitle = Paragraph::new(Line::from(vec![
            Span::styled("The American Politics Simulator", Style::default().fg(Color::Rgb(180, 180, 200))),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(subtitle, layout[2]);

        // Menu
        self.render_menu(frame, layout[4]);

        // Footer
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑↓ Navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter Select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Q Quit", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(footer, layout[5]);
    }

    fn render_flag_and_title(&self, frame: &mut Frame, area: Rect) {
        // ASCII American flag with POLIT title next to it
        let flag_and_title = vec![
            Line::from(vec![
                Span::styled("  ★ ★ ★ ★ ★ ★", Style::default().fg(Color::White).bg(Color::Rgb(0, 40, 104))),
                Span::styled("═══════════════════", Style::default().fg(Color::Rgb(191, 10, 48))),
                Span::raw("    "),
                Span::styled("██████╗  ", Style::default().fg(Color::White).bold()),
                Span::styled("██████╗ ", Style::default().fg(Color::White).bold()),
                Span::styled("██╗     ", Style::default().fg(Color::White).bold()),
                Span::styled("██╗", Style::default().fg(Color::White).bold()),
                Span::styled("████████╗", Style::default().fg(Color::White).bold()),
            ]),
            Line::from(vec![
                Span::styled("   ★ ★ ★ ★ ★ ", Style::default().fg(Color::White).bg(Color::Rgb(0, 40, 104))),
                Span::styled("═══════════════════", Style::default().fg(Color::White)),
                Span::raw("    "),
                Span::styled("██╔══██╗ ", Style::default().fg(Color::White).bold()),
                Span::styled("██╔══██╗", Style::default().fg(Color::White).bold()),
                Span::styled("██║     ", Style::default().fg(Color::White).bold()),
                Span::styled("██║", Style::default().fg(Color::White).bold()),
                Span::styled("╚══██╔══╝", Style::default().fg(Color::White).bold()),
            ]),
            Line::from(vec![
                Span::styled("  ★ ★ ★ ★ ★ ★", Style::default().fg(Color::White).bg(Color::Rgb(0, 40, 104))),
                Span::styled("═══════════════════", Style::default().fg(Color::Rgb(191, 10, 48))),
                Span::raw("    "),
                Span::styled("██████╔╝ ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║  ██║", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║     ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("   ██║   ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
            ]),
            Line::from(vec![
                Span::styled("   ★ ★ ★ ★ ★ ", Style::default().fg(Color::White).bg(Color::Rgb(0, 40, 104))),
                Span::styled("═══════════════════", Style::default().fg(Color::White)),
                Span::raw("    "),
                Span::styled("██╔═══╝  ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║  ██║", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║     ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("██║", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                Span::styled("   ██║   ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
            ]),
            Line::from(vec![
                Span::styled("  ★ ★ ★ ★ ★ ★", Style::default().fg(Color::White).bg(Color::Rgb(0, 40, 104))),
                Span::styled("═══════════════════", Style::default().fg(Color::Rgb(191, 10, 48))),
                Span::raw("    "),
                Span::styled("██║      ", Style::default().fg(Color::Rgb(0, 40, 104)).bold()),
                Span::styled("╚█████╔╝", Style::default().fg(Color::Rgb(0, 40, 104)).bold()),
                Span::styled("███████╗", Style::default().fg(Color::Rgb(0, 40, 104)).bold()),
                Span::styled("██║", Style::default().fg(Color::Rgb(0, 40, 104)).bold()),
                Span::styled("   ██║   ", Style::default().fg(Color::Rgb(0, 40, 104)).bold()),
            ]),
            Line::from(vec![
                Span::styled("                  ", Style::default()),
                Span::styled("═══════════════════", Style::default().fg(Color::White)),
                Span::raw("    "),
                Span::styled("╚═╝      ", Style::default().fg(Color::Rgb(0, 40, 104))),
                Span::styled(" ╚════╝ ", Style::default().fg(Color::Rgb(0, 40, 104))),
                Span::styled("╚══════╝", Style::default().fg(Color::Rgb(0, 40, 104))),
                Span::styled("╚═╝", Style::default().fg(Color::Rgb(0, 40, 104))),
                Span::styled("   ╚═╝   ", Style::default().fg(Color::Rgb(0, 40, 104))),
            ]),
            Line::from(vec![
                Span::styled("                  ", Style::default()),
                Span::styled("═══════════════════", Style::default().fg(Color::Rgb(191, 10, 48))),
            ]),
            Line::from(vec![
                Span::styled("                  ", Style::default()),
                Span::styled("═══════════════════", Style::default().fg(Color::White)),
            ]),
        ];

        let title_widget = Paragraph::new(flag_and_title)
            .alignment(Alignment::Center);
        frame.render_widget(title_widget, area);
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let menu_area = centered_rect_fixed(30, self.items.len() as u16 + 2, area);

        let items: Vec<Line> = self.items.iter().enumerate().map(|(i, (label, _))| {
            if i == self.selected {
                Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Color::Rgb(191, 10, 48)).bold()),
                    Span::styled(*label, Style::default().fg(Color::White).bold()),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(*label, Style::default().fg(Color::Rgb(140, 140, 160))),
                ])
            }
        }).collect();

        let menu = Paragraph::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                .style(Style::default().bg(Color::Rgb(15, 15, 25))));

        frame.render_widget(Clear, menu_area);
        frame.render_widget(menu, menu_area);
    }
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
