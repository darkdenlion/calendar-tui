use std::collections::HashSet;

use chrono::{Datelike, Local, NaiveDate};
use color_eyre::Result;

use crate::calendar::{CalendarEvent, CalendarInfo, Reminder, Store};
use crate::components::event_form::{EventFormState, FormField};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Month,
    Week,
    Day,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Form,
    Reminders,
}

pub struct App {
    pub running: bool,
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub selected_date: NaiveDate,
    pub today: NaiveDate,
    pub calendars: Vec<CalendarInfo>,
    pub month_events: Vec<CalendarEvent>,
    pub week_events: Vec<CalendarEvent>,
    pub day_events: Vec<CalendarEvent>,
    pub days_with_events: HashSet<u32>,
    pub access_granted: bool,
    pub day_scroll: usize,
    // Reminders
    pub reminders: Vec<Reminder>,
    pub show_reminders: bool,
    pub reminder_index: usize,
    // Event form
    pub form_state: Option<EventFormState>,
    // Status message
    pub status_message: Option<String>,
    store: Store,
}

impl App {
    pub fn new() -> Result<Self> {
        let store = Store::new()?;
        let today = Local::now().date_naive();

        let mut app = Self {
            running: true,
            view_mode: ViewMode::Month,
            input_mode: InputMode::Normal,
            selected_date: today,
            today,
            calendars: Vec::new(),
            month_events: Vec::new(),
            week_events: Vec::new(),
            day_events: Vec::new(),
            days_with_events: HashSet::new(),
            access_granted: false,
            day_scroll: 0,
            reminders: Vec::new(),
            show_reminders: false,
            reminder_index: 0,
            form_state: None,
            status_message: None,
            store,
        };

        app.access_granted = app.store.request_access()?;
        if app.access_granted {
            app.calendars = app.store.calendars();
            app.refresh_events();
        }

        Ok(app)
    }

    pub fn refresh_events(&mut self) {
        let year = self.selected_date.year();
        let month = self.selected_date.month();

        self.month_events = self.store.events_for_month(year, month);
        self.day_events = self.store.events_for_date(self.selected_date);
        self.week_events = self.store.events_for_week(self.selected_date);
        self.day_scroll = 0;

        self.days_with_events.clear();
        for ev in &self.month_events {
            let ev_date = ev.start.date_naive();
            if ev_date.year() == year && ev_date.month() == month {
                self.days_with_events.insert(ev_date.day());
            }
        }
    }

    pub fn refresh_reminders(&mut self) {
        self.reminders = self.store.fetch_incomplete_reminders();
        // Sort by calendar name, then by due date
        self.reminders.sort_by(|a, b| {
            a.calendar_name
                .cmp(&b.calendar_name)
                .then(a.due_date.cmp(&b.due_date))
        });
    }

    pub fn week_start(&self) -> NaiveDate {
        let days_since_sunday = self.selected_date.weekday().num_days_from_sunday();
        self.selected_date - chrono::Duration::days(days_since_sunday as i64)
    }

    // ── Navigation ──

    pub fn next_day(&mut self) {
        self.selected_date = self.selected_date.succ_opt().unwrap_or(self.selected_date);
        self.on_date_changed();
    }

    pub fn prev_day(&mut self) {
        self.selected_date = self.selected_date.pred_opt().unwrap_or(self.selected_date);
        self.on_date_changed();
    }

    pub fn next_week(&mut self) {
        self.selected_date += chrono::Duration::weeks(1);
        self.on_date_changed();
    }

    pub fn prev_week(&mut self) {
        self.selected_date -= chrono::Duration::weeks(1);
        self.on_date_changed();
    }

    pub fn next_month(&mut self) {
        let month = self.selected_date.month();
        let year = self.selected_date.year();
        let (new_year, new_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let day = self.selected_date.day().min(days_in_month(new_year, new_month));
        self.selected_date = NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap();
        self.on_date_changed();
    }

    pub fn prev_month(&mut self) {
        let month = self.selected_date.month();
        let year = self.selected_date.year();
        let (new_year, new_month) = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
        let day = self.selected_date.day().min(days_in_month(new_year, new_month));
        self.selected_date = NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap();
        self.on_date_changed();
    }

    pub fn go_to_today(&mut self) {
        self.today = Local::now().date_naive();
        self.selected_date = self.today;
        self.on_date_changed();
    }

    pub fn scroll_day_down(&mut self) {
        if self.day_scroll < self.day_events.len().saturating_sub(1) {
            self.day_scroll += 1;
        }
    }

    pub fn scroll_day_up(&mut self) {
        self.day_scroll = self.day_scroll.saturating_sub(1);
    }

    // ── Reminders ──

    pub fn toggle_reminder_panel(&mut self) {
        self.show_reminders = !self.show_reminders;
        if self.show_reminders {
            self.refresh_reminders();
            self.input_mode = InputMode::Reminders;
        } else {
            self.input_mode = InputMode::Normal;
        }
    }

    pub fn reminder_next(&mut self) {
        if !self.reminders.is_empty() {
            self.reminder_index = (self.reminder_index + 1) % self.reminders.len();
        }
    }

    pub fn reminder_prev(&mut self) {
        if !self.reminders.is_empty() {
            self.reminder_index = self
                .reminder_index
                .checked_sub(1)
                .unwrap_or(self.reminders.len() - 1);
        }
    }

    pub fn toggle_selected_reminder(&mut self) {
        if let Some(reminder) = self.reminders.get(self.reminder_index) {
            let id = reminder.id.clone();
            match self.store.toggle_reminder(&id) {
                Ok(new_state) => {
                    let action = if new_state { "completed" } else { "uncompleted" };
                    self.status_message = Some(format!("Reminder {}", action));
                    self.refresh_reminders();
                    if self.reminder_index >= self.reminders.len() && !self.reminders.is_empty() {
                        self.reminder_index = self.reminders.len() - 1;
                    }
                }
                Err(e) => {
                    self.status_message = Some(format!("Error: {}", e));
                }
            }
        }
    }

    // ── Event form ──

    pub fn open_event_form(&mut self) {
        self.form_state = Some(EventFormState::new(self.selected_date));
        self.input_mode = InputMode::Form;
    }

    pub fn close_event_form(&mut self) {
        self.form_state = None;
        self.input_mode = if self.show_reminders {
            InputMode::Reminders
        } else {
            InputMode::Normal
        };
    }

    pub fn submit_event_form(&mut self) {
        let form = match &self.form_state {
            Some(f) if f.is_valid() => f.clone(),
            Some(_) => {
                self.status_message = Some("Invalid form data".to_string());
                return;
            }
            None => return,
        };

        let date = form.parsed_date().unwrap();
        let start_time = form.parsed_start_time().unwrap_or(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let end_time = form.parsed_end_time().unwrap_or(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        let cal_id = self.calendars.get(form.calendar_index).map(|c| c.id.as_str());

        match self.store.create_event(
            &form.title,
            date,
            start_time,
            end_time,
            form.is_all_day,
            cal_id,
        ) {
            Ok(()) => {
                self.status_message = Some(format!("Created: {}", form.title));
                self.close_event_form();
                self.refresh_events();
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    pub fn form_tab(&mut self) {
        if let Some(ref mut form) = self.form_state {
            form.active_field = form.active_field.next();
        }
    }

    pub fn form_backtab(&mut self) {
        if let Some(ref mut form) = self.form_state {
            form.active_field = form.active_field.prev();
        }
    }

    pub fn form_input_char(&mut self, c: char) {
        if let Some(ref mut form) = self.form_state {
            match form.active_field {
                FormField::AllDay => form.toggle_all_day(),
                FormField::Calendar => form.next_calendar(self.calendars.len()),
                _ => form.input_char(c),
            }
        }
    }

    pub fn form_backspace(&mut self) {
        if let Some(ref mut form) = self.form_state {
            form.backspace();
        }
    }

    // ── Event deletion ──

    pub fn delete_selected_event(&mut self) {
        if self.day_events.is_empty() {
            return;
        }
        let idx = self.day_scroll.min(self.day_events.len().saturating_sub(1));
        let event_id = self.day_events[idx].id.clone();
        let event_title = self.day_events[idx].title.clone();

        match self.store.delete_event(&event_id) {
            Ok(()) => {
                self.status_message = Some(format!("Deleted: {}", event_title));
                self.refresh_events();
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    // ── Internal ──

    fn on_date_changed(&mut self) {
        let old_month = self.month_events.first().map(|e| e.start.date_naive().month());
        let new_month = self.selected_date.month();
        self.day_scroll = 0;

        if old_month != Some(new_month) || self.month_events.is_empty() {
            self.refresh_events();
        } else {
            self.day_events = self.store.events_for_date(self.selected_date);
            self.week_events = self.store.events_for_week(self.selected_date);
        }
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
    .num_days() as u32
}
