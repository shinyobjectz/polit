use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::collections::HashMap;

use super::chat::{ChatStream, NpcAvatar};
use super::music::MusicController;
use super::theme;
use crate::ai::agent::Agent;
use crate::ai::async_chat::{AiResponse, AsyncAiChat};
use crate::ai::context::GameContext;
use crate::ai::tools::ToolCall;
use crate::ai::{AiProvider, DmMode};
use crate::state::GameStateFs;

/// Character data — thin wrapper around GameStateFs.
/// All reads/writes go to character.yaml on disk.
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
        let total_fields = 11;
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
            "Tone",
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
    form_page: usize,    // 0=name (first+last), 1=design (head/eyes/color)
    name_field: usize,   // within page 0: 0=first, 1=last
    design_field: usize, // within page 1: 0=head, 1=eyes, 2=color
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
    async_ai: Option<AsyncAiChat>,
    thinking: bool,
    thinking_dots: u8,
    thinking_start: std::time::Instant,
    creation_complete: bool,
    dm_question_count: u32,
    show_sheet: bool,
    save_fs: Option<GameStateFs>, // file-backed state
}

/// Tone instructions for character creation (passed as context.tone_instructions)
const CREATION_TONE: &str = "Sharp, funny, vivid. Like a comedy writer helping someone build a character. Match their energy — weird gets weirder, serious gets deeper. Never generic, never robotic.";

impl CharacterCreationScreen {
    pub fn new() -> Self {
        Self {
            phase: CreationPhase::BasicForm,
            form_page: 0,
            name_field: 0,
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
            async_ai: None,
            thinking: false,
            thinking_dots: 0,
            thinking_start: std::time::Instant::now(),
            creation_complete: false,
            dm_question_count: 0,
            show_sheet: false,
            save_fs: None,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        provider: Box<dyn AiProvider>,
        music: &MusicController,
    ) -> Result<Option<CharacterData>, Box<dyn std::error::Error>> {
        // Spawn async AI thread with an Agent in CharacterCreation mode.
        // The agent owns conversation memory so history accumulates automatically.
        let agent = Agent::new(DmMode::CharacterCreation);
        let mut async_ai = AsyncAiChat::with_agent(provider, agent);

        loop {
            match self.phase {
                CreationPhase::BasicForm => {
                    self.draw_form(terminal)?;
                    if self.handle_form_input()? {
                        let full_name =
                            format!("{} {}", self.first_name.trim(), self.last_name.trim());
                        let avatar_face = build_avatar(self.head_selected, self.eyes_selected);
                        let (avatar_color, _) = COLOR_OPTIONS[self.color_selected];
                        self.character.set("name", &full_name);
                        self.character.set("avatar_face", &avatar_face);
                        self.character
                            .set("avatar_color", &format!("{:?}", avatar_color));

                        // Initialize file-backed save state
                        if let Some(home) = std::env::var_os("HOME") {
                            let save_path = std::path::PathBuf::from(home)
                                .join(".polit/saves/current");
                            if let Ok(fs) = GameStateFs::open(&save_path) {
                                // Write initial character data
                                let mut char_file = fs.read_character();
                                char_file.name = full_name.clone();
                                char_file.avatar_face = avatar_face.clone();
                                char_file.avatar_color = format!("{:?}", avatar_color);
                                let _ = fs.write_character(&char_file);
                                let _ = fs.write_world(&crate::state::gamestate_fs::WorldFile::default());
                                self.save_fs = Some(fs);
                                tracing::info!("Save state initialized at {:?}", save_path);
                            }
                        }

                        // Request AI greeting via the agent (with memory + tools)
                        let ctx = self.build_creation_context();
                        async_ai.request_agent_turn(
                            &format!(
                                "My character's name is {}. Let's figure out who they are.",
                                full_name
                            ),
                            ctx,
                            DmMode::CharacterCreation,
                        );
                        self.thinking = true;
                        self.thinking_start = std::time::Instant::now();
                        self.dm_question_count = 1;
                        self.show_sheet = true; // Show character card by default
                        self.phase = CreationPhase::AiChat;
                    }
                }
                CreationPhase::AiChat => {
                    if self.creation_complete {
                        async_ai.shutdown();
                        return Ok(Some(self.character.clone()));
                    }

                    // Poll for AI responses (non-blocking)
                    if self.thinking {
                        self.thinking_dots = ((self.frame_count / 10) % 4) as u8;
                        // Drain all available responses (steps + final)
                        while let Some(resp) = async_ai.poll_response() {
                            match resp {
                                AiResponse::Step(step) => {
                                    use crate::ai::async_chat::AgentStep;
                                    match step {
                                        AgentStep::Thinking(thought) => {
                                            // Show thinking as dimmed line
                                            let short = if thought.len() > 80 {
                                                format!("{}...", &thought[..77])
                                            } else {
                                                thought
                                            };
                                            self.chat.add_system(&format!("  ◇ {}", short));
                                        }
                                        AgentStep::ToolExecuted(tool_desc) => {
                                            self.chat.add_system(&format!("  ✓ {}", tool_desc));
                                        }
                                        AgentStep::Generating(iter) => {
                                            if iter > 1 {
                                                self.chat.add_system(&format!("  ↻ step {}", iter));
                                            }
                                        }
                                    }
                                }
                                AiResponse::AgentDone(agent_resp) => {
                                    self.thinking = false;

                                    // Process tool calls from the agent
                                    self.process_agent_tools(&agent_resp.tool_calls);

                                    // Show narration in chat
                                    if !agent_resp.narration.is_empty() {
                                        self.chat.add_npc(
                                            "Narrator",
                                            &agent_resp.narration,
                                            Some(NpcAvatar {
                                                face: "✦✦".to_string(),
                                                color: theme::ACCENT,
                                                name: "Narrator".to_string(),
                                            }),
                                        );
                                    }

                                    tracing::info!(
                                        "Agent: {} iterations, {} tools, depth={}%",
                                        agent_resp.iterations,
                                        agent_resp.tool_calls.len(),
                                        self.character.depth_percent()
                                    );
                                }
                                AiResponse::Done(dm_resp) => {
                                    self.thinking = false;
                                    if !dm_resp.narration.is_empty() {
                                        self.chat.add_npc(
                                            "Narrator",
                                            &dm_resp.narration,
                                            Some(NpcAvatar {
                                                face: "✦✦".to_string(),
                                                color: theme::ACCENT,
                                                name: "Narrator".to_string(),
                                            }),
                                        );
                                    }
                                }
                                AiResponse::Error(e) => {
                                    self.thinking = false;
                                    self.chat.add_system(&format!("(AI error: {})", e));
                                }
                                _ => {}
                            }
                        }
                    }

                    self.frame_count += 1;
                    self.draw(terminal)?;
                    self.handle_chat_input(&mut async_ai, music)?;
                }
            }
        }
    }

    /// Build a GameContext for character creation (no game state, just player info)
    fn build_creation_context(&self) -> GameContext {
        let char_summary = self
            .character
            .fields
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        GameContext {
            tone_instructions: CREATION_TONE.to_string(),
            player_name: self
                .character
                .get("name")
                .unwrap_or("Player")
                .to_string(),
            player_office: if char_summary.is_empty() {
                "Unknown — being created".into()
            } else {
                format!("Creating character: [{}]", char_summary)
            },
            phase: "Character Creation".into(),
            ..GameContext::default()
        }
    }

    /// Process tool calls from the agent response
    fn process_agent_tools(&mut self, tools: &[ToolCall]) {
        let mut seen_fields = std::collections::HashSet::new();
        for tool in tools {
            match tool {
                ToolCall::LockField { field, value } => {
                    let clean_field = field.trim_matches('"').trim().to_lowercase().replace(' ', "_");
                    let clean_value = value.trim_matches('"').trim().to_string();

                    // Name is already set from the form — don't let the model change it
                    if clean_field == "name" {
                        tracing::info!("Skipping lock_field for 'name' — already set from form");
                        continue;
                    }
                    let is_update = self.character.fields.contains_key(&clean_field);
                    if is_update {
                        let existing = self.character.get(&clean_field).unwrap().to_string();
                        if existing == clean_value || clean_value.is_empty() {
                            tracing::info!("Skipping duplicate lock_field: '{}'", clean_field);
                        } else {
                            let merged = format!("{}; {}", existing, clean_value);
                            self.character.set(&clean_field, &merged);
                            // Sync to disk
                            if let Some(ref fs) = self.save_fs {
                                let _ = fs.append_character_field(&clean_field, &clean_value);
                            }
                            tracing::info!("Appending to field: '{}' += '{}'", clean_field, clean_value);
                            if seen_fields.insert(clean_field.clone()) {
                                self.chat.add_system(&format!("✓ adding to {}", clean_field));
                            }
                        }
                    } else {
                        self.character.set(&clean_field, &clean_value);
                        // Sync to disk
                        if let Some(ref fs) = self.save_fs {
                            let _ = fs.set_character_field(&clean_field, &clean_value);
                            // Also write tone.yaml when tone is set
                            if clean_field == "tone" {
                                let _ = fs.write_tone(&crate::state::gamestate_fs::ToneFile {
                                    style: clean_value.clone(),
                                    description: String::new(),
                                });
                            }
                        }
                        tracing::info!("Locking field: '{}' = '{}'", clean_field, clean_value);
                        if seen_fields.insert(clean_field.clone()) {
                            self.chat.add_system(&format!("✓ {} set", clean_field));
                        }
                    }
                }
                ToolCall::SuggestOptions {
                    field,
                    options,
                    prompt,
                } => {
                    // Show options to player in chat
                    let opts_text = options
                        .iter()
                        .enumerate()
                        .map(|(i, o)| format!("  {}. {}", i + 1, o))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.chat
                        .add_system(&format!("{}\n{}", prompt, opts_text));
                }
                ToolCall::AskQuestion { topic, question } => {
                    // The question is typically embedded in the narration,
                    // but log it for debugging
                    tracing::info!("Agent asking about {}: {}", topic, question);
                }
                _ => {
                    tracing::warn!("Unexpected tool in character creation: {:?}", tool);
                }
            }
        }

        // Check if character is complete enough
        let dm_lower = self
            .chat
            .messages
            .last()
            .map(|m| m.text.to_lowercase())
            .unwrap_or_default();
        if dm_lower.contains("shall we begin") || dm_lower.contains("story is ready") {
            self.creation_complete = true;
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
                0 => "Name your character",
                1 => "Design your look",
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

            if page == 1 {
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

            let name_field = self.name_field;
            match page {
                // Page 0: First + Last name on same card
                0 => {
                    let w = 44u16;
                    let cx = area.x + (area.width.saturating_sub(w)) / 2;
                    let ca = Rect::new(cx, layout[7].y, w, 8);
                    let blk = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::BORDER))
                        .style(Style::default().bg(theme::BG_SUBTLE));
                    let inner = blk.inner(ca);
                    frame.render_widget(blk, ca);

                    let mut name_lines: Vec<Line> = vec![Line::from("")];

                    // First name
                    let first_active = name_field == 0;
                    name_lines.push(Line::from(Span::styled(
                        "  First Name",
                        Style::default().fg(if first_active {
                            theme::FG
                        } else {
                            theme::FG_DIM
                        }),
                    )));
                    if first_active {
                        name_lines.push(Line::from(vec![
                            Span::styled("  ▶ ", Style::default().fg(theme::ACCENT)),
                            Span::styled(&input, Style::default().fg(theme::FG)),
                            Span::styled(
                                "▊",
                                Style::default()
                                    .fg(theme::FG_DIM)
                                    .add_modifier(Modifier::SLOW_BLINK),
                            ),
                        ]));
                    } else {
                        let v = if first.is_empty() { "..." } else { &first };
                        name_lines.push(Line::from(Span::styled(
                            format!("    {}", v),
                            Style::default().fg(theme::FG_DIM),
                        )));
                    }
                    name_lines.push(Line::from(""));

                    // Last name
                    let last_active = name_field == 1;
                    name_lines.push(Line::from(Span::styled(
                        "  Last Name",
                        Style::default().fg(if last_active {
                            theme::FG
                        } else {
                            theme::FG_DIM
                        }),
                    )));
                    if last_active {
                        name_lines.push(Line::from(vec![
                            Span::styled("  ▶ ", Style::default().fg(theme::ACCENT)),
                            Span::styled(&input, Style::default().fg(theme::FG)),
                            Span::styled(
                                "▊",
                                Style::default()
                                    .fg(theme::FG_DIM)
                                    .add_modifier(Modifier::SLOW_BLINK),
                            ),
                        ]));
                    } else {
                        let v = if last.is_empty() { "..." } else { &last };
                        name_lines.push(Line::from(Span::styled(
                            format!("    {}", v),
                            Style::default().fg(theme::FG_DIM),
                        )));
                    }

                    frame.render_widget(Paragraph::new(name_lines), inner);
                }
                // Page 1: Design
                1 => {
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
                0 => "Enter to confirm   Shift+Enter to go back",
                1 => match design_field {
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

                // Shift+Enter goes backward
                if key.code == KeyCode::Enter
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::SHIFT)
                {
                    match self.form_page {
                        0 => {
                            // Go back within name fields
                            if self.name_field > 0 {
                                self.name_field -= 1;
                                self.form_input = self.first_name.clone();
                            }
                        }
                        1 => {
                            if self.design_field > 0 {
                                self.design_field -= 1;
                            } else {
                                self.form_page = 0;
                                self.name_field = 1;
                                self.form_input = self.last_name.clone();
                            }
                        }
                        _ => {}
                    }
                    return Ok(false);
                }

                match self.form_page {
                    // Page 0: Name (first + last on same page)
                    0 => match key.code {
                        KeyCode::Enter => {
                            if !self.form_input.is_empty() {
                                if self.name_field == 0 {
                                    self.first_name = self.form_input.clone();
                                    self.form_input.clear();
                                    self.name_field = 1;
                                } else {
                                    self.last_name = self.form_input.clone();
                                    self.form_input.clear();
                                    self.form_page = 1; // → Design page
                                }
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
                    // Page 1: Design (head/eyes/color)
                    1 => match key.code {
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

    // parse_and_lock_fields removed — the agent now uses LockField tool calls
    // to set character data. No more heuristic guessing.

    fn draw(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Show thinking indicator
        let thinking = self.thinking;
        let dots = ".".repeat((self.thinking_dots as usize % 3) + 1);

        // Pre-compute before borrowing chat mutably
        let depth = self.character.depth_percent();
        let depth_label = self.character.depth_label().to_string();
        let can_start = self.character.can_start();
        let input_str = self.input.clone();
        let summary = self.character.summary_lines();

        // Use component for proper wrap-aware height
        let input_height = super::components::input_bar::height_for(&input_str, theme::MAX_CONTENT_WIDTH);

        // Loading indicator — just dots + elapsed, no "thinking" label
        if thinking {
            if self
                .chat
                .messages
                .last()
                .map(|m| m.text.contains("generating (") || m.text.starts_with("✦"))
                .unwrap_or(false)
            {
                self.chat.messages.pop();
            }
            let elapsed = self.thinking_start.elapsed().as_secs();
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spin_char = spinner[(self.frame_count as usize / 5) % spinner.len()];
            self.chat
                .add_system(&format!("{} generating ({}s)", spin_char, elapsed));
        } else {
            // Remove loading indicator when done
            if self
                .chat
                .messages
                .last()
                .map(|m| m.text.contains("generating (") || m.text.starts_with("✦"))
                .unwrap_or(false)
            {
                self.chat.messages.pop();
            }
        }

        let chat_height = terminal.size()?.height.saturating_sub(input_height + 4);
        let chat_widget = self.chat.render(chat_height);

        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),               // Chat
                    Constraint::Length(input_height),  // Input
                    Constraint::Length(1),             // Gap
                    Constraint::Length(2),             // Footer
                ])
                .split(area);

            // Footer status bar (component)
            crate::ui::components::status_bar::render_creation(
                frame, layout[3], depth, &depth_label, can_start,
            );


            // Chat — centered column (layout[0] now)
            let chat_area = theme::centered_content(layout[0]);
            frame.render_widget(chat_widget, chat_area);

            // Input — floating card bar with wrapping (layout[1])
            let input_content_area = theme::centered_content(layout[1]);
            crate::ui::components::input_bar::render(frame, input_content_area, &input_str);

            // Character sheet — floating overlay on chat area (Tab to toggle)
            let show_sheet = self.show_sheet;
            if show_sheet && !summary.is_empty() && summary.iter().any(|(_, _, filled)| *filled) {
                let filled_count = summary.iter().filter(|(_, _, f)| *f).count() as u16;
                let block_height = (filled_count + 3).min(chat_area.height);
                let block_width = 40u16.min(area.width.saturating_sub(4));

                // Center the sheet overlay in the chat area
                let block_x = area.x + (area.width.saturating_sub(block_width)) / 2;
                let block_y = chat_area.y + 1;
                let block_area = Rect::new(block_x, block_y, block_width, block_height);

                let summary_lines: Vec<Line> = summary
                    .iter()
                    .filter(|(_, _, filled)| *filled)
                    .map(|(key, value, _)| {
                        let display_val = if value.len() > 26 {
                            format!("{}…", &value[..25])
                        } else {
                            value.clone()
                        };
                        Line::from(vec![
                            Span::styled("✓ ", Style::default().fg(theme::SUCCESS)),
                            Span::styled(
                                format!("{}: ", key),
                                Style::default().fg(theme::FG_DIM),
                            ),
                            Span::styled(display_val, Style::default().fg(theme::FG)),
                        ])
                    })
                    .collect();

                let summary_block = Paragraph::new(summary_lines).block(
                    Block::default()
                        .title(" Character [Tab to close] ")
                        .title_style(Style::default().fg(theme::FG_MUTED))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::ACCENT_BLUE))
                        .style(Style::default().bg(theme::BG)),
                );
                frame.render_widget(ratatui::widgets::Clear, block_area);
                frame.render_widget(summary_block, block_area);
            }
        })?;
        Ok(())
    }

    fn handle_chat_input(
        &mut self,
        async_ai: &mut AsyncAiChat,
        music: &MusicController,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            if let Event::Mouse(mouse) = evt {
                match mouse.kind {
                    crossterm::event::MouseEventKind::ScrollUp => {
                        tracing::debug!("Mouse scroll up, scroll_up={}, total={}", self.chat.scroll_up, self.chat.total_lines());
                        self.chat.scroll_up_by(3);
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        tracing::debug!("Mouse scroll down, scroll_up={}", self.chat.scroll_up);
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

                // Allow scroll and Ctrl+C even while thinking
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.creation_complete = true;
                        return Ok(());
                    }
                    KeyCode::Up => {
                        self.chat.scroll_up_by(3);
                        return Ok(());
                    }
                    KeyCode::Down => {
                        self.chat.scroll_down_by(3);
                        return Ok(());
                    }
                    KeyCode::Char('R') => {
                        self.chat.scroll_to_bottom();
                        return Ok(());
                    }
                    _ => {}
                }

                // Block text input while AI is thinking
                if self.thinking {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Right if self.input.is_empty() && self.character.can_start() => {
                        self.creation_complete = true;
                    }
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::SHIFT) => {
                        self.input.push('\n');
                    }
                    KeyCode::Enter => {
                        if !self.input.is_empty() {
                            music.play_select();
                            let input = self.input.clone();
                            self.input.clear();
                            self.dm_question_count += 1;

                            // Show player message with their avatar
                            let player_face = build_avatar(self.head_selected, self.eyes_selected);
                            let (player_color, _) = COLOR_OPTIONS[self.color_selected];
                            let player_name =
                                self.character.get("name").unwrap_or("Player").to_string();
                            self.chat.add_player_with_avatar(
                                &input,
                                NpcAvatar {
                                    face: player_face,
                                    color: player_color,
                                    name: player_name,
                                },
                            );

                            // Check if player wants to start
                            let input_lower = input.to_lowercase();
                            if (input_lower.contains("ready")
                                || input_lower.contains("begin")
                                || input_lower.contains("start"))
                                && self.character.can_start()
                            {
                                self.chat.add_npc(
                                    "Narrator",
                                    "Let's begin your story.",
                                    Some(NpcAvatar {
                                        face: "✦✦".to_string(),
                                        color: theme::ACCENT,
                                        name: "Narrator".to_string(),
                                    }),
                                );
                                self.creation_complete = true;
                                return Ok(());
                            }

                            // Send to agent — it handles memory, context, and tool calls
                            let ctx = self.build_creation_context();
                            async_ai.request_agent_turn(
                                &input,
                                ctx,
                                DmMode::CharacterCreation,
                            );
                            self.thinking = true;
                            self.thinking_start = std::time::Instant::now();
                        }
                    }
                    KeyCode::Tab => {
                        self.show_sheet = !self.show_sheet;
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
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
