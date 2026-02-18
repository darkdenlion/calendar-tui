use std::path::PathBuf;
use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the active theme (loaded once on first call).
pub fn current() -> &'static Theme {
    THEME.get_or_init(|| Theme::load().unwrap_or_default())
}

// Const fallbacks used in places that need compile-time styles
pub const HEADER_STYLE: Style = Style::new()
    .fg(Color::White)
    .add_modifier(Modifier::BOLD);
pub const DIM_STYLE: Style = Style::new().fg(Color::DarkGray);
pub const BORDER_STYLE: Style = Style::new().fg(Color::Gray);
pub const STATUS_STYLE: Style = Style::new().fg(Color::White).bg(Color::DarkGray);
pub const SELECTED_STYLE: Style = Style::new().fg(Color::Black).bg(Color::Cyan);

#[allow(dead_code)]
pub const EVENT_DOT_STYLE: Style = Style::new().fg(Color::Green);

#[allow(dead_code)]
pub fn calendar_color_to_ratatui(r: f64, g: f64, b: f64) -> Color {
    Color::Rgb(
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
    )
}

#[derive(Debug, Clone)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: String,
    pub today: Style,
    pub selected: Style,
    pub header: Style,
    pub dim: Style,
    pub border: Style,
    pub status: Style,
    pub highlight: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            today: Style::default().fg(Color::Black).bg(Color::Yellow),
            selected: Style::default().fg(Color::Black).bg(Color::Cyan),
            header: Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            dim: Style::default().fg(Color::DarkGray),
            border: Style::default().fg(Color::Gray),
            status: Style::default().fg(Color::White).bg(Color::DarkGray),
            highlight: Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
        }
    }
}

impl Theme {
    pub fn load() -> Option<Self> {
        let path = config_path()?;
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let config: ThemeConfig = toml::from_str(&content).ok()?;
        Some(config.into_theme())
    }

    /// Get a built-in preset by name.
    pub fn preset(name: &str) -> Self {
        match name {
            "dracula" => Self::dracula(),
            "gruvbox" => Self::gruvbox(),
            "nord" => Self::nord(),
            _ => Self::default(),
        }
    }

    fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            today: Style::default().fg(Color::Black).bg(Color::Rgb(189, 147, 249)), // purple
            selected: Style::default().fg(Color::Black).bg(Color::Rgb(139, 233, 253)), // cyan
            header: Style::default().fg(Color::Rgb(248, 248, 242)).add_modifier(Modifier::BOLD),
            dim: Style::default().fg(Color::Rgb(98, 114, 164)),
            border: Style::default().fg(Color::Rgb(68, 71, 90)),
            status: Style::default()
                .fg(Color::Rgb(248, 248, 242))
                .bg(Color::Rgb(68, 71, 90)),
            highlight: Style::default()
                .bg(Color::Rgb(68, 71, 90))
                .add_modifier(Modifier::BOLD),
        }
    }

    fn gruvbox() -> Self {
        Self {
            name: "gruvbox".to_string(),
            today: Style::default().fg(Color::Black).bg(Color::Rgb(250, 189, 47)), // yellow
            selected: Style::default().fg(Color::Black).bg(Color::Rgb(131, 165, 152)), // aqua
            header: Style::default().fg(Color::Rgb(235, 219, 178)).add_modifier(Modifier::BOLD),
            dim: Style::default().fg(Color::Rgb(146, 131, 116)),
            border: Style::default().fg(Color::Rgb(102, 92, 84)),
            status: Style::default()
                .fg(Color::Rgb(235, 219, 178))
                .bg(Color::Rgb(80, 73, 69)),
            highlight: Style::default()
                .bg(Color::Rgb(80, 73, 69))
                .add_modifier(Modifier::BOLD),
        }
    }

    fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            today: Style::default().fg(Color::Black).bg(Color::Rgb(235, 203, 139)), // yellow
            selected: Style::default().fg(Color::Black).bg(Color::Rgb(136, 192, 208)), // frost
            header: Style::default().fg(Color::Rgb(229, 233, 240)).add_modifier(Modifier::BOLD),
            dim: Style::default().fg(Color::Rgb(76, 86, 106)),
            border: Style::default().fg(Color::Rgb(67, 76, 94)),
            status: Style::default()
                .fg(Color::Rgb(229, 233, 240))
                .bg(Color::Rgb(67, 76, 94)),
            highlight: Style::default()
                .bg(Color::Rgb(67, 76, 94))
                .add_modifier(Modifier::BOLD),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("calendar-tui").join("theme.toml"))
}

// ── TOML config types ──

#[derive(Debug, Deserialize, Default)]
struct ThemeConfig {
    preset: Option<String>,
    today_fg: Option<String>,
    today_bg: Option<String>,
    selected_fg: Option<String>,
    selected_bg: Option<String>,
    header_fg: Option<String>,
    dim_fg: Option<String>,
    border_fg: Option<String>,
    status_fg: Option<String>,
    status_bg: Option<String>,
    highlight_bg: Option<String>,
}

impl ThemeConfig {
    fn into_theme(self) -> Theme {
        // Start from preset or default
        let mut theme = self
            .preset
            .as_deref()
            .map(Theme::preset)
            .unwrap_or_default();

        // Override individual colors
        if let Some(c) = self.today_fg.as_deref().and_then(parse_color) {
            theme.today = theme.today.fg(c);
        }
        if let Some(c) = self.today_bg.as_deref().and_then(parse_color) {
            theme.today = theme.today.bg(c);
        }
        if let Some(c) = self.selected_fg.as_deref().and_then(parse_color) {
            theme.selected = theme.selected.fg(c);
        }
        if let Some(c) = self.selected_bg.as_deref().and_then(parse_color) {
            theme.selected = theme.selected.bg(c);
        }
        if let Some(c) = self.header_fg.as_deref().and_then(parse_color) {
            theme.header = theme.header.fg(c);
        }
        if let Some(c) = self.dim_fg.as_deref().and_then(parse_color) {
            theme.dim = theme.dim.fg(c);
        }
        if let Some(c) = self.border_fg.as_deref().and_then(parse_color) {
            theme.border = theme.border.fg(c);
        }
        if let Some(c) = self.status_fg.as_deref().and_then(parse_color) {
            theme.status = theme.status.fg(c);
        }
        if let Some(c) = self.status_bg.as_deref().and_then(parse_color) {
            theme.status = theme.status.bg(c);
        }
        if let Some(c) = self.highlight_bg.as_deref().and_then(parse_color) {
            theme.highlight = theme.highlight.bg(c);
        }

        theme
    }
}

/// Parse a color string: hex "#rrggbb", or named colors.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ => None,
    }
}
