use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::music::MusicController;
use super::theme;

/// Scenario configuration result
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub era: Era,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    Modern,
    Historical,
    AlternateHistory,
    Speculative,
}

impl Era {
    pub fn label(&self) -> &str {
        match self {
            Era::Modern => "Modern America",
            Era::Historical => "Historical",
            Era::AlternateHistory => "Alternate History",
            Era::Speculative => "Speculative Future",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Era::Modern => "Start in 2024 with real conditions",
            Era::Historical => "Pick a year from American history",
            Era::AlternateHistory => "Fork from a historical turning point",
            Era::Speculative => "A plausible future America",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Story,
    Standard,
    Ironman,
    Nightmare,
}

impl Difficulty {
    pub fn label(&self) -> &str {
        match self {
            Difficulty::Story => "Story",
            Difficulty::Standard => "Standard",
            Difficulty::Ironman => "Ironman",
            Difficulty::Nightmare => "Nightmare",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Difficulty::Story => "Reduced DCs, forgiving. Learn the systems.",
            Difficulty::Standard => "Balanced challenge. Fair dice.",
            Difficulty::Ironman => "No reloads. Consequences stick.",
            Difficulty::Nightmare => "Hostile media. Scheming NPCs. Volatile economy.",
        }
    }
}

enum Phase {
    PickEra,
    PickDifficulty,
}

pub struct ScenarioScreen {
    phase: Phase,
    era_selected: usize,
    diff_selected: usize,
    chosen_era: Option<Era>,
}

const ERAS: &[Era] = &[
    Era::Modern,
    Era::Historical,
    Era::AlternateHistory,
    Era::Speculative,
];

const DIFFICULTIES: &[Difficulty] = &[
    Difficulty::Story,
    Difficulty::Standard,
    Difficulty::Ironman,
    Difficulty::Nightmare,
];

impl ScenarioScreen {
    pub fn new() -> Self {
        Self {
            phase: Phase::PickEra,
            era_selected: 0,
            diff_selected: 1, // Default to Standard
            chosen_era: None,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        music: &MusicController,
    ) -> Result<Option<ScenarioConfig>, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|frame| self.render(frame, music))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Char('q') => return Ok(None),
                        KeyCode::Char('m') => {
                            music.toggle_mute();
                        }
                        KeyCode::Char('c')
                            if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                        {
                            return Ok(None);
                        }
                        KeyCode::Up | KeyCode::Char('k') => match self.phase {
                            Phase::PickEra => {
                                if self.era_selected > 0 {
                                    self.era_selected -= 1;
                                    music.play_nav();
                                }
                            }
                            Phase::PickDifficulty => {
                                if self.diff_selected > 0 {
                                    self.diff_selected -= 1;
                                    music.play_nav();
                                }
                            }
                        },
                        KeyCode::Down | KeyCode::Char('j') => match self.phase {
                            Phase::PickEra => {
                                if self.era_selected < ERAS.len() - 1 {
                                    self.era_selected += 1;
                                    music.play_nav();
                                }
                            }
                            Phase::PickDifficulty => {
                                if self.diff_selected < DIFFICULTIES.len() - 1 {
                                    self.diff_selected += 1;
                                    music.play_nav();
                                }
                            }
                        },
                        KeyCode::Enter => match self.phase {
                            Phase::PickEra => {
                                music.play_select();
                                self.chosen_era = Some(ERAS[self.era_selected]);
                                self.phase = Phase::PickDifficulty;
                            }
                            Phase::PickDifficulty => {
                                music.play_select();
                                return Ok(Some(ScenarioConfig {
                                    era: self.chosen_era.unwrap(),
                                    difficulty: DIFFICULTIES[self.diff_selected],
                                }));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame, music: &MusicController) {
        let area = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

        // Calculate the card height needed
        let card_height = match self.phase {
            Phase::PickEra => ERAS.len() as u16 * 3 + 3,
            Phase::PickDifficulty => DIFFICULTIES.len() as u16 * 3 + 3,
        };

        // Total content block: title(2) + subtitle(2) + spacer(2) + card + footer spacing
        let content_height = 2 + 2 + 2 + card_height;
        let top_margin = area.height.saturating_sub(content_height + 4) / 3; // bias toward upper third

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_margin),  // computed margin
                Constraint::Length(2),           // POLIT title
                Constraint::Length(2),           // subtitle
                Constraint::Length(2),           // spacer
                Constraint::Length(card_height), // menu card (exact fit)
                Constraint::Min(1),              // fill below
                Constraint::Length(2),           // footer
            ])
            .split(area);

        // POLIT header
        let title = Paragraph::new(Line::from(vec![
            Span::styled("🇺🇸 ", Style::default()),
            Span::styled("P O L I T", Style::default().fg(theme::FG).bold()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(title, layout[1]);

        // Subtitle
        let subtitle_text = match self.phase {
            Phase::PickEra => "Choose your era".to_string(),
            Phase::PickDifficulty => {
                let era = self.chosen_era.unwrap();
                format!("{} · Choose difficulty", era.label())
            }
        };
        let subtitle = Paragraph::new(Line::from(Span::styled(
            subtitle_text,
            Style::default().fg(theme::FG_DIM),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(subtitle, layout[2]);

        // Menu card
        match self.phase {
            Phase::PickEra => self.render_era_menu(frame, layout[4]),
            Phase::PickDifficulty => self.render_difficulty_menu(frame, layout[4]),
        }

        // Footer
        let mute_label = if music.is_muted() {
            "M \u{266b}off"
        } else {
            "M \u{266b}on"
        };
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(
                "\u{2191}\u{2193} Navigate  ",
                Style::default().fg(theme::FG_MUTED),
            ),
            Span::styled("Enter Select  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(mute_label, Style::default().fg(theme::FG_MUTED)),
            Span::styled("  Esc Back", Style::default().fg(theme::FG_MUTED)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(footer, layout[6]);
    }

    fn render_era_menu(&self, frame: &mut Frame, area: Rect) {
        let menu_area = centered_rect_fixed(56, ERAS.len() as u16 * 3 + 3, area);
        let mut lines: Vec<Line> = vec![Line::from("")];

        for (i, era) in ERAS.iter().enumerate() {
            if i == self.era_selected {
                lines.push(Line::from(vec![
                    Span::styled("    ▶  ", Style::default().fg(theme::ACCENT).bold()),
                    Span::styled(era.label(), Style::default().fg(theme::FG).bold()),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled(
                        era.description(),
                        Style::default().fg(theme::FG_DIM).italic(),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled(era.label(), Style::default().fg(theme::FG_DIM)),
                ]));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(""));
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

    fn render_difficulty_menu(&self, frame: &mut Frame, area: Rect) {
        let menu_area = centered_rect_fixed(56, DIFFICULTIES.len() as u16 * 3 + 3, area);
        let mut lines: Vec<Line> = vec![Line::from("")];

        for (i, diff) in DIFFICULTIES.iter().enumerate() {
            if i == self.diff_selected {
                lines.push(Line::from(vec![
                    Span::styled("    ▶  ", Style::default().fg(theme::ACCENT).bold()),
                    Span::styled(diff.label(), Style::default().fg(theme::FG).bold()),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled(
                        diff.description(),
                        Style::default().fg(theme::FG_DIM).italic(),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled(diff.label(), Style::default().fg(theme::FG_DIM)),
                ]));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(""));
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
