use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// A single message in the chat stream
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub style: MessageStyle,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum MessageStyle {
    Narration,      // White
    NpcDialogue,    // Cyan
    PlayerInput,    // Green
    SystemEvent,    // Yellow
    Warning,        // Red
    Success,        // Green
    DiceRoll,       // Magenta
    PhaseHeader,    // Bold white with separators
}

/// Scrollable chat stream
pub struct ChatStream {
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: u16,
}

impl ChatStream {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn add_narration(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::Narration,
            text: text.to_string(),
        });
    }

    pub fn add_npc(&mut self, name: &str, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::NpcDialogue,
            text: format!("■ {}\n{}", name.to_uppercase(), text),
        });
    }

    pub fn add_player(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::PlayerInput,
            text: format!("> {}", text),
        });
    }

    pub fn add_system(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::SystemEvent,
            text: text.to_string(),
        });
    }

    pub fn add_success(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::Success,
            text: text.to_string(),
        });
    }

    pub fn add_warning(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::Warning,
            text: text.to_string(),
        });
    }

    pub fn add_dice(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::DiceRoll,
            text: text.to_string(),
        });
    }

    pub fn add_phase_header(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            style: MessageStyle::PhaseHeader,
            text: format!("┄┄┄┄┄┄┄┄┄┄┄┄ {} ┄┄┄┄┄┄┄┄┄┄┄┄", text),
        });
    }

    pub fn render(&self, viewport_height: u16) -> Paragraph<'_> {
        let lines: Vec<Line> = self.messages.iter().flat_map(|msg| {
            let style = match msg.style {
                MessageStyle::Narration => Style::default().fg(Color::White),
                MessageStyle::NpcDialogue => Style::default().fg(Color::Cyan),
                MessageStyle::PlayerInput => Style::default().fg(Color::Green),
                MessageStyle::SystemEvent => Style::default().fg(Color::Yellow),
                MessageStyle::Warning => Style::default().fg(Color::Red),
                MessageStyle::Success => Style::default().fg(Color::Green),
                MessageStyle::DiceRoll => Style::default().fg(Color::Magenta),
                MessageStyle::PhaseHeader => Style::default().fg(Color::White).bold(),
            };

            msg.text.lines().map(move |line| {
                Line::from(Span::styled(line.to_string(), style))
            }).chain(std::iter::once(Line::from("")))
            .collect::<Vec<_>>()
        }).collect();

        let block = Block::default()
            .borders(Borders::NONE);

        // Auto-scroll to bottom
        let total_lines = lines.len() as u16;
        let scroll = if total_lines > viewport_height.saturating_sub(2) {
            total_lines - viewport_height.saturating_sub(2)
        } else {
            0
        };

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
    }
}
