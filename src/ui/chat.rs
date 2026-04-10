use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph, Wrap},
};

use super::theme;

/// A single message in the chat stream
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub style: MessageStyle,
    pub text: String,
    /// Optional NPC avatar (face chars + color)
    pub avatar: Option<NpcAvatar>,
}

#[derive(Debug, Clone)]
pub struct NpcAvatar {
    pub face: String, // e.g., "°°", "──", "^^"
    pub color: Color,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum MessageStyle {
    Narration,
    NpcDialogue,
    PlayerInput,
    SystemEvent,
    Warning,
    Success,
    DiceRoll,
    PhaseHeader,
}

/// Scrollable chat stream
pub struct ChatStream {
    pub messages: Vec<ChatMessage>,
    pub scroll_up: u16,
    pub user_scrolled: bool,
    total_lines: u16,
}

impl ChatStream {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_up: 0,
            user_scrolled: false,
            total_lines: 0,
        }
    }

    fn add_message(&mut self, style: MessageStyle, text: &str) {
        self.messages.push(ChatMessage {
            style,
            text: text.to_string(),
            avatar: None,
        });
        if !self.user_scrolled {
            self.scroll_up = 0;
        }
    }

    pub fn add_narration(&mut self, text: &str) {
        self.add_message(MessageStyle::Narration, text);
    }

    pub fn add_npc(&mut self, name: &str, text: &str, avatar: Option<NpcAvatar>) {
        self.messages.push(ChatMessage {
            style: MessageStyle::NpcDialogue,
            text: text.to_string(),
            avatar: avatar.or_else(|| {
                Some(NpcAvatar {
                    face: "••".to_string(),
                    color: Color::Cyan,
                    name: name.to_string(),
                })
            }),
        });
        if !self.user_scrolled {
            self.scroll_up = 0;
        }
    }

    pub fn add_player(&mut self, text: &str) {
        self.add_message(MessageStyle::PlayerInput, &format!("> {}", text));
    }

    pub fn add_system(&mut self, text: &str) {
        self.add_message(MessageStyle::SystemEvent, text);
    }

    pub fn add_success(&mut self, text: &str) {
        self.add_message(MessageStyle::Success, text);
    }

    pub fn add_warning(&mut self, text: &str) {
        self.add_message(MessageStyle::Warning, text);
    }

    pub fn add_dice(&mut self, text: &str) {
        self.add_message(MessageStyle::DiceRoll, text);
    }

    pub fn add_phase_header(&mut self, text: &str) {
        self.add_message(MessageStyle::PhaseHeader, text);
    }

    pub fn scroll_up_by(&mut self, lines: u16) {
        self.scroll_up = (self.scroll_up + lines).min(self.total_lines);
        self.user_scrolled = true;
    }

    pub fn scroll_down_by(&mut self, lines: u16) {
        if self.scroll_up <= lines {
            self.scroll_up = 0;
            self.user_scrolled = false;
        } else {
            self.scroll_up -= lines;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_up = 0;
        self.user_scrolled = false;
    }

    pub fn render(&mut self, viewport_height: u16) -> Paragraph<'_> {
        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|msg| {
                let mut result = Vec::new();

                // NPC avatar + name header
                if let Some(ref avatar) = msg.avatar {
                    result.push(Line::from(vec![
                        Span::styled(
                            format!("{} ", avatar.face),
                            Style::default().fg(avatar.color),
                        ),
                        Span::styled(
                            avatar.name.to_uppercase(),
                            Style::default().fg(avatar.color).bold(),
                        ),
                    ]));
                }

                let style = match msg.style {
                    MessageStyle::Narration => Style::default().fg(theme::NARRATION),
                    MessageStyle::NpcDialogue => Style::default().fg(theme::NPC_DIALOGUE),
                    MessageStyle::PlayerInput => Style::default().fg(theme::PLAYER_INPUT),
                    MessageStyle::SystemEvent => Style::default().fg(theme::FG_DIM),
                    MessageStyle::Warning => Style::default().fg(theme::WARNING),
                    MessageStyle::Success => Style::default().fg(theme::SUCCESS),
                    MessageStyle::DiceRoll => Style::default().fg(theme::DICE),
                    MessageStyle::PhaseHeader => Style::default().fg(theme::PHASE_HEADER),
                };

                // Phase headers get a subtle separator
                if matches!(msg.style, MessageStyle::PhaseHeader) {
                    result.push(Line::from(""));
                    result.push(Line::from(Span::styled(
                        format!("  ─── {} ───", msg.text),
                        style,
                    )));
                    result.push(Line::from(""));
                } else {
                    for line in msg.text.lines() {
                        result.push(Line::from(Span::styled(line.to_string(), style)));
                    }
                    result.push(Line::from(""));
                }

                result
            })
            .collect();

        self.total_lines = lines.len() as u16;

        let block = Block::default().style(Style::default().bg(theme::BG));

        let visible = viewport_height.saturating_sub(1);
        let max_scroll = self.total_lines.saturating_sub(visible);
        let scroll = max_scroll.saturating_sub(self.scroll_up);

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
    }
}
