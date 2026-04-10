use ratatui::prelude::*;

/// Global dark theme constants
pub const BG: Color = Color::Rgb(8, 8, 16);
pub const BG_SUBTLE: Color = Color::Rgb(15, 15, 25);
pub const BG_HIGHLIGHT: Color = Color::Rgb(25, 25, 40);

pub const FG: Color = Color::Rgb(220, 220, 230);
pub const FG_DIM: Color = Color::Rgb(100, 100, 120);
pub const FG_MUTED: Color = Color::Rgb(60, 60, 80);

pub const ACCENT: Color = Color::Rgb(191, 10, 48); // Red accent (flag red)
pub const ACCENT_BLUE: Color = Color::Rgb(0, 40, 104); // Flag blue

pub const BORDER: Color = Color::Rgb(40, 40, 55);
pub const BORDER_ACTIVE: Color = Color::Rgb(80, 80, 110);

// Chat message colors
pub const NARRATION: Color = FG;
pub const NPC_DIALOGUE: Color = Color::Cyan;
pub const PLAYER_INPUT: Color = Color::Rgb(100, 220, 100);
pub const SYSTEM: Color = Color::Rgb(180, 180, 60);
pub const WARNING: Color = Color::Rgb(255, 100, 100);
pub const SUCCESS: Color = Color::Rgb(100, 230, 100);
pub const DICE: Color = Color::Magenta;
pub const PHASE_HEADER: Color = FG_DIM;

// Content width
pub const MAX_CONTENT_WIDTH: u16 = 80;

/// Compute centered content area with margins
pub fn centered_content(area: Rect) -> Rect {
    if area.width <= MAX_CONTENT_WIDTH + 4 {
        // Terminal too narrow, use full width with small padding
        Rect::new(
            area.x + 2,
            area.y,
            area.width.saturating_sub(4),
            area.height,
        )
    } else {
        let margin = (area.width - MAX_CONTENT_WIDTH) / 2;
        Rect::new(area.x + margin, area.y, MAX_CONTENT_WIDTH, area.height)
    }
}
