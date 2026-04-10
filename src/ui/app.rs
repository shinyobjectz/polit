use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::chat::ChatStream;
use crate::engine::channels::{UiChannels, UiCommand, UiMessage};

/// Main application state (UI thread only)
pub struct App {
    pub chat: ChatStream,
    pub input: String,
    pub should_quit: bool,
    pub overlay: Option<Overlay>,
    pub channels: UiChannels,
    // Status bar state (updated via messages from game thread)
    pub week: u32,
    pub year: u32,
    pub phase: String,
    pub ap_current: i32,
    pub ap_max: i32,
}

/// Floating overlay types
#[derive(Debug, Clone)]
pub enum Overlay {
    CommandPalette,
    Relationships,
    Deck,
    Map,
    Laws,
    News,
    Staff,
    Intel,
    Economy,
    Help,
}

impl App {
    pub fn new(channels: UiChannels) -> Self {
        Self {
            chat: ChatStream::new(),
            input: String::new(),
            should_quit: false,
            overlay: None,
            channels,
            week: 1,
            year: 2024,
            phase: "Starting".into(),
            ap_current: 5,
            ap_max: 5,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while !self.should_quit {
            // Drain messages from game thread (non-blocking)
            self.process_game_messages();

            // Render
            terminal.draw(|frame| self.render(frame))?;

            // Handle keyboard input (with short poll so we stay responsive)
            self.handle_input()?;
        }
        Ok(())
    }

    fn process_game_messages(&mut self) {
        for msg in self.channels.drain_messages() {
            match msg {
                UiMessage::Narrate(text) => self.chat.add_narration(&text),
                UiMessage::NpcDialogue { name, text } => self.chat.add_npc(&name, &text),
                UiMessage::System(text) => self.chat.add_system(&text),
                UiMessage::Warning(text) => self.chat.add_warning(&text),
                UiMessage::Success(text) => self.chat.add_success(&text),
                UiMessage::DiceRoll(text) => self.chat.add_dice(&text),
                UiMessage::PhaseHeader(text) => self.chat.add_phase_header(&text),
                UiMessage::StatusUpdate {
                    week,
                    year,
                    phase,
                    ap_current,
                    ap_max,
                } => {
                    self.week = week;
                    self.year = year;
                    self.phase = phase;
                    self.ap_current = ap_current;
                    self.ap_max = ap_max;
                }
                UiMessage::Event(_) => {
                    // Will be used by overlay systems in later phases
                }
                UiMessage::Shutdown => {
                    self.should_quit = true;
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status bar
                Constraint::Min(5),    // Chat stream
                Constraint::Length(3), // Input
            ])
            .split(area);

        // Status bar
        frame.render_widget(self.render_status_bar(), layout[0]);

        // Chat stream (auto-scroll to bottom)
        let chat_widget = self.chat.render(layout[1].height);
        frame.render_widget(chat_widget, layout[1]);

        // Input line
        let phase_hint = match self.phase.as_str() {
            "Action" => {
                if self.ap_current > 0 {
                    format!("[AP: {}/{}]", self.ap_current, self.ap_max)
                } else {
                    "[AP: 0 — /end to advance]".into()
                }
            }
            "Dawn" => "[Briefing...]".into(),
            _ => format!("[{}]", self.phase),
        };

        let input_block = Block::default().borders(Borders::TOP);
        let input_text = Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::raw(&self.input),
            Span::styled(
                "▊",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::raw("  "),
            Span::styled(phase_hint, Style::default().fg(Color::DarkGray)),
        ]))
        .block(input_block);
        frame.render_widget(input_text, layout[2]);

        // Overlay
        if let Some(ref overlay) = self.overlay {
            self.render_overlay(frame, overlay, area);
        }
    }

    fn render_status_bar(&self) -> Paragraph<'_> {
        let filled = "█".repeat(self.ap_current.max(0) as usize);
        let empty = "░".repeat((self.ap_max - self.ap_current).max(0) as usize);
        let ap_bar = format!("{}{}", filled, empty);

        Paragraph::new(Line::from(vec![
            Span::styled(
                " POLIT ",
                Style::default().fg(Color::Black).bg(Color::White).bold(),
            ),
            Span::styled(
                format!(
                    " │ Week {}, {} │ {} │ AP: {} {}/{} │ ",
                    self.week, self.year, self.phase, ap_bar, self.ap_current, self.ap_max
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled("[Tab] Menu", Style::default().fg(Color::DarkGray)),
        ]))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)))
    }

    fn render_overlay(&self, frame: &mut Frame, overlay: &Overlay, area: Rect) {
        let overlay_area = centered_rect(50, 70, area);

        let (title, items) = match overlay {
            Overlay::CommandPalette => (
                "≡ COMMAND PALETTE",
                vec![
                    "",
                    "  /meet <npc>      Meet with an NPC (2 AP)",
                    "  /speech <topic>  Give a speech (1 AP)",
                    "  /draft           Draft legislation",
                    "  /end             End turn",
                    "",
                    "  /cards           View your deck",
                    "  /map             View map",
                    "  /news            News archive",
                    "  /stats           Economic dashboard",
                    "  /staff           Staff management",
                    "  /intel           Intelligence briefing",
                    "",
                    "  /save [name]     Save game",
                    "  /load <name>     Load game",
                    "  /help            Full help",
                    "  /quit            Quit game",
                    "",
                    "  [Esc] Close   [Tab] Toggle",
                ],
            ),
            Overlay::Help => (
                "HELP",
                vec![
                    "",
                    "  POLIT — The American Politics Simulator",
                    "",
                    "  You are a newly elected city council member.",
                    "  Each week, you get Action Points (AP) to spend",
                    "  on meetings, speeches, legislation, and more.",
                    "",
                    "  Type freely to speak or act.",
                    "  Use /commands for specific actions.",
                    "  Press Tab for the command palette.",
                    "",
                    "  The AI Dungeon Master will respond to your",
                    "  actions and narrate the consequences.",
                    "  (AI integration coming in Phase 2)",
                    "",
                    "  [Esc] Close",
                ],
            ),
            Overlay::Deck => (
                "🃏 CARDS & DECK",
                vec![
                    "",
                    "  Your deck is empty.",
                    "  Cards are acquired through gameplay —",
                    "  win negotiations, pass bills, build relationships.",
                    "",
                    "  Card types:",
                    "    [T] Tactic  — actions you can take",
                    "    [A] Asset   — resources you hold",
                    "    [P] Position — what you stand for",
                    "",
                    "  (Card system coming in Phase 4)",
                    "",
                    "  [Esc] Close",
                ],
            ),
            Overlay::Map => (
                "🗺  MAP",
                vec![
                    "",
                    "       ┌───────────────────────┐",
                    "       │    SPRINGFIELD         │",
                    "       │                        │",
                    "       │   [D1]  [D2]  [D3]    │",
                    "       │   Urban Suburb Rural   │",
                    "       │                        │",
                    "       │   [D4]  [D5]           │",
                    "       │   College Industrial   │",
                    "       │                        │",
                    "       └───────────────────────┘",
                    "",
                    "  You represent District 1 (Urban)",
                    "  Population: ~45,000",
                    "",
                    "  (Full map coming in Phase 7)",
                    "",
                    "  [Esc] Close",
                ],
            ),
            _ => {
                let name = format!("{:?}", overlay);
                (
                    name.leak() as &str,
                    vec![
                        "",
                        "  (Content coming in later phases)",
                        "",
                        "  [Esc] Close",
                    ],
                )
            }
        };

        let text: Vec<Line> = items.iter().map(|s| Line::from(*s)).collect();
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(15, 15, 25)));

        frame.render_widget(Clear, overlay_area);
        frame.render_widget(Paragraph::new(text).block(block), overlay_area);
    }

    fn handle_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.channels.send(UiCommand::Quit);
                        self.should_quit = true;
                    }
                    KeyCode::Esc => {
                        if self.overlay.is_some() {
                            self.overlay = None;
                        }
                    }
                    KeyCode::Tab => {
                        self.overlay = if self.overlay.is_some() {
                            None
                        } else {
                            Some(Overlay::CommandPalette)
                        };
                    }
                    KeyCode::Enter => {
                        if !self.input.is_empty() {
                            let input = self.input.clone();
                            self.input.clear();
                            self.process_input(&input);
                        }
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn process_input(&mut self, input: &str) {
        if input.starts_with('/') {
            let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
            let cmd = parts[0].to_string();
            let args: Vec<String> = if parts.len() > 1 {
                parts[1].split_whitespace().map(|s| s.to_string()).collect()
            } else {
                vec![]
            };

            // Handle UI-only commands locally
            match cmd.as_str() {
                "help" => {
                    self.overlay = Some(Overlay::Help);
                    return;
                }
                "cards" => {
                    self.overlay = Some(Overlay::Deck);
                    return;
                }
                "map" => {
                    self.overlay = Some(Overlay::Map);
                    return;
                }
                "news" => {
                    self.overlay = Some(Overlay::News);
                    return;
                }
                "stats" => {
                    self.overlay = Some(Overlay::Economy);
                    return;
                }
                "staff" => {
                    self.overlay = Some(Overlay::Staff);
                    return;
                }
                "intel" => {
                    self.overlay = Some(Overlay::Intel);
                    return;
                }
                "quit" => {
                    self.channels.send(UiCommand::Quit);
                    self.should_quit = true;
                    return;
                }
                "end" => {
                    self.channels.send(UiCommand::EndTurn);
                    return;
                }
                "save" => {
                    let name = if args.is_empty() {
                        format!("manual_w{}_y{}", self.week, self.year)
                    } else {
                        args.join("_")
                    };
                    self.channels.send(UiCommand::SaveGame(name));
                    return;
                }
                "load" => {
                    if args.is_empty() {
                        self.chat.add_system("Usage: /load <save_name>");
                    } else {
                        self.channels.send(UiCommand::LoadGame(args.join(" ")));
                    }
                    return;
                }
                _ => {
                    // Send to game thread for handling
                    self.channels.send(UiCommand::SlashCommand { cmd, args });
                }
            }
        } else {
            // Free text → game thread
            self.channels
                .send(UiCommand::PlayerInput(input.to_string()));
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
