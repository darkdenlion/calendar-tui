use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ViewMode;
use crate::theme;

pub struct StatusBar;

impl StatusBar {
    pub fn render(frame: &mut Frame, area: Rect, mode: &ViewMode) {
        let mode_str = match mode {
            ViewMode::Month => "[1]Month",
            ViewMode::Week => "[2]Week",
            ViewMode::Day => "[3]Day",
        };

        let hints = " arrows:Navigate  [/]:Month  t:Today  q:Quit";

        let line = Line::from(vec![
            Span::styled(format!(" {} ", mode_str), theme::STATUS_STYLE),
            Span::styled(hints, theme::STATUS_STYLE),
        ]);

        let bar = Paragraph::new(line).style(theme::STATUS_STYLE);
        frame.render_widget(bar, area);
    }
}
