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
    #[allow(dead_code)]
    Reminders,
}

/// Identifies what kind of item is at a given scroll position in the day view.
#[derive(Debug, Clone, Copy)]
pub enum DayAction {
    None,
    Event(usize),
    Reminder(usize),
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
    pub days_with_reminders: HashSet<u32>,
    pub access_granted: bool,
    pub day_scroll: usize,
    // Reminders (inline in day view)
    pub reminders: Vec<Reminder>,
    pub day_reminders: Vec<Reminder>,
    // Event form
    pub form_state: Option<EventFormState>,
    // Detail popup (index into day_events or day_reminders via DayAction)
    pub detail_item: Option<DayAction>,
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
            days_with_reminders: HashSet::new(),
            access_granted: false,
            day_scroll: 0,
            reminders: Vec::new(),
            day_reminders: Vec::new(),
            form_state: None,
            detail_item: None,
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
        self.days_with_events.clear();
        for ev in &self.month_events {
            let ev_date = ev.start.date_naive();
            if ev_date.year() == year && ev_date.month() == month {
                self.days_with_events.insert(ev_date.day());
            }
        }

        // Fetch reminders and populate day + month indicators
        self.refresh_reminders();
        self.day_reminders = self.filter_day_reminders();

        // Set scroll to first actionable item after data loads
        self.day_scroll = 0; // temporary, reset after reminders load

        self.days_with_reminders.clear();
        for rem in &self.reminders {
            if let Some(due) = &rem.due_date {
                let due_date = due.date_naive();
                if due_date.year() == year && due_date.month() == month {
                    self.days_with_reminders.insert(due_date.day());
                }
            }
        }

        self.day_scroll = self.first_actionable_scroll();
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
        let len = self.day_list_len();
        if len == 0 {
            return;
        }
        let mut next = self.day_scroll + 1;
        // Skip headers and spacers
        while next < len {
            if !matches!(self.day_action_at(next), DayAction::None) {
                break;
            }
            next += 1;
        }
        if next < len {
            self.day_scroll = next;
        }
    }

    pub fn scroll_day_up(&mut self) {
        if self.day_scroll == 0 {
            return;
        }
        let mut prev = self.day_scroll - 1;
        // Skip headers and spacers
        loop {
            if !matches!(self.day_action_at(prev), DayAction::None) {
                break;
            }
            if prev == 0 {
                // No actionable item above, stay at first actionable
                return;
            }
            prev -= 1;
        }
        self.day_scroll = prev;
    }

    /// Find the first actionable item position in the day list.
    fn first_actionable_scroll(&self) -> usize {
        let len = self.day_list_len();
        for i in 0..len {
            if !matches!(self.day_action_at(i), DayAction::None) {
                return i;
            }
        }
        0
    }

    // ── Reminders (inline in day view) ──

    /// Filter reminders for the selected date — only show reminders due on that exact date.
    fn filter_day_reminders(&self) -> Vec<Reminder> {
        let date = self.selected_date;
        self.reminders
            .iter()
            .filter(|r| match r.due_date {
                Some(due) => due.date_naive() == date,
                None => false,
            })
            .cloned()
            .collect()
    }

    /// Total number of visual items in the day list (headers + items + spacers).
    pub fn day_list_len(&self) -> usize {
        let all_day = self.day_events.iter().filter(|e| e.is_all_day).count();
        let timed = self.day_events.iter().filter(|e| !e.is_all_day).count();
        let rems = self.day_reminders.len();

        let mut len = 0;
        if all_day > 0 {
            len += 1 + all_day; // header + items
            if rems > 0 || timed > 0 {
                len += 1; // spacer
            }
        }
        if rems > 0 {
            len += 1 + rems; // header + items
            if timed > 0 {
                len += 1; // spacer
            }
        }
        len += timed;
        len
    }

    /// Determine what kind of item is at the current scroll position.
    pub fn day_action_at_scroll(&self) -> DayAction {
        self.day_action_at(self.day_scroll)
    }

    /// Determine what kind of item is at the given position.
    pub fn day_action_at(&self, scroll: usize) -> DayAction {
        let all_day_indices: Vec<usize> = self
            .day_events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_all_day)
            .map(|(i, _)| i)
            .collect();
        let timed_indices: Vec<usize> = self
            .day_events
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_all_day)
            .map(|(i, _)| i)
            .collect();
        let rems = self.day_reminders.len();

        let mut pos = 0;

        // All-day section
        if !all_day_indices.is_empty() {
            if scroll == pos {
                return DayAction::None;
            }
            pos += 1; // header
            for &idx in &all_day_indices {
                if scroll == pos {
                    return DayAction::Event(idx);
                }
                pos += 1;
            }
            if rems > 0 || !timed_indices.is_empty() {
                if scroll == pos {
                    return DayAction::None;
                }
                pos += 1; // spacer
            }
        }

        // Reminders section
        if rems > 0 {
            if scroll == pos {
                return DayAction::None;
            }
            pos += 1; // header
            for i in 0..rems {
                if scroll == pos {
                    return DayAction::Reminder(i);
                }
                pos += 1;
            }
            if !timed_indices.is_empty() {
                if scroll == pos {
                    return DayAction::None;
                }
                pos += 1; // spacer
            }
        }

        // Timed events
        for &idx in &timed_indices {
            if scroll == pos {
                return DayAction::Event(idx);
            }
            pos += 1;
        }

        DayAction::None
    }

    /// Toggle the reminder at the current scroll position (if it is a reminder).
    pub fn toggle_day_reminder(&mut self) {
        if let DayAction::Reminder(rem_idx) = self.day_action_at_scroll() {
            if let Some(reminder) = self.day_reminders.get(rem_idx) {
                let id = reminder.id.clone();
                match self.store.toggle_reminder(&id) {
                    Ok(new_state) => {
                        let action = if new_state { "completed" } else { "uncompleted" };
                        self.status_message = Some(format!("Reminder {}", action));
                        self.refresh_reminders();
                        self.day_reminders = self.filter_day_reminders();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
    }

    // ── Detail popup ──

    pub fn show_detail(&mut self) {
        let action = self.day_action_at_scroll();
        match action {
            DayAction::Event(_) | DayAction::Reminder(_) => {
                self.detail_item = Some(action);
            }
            DayAction::None => {}
        }
    }

    pub fn close_detail(&mut self) {
        self.detail_item = None;
    }

    // ── Event form ──

    pub fn open_event_form(&mut self) {
        self.form_state = Some(EventFormState::new(self.selected_date));
        self.input_mode = InputMode::Form;
    }

    pub fn close_event_form(&mut self) {
        self.form_state = None;
        self.input_mode = InputMode::Normal;
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
        if let DayAction::Event(idx) = self.day_action_at_scroll() {
            if let Some(ev) = self.day_events.get(idx) {
                let event_id = ev.id.clone();
                let event_title = ev.title.clone();

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
        }
    }

    // ── Internal ──

    fn on_date_changed(&mut self) {
        let old_month = self.month_events.first().map(|e| e.start.date_naive().month());
        let new_month = self.selected_date.month();

        if old_month != Some(new_month) || self.month_events.is_empty() {
            self.refresh_events();
        } else {
            self.day_events = self.store.events_for_date(self.selected_date);
            self.week_events = self.store.events_for_week(self.selected_date);
            self.day_reminders = self.filter_day_reminders();
            self.day_scroll = self.first_actionable_scroll();
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
