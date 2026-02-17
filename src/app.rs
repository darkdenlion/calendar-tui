use std::collections::HashSet;

use chrono::{Datelike, Local, NaiveDate};
use color_eyre::Result;

use crate::calendar::{CalendarEvent, CalendarInfo, Store};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Month,
    Week,
    Day,
}

pub struct App {
    pub running: bool,
    pub view_mode: ViewMode,
    pub selected_date: NaiveDate,
    pub today: NaiveDate,
    pub calendars: Vec<CalendarInfo>,
    pub month_events: Vec<CalendarEvent>,
    pub day_events: Vec<CalendarEvent>,
    pub days_with_events: HashSet<u32>,
    pub access_granted: bool,
    store: Store,
}

impl App {
    pub fn new() -> Result<Self> {
        let store = Store::new()?;
        let today = Local::now().date_naive();

        let mut app = Self {
            running: true,
            view_mode: ViewMode::Month,
            selected_date: today,
            today,
            calendars: Vec::new(),
            month_events: Vec::new(),
            day_events: Vec::new(),
            days_with_events: HashSet::new(),
            access_granted: false,
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

        self.days_with_events.clear();
        for ev in &self.month_events {
            let ev_date = ev.start.date_naive();
            if ev_date.year() == year && ev_date.month() == month {
                self.days_with_events.insert(ev_date.day());
            }
        }
    }

    pub fn next_day(&mut self) {
        self.selected_date = self
            .selected_date
            .succ_opt()
            .unwrap_or(self.selected_date);
        self.on_date_changed();
    }

    pub fn prev_day(&mut self) {
        self.selected_date = self
            .selected_date
            .pred_opt()
            .unwrap_or(self.selected_date);
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
        let (new_year, new_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let day = self
            .selected_date
            .day()
            .min(days_in_month(new_year, new_month));
        self.selected_date =
            NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap();
        self.on_date_changed();
    }

    pub fn prev_month(&mut self) {
        let month = self.selected_date.month();
        let year = self.selected_date.year();
        let (new_year, new_month) = if month == 1 {
            (year - 1, 12)
        } else {
            (year, month - 1)
        };
        let day = self
            .selected_date
            .day()
            .min(days_in_month(new_year, new_month));
        self.selected_date =
            NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap();
        self.on_date_changed();
    }

    pub fn go_to_today(&mut self) {
        self.today = Local::now().date_naive();
        self.selected_date = self.today;
        self.on_date_changed();
    }

    fn on_date_changed(&mut self) {
        let old_month = self.month_events.first().map(|e| e.start.date_naive().month());
        let new_month = self.selected_date.month();

        // Only refetch month events if the month changed
        if old_month != Some(new_month) || self.month_events.is_empty() {
            self.refresh_events();
        } else {
            self.day_events = self.store.events_for_date(self.selected_date);
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
