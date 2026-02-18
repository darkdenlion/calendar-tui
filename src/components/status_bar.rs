#[allow(dead_code)]
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ViewMode;
use crate::theme;

#[allow(dead_code)]
pub struct StatusBar;

#[allow(dead_code)]
impl StatusBar {
    pub fn render(frame: &mut Frame, area: Rect, mode: &ViewMode) {
        let w = area.width as usize;

        let mode_str = match mode {
            ViewMode::Month => "[1]Month",
            ViewMode::Week => "[2]Week",
            ViewMode::Day => "[3]Day",
        };

        let hints = if w >= 70 {
            " \u{2190}\u{2191}\u{2192}\u{2193}:Navigate  [/]:Month  t:Today  q:Quit"
        } else if w >= 40 {
            " arrows:Nav [/]:Mon t:Today q:Quit"
        } else {
            " q:Quit"
        };

        let padding = " ".repeat(w.saturating_sub(mode_str.len() + 1 + hints.len()));

        let line = Line::from(vec![
            Span::styled(format!(" {} ", mode_str), theme::current().status),
            Span::styled(padding, theme::current().status),
            Span::styled(hints.to_string(), theme::current().status),
        ]);

        let bar = Paragraph::new(line).style(theme::current().status);
        frame.render_widget(bar, area);
    }
}
