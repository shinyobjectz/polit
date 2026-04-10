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
    Narration,   // White
    NpcDialogue, // Cyan
    PlayerInput, // Green
    SystemEvent, // Yellow
    Warning,     // Red
    Success,     // Green
    DiceRoll,    // Magenta
    PhaseHeader, // Bold white with separators
}

/// Scrollable chat stream with manual scroll support
pub struct ChatStream {
    pub messages: Vec<ChatMessage>,
    /// Manual scroll offset from bottom (0 = at bottom)
    pub scroll_up: u16,
    /// Whether user has manually scrolled (disables auto-scroll)
    pub user_scrolled: bool,
    /// Total rendered lines (for scroll bounds)
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
        });
        // Auto-scroll to bottom on new content (unless user scrolled up)
        if !self.user_scrolled {
            self.scroll_up = 0;
        }
    }

    pub fn add_narration(&mut self, text: &str) {
        self.add_message(MessageStyle::Narration, text);
    }

    pub fn add_npc(&mut self, name: &str, text: &str) {
        self.add_message(
            MessageStyle::NpcDialogue,
            &format!("■ {}\n{}", name.to_uppercase(), text),
        );
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
        self.add_message(
            MessageStyle::PhaseHeader,
            &format!("┄┄┄┄┄┄┄┄┄┄┄┄ {} ┄┄┄┄┄┄┄┄┄┄┄┄", text),
        );
    }

    /// Scroll up by N lines
    pub fn scroll_up_by(&mut self, lines: u16) {
        let max_scroll = self.total_lines.saturating_sub(5);
        self.scroll_up = (self.scroll_up + lines).min(max_scroll);
        self.user_scrolled = true;
    }

    /// Scroll down by N lines
    pub fn scroll_down_by(&mut self, lines: u16) {
        if self.scroll_up <= lines {
            self.scroll_up = 0;
            self.user_scrolled = false; // Back at bottom = resume auto-scroll
        } else {
            self.scroll_up -= lines;
        }
    }

    /// Jump to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_up = 0;
        self.user_scrolled = false;
    }

    pub fn render(&mut self, viewport_height: u16) -> Paragraph<'_> {
        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|msg| {
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

                msg.text
                    .lines()
                    .map(move |line| Line::from(Span::styled(line.to_string(), style)))
                    .chain(std::iter::once(Line::from("")))
                    .collect::<Vec<_>>()
            })
            .collect();

        self.total_lines = lines.len() as u16;

        let block = Block::default().borders(Borders::NONE);

        // Calculate scroll position: bottom minus user offset
        let visible = viewport_height.saturating_sub(2);
        let max_scroll = self.total_lines.saturating_sub(visible);
        let scroll = max_scroll.saturating_sub(self.scroll_up);

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
    }
}
