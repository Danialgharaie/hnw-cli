// Hallmark · pre-emit critique: P5 H4 E5 S5 R5 V4
// Hallmark · genre: modern-minimal · macrostructure: Workbench · theme: Cobalt
// tone: technical/utilitarian · anchor hue: cobalt · contrast: pass

use ratatui::style::{Color, Modifier, Style};

pub const PAPER: Color = Color::Rgb(10, 15, 24);
pub const PANEL: Color = Color::Rgb(16, 24, 38);
pub const PANEL_ACTIVE: Color = Color::Rgb(24, 36, 56);
pub const INK: Color = Color::Rgb(226, 232, 240);
pub const MUTED: Color = Color::Rgb(128, 145, 168);
pub const RULE: Color = Color::Rgb(48, 65, 88);
pub const ACCENT: Color = Color::Rgb(83, 146, 255);
pub const SUCCESS: Color = Color::Rgb(69, 196, 140);
pub const WARNING: Color = Color::Rgb(244, 187, 89);
pub const DANGER: Color = Color::Rgb(244, 104, 116);

pub fn base() -> Style {
    Style::default().fg(INK).bg(PAPER)
}

pub fn muted() -> Style {
    Style::default().fg(MUTED).bg(PAPER)
}

pub fn accent() -> Style {
    Style::default()
        .fg(ACCENT)
        .bg(PAPER)
        .add_modifier(Modifier::BOLD)
}

pub fn selected() -> Style {
    Style::default()
        .fg(INK)
        .bg(PANEL_ACTIVE)
        .add_modifier(Modifier::BOLD)
}
