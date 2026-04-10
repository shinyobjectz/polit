use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::HashMap;

use super::chat::{ChatStream, NpcAvatar};
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

/// Character creation screen
pub struct CharacterCreationScreen {
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
        let mut chat = ChatStream::new();
        chat.add_system("Character Creation");
        Self {
            chat,
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
    ) -> Result<Option<CharacterData>, Box<dyn std::error::Error>> {
        // Initial DM greeting
        let greeting = self.generate_ai_response(
            ai,
            "Start the character creation. Ask for the player's name.",
        );
        self.chat.add_npc(
            "Dungeon Master",
            &greeting,
            Some(NpcAvatar {
                face: "◆◆".to_string(),
                color: Color::LightYellow,
                name: "Dungeon Master".to_string(),
            }),
        );

        loop {
            if self.creation_complete {
                return Ok(Some(self.character.clone()));
            }

            self.draw(terminal)?;
            self.handle_input(ai)?;
        }
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
        let chat_height = terminal.size()?.height.saturating_sub(5);
        let chat_widget = self.chat.render(chat_height);
        let input_str = self.input.clone();
        let depth = self.character.depth_percent();
        let depth_label = self.character.depth_label().to_string();
        let can_start = self.character.can_start();
        let summary = self.character.summary_lines();

        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Header
                    Constraint::Min(5),    // Chat
                    Constraint::Length(2), // Input
                ])
                .split(area);

            // Header: "Character Creation" + depth meter
            let depth_bar_filled = (depth as f32 / 100.0 * 20.0) as usize;
            let depth_bar = format!(
                "{}{}",
                "█".repeat(depth_bar_filled),
                "░".repeat(20 - depth_bar_filled)
            );
            let header = Paragraph::new(Line::from(vec![
                Span::styled("Character Creation", Style::default().fg(theme::FG).bold()),
                Span::raw("  "),
                Span::styled(
                    format!("{} {} {}%", depth_label, depth_bar, depth),
                    Style::default().fg(if depth >= 30 {
                        theme::SUCCESS
                    } else {
                        theme::FG_DIM
                    }),
                ),
                if can_start {
                    Span::styled("  → ready to begin", Style::default().fg(theme::SUCCESS))
                } else {
                    Span::raw("")
                },
            ]))
            .style(Style::default().bg(theme::BG));

            let header_area = theme::centered_content(layout[0]);
            frame.render_widget(header, header_area);

            // Chat
            let chat_area = theme::centered_content(layout[1]);
            frame.render_widget(chat_widget, chat_area);

            // Input
            let input_area = theme::centered_content(layout[2]);
            let input_widget = Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme::PLAYER_INPUT)),
                Span::styled(&input_str, Style::default().fg(theme::FG)),
                Span::styled(
                    "▊",
                    Style::default()
                        .fg(theme::FG)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]))
            .style(Style::default().bg(theme::BG));
            frame.render_widget(input_widget, input_area);

            // Character summary block (inline in chat area, bottom-right)
            if !summary.is_empty() && summary.iter().any(|(_, _, filled)| *filled) {
                let block_height = summary.iter().filter(|(_, _, f)| *f).count() as u16 + 2;
                let block_width = 40;
                let block_x = chat_area.right().saturating_sub(block_width + 1);
                let block_y = layout[1].bottom().saturating_sub(block_height + 1);
                let block_area = Rect::new(block_x, block_y, block_width, block_height);

                let lines: Vec<Line> = summary
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

                let summary_block = Paragraph::new(lines).block(
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

    fn handle_input(&mut self, ai: &mut dyn AiProvider) -> Result<(), Box<dyn std::error::Error>> {
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
                    KeyCode::Enter => {
                        if !self.input.is_empty() {
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
                                    "Dungeon Master",
                                    "Let's begin your story.",
                                    Some(NpcAvatar {
                                        face: "◆◆".to_string(),
                                        color: Color::LightYellow,
                                        name: "Dungeon Master".to_string(),
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
                                "Dungeon Master",
                                &response,
                                Some(NpcAvatar {
                                    face: "◆◆".to_string(),
                                    color: Color::LightYellow,
                                    name: "Dungeon Master".to_string(),
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
