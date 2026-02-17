use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub is_completed: bool,
    pub due_date: Option<DateTime<Local>>,
    pub calendar_name: String,
    pub priority: u8,
}
