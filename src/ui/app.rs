use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::chat::{ChatStream, NpcAvatar};
use super::theme;
use crate::engine::channels::{UiChannels, UiCommand, UiMessage};

/// Main application state (UI thread only)
pub struct App {
    pub chat: ChatStream,
    pub input: String,
    pub should_quit: bool,
    pub channels: UiChannels,
    // Status bar
    pub week: u32,
    pub year: u32,
    pub phase: String,
    pub ap_current: i32,
    pub ap_max: i32,
    // Slash autocomplete
    pub showing_slash_menu: bool,
    pub slash_filter: String,
    pub slash_selected: usize,
}

/// Slash commands available
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("meet", "Meet with someone"),
    ("call", "Phone call (1 AP)"),
    ("speech", "Give a speech"),
    ("campaign", "Campaign in district"),
    ("draft", "Draft legislation"),
    ("end", "End turn"),
    ("save", "Save game"),
    ("load", "Load save"),
    ("quit", "Quit game"),
];

impl App {
    pub fn new(channels: UiChannels) -> Self {
        Self {
            chat: ChatStream::new(),
            input: String::new(),
            should_quit: false,
            channels,
            week: 1,
            year: 2024,
            phase: "Starting".into(),
            ap_current: 5,
            ap_max: 5,
            showing_slash_menu: false,
            slash_filter: String::new(),
            slash_selected: 0,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while !self.should_quit {
            self.process_game_messages();
            self.draw(terminal)?;
            self.handle_input()?;
        }
        Ok(())
    }

    fn process_game_messages(&mut self) {
        for msg in self.channels.drain_messages() {
            match msg {
                UiMessage::Narrate(text) => self.chat.add_narration(&text),
                UiMessage::NpcDialogue { name, text } => {
                    let avatar = get_npc_avatar(&name);
                    self.chat.add_npc(&name, &text, Some(avatar));
                }
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
                UiMessage::Event(_) => {}
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
        // Pre-compute everything before borrowing chat mutably
        let status = self.build_status_bar();
        let input_str = self.input.clone();
        let showing_slash = self.showing_slash_menu;
        let filtered_cmds = self.filtered_commands();
        let slash_selected = self.slash_selected;
        let chat_height = terminal.size()?.height.saturating_sub(3);
        let chat_widget = self.chat.render(chat_height);

        terminal.draw(|frame| {
            let area = frame.area();

            // Dark background
            frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

            // Calculate input height based on content lines
            let input_lines: Vec<&str> = input_str.split('\n').collect();
            let input_height = (input_lines.len() as u16 + 2).max(3).min(10); // border + content, min 3, max 10

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),            // Status bar (full width)
                    Constraint::Min(3),               // Chat (centered)
                    Constraint::Length(input_height), // Input (dynamic height)
                    Constraint::Length(1),            // Bottom margin
                ])
                .split(area);

            // Status bar — FULL WIDTH
            frame.render_widget(
                status.clone().style(Style::default().bg(theme::BG_SUBTLE)),
                layout[0],
            );

            // Chat — centered column
            let chat_area = theme::centered_content(layout[1]);
            frame.render_widget(chat_widget, chat_area);

            // Input — floating card bar, centered, grows with content
            let input_content_area = theme::centered_content(layout[2]);
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::ACCENT_BLUE))
                .style(Style::default().bg(theme::BG_HIGHLIGHT));
            let inner_area = input_block.inner(input_content_area);
            frame.render_widget(input_block, input_content_area);

            // Render multi-line input
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
                // Cursor on the last line
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
            let input_widget = Paragraph::new(lines);
            frame.render_widget(input_widget, inner_area);

            // Slash autocomplete menu
            if showing_slash && !filtered_cmds.is_empty() {
                let menu_height = (filtered_cmds.len() as u16 + 2).min(12);
                let menu_width = 35;
                let menu_x = input_content_area.x + 1;
                let menu_y = input_content_area.y.saturating_sub(menu_height);
                let menu_area = Rect::new(menu_x, menu_y, menu_width, menu_height);

                let items: Vec<Line> = filtered_cmds
                    .iter()
                    .enumerate()
                    .map(|(i, (cmd, desc))| {
                        if i == slash_selected {
                            Line::from(vec![
                                Span::styled(
                                    format!(" /{} ", cmd),
                                    Style::default()
                                        .fg(Color::White)
                                        .bg(theme::BG_HIGHLIGHT)
                                        .bold(),
                                ),
                                Span::styled(
                                    format!(" {}", desc),
                                    Style::default().fg(theme::FG_DIM).bg(theme::BG_HIGHLIGHT),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::styled(format!(" /{} ", cmd), Style::default().fg(theme::FG)),
                                Span::styled(
                                    format!(" {}", desc),
                                    Style::default().fg(theme::FG_MUTED),
                                ),
                            ])
                        }
                    })
                    .collect();

                let menu = Paragraph::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::BORDER))
                        .style(Style::default().bg(theme::BG_SUBTLE)),
                );
                frame.render_widget(Clear, menu_area);
                frame.render_widget(menu, menu_area);
            }
        })?;
        Ok(())
    }

    fn build_status_bar(&self) -> Paragraph<'static> {
        let filled = "█".repeat(self.ap_current.max(0) as usize);
        let empty = "░".repeat((self.ap_max - self.ap_current).max(0) as usize);

        Paragraph::new(Line::from(vec![
            Span::styled("  🇺🇸 ", Style::default()),
            Span::styled("POLIT", Style::default().fg(theme::FG).bold()),
            Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(
                format!("Week {}, {}", self.week, self.year),
                Style::default().fg(theme::FG_DIM),
            ),
            Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(self.phase.clone(), Style::default().fg(theme::FG)),
            Span::styled("  │  ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(
                format!("AP {}{} ", filled, empty),
                Style::default().fg(theme::FG_DIM),
            ),
            Span::styled(
                format!("{}/{}", self.ap_current, self.ap_max),
                Style::default().fg(theme::FG),
            ),
        ]))
        .style(Style::default().bg(theme::BG_SUBTLE))
    }

    fn filtered_commands(&self) -> Vec<(String, String)> {
        let filter = self.slash_filter.to_lowercase();
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| filter.is_empty() || cmd.starts_with(&filter))
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect()
    }

    fn handle_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            // Mouse scroll
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

                // Slash menu navigation
                if self.showing_slash_menu {
                    match key.code {
                        KeyCode::Esc => {
                            self.showing_slash_menu = false;
                            self.input.clear();
                        }
                        KeyCode::Up => {
                            if self.slash_selected > 0 {
                                self.slash_selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            let max = self.filtered_commands().len().saturating_sub(1);
                            if self.slash_selected < max {
                                self.slash_selected += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Tab => {
                            let cmds = self.filtered_commands();
                            if let Some((cmd, _)) = cmds.get(self.slash_selected) {
                                self.input = format!("/{} ", cmd);
                            }
                            self.showing_slash_menu = false;
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                            if !self.input.starts_with('/') {
                                self.showing_slash_menu = false;
                            } else {
                                self.slash_filter = self.input[1..].to_string();
                                self.slash_selected = 0;
                            }
                        }
                        KeyCode::Char(c) => {
                            self.input.push(c);
                            self.slash_filter = self.input[1..].to_string();
                            self.slash_selected = 0;
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.channels.send(UiCommand::Quit);
                        self.should_quit = true;
                    }
                    // Shift+R = return to bottom
                    KeyCode::Char('R') => {
                        self.chat.scroll_to_bottom();
                    }
                    // Shift+Enter = newline in input
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::SHIFT) => {
                        self.input.push('\n');
                    }
                    // Enter = submit
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
                    KeyCode::Char('/') if self.input.is_empty() => {
                        self.input.push('/');
                        self.showing_slash_menu = true;
                        self.slash_filter.clear();
                        self.slash_selected = 0;
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

    fn process_input(&mut self, input: &str) {
        if input.starts_with('/') {
            let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
            let cmd = parts[0].to_string();
            let args: Vec<String> = if parts.len() > 1 {
                parts[1].split_whitespace().map(|s| s.to_string()).collect()
            } else {
                vec![]
            };

            match cmd.as_str() {
                "quit" => {
                    self.channels.send(UiCommand::Quit);
                    self.should_quit = true;
                }
                "end" => {
                    self.channels.send(UiCommand::EndTurn);
                }
                "save" => {
                    let name = if args.is_empty() {
                        format!("save_w{}_y{}", self.week, self.year)
                    } else {
                        args.join("_")
                    };
                    self.channels.send(UiCommand::SaveGame(name));
                }
                "load" => {
                    if args.is_empty() {
                        self.chat.add_system("Usage: /load <save_name>");
                    } else {
                        self.channels.send(UiCommand::LoadGame(args.join(" ")));
                    }
                }
                _ => {
                    self.channels.send(UiCommand::SlashCommand { cmd, args });
                }
            }
        } else {
            self.channels
                .send(UiCommand::PlayerInput(input.to_string()));
        }
    }
}

/// Get NPC avatar based on name
fn get_npc_avatar(name: &str) -> NpcAvatar {
    let lower = name.to_lowercase();
    let (face, color) = if lower.contains("davis") {
        ("°°", Color::Cyan)
    } else if lower.contains("kowalski") {
        ("──", Color::Yellow)
    } else if lower.contains("martinez") {
        ("^^", Color::Green)
    } else if lower.contains("chen") {
        ("¬¬", Color::Red)
    } else if lower.contains("kim") {
        ("••", Color::Magenta)
    } else {
        // Generate from name hash
        let faces = ["••", "°°", "^^", "──", "¬¬", "◦◦", "∘∘", "··"];
        let colors = [
            Color::Cyan,
            Color::Yellow,
            Color::Green,
            Color::Red,
            Color::Magenta,
            Color::LightBlue,
            Color::LightGreen,
            Color::LightRed,
        ];
        let hash = name
            .bytes()
            .fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        (faces[hash % faces.len()], colors[hash % colors.len()])
    };

    NpcAvatar {
        face: face.to_string(),
        color,
        name: name.to_string(),
    }
}
