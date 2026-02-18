use std::sync::mpsc;

use block2::RcBlock;
use chrono::{Datelike, DateTime, Local, NaiveDate, NaiveTime, TimeZone};
use color_eyre::eyre::{eyre, Result};
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_event_kit::{
    EKAuthorizationStatus, EKEntityType, EKEvent, EKEventStore, EKReminder, EKSpan,
};
use objc2_foundation::{NSArray, NSDate, NSError, NSRunLoop, NSString};
use ratatui::style::Color;

use super::calendar::CalendarInfo;
use super::event::CalendarEvent;
use super::reminder::Reminder;

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

    #[allow(dead_code)]
    pub fn reminder_authorization_status() -> EKAuthorizationStatus {
        unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Reminder) }
    }

    pub fn request_access(&self) -> Result<bool> {
        let event_access = self.request_entity_access(|store, block| unsafe {
            store.requestFullAccessToEventsWithCompletion(block);
        })?;

        // Also request reminder access
        let _reminder_access = self.request_entity_access(|store, block| unsafe {
            store.requestFullAccessToRemindersWithCompletion(block);
        })?;

        Ok(event_access)
    }

    fn request_entity_access(
        &self,
        request_fn: impl FnOnce(&EKEventStore, *mut block2::Block<dyn Fn(Bool, *mut NSError)>),
    ) -> Result<bool> {
        // Check if already authorized for events
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

        request_fn(&self.store, &*block as *const _ as *mut _);

        let run_loop = NSRunLoop::currentRunLoop();
        loop {
            match rx.try_recv() {
                Ok(granted) => return Ok(granted),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(eyre!("Access request failed"));
                }
                Err(mpsc::TryRecvError::Empty) => {
                    let until = NSDate::dateWithTimeIntervalSinceNow(0.1);
                    let _ran = unsafe {
                        run_loop.runMode_beforeDate(
                            objc2_foundation::NSDefaultRunLoopMode,
                            &until,
                        )
                    };
                }
            }
        }
    }

    // ── Calendar queries ──

    pub fn calendars(&self) -> Vec<CalendarInfo> {
        let ek_calendars = unsafe {
            self.store.calendarsForEntityType(EKEntityType::Event)
        };
        convert_calendars(&ek_calendars)
    }

    #[allow(dead_code)]
    pub fn reminder_calendars(&self) -> Vec<CalendarInfo> {
        let ek_calendars = unsafe {
            self.store.calendarsForEntityType(EKEntityType::Reminder)
        };
        convert_calendars(&ek_calendars)
    }

    // ── Event queries ──

    pub fn events_for_date(&self, date: NaiveDate) -> Vec<CalendarEvent> {
        let start_of_day = date.and_hms_opt(0, 0, 0).expect("valid time");
        let end_of_day = date.and_hms_opt(23, 59, 59).expect("valid time");

        let start_dt = Local.from_local_datetime(&start_of_day).single().expect("valid");
        let end_dt = Local.from_local_datetime(&end_of_day).single().expect("valid");

        self.events_in_range(start_dt, end_dt)
    }

    pub fn events_for_week(&self, date: NaiveDate) -> Vec<CalendarEvent> {
        let days_since_sunday = date.weekday().num_days_from_sunday();
        let week_start = date - chrono::Duration::days(days_since_sunday as i64);
        let week_end = week_start + chrono::Duration::days(7);

        let start_dt = Local.from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
            .single().expect("valid");
        let end_dt = Local.from_local_datetime(&week_end.and_hms_opt(0, 0, 0).unwrap())
            .single().expect("valid");

        self.events_in_range(start_dt, end_dt)
    }

    pub fn events_for_month(&self, year: i32, month: u32) -> Vec<CalendarEvent> {
        let start = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
        let end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid date")
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid date")
        };

        let start_dt = Local.from_local_datetime(&start.and_hms_opt(0, 0, 0).unwrap())
            .single().expect("valid");
        let end_dt = Local.from_local_datetime(&end.and_hms_opt(0, 0, 0).unwrap())
            .single().expect("valid");

        self.events_in_range(start_dt, end_dt)
    }

    fn events_in_range(&self, start: DateTime<Local>, end: DateTime<Local>) -> Vec<CalendarEvent> {
        let ns_start = datetime_to_nsdate(&start);
        let ns_end = datetime_to_nsdate(&end);

        let predicate = unsafe {
            self.store.predicateForEventsWithStartDate_endDate_calendars(
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

    // ── Event write operations ──

    pub fn create_event(
        &self,
        title: &str,
        date: NaiveDate,
        start_time: NaiveTime,
        end_time: NaiveTime,
        is_all_day: bool,
        calendar_id: Option<&str>,
    ) -> Result<()> {
        let event = unsafe { EKEvent::eventWithEventStore(&self.store) };

        let ns_title = NSString::from_str(title);
        unsafe { event.setTitle(Some(&ns_title)) };

        if is_all_day {
            unsafe { event.setAllDay(true) };
            let start = date.and_hms_opt(0, 0, 0).unwrap();
            let start_dt = Local.from_local_datetime(&start).single().expect("valid");
            let ns_start = datetime_to_nsdate(&start_dt);
            unsafe {
                event.setStartDate(Some(&ns_start));
                event.setEndDate(Some(&ns_start));
            };
        } else {
            let start = date.and_time(start_time);
            let end = date.and_time(end_time);
            let start_dt = Local.from_local_datetime(&start).single().expect("valid");
            let end_dt = Local.from_local_datetime(&end).single().expect("valid");
            let ns_start = datetime_to_nsdate(&start_dt);
            let ns_end = datetime_to_nsdate(&end_dt);
            unsafe {
                event.setStartDate(Some(&ns_start));
                event.setEndDate(Some(&ns_end));
            };
        }

        // Set calendar
        if let Some(cal_id) = calendar_id {
            let ns_cal_id = NSString::from_str(cal_id);
            if let Some(cal) = unsafe { self.store.calendarWithIdentifier(&ns_cal_id) } {
                unsafe { event.setCalendar(Some(&cal)) };
            }
        } else if let Some(default_cal) = unsafe { self.store.defaultCalendarForNewEvents() } {
            unsafe { event.setCalendar(Some(&default_cal)) };
        }

        unsafe {
            self.store.saveEvent_span_error(&event, EKSpan::ThisEvent)
                .map_err(|e| eyre!("Failed to save event: {:?}", e))?;
        }

        Ok(())
    }

    pub fn delete_event(&self, event_id: &str) -> Result<()> {
        let ns_id = NSString::from_str(event_id);
        let event = unsafe { self.store.eventWithIdentifier(&ns_id) }
            .ok_or_else(|| eyre!("Event not found"))?;

        unsafe {
            self.store.removeEvent_span_error(&event, EKSpan::ThisEvent)
                .map_err(|e| eyre!("Failed to delete event: {:?}", e))?;
        }

        Ok(())
    }

    // ── Reminder queries ──

    #[allow(dead_code)]
    pub fn fetch_reminders(&self) -> Vec<Reminder> {
        let predicate = unsafe {
            self.store.predicateForRemindersInCalendars(None)
        };

        let (tx, rx) = mpsc::channel::<Vec<Reminder>>();

        let block = RcBlock::new(move |reminders_ptr: *mut NSArray<EKReminder>| {
            let mut result = Vec::new();
            if !reminders_ptr.is_null() {
                let reminders = unsafe { &*reminders_ptr };
                let count = reminders.len();
                for i in 0..count {
                    let r = reminders.objectAtIndex(i);
                    if let Some(reminder) = convert_reminder(&r) {
                        result.push(reminder);
                    }
                }
            }
            let _ = tx.send(result);
        });

        unsafe {
            self.store.fetchRemindersMatchingPredicate_completion(
                &predicate,
                &block,
            );
        };

        // Spin run loop waiting for completion
        let run_loop = NSRunLoop::currentRunLoop();
        loop {
            match rx.try_recv() {
                Ok(reminders) => return reminders,
                Err(mpsc::TryRecvError::Disconnected) => return Vec::new(),
                Err(mpsc::TryRecvError::Empty) => {
                    let until = NSDate::dateWithTimeIntervalSinceNow(0.1);
                    let _ran = unsafe {
                        run_loop.runMode_beforeDate(
                            objc2_foundation::NSDefaultRunLoopMode,
                            &until,
                        )
                    };
                }
            }
        }
    }

    pub fn fetch_incomplete_reminders(&self) -> Vec<Reminder> {
        let predicate = unsafe {
            self.store.predicateForIncompleteRemindersWithDueDateStarting_ending_calendars(
                None, None, None,
            )
        };

        let (tx, rx) = mpsc::channel::<Vec<Reminder>>();

        let block = RcBlock::new(move |reminders_ptr: *mut NSArray<EKReminder>| {
            let mut result = Vec::new();
            if !reminders_ptr.is_null() {
                let reminders = unsafe { &*reminders_ptr };
                let count = reminders.len();
                for i in 0..count {
                    let r = reminders.objectAtIndex(i);
                    if let Some(reminder) = convert_reminder(&r) {
                        result.push(reminder);
                    }
                }
            }
            let _ = tx.send(result);
        });

        unsafe {
            self.store.fetchRemindersMatchingPredicate_completion(
                &predicate,
                &block,
            );
        };

        let run_loop = NSRunLoop::currentRunLoop();
        loop {
            match rx.try_recv() {
                Ok(reminders) => return reminders,
                Err(mpsc::TryRecvError::Disconnected) => return Vec::new(),
                Err(mpsc::TryRecvError::Empty) => {
                    let until = NSDate::dateWithTimeIntervalSinceNow(0.1);
                    let _ran = unsafe {
                        run_loop.runMode_beforeDate(
                            objc2_foundation::NSDefaultRunLoopMode,
                            &until,
                        )
                    };
                }
            }
        }
    }

    pub fn fetch_completed_reminders(&self) -> Vec<Reminder> {
        let predicate = unsafe {
            self.store.predicateForCompletedRemindersWithCompletionDateStarting_ending_calendars(
                None, None, None,
            )
        };

        let (tx, rx) = mpsc::channel::<Vec<Reminder>>();

        let block = RcBlock::new(move |reminders_ptr: *mut NSArray<EKReminder>| {
            let mut result = Vec::new();
            if !reminders_ptr.is_null() {
                let reminders = unsafe { &*reminders_ptr };
                let count = reminders.len();
                for i in 0..count {
                    let r = reminders.objectAtIndex(i);
                    if let Some(reminder) = convert_reminder(&r) {
                        result.push(reminder);
                    }
                }
            }
            let _ = tx.send(result);
        });

        unsafe {
            self.store.fetchRemindersMatchingPredicate_completion(
                &predicate,
                &block,
            );
        };

        let run_loop = NSRunLoop::currentRunLoop();
        loop {
            match rx.try_recv() {
                Ok(reminders) => return reminders,
                Err(mpsc::TryRecvError::Disconnected) => return Vec::new(),
                Err(mpsc::TryRecvError::Empty) => {
                    let until = NSDate::dateWithTimeIntervalSinceNow(0.1);
                    let _ran = unsafe {
                        run_loop.runMode_beforeDate(
                            objc2_foundation::NSDefaultRunLoopMode,
                            &until,
                        )
                    };
                }
            }
        }
    }

    // ── Reminder write operations ──

    pub fn toggle_reminder(&self, reminder_id: &str) -> Result<bool> {
        let ns_id = NSString::from_str(reminder_id);
        let item = unsafe { self.store.calendarItemWithIdentifier(&ns_id) }
            .ok_or_else(|| eyre!("Reminder not found"))?;

        // Cast to EKReminder
        let reminder: &EKReminder = unsafe { &*((&*item) as *const _ as *const EKReminder) };

        let current = unsafe { reminder.isCompleted() };
        let new_state = !current;
        unsafe { reminder.setCompleted(new_state) };

        unsafe {
            self.store.saveReminder_commit_error(reminder, true)
                .map_err(|e| eyre!("Failed to save reminder: {:?}", e))?;
        }

        Ok(new_state)
    }
}

// ── Helper functions ──

fn convert_calendars(ek_calendars: &NSArray<objc2_event_kit::EKCalendar>) -> Vec<CalendarInfo> {
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

        calendars.push(CalendarInfo { id, title, color, source });
    }

    calendars
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
        id, title, start, end, is_all_day,
        calendar_name, calendar_color, location, notes,
    })
}

fn convert_reminder(r: &EKReminder) -> Option<Reminder> {
    let id = unsafe {
        // EKReminder inherits from EKCalendarItem which has calendarItemIdentifier
        use objc2::msg_send;
        let id: Retained<NSString> = msg_send![r, calendarItemIdentifier];
        id.to_string()
    };
    let title = unsafe { r.title().to_string() };
    let is_completed = unsafe { r.isCompleted() };
    let priority = unsafe { r.priority() } as u8;

    let due_date = unsafe {
        r.dueDateComponents().and_then(|components| {
            // Extract date components manually
            use objc2::msg_send;
            let year: isize = msg_send![&*components, year];
            let month: isize = msg_send![&*components, month];
            let day: isize = msg_send![&*components, day];

            if year > 0 && month > 0 && day > 0 {
                NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
            } else {
                None
            }
        })
    };

    let (calendar_name, calendar_color) = unsafe {
        r.calendar()
            .map(|cal| (cal.title().to_string(), calendar_color(&cal)))
            .unwrap_or(("Unknown".to_string(), Color::White))
    };

    Some(Reminder {
        id, title, is_completed, due_date,
        calendar_name, calendar_color, priority,
    })
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorGetNumberOfComponents(color: *const std::ffi::c_void) -> usize;
    fn CGColorGetComponents(color: *const std::ffi::c_void) -> *const f64;
}

fn calendar_color(cal: &objc2_event_kit::EKCalendar) -> Color {
    unsafe {
        if let Some(cg_color) = cal.CGColor() {
            let ptr = &*cg_color as *const _ as *const std::ffi::c_void;
            let num_components = CGColorGetNumberOfComponents(ptr);
            if num_components >= 3 {
                let components = CGColorGetComponents(ptr);
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
