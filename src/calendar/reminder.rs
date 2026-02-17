use chrono::{DateTime, Local};
use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub is_completed: bool,
    pub due_date: Option<DateTime<Local>>,
    pub calendar_name: String,
    pub calendar_color: Color,
    pub priority: u8,
}
