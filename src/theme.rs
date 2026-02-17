use ratatui::style::{Color, Modifier, Style};

pub const TODAY_STYLE: Style = Style::new().fg(Color::Black).bg(Color::Yellow);
pub const SELECTED_STYLE: Style = Style::new().fg(Color::Black).bg(Color::Cyan);
pub const HEADER_STYLE: Style = Style::new()
    .fg(Color::White)
    .add_modifier(Modifier::BOLD);
pub const DIM_STYLE: Style = Style::new().fg(Color::DarkGray);
#[allow(dead_code)]
pub const EVENT_DOT_STYLE: Style = Style::new().fg(Color::Green);
pub const BORDER_STYLE: Style = Style::new().fg(Color::Gray);
pub const STATUS_STYLE: Style = Style::new().fg(Color::White).bg(Color::DarkGray);

#[allow(dead_code)]
pub fn calendar_color_to_ratatui(r: f64, g: f64, b: f64) -> Color {
    Color::Rgb(
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
    )
}
