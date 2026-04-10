use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::chat::ChatStream;
use crate::engine::channels::{UiChannels, UiCommand, UiMessage};

/// A card as displayed in the UI (lightweight view, no gameplay logic)
#[derive(Debug, Clone)]
pub struct CardView {
    pub id: String,
    pub name: String,
    pub card_type: String, // "tactic", "asset", "position"
    pub category: String,
    pub rarity: String,
    pub description: String,
    pub play_count: u32,
}

/// Main application state (UI thread only)
pub struct App {
    pub chat: ChatStream,
    pub input: String,
    pub should_quit: bool,
    pub overlay: Option<Overlay>,
    pub channels: UiChannels,
    // Player's deck (for display in overlay)
    pub deck: Vec<CardView>,
    pub coherence_label: String,
    pub coherence_score: i32,
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
            deck: vec![
                CardView {
                    id: "stump_speech".into(),
                    name: "Stump Speech".into(),
                    card_type: "tactic".into(),
                    category: "campaign".into(),
                    rarity: "common".into(),
                    description: "A basic public address.".into(),
                    play_count: 0,
                },
                CardView {
                    id: "grassroots_support".into(),
                    name: "Grassroots Support".into(),
                    card_type: "asset".into(),
                    category: "organization".into(),
                    rarity: "common".into(),
                    description: "Community volunteers backing you.".into(),
                    play_count: 0,
                },
                CardView {
                    id: "pro_environment".into(),
                    name: "Pro-Environment".into(),
                    card_type: "position".into(),
                    category: "wedge_issue".into(),
                    rarity: "common".into(),
                    description: "You stand for environmental protection.".into(),
                    play_count: 0,
                },
                CardView {
                    id: "transparency".into(),
                    name: "Transparency".into(),
                    card_type: "position".into(),
                    category: "governance".into(),
                    rarity: "common".into(),
                    description: "You believe in open government.".into(),
                    play_count: 0,
                },
            ],
            coherence_label: "Pragmatist".into(),
            coherence_score: 0,
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
            self.draw(terminal)?;

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

    fn draw(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Pre-compute values needed by the render closure
        let status_bar = self.build_status_bar();
        let chat_height = terminal.size()?.height.saturating_sub(4);
        let chat_widget = self.chat.render(chat_height);
        let phase_hint = match self.phase.as_str() {
            "Action" => {
                if self.ap_current > 0 {
                    format!("[AP: {}/{}]", self.ap_current, self.ap_max)
                } else {
                    "[AP: 0 — /end to advance]".into()
                }
            }
            "Conversation" => "[Free talk — /leave to end]".into(),
            "Dawn" => "[Briefing...]".into(),
            "Dusk" => "[Resolving...]".into(),
            _ => format!("[{}]", self.phase),
        };
        let input_str = self.input.clone();
        let overlay = self.overlay.clone();
        let deck = self.deck.clone();
        let coherence_label = self.coherence_label.clone();
        let coherence_score = self.coherence_score;

        terminal.draw(|frame| {
            let area = frame.area();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(area);

            // Status bar
            frame.render_widget(status_bar, layout[0]);

            // Chat
            frame.render_widget(chat_widget, layout[1]);

            // Input line
            let input_block = Block::default().borders(Borders::TOP);
            let input_text = Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::raw(&input_str),
                Span::styled(
                    "▊",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
                Span::raw("  "),
                Span::styled(&phase_hint, Style::default().fg(Color::DarkGray)),
            ]))
            .block(input_block);
            frame.render_widget(input_text, layout[2]);

            // Overlay
            if let Some(ref ov) = overlay {
                let overlay_area = centered_rect(50, 70, area);
                render_overlay_static(
                    frame,
                    ov,
                    overlay_area,
                    &deck,
                    &coherence_label,
                    coherence_score,
                );
            }
        })?;
        Ok(())
    }

    fn build_status_bar(&self) -> Paragraph<'static> {
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
            if self.chat.user_scrolled {
                Span::styled(
                    format!("[↑{} scrolled — End to return] ", self.chat.scroll_up),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::styled(
                    "[Tab] Menu  [PgUp/PgDn] Scroll",
                    Style::default().fg(Color::DarkGray),
                )
            },
        ]))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)))
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
                    KeyCode::PageUp => {
                        self.chat.scroll_up_by(10);
                    }
                    KeyCode::PageDown => {
                        self.chat.scroll_down_by(10);
                    }
                    KeyCode::Home => {
                        self.chat.scroll_up_by(1000); // Jump to top
                    }
                    KeyCode::End => {
                        self.chat.scroll_to_bottom();
                    }
                    KeyCode::Up if self.input.is_empty() => {
                        self.chat.scroll_up_by(3);
                    }
                    KeyCode::Down if self.input.is_empty() => {
                        self.chat.scroll_down_by(3);
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                        // Resume auto-scroll when user types
                        self.chat.scroll_to_bottom();
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

fn render_overlay_static(
    frame: &mut Frame,
    overlay: &Overlay,
    area: Rect,
    deck: &[CardView],
    coherence_label: &str,
    coherence_score: i32,
) {
    match overlay {
        Overlay::Deck => {
            render_deck_static(frame, area, deck, coherence_label, coherence_score);
            return;
        }
        _ => {}
    }

    let (title, items): (&str, Vec<&str>) = match overlay {
        Overlay::CommandPalette => (
            "≡ COMMAND PALETTE",
            vec![
                "",
                "  /meet <npc>      Meet with an NPC (2 AP)",
                "  /call <npc>      Phone call (1 AP)",
                "  /speech <topic>  Give a speech (1 AP)",
                "  /campaign        Campaign in district (2 AP)",
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
                "  You are a politician. Each week you get Action Points",
                "  to spend on meetings, speeches, legislation, and more.",
                "",
                "  Type freely to speak or act.",
                "  Use /commands for specific actions.",
                "  Press Tab for the command palette.",
                "",
                "  Scrolling: PgUp/PgDn or ↑↓ (when input empty)",
                "  Home = scroll to top, End = back to bottom",
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
                "       └───────────────────────┘",
                "",
                "  You represent District 1 (Urban)",
                "",
                "  [Esc] Close",
            ],
        ),
        _ => {
            let name = format!("{:?}", overlay);
            (
                Box::leak(name.into_boxed_str()),
                vec!["", "  (Coming soon)", "", "  [Esc] Close"],
            )
        }
    };

    let text: Vec<Line> = items.iter().map(|s| Line::from(*s)).collect();
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_deck_static(
    frame: &mut Frame,
    area: Rect,
    deck: &[CardView],
    coherence_label: &str,
    coherence_score: i32,
) {
    let mut lines: Vec<Line> = vec![Line::from("")];

    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} cards in deck", deck.len()),
            Style::default().fg(Color::White),
        ),
        Span::raw("  │  "),
        Span::styled(
            format!("Coherence: {} ({})", coherence_label, coherence_score),
            Style::default().fg(match coherence_label {
                "Principled" => Color::Green,
                "Flip-Flopper" => Color::Red,
                _ => Color::Yellow,
            }),
        ),
    ]));
    lines.push(Line::from(""));

    let tactics: Vec<_> = deck.iter().filter(|c| c.card_type == "tactic").collect();
    let assets: Vec<_> = deck.iter().filter(|c| c.card_type == "asset").collect();
    let positions: Vec<_> = deck.iter().filter(|c| c.card_type == "position").collect();

    for (label, color, tag, cards) in [
        ("── TACTICS ──", Color::Cyan, "T", &tactics),
        ("── ASSETS ──", Color::Yellow, "A", &assets),
        ("── POSITIONS ──", Color::Magenta, "P", &positions),
    ] {
        if !cards.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {}", label),
                Style::default().fg(color).bold(),
            )));
            for card in cards {
                lines.push(render_card_line(card, tag));
            }
            lines.push(Line::from(""));
        }
    }

    if deck.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Your deck is empty.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  [Esc] Close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(" 🃏 CARDS & DECK ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_card_line<'a>(card: &'a CardView, type_tag: &'a str) -> Line<'a> {
    let tag_color = match type_tag {
        "T" => Color::Cyan,
        "A" => Color::Yellow,
        "P" => Color::Magenta,
        _ => Color::White,
    };

    let rarity_indicator = match card.rarity.as_str() {
        "common" => "  ",
        "uncommon" => "★ ",
        "rare" => "★★",
        "legendary" => "★★★",
        _ => "  ",
    };

    let rarity_color = match card.rarity.as_str() {
        "common" => Color::DarkGray,
        "uncommon" => Color::Green,
        "rare" => Color::Rgb(100, 149, 237), // cornflower blue
        "legendary" => Color::Rgb(255, 215, 0), // gold
        _ => Color::DarkGray,
    };

    Line::from(vec![
        Span::styled(format!("  [{}] ", type_tag), Style::default().fg(tag_color)),
        Span::styled(&card.name, Style::default().fg(Color::White).bold()),
        Span::raw("  "),
        Span::styled(rarity_indicator, Style::default().fg(rarity_color)),
        Span::raw("  "),
        Span::styled(
            &card.description,
            Style::default().fg(Color::Rgb(120, 120, 140)),
        ),
    ])
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
