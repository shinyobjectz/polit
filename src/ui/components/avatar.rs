use ratatui::prelude::*;

use crate::ui::chat::NpcAvatar;
use crate::ui::theme;

/// Head shape options for character creation
pub const HEAD_OPTIONS: &[(&str, &str, &str)] = &[
    ("[", "]", "Square"),
    ("(", ")", "Round"),
    ("{", "}", "Curly"),
    ("|", "|", "Pipe"),
    ("⟦", "⟧", "Formal"),
    ("⟨", "⟩", "Sleek"),
    ("╔", "╗", "Rigid"),
    ("▐", "▌", "Solid"),
];

/// Eye options for character creation
pub const EYE_OPTIONS: &[(&str, &str)] = &[
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

/// Color options for character creation
pub const COLOR_OPTIONS: &[(Color, &str)] = &[
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

/// Build the avatar string from head + eyes selections
pub fn build_avatar(head: usize, eyes: usize) -> String {
    let (left, right, _) = HEAD_OPTIONS[head];
    let (eye_chars, _) = EYE_OPTIONS[eyes];
    format!("{}{}{}", left, eye_chars, right)
}

/// Build an animated avatar string with periodic blink
pub fn build_animated_avatar(head: usize, eyes: usize, frame_count: u64) -> String {
    let (left, right, _) = HEAD_OPTIONS[head];
    let (eye_chars, _) = EYE_OPTIONS[eyes];
    let cycle = frame_count % 90;
    let display_eyes = if cycle >= 86 { "--" } else { eye_chars };
    format!("{}{}{}", left, display_eyes, right)
}

/// Get NPC avatar based on name (for NPCs without custom avatars)
pub fn get_npc_avatar(name: &str) -> NpcAvatar {
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

/// Narrator avatar constant
pub fn narrator_avatar() -> NpcAvatar {
    NpcAvatar {
        face: "✦✦".to_string(),
        color: theme::ACCENT,
        name: "Narrator".to_string(),
    }
}
