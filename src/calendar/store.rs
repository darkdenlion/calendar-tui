use std::sync::mpsc;

use block2::RcBlock;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use color_eyre::eyre::{eyre, Result};
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_event_kit::{
    EKAuthorizationStatus, EKEntityType, EKEvent, EKEventStore,
};
use objc2_foundation::{NSDate, NSError};
use ratatui::style::Color;

use super::calendar::CalendarInfo;
use super::event::CalendarEvent;

/// Seconds between Unix epoch (1970-01-01) and NSDate reference date (2001-01-01)
const NSDATE_UNIX_OFFSET: f64 = 978307200.0;

pub struct Store {
    store: Retained<EKEventStore>,
}

impl Store {
    pub fn new() -> Result<Self> {
        let store = unsafe { EKEventStore::new() };
        Ok(Self { store })
    }

    pub fn authorization_status() -> EKAuthorizationStatus {
        unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) }
    }

    pub fn request_access(&self) -> Result<bool> {
        let status = Self::authorization_status();

        match status {
            EKAuthorizationStatus::FullAccess => return Ok(true),
            EKAuthorizationStatus::Denied | EKAuthorizationStatus::Restricted => {
                return Ok(false);
            }
            _ => {}
        }

        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            let _ = tx.send(granted.as_bool());
        });

        unsafe {
            self.store
                .requestFullAccessToEventsWithCompletion(&*block as *const _ as *mut _);
        }

        let granted = rx
            .recv()
            .map_err(|_| eyre!("Failed to receive calendar access response"))?;
        Ok(granted)
    }

    pub fn calendars(&self) -> Vec<CalendarInfo> {
        let ek_calendars = unsafe {
            self.store
                .calendarsForEntityType(EKEntityType::Event)
        };

        let mut calendars = Vec::new();
        let count = ek_calendars.len();

        for i in 0..count {
            let cal = ek_calendars.objectAtIndex(i);
            let id = unsafe { cal.calendarIdentifier().to_string() };
            let title = unsafe { cal.title().to_string() };
            let source = unsafe {
                cal.source()
                    .map(|s| s.title().to_string())
                    .unwrap_or_default()
            };
            let color = calendar_color(&cal);

            calendars.push(CalendarInfo {
                id,
                title,
                color,
                source,
            });
        }

        calendars
    }

    pub fn events_for_date(&self, date: NaiveDate) -> Vec<CalendarEvent> {
        let start_of_day = date
            .and_hms_opt(0, 0, 0)
            .expect("valid time");
        let end_of_day = date
            .and_hms_opt(23, 59, 59)
            .expect("valid time");

        let start_dt = Local
            .from_local_datetime(&start_of_day)
            .single()
            .expect("valid local datetime");
        let end_dt = Local
            .from_local_datetime(&end_of_day)
            .single()
            .expect("valid local datetime");

        self.events_in_range(start_dt, end_dt)
    }

    pub fn events_for_month(&self, year: i32, month: u32) -> Vec<CalendarEvent> {
        let start = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
        let end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid date")
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid date")
        };

        let start_dt = Local
            .from_local_datetime(&start.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .expect("valid local datetime");
        let end_dt = Local
            .from_local_datetime(&end.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .expect("valid local datetime");

        self.events_in_range(start_dt, end_dt)
    }

    fn events_in_range(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Vec<CalendarEvent> {
        let ns_start = datetime_to_nsdate(&start);
        let ns_end = datetime_to_nsdate(&end);

        let predicate = unsafe {
            self.store
                .predicateForEventsWithStartDate_endDate_calendars(
                    &ns_start, &ns_end, None,
                )
        };

        let ek_events = unsafe { self.store.eventsMatchingPredicate(&predicate) };
        let count = ek_events.len();
        let mut events = Vec::new();

        for i in 0..count {
            let ev = ek_events.objectAtIndex(i);
            if let Some(event) = convert_event(&ev) {
                events.push(event);
            }
        }

        events.sort_by_key(|e| e.start);
        events
    }
}

fn convert_event(ev: &EKEvent) -> Option<CalendarEvent> {
    let id = unsafe {
        ev.eventIdentifier()
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let title = unsafe { ev.title().to_string() };
    let start = unsafe { nsdate_to_datetime(&ev.startDate()) };
    let end = unsafe { nsdate_to_datetime(&ev.endDate()) };
    let is_all_day = unsafe { ev.isAllDay() };
    let location = unsafe { ev.location().map(|s| s.to_string()) };
    let notes = unsafe { ev.notes().map(|s| s.to_string()) };
    let (calendar_name, calendar_color) = unsafe {
        ev.calendar()
            .map(|cal| (cal.title().to_string(), calendar_color(&cal)))
            .unwrap_or(("Unknown".to_string(), Color::White))
    };

    Some(CalendarEvent {
        id,
        title,
        start,
        end,
        is_all_day,
        calendar_name,
        calendar_color,
        location,
        notes,
    })
}

fn calendar_color(cal: &objc2_event_kit::EKCalendar) -> Color {
    unsafe {
        if let Some(cg_color) = cal.CGColor() {
            use objc2::msg_send;
            let num_components: usize = msg_send![&*cg_color, numberOfComponents];
            if num_components >= 3 {
                let components: *const f64 = msg_send![&*cg_color, components];
                let r = *components;
                let g = *components.add(1);
                let b = *components.add(2);
                return Color::Rgb(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                );
            }
        }
    }
    Color::White
}

fn datetime_to_nsdate(dt: &DateTime<Local>) -> Retained<NSDate> {
    let unix_ts = dt.timestamp() as f64;
    let nsdate_ts = unix_ts - NSDATE_UNIX_OFFSET;
    NSDate::dateWithTimeIntervalSinceReferenceDate(nsdate_ts)
}

fn nsdate_to_datetime(date: &NSDate) -> DateTime<Local> {
    let nsdate_ts = date.timeIntervalSinceReferenceDate();
    let unix_ts = (nsdate_ts + NSDATE_UNIX_OFFSET) as i64;
    Local
        .timestamp_opt(unix_ts, 0)
        .single()
        .unwrap_or_else(|| Local::now())
}
