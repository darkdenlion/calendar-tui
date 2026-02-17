use chrono::{DateTime, Local};
use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub is_all_day: bool,
    pub calendar_name: String,
    pub calendar_color: Color,
    pub location: Option<String>,
    pub notes: Option<String>,
}

impl CalendarEvent {
    pub fn duration_display(&self) -> String {
        if self.is_all_day {
            "All day".to_string()
        } else {
            let start = self.start.format("%H:%M");
            let end = self.end.format("%H:%M");
            format!("{} - {}", start, end)
        }
    }
}
