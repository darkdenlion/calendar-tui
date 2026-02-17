use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    pub color: Color,
    pub source: String,
}
