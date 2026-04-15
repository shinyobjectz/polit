use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::prelude::*;
use ratatui::Terminal;
use ratatui::widgets::{Block, Paragraph};
use serde::Deserialize;
use std::time::{Duration, Instant};

use super::music::MusicController;
use super::theme;

/// A single intro slide
#[derive(Debug, Clone, Deserialize)]
pub struct IntroSlide {
    pub text: String,
}

/// Container for loading from TOML
#[derive(Debug, Deserialize)]
pub struct IntroFile {
    pub slides: Vec<IntroSlide>,
}

/// Cinematic intro screen
pub struct IntroScreen {
    slides: Vec<IntroSlide>,
    current_slide: usize,
    chars_revealed: usize,
    animation_done: bool,
    last_char_time: Instant,
}

const TYPEWRITER_DELAY_MS: u64 = 35;

impl IntroScreen {
    pub fn new(slides: Vec<IntroSlide>) -> Self {
        Self {
            slides,
            current_slide: 0,
            chars_revealed: 0,
            animation_done: false,
            last_char_time: Instant::now(),
        }
    }

    /// Load slides from a TOML file
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let file: IntroFile = toml::from_str(toml_str)?;
        Ok(Self::new(file.slides))
    }

    /// Load from scenario directory
    pub fn load_scenario(scenario_path: &str) -> Option<Self> {
        let intro_path = format!("{}/intro.toml", scenario_path);
        let content = std::fs::read_to_string(&intro_path).ok()?;
        Self::from_toml(&content).ok()
    }

    /// Run the intro sequence. Returns true if completed, false if skipped.
    pub fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        music: &MusicController,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.slides.is_empty() {
            return Ok(true);
        }

        // Switch to intro score (starts on slide 0)
        music.switch_to_intro();

        loop {
            // Animate typewriter
            if !self.animation_done {
                let now = Instant::now();
                if now.duration_since(self.last_char_time)
                    >= Duration::from_millis(TYPEWRITER_DELAY_MS)
                {
                    let total_chars = self.slides[self.current_slide].text.len();
                    if self.chars_revealed < total_chars {
                        self.chars_revealed += 1;
                        self.last_char_time = now;

                        // Tick per character (skip whitespace)
                        let ch = self.slides[self.current_slide]
                            .text
                            .chars()
                            .nth(self.chars_revealed - 1);
                        if ch.map_or(false, |c| !c.is_whitespace()) {
                            music.play_typewriter_tick();
                        }
                    } else {
                        self.animation_done = true;
                    }
                }
            }

            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Shift+Space = skip all remaining slides
                    if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::SHIFT)
                    {
                        return Ok(false);
                    }

                    match key.code {
                        KeyCode::Char('m') => {
                            music.toggle_mute();
                        }
                        // → or Enter to advance (only after animation done)
                        KeyCode::Right | KeyCode::Enter => {
                            if !self.animation_done {
                                // Complete current animation instantly
                                self.chars_revealed = self.slides[self.current_slide].text.len();
                                self.animation_done = true;
                            } else {
                                // Next slide
                                self.current_slide += 1;
                                if self.current_slide >= self.slides.len() {
                                    return Ok(true); // All slides shown
                                }
                                self.chars_revealed = 0;
                                self.animation_done = false;
                                self.last_char_time = Instant::now();

                                // Advance intro score to match slide
                                music.advance_slide(self.current_slide);
                            }
                        }
                        KeyCode::Esc => return Ok(false),
                        _ => {}
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

        if self.current_slide >= self.slides.len() {
            return;
        }

        let slide = &self.slides[self.current_slide];
        let revealed_text: String = slide.text.chars().take(self.chars_revealed).collect();

        // Center the text vertically and horizontally
        let text_lines: Vec<&str> = revealed_text.lines().collect();
        let text_height = text_lines.len() as u16;
        let top_margin = area.height.saturating_sub(text_height + 4) / 2;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_margin),
                Constraint::Min(text_height + 2),
                Constraint::Length(2), // footer
            ])
            .split(area);

        // Slide text
        let lines: Vec<Line> = text_lines
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(theme::FG),
                ))
            })
            .collect();

        let content_area = theme::centered_content(layout[1]);
        let text_widget = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(text_widget, content_area);

        // Footer: skip hint (bottom right)
        let hint = if self.animation_done {
            "→ continue"
        } else {
            ""
        };
        let skip_text = "Shift+Space skip";

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(hint, Style::default().fg(theme::FG_DIM)),
            Span::raw("    "),
            Span::styled(skip_text, Style::default().fg(theme::FG_MUTED)),
        ]))
        .alignment(Alignment::Right);
        frame.render_widget(footer, layout[2]);
    }
}
