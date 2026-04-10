use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::HashMap;

use super::chat::{ChatStream, NpcAvatar};
use super::music::MusicController;
use super::theme;
use crate::ai::context::GameContext;
use crate::ai::{AiProvider, DmMode};

/// Character data built up during creation
#[derive(Debug, Clone, Default)]
pub struct CharacterData {
    pub fields: HashMap<String, String>,
}

impl CharacterData {
    pub fn set(&mut self, key: &str, value: &str) {
        self.fields.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    pub fn depth_percent(&self) -> u32 {
        let total_fields = 10; // name, background, archetype, office, party, traits, family, motivation, rival, secret
        let filled = self.fields.len() as u32;
        ((filled as f32 / total_fields as f32) * 100.0).min(100.0) as u32
    }

    pub fn depth_label(&self) -> &str {
        let pct = self.depth_percent();
        if pct >= 80 {
            "Deep Lore"
        } else if pct >= 60 {
            "Detailed"
        } else if pct >= 30 {
            "Forming"
        } else {
            "Basics"
        }
    }

    pub fn can_start(&self) -> bool {
        self.depth_percent() >= 30
    }

    pub fn summary_lines(&self) -> Vec<(String, String, bool)> {
        let keys = [
            "Name",
            "Background",
            "Archetype",
            "Starting Office",
            "Party",
            "Traits",
            "Family",
            "Motivation",
            "Rival",
            "Secret",
        ];
        keys.iter()
            .map(|k| {
                let key_lower = k.to_lowercase().replace(" ", "_");
                let filled = self.fields.contains_key(&key_lower);
                let value = self.fields.get(&key_lower).cloned().unwrap_or_default();
                (k.to_string(), value, filled)
            })
            .collect()
    }
}

/// Available avatar options
/// Head shapes for avatar
const HEAD_OPTIONS: &[(&str, &str, &str)] = &[
    // (left, right, label)
    ("[", "]", "Square"),
    ("(", ")", "Round"),
    ("{", "}", "Curly"),
    ("|", "|", "Pipe"),
    ("⟦", "⟧", "Formal"),
    ("⟨", "⟩", "Sleek"),
    ("╔", "╗", "Rigid"),
    ("▐", "▌", "Solid"),
];

/// Eye options for avatar
const EYE_OPTIONS: &[(&str, &str)] = &[
    ("••", "Alert"),
    ("°°", "Glasses"),
    ("^^", "Friendly"),
    ("──", "Stern"),
    ("¬¬", "Skeptical"),
    ("..", "Quiet"),
    ("**", "Intense"),
    ("@@", "Wide-eyed"),
    (">>", "Determined"),
    ("==", "Squinting"),
    ("~~", "Relaxed"),
    ("oo", "Open"),
    ("--", "Tired"),
    ("::", "Analytical"),
];

/// Color options for avatar
const COLOR_OPTIONS: &[(Color, &str)] = &[
    (Color::Cyan, "Cyan"),
    (Color::LightBlue, "Blue"),
    (Color::Green, "Green"),
    (Color::Yellow, "Gold"),
    (Color::Red, "Red"),
    (Color::Magenta, "Purple"),
    (Color::LightGreen, "Lime"),
    (Color::LightCyan, "Teal"),
    (Color::LightRed, "Coral"),
    (Color::White, "White"),
];

/// Build the avatar string from selections
fn build_avatar(head: usize, eyes: usize) -> String {
    let (left, right, _) = HEAD_OPTIONS[head];
    let (eye_chars, _) = EYE_OPTIONS[eyes];
    format!("{}{}{}", left, eye_chars, right)
}

enum CreationPhase {
    BasicForm, // First/last name + avatar build
    AiChat,    // AI-guided deeper creation
}

/// Character creation screen
pub struct CharacterCreationScreen {
    phase: CreationPhase,
    // Basic form fields — paginated
    form_page: usize,    // 0=first name, 1=last name, 2=design (head/eyes/color)
    design_field: usize, // within page 2: 0=head, 1=eyes, 2=color
    first_name: String,
    last_name: String,
    head_selected: usize,
    eyes_selected: usize,
    color_selected: usize,
    form_input: String,
    frame_count: u64, // for animation timing
    // AI chat phase
    chat: ChatStream,
    input: String,
    character: CharacterData,
    awaiting_response: bool,
    creation_complete: bool,
    dm_question_count: u32,
}

const SYSTEM_PROMPT: &str = r#"You are the dungeon master for POLIT, an American politics simulator. You are helping the player create their character.

Ask questions ONE AT A TIME to build their character. Start with their name, then background, then what kind of politician they want to be.

Keep your responses short (2-3 sentences max). Be conversational and warm.

After each answer, acknowledge it briefly and ask the next question. The fields to fill in order:
1. Name
2. Background (what they did before politics)
3. Archetype (suggest options: Idealist, Machine Politician, Outsider, Dealmaker, Prosecutor, Activist, Veteran, Mogul)
4. Starting office (City Council, School Board, State Legislature, etc.)
5. Party affiliation
6. Two character traits (one positive, one negative)
7. Family situation
8. Core motivation (why politics?)

When the player seems ready or says they want to start, say "Let's begin your story." and nothing else."#;

impl CharacterCreationScreen {
    pub fn new() -> Self {
        Self {
            phase: CreationPhase::BasicForm,
            form_page: 0,
            design_field: 0,
            first_name: String::new(),
            last_name: String::new(),
            head_selected: 0,
            eyes_selected: 0,
            color_selected: 0,
            form_input: String::new(),
            frame_count: 0,
            chat: ChatStream::new(),
            input: String::new(),
            character: CharacterData::default(),
            awaiting_response: false,
            creation_complete: false,
            dm_question_count: 0,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        ai: &mut dyn AiProvider,
        music: &MusicController,
    ) -> Result<Option<CharacterData>, Box<dyn std::error::Error>> {
        // Phase 1: Basic form
        loop {
            match self.phase {
                CreationPhase::BasicForm => {
                    self.draw_form(terminal)?;
                    if self.handle_form_input()? {
                        // Form complete — transition to AI chat
                        let full_name =
                            format!("{} {}", self.first_name.trim(), self.last_name.trim());
                        let avatar_face = build_avatar(self.head_selected, self.eyes_selected);
                        let (avatar_color, _) = COLOR_OPTIONS[self.color_selected];
                        self.character.set("name", &full_name);
                        self.character.set("avatar_face", &avatar_face);
                        self.character
                            .set("avatar_color", &format!("{:?}", avatar_color));

                        // Start AI chat with context
                        let greeting = self.generate_ai_response(
                            ai,
                            &format!(
                                "The player's name is {}. Greet them warmly by name and ask about their background — what did they do before entering politics?",
                                full_name
                            ),
                        );
                        self.chat.add_npc(
                            "DM",
                            &greeting,
                            Some(NpcAvatar {
                                face: "◆◆".to_string(),
                                color: Color::LightYellow,
                                name: "DM".to_string(),
                            }),
                        );
                        self.dm_question_count = 1; // Name already done
                        self.phase = CreationPhase::AiChat;
                    }
                }
                CreationPhase::AiChat => {
                    if self.creation_complete {
                        return Ok(Some(self.character.clone()));
                    }
                    self.draw(terminal)?;
                    self.handle_input(ai, music)?;
                }
            }
        }
    }

    fn animated_avatar(&self) -> String {
        let (left, right, _) = HEAD_OPTIONS[self.head_selected];
        let (eyes, _) = EYE_OPTIONS[self.eyes_selected];
        let cycle = self.frame_count % 90;
        let display_eyes = if cycle >= 86 { "--" } else { eyes };
        format!("{}{}{}", left, display_eyes, right)
    }

    fn breathing_offset(&self) -> u16 {
        0 // Disabled — too much movement for small avatar. Just blink.
    }

    fn draw_form(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.frame_count += 1;
        let page = self.form_page;
        let input = self.form_input.clone();
        let first = self.first_name.clone();
        let last = self.last_name.clone();
        let avatar = self.animated_avatar();
        let (avatar_color, _) = COLOR_OPTIONS[self.color_selected];
        let breath = self.breathing_offset();
        let head_sel = self.head_selected;
        let eyes_sel = self.eyes_selected;
        let color_sel = self.color_selected;
        let design_field = self.design_field;
        let full_name = format!("{} {}", first, last);
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);
            // Fixed layout — avatar breathes within its own space, nothing else moves
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Length(2), // POLIT
                    Constraint::Length(2), // subtitle
                    Constraint::Length(2), // spacer (fixed)
                    Constraint::Length(2), // avatar area (2 lines: 1 for breathing room + 1 for face)
                    Constraint::Length(1), // name
                    Constraint::Length(2), // spacer
                    Constraint::Min(8),    // content
                    Constraint::Length(2), // footer
                ])
                .split(area);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("🇺🇸 ", Style::default()),
                    Span::styled("P O L I T", Style::default().fg(theme::FG).bold()),
                ]))
                .alignment(Alignment::Center),
                layout[1],
            );

            let sub = match page {
                0 => "What's your name?",
                1 => "And your family name?",
                2 => "Design your look",
                _ => "",
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    sub,
                    Style::default().fg(theme::FG_DIM),
                )))
                .alignment(Alignment::Center),
                layout[2],
            );

            if page == 2 {
                // Avatar breathes within its 2-line area by scrolling
                let avatar_text = if breath == 0 {
                    vec![
                        Line::from(Span::styled(
                            &avatar,
                            Style::default().fg(avatar_color).bold(),
                        )),
                        Line::from(""),
                    ]
                } else {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            &avatar,
                            Style::default().fg(avatar_color).bold(),
                        )),
                    ]
                };
                frame.render_widget(
                    Paragraph::new(avatar_text).alignment(Alignment::Center),
                    layout[4],
                );
                let ns = if page == 0 {
                    first.clone()
                } else {
                    full_name.trim().to_string()
                };
                if !ns.is_empty() {
                    frame.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            ns,
                            Style::default().fg(avatar_color),
                        )))
                        .alignment(Alignment::Center),
                        layout[5],
                    );
                }
            }

            match page {
                0 | 1 => {
                    let w = 40u16;
                    let cx = area.x + (area.width.saturating_sub(w)) / 2;
                    let ca = Rect::new(cx, layout[7].y, w, 3);
                    let blk = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::ACCENT_BLUE))
                        .style(Style::default().bg(theme::BG_HIGHLIGHT));
                    let inner = blk.inner(ca);
                    frame.render_widget(blk, ca);
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(theme::ACCENT)),
                            Span::styled(&input, Style::default().fg(theme::FG)),
                            Span::styled(
                                "▊",
                                Style::default()
                                    .fg(theme::FG_DIM)
                                    .add_modifier(Modifier::SLOW_BLINK),
                            ),
                        ])),
                        inner,
                    );
                }
                2 => {
                    let w = 56u16;
                    let cx = area.x + (area.width.saturating_sub(w)) / 2;
                    let ca = Rect::new(cx, layout[7].y, w, 14);
                    let mut lines: Vec<Line> = vec![Line::from("")];
                    let hfg = if design_field == 0 {
                        theme::FG
                    } else {
                        theme::FG_DIM
                    };
                    lines.push(Line::from(Span::styled(
                        if design_field == 0 {
                            "  ▶ Head"
                        } else {
                            "    Head"
                        },
                        Style::default().fg(hfg),
                    )));
                    let mut hs = vec![Span::raw("    ")];
                    for (i, (l, r, _)) in HEAD_OPTIONS.iter().enumerate() {
                        let f = format!("{}••{}", l, r);
                        let s = if i == head_sel {
                            Style::default()
                                .fg(theme::FG)
                                .bold()
                                .bg(theme::BG_HIGHLIGHT)
                        } else {
                            Style::default().fg(theme::FG_MUTED)
                        };
                        hs.push(Span::styled(format!(" {} ", f), s));
                    }
                    lines.push(Line::from(hs));
                    lines.push(Line::from(""));
                    let efg = if design_field == 1 {
                        theme::FG
                    } else {
                        theme::FG_DIM
                    };
                    lines.push(Line::from(Span::styled(
                        if design_field == 1 {
                            "  ▶ Eyes"
                        } else {
                            "    Eyes"
                        },
                        Style::default().fg(efg),
                    )));
                    for rs in (0..EYE_OPTIONS.len()).step_by(7) {
                        let mut es = vec![Span::raw("    ")];
                        for i in rs..(rs + 7).min(EYE_OPTIONS.len()) {
                            let (e, _) = EYE_OPTIONS[i];
                            let s = if i == eyes_sel {
                                Style::default()
                                    .fg(theme::FG)
                                    .bold()
                                    .bg(theme::BG_HIGHLIGHT)
                            } else {
                                Style::default().fg(theme::FG_MUTED)
                            };
                            es.push(Span::styled(format!(" {} ", e), s));
                        }
                        lines.push(Line::from(es));
                    }
                    lines.push(Line::from(""));
                    let cfg = if design_field == 2 {
                        theme::FG
                    } else {
                        theme::FG_DIM
                    };
                    lines.push(Line::from(Span::styled(
                        if design_field == 2 {
                            "  ▶ Color"
                        } else {
                            "    Color"
                        },
                        Style::default().fg(cfg),
                    )));
                    let mut cs = vec![Span::raw("    ")];
                    for (i, (c, _)) in COLOR_OPTIONS.iter().enumerate() {
                        let s = if i == color_sel {
                            Style::default().fg(*c).bg(theme::BG_HIGHLIGHT)
                        } else {
                            Style::default().fg(*c)
                        };
                        cs.push(Span::styled(" ██ ", s));
                    }
                    lines.push(Line::from(cs));
                    lines.push(Line::from("")); // bottom padding
                    let form = Paragraph::new(lines).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(theme::BORDER))
                            .style(Style::default().bg(theme::BG_SUBTLE)),
                    );
                    frame.render_widget(ratatui::widgets::Clear, ca);
                    frame.render_widget(form, ca);
                }
                _ => {}
            }
            let ft = match page {
                0 => "Enter to continue",
                1 => "Enter to continue   Shift+Enter back",
                2 => match design_field {
                    0 => "← → head shape   Enter to lock   Shift+Enter back",
                    1 => "← → eyes   Enter to lock   Shift+Enter back",
                    2 => "← → color   Enter to finish   Shift+Enter back",
                    _ => "",
                },
                _ => "",
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    ft,
                    Style::default().fg(theme::FG_MUTED),
                )))
                .alignment(Alignment::Center),
                layout[8],
            );
        })?;
        Ok(())
    }

    fn handle_form_input(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if crossterm::event::poll(std::time::Duration::from_millis(33))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(false);
                }

                // Shift+Enter goes backward on ALL pages
                if key.code == KeyCode::Enter
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::SHIFT)
                {
                    match self.form_page {
                        0 => {} // Can't go back from first page
                        1 => {
                            self.form_page = 0;
                            self.form_input = self.first_name.clone();
                        }
                        2 => {
                            if self.design_field > 0 {
                                self.design_field -= 1;
                            } else {
                                self.form_page = 1;
                                self.form_input = self.last_name.clone();
                            }
                        }
                        _ => {}
                    }
                    return Ok(false);
                }

                match self.form_page {
                    0 => match key.code {
                        KeyCode::Enter => {
                            if !self.form_input.is_empty() {
                                self.first_name = self.form_input.clone();
                                self.form_input.clear();
                                self.form_page = 1;
                            }
                        }
                        KeyCode::Backspace => {
                            self.form_input.pop();
                        }
                        KeyCode::Char(c) => {
                            self.form_input.push(c);
                        }
                        _ => {}
                    },
                    1 => match key.code {
                        KeyCode::Enter => {
                            if !self.form_input.is_empty() {
                                self.last_name = self.form_input.clone();
                                self.form_input.clear();
                                self.form_page = 2;
                            }
                        }
                        KeyCode::Backspace => {
                            self.form_input.pop();
                        }
                        KeyCode::Char(c) => {
                            self.form_input.push(c);
                        }
                        _ => {}
                    },
                    2 => match key.code {
                        KeyCode::Left => match self.design_field {
                            0 => {
                                if self.head_selected > 0 {
                                    self.head_selected -= 1;
                                }
                            }
                            1 => {
                                if self.eyes_selected > 0 {
                                    self.eyes_selected -= 1;
                                }
                            }
                            2 => {
                                if self.color_selected > 0 {
                                    self.color_selected -= 1;
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Right => match self.design_field {
                            0 => {
                                if self.head_selected < HEAD_OPTIONS.len() - 1 {
                                    self.head_selected += 1;
                                }
                            }
                            1 => {
                                if self.eyes_selected < EYE_OPTIONS.len() - 1 {
                                    self.eyes_selected += 1;
                                }
                            }
                            2 => {
                                if self.color_selected < COLOR_OPTIONS.len() - 1 {
                                    self.color_selected += 1;
                                }
                            }
                            _ => {}
                        },
                        // Enter locks current row and advances to next
                        KeyCode::Enter => {
                            if self.design_field < 2 {
                                self.design_field += 1;
                            } else {
                                return Ok(true); // Color confirmed = form complete
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    fn generate_ai_response(&mut self, ai: &mut dyn AiProvider, user_input: &str) -> String {
        let ctx = GameContext {
            tone_instructions: SYSTEM_PROMPT.to_string(),
            ..GameContext::default()
        };

        // Build prompt with character context
        let char_context = self
            .character
            .fields
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        let full_prompt = if char_context.is_empty() {
            user_input.to_string()
        } else {
            format!(
                "Character so far: [{}]\n\nPlayer says: {}",
                char_context, user_input
            )
        };

        let prompt = ctx.build_prompt(&full_prompt, DmMode::Conversation);
        match ai.generate(&prompt, DmMode::Conversation) {
            Ok(response) => response.narration,
            Err(_) => "Tell me about yourself. What's your name?".to_string(),
        }
    }

    fn parse_and_lock_fields(&mut self, user_input: &str, dm_response: &str) {
        let input_lower = user_input.to_lowercase();
        let q = self.dm_question_count;

        // Simple heuristic: based on question order, lock the field
        match q {
            1 => {
                // First answer is the name
                let name = user_input
                    .split_whitespace()
                    .filter(|w| w.len() > 1)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !name.is_empty() {
                    self.character.set("name", &name);
                }
            }
            2 => {
                self.character.set("background", user_input.trim());
            }
            3 => {
                // Archetype detection
                for archetype in &[
                    "idealist",
                    "machine",
                    "outsider",
                    "dealmaker",
                    "prosecutor",
                    "activist",
                    "veteran",
                    "mogul",
                ] {
                    if input_lower.contains(archetype) {
                        let capitalized =
                            format!("The {}{}", archetype[..1].to_uppercase(), &archetype[1..]);
                        self.character.set("archetype", &capitalized);
                        break;
                    }
                }
                if !self.character.fields.contains_key("archetype") {
                    self.character.set("archetype", user_input.trim());
                }
            }
            4 => {
                self.character.set("starting_office", user_input.trim());
            }
            5 => {
                self.character.set("party", user_input.trim());
            }
            6 => {
                self.character.set("traits", user_input.trim());
            }
            7 => {
                self.character.set("family", user_input.trim());
            }
            8 => {
                self.character.set("motivation", user_input.trim());
            }
            _ => {}
        }

        // Check if DM says we're ready
        let dm_lower = dm_response.to_lowercase();
        if dm_lower.contains("let's begin")
            || dm_lower.contains("let's begin")
            || dm_lower.contains("shall we begin")
        {
            self.creation_complete = true;
        }
    }

    fn draw(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Pre-compute before borrowing chat mutably
        let depth = self.character.depth_percent();
        let depth_label = self.character.depth_label().to_string();
        let can_start = self.character.can_start();
        let input_str = self.input.clone();
        let summary = self.character.summary_lines();

        let input_lines: Vec<&str> = input_str.split('\n').collect();
        let input_height = (input_lines.len() as u16 + 2).max(3).min(10);

        let chat_height = terminal.size()?.height.saturating_sub(input_height + 4);
        let chat_widget = self.chat.render(chat_height);

        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),            // Header (full width)
                    Constraint::Min(3),               // Chat (centered)
                    Constraint::Length(input_height), // Input (dynamic)
                    Constraint::Length(1),            // Bottom margin
                ])
                .split(area);

            // Header — FULL WIDTH, same style as game UI
            let depth_bar_filled = (depth as f32 / 100.0 * 20.0) as usize;
            let depth_bar = format!(
                "{}{}",
                "█".repeat(depth_bar_filled),
                "░".repeat(20 - depth_bar_filled)
            );
            let header = Paragraph::new(Line::from(vec![
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
                if can_start {
                    Span::styled("  → ready", Style::default().fg(theme::SUCCESS))
                } else {
                    Span::raw("")
                },
            ]))
            .style(Style::default().bg(theme::BG_SUBTLE));
            frame.render_widget(header, layout[0]);

            // Chat — centered column
            let chat_area = theme::centered_content(layout[1]);
            frame.render_widget(chat_widget, chat_area);

            // Input — floating card bar, same as game UI
            let input_content_area = theme::centered_content(layout[2]);
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::ACCENT_BLUE))
                .style(Style::default().bg(theme::BG_HIGHLIGHT));
            let inner_area = input_block.inner(input_content_area);
            frame.render_widget(input_block, input_content_area);

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

            // Character summary block (bottom-right of chat area)
            if !summary.is_empty() && summary.iter().any(|(_, _, filled)| *filled) {
                let filled_count = summary.iter().filter(|(_, _, f)| *f).count() as u16;
                let block_height = filled_count + 2;
                let block_width = 42;
                let block_x = chat_area.right().saturating_sub(block_width + 1);
                let block_y = layout[1].bottom().saturating_sub(block_height + 1);
                let block_area = Rect::new(block_x, block_y, block_width, block_height);

                let summary_lines: Vec<Line> = summary
                    .iter()
                    .filter(|(_, _, filled)| *filled)
                    .map(|(key, value, _)| {
                        Line::from(vec![
                            Span::styled("✓ ", Style::default().fg(theme::SUCCESS)),
                            Span::styled(format!("{}: ", key), Style::default().fg(theme::FG_DIM)),
                            Span::styled(value, Style::default().fg(theme::FG)),
                        ])
                    })
                    .collect();

                let summary_block = Paragraph::new(summary_lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::BORDER))
                        .style(Style::default().bg(theme::BG_SUBTLE)),
                );
                frame.render_widget(ratatui::widgets::Clear, block_area);
                frame.render_widget(summary_block, block_area);
            }
        })?;
        Ok(())
    }

    fn handle_input(
        &mut self,
        ai: &mut dyn AiProvider,
        music: &MusicController,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            if let Event::Mouse(mouse) = evt {
                match mouse.kind {
                    crossterm::event::MouseEventKind::ScrollUp => {
                        self.chat.scroll_up_by(3);
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        self.chat.scroll_down_by(3);
                    }
                    _ => {}
                }
                return Ok(());
            }

            if let Event::Key(key) = evt {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.creation_complete = true;
                    }
                    // → to start game (when ready)
                    KeyCode::Right if self.input.is_empty() && self.character.can_start() => {
                        self.creation_complete = true;
                    }
                    // Shift+Enter = newline
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::SHIFT) => {
                        self.input.push('\n');
                    }
                    KeyCode::Enter => {
                        if !self.input.is_empty() {
                            music.play_select();
                            let input = self.input.clone();
                            self.input.clear();
                            self.dm_question_count += 1;

                            // Show player input
                            self.chat.add_player(&input);

                            // Check if player wants to start
                            let input_lower = input.to_lowercase();
                            if (input_lower.contains("ready")
                                || input_lower.contains("begin")
                                || input_lower.contains("start"))
                                && self.character.can_start()
                            {
                                self.chat.add_npc(
                                    "DM",
                                    "Let's begin your story.",
                                    Some(NpcAvatar {
                                        face: "◆◆".to_string(),
                                        color: Color::LightYellow,
                                        name: "DM".to_string(),
                                    }),
                                );
                                self.creation_complete = true;
                                return Ok(());
                            }

                            // Generate AI response
                            let response = self.generate_ai_response(ai, &input);

                            // Parse and lock fields
                            self.parse_and_lock_fields(&input, &response);

                            // Show DM response
                            self.chat.add_npc(
                                "DM",
                                &response,
                                Some(NpcAvatar {
                                    face: "◆◆".to_string(),
                                    color: Color::LightYellow,
                                    name: "DM".to_string(),
                                }),
                            );
                        }
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                    }
                    KeyCode::Char('R') => {
                        self.chat.scroll_to_bottom();
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                        self.chat.scroll_to_bottom();
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
