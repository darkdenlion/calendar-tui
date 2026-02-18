use chrono::NaiveDate;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::calendar::CalendarInfo;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormField {
    Title,
    Date,
    StartTime,
    EndTime,
    AllDay,
    Calendar,
}

impl FormField {
    pub fn next(&self) -> Self {
        match self {
            FormField::Title => FormField::Date,
            FormField::Date => FormField::StartTime,
            FormField::StartTime => FormField::EndTime,
            FormField::EndTime => FormField::AllDay,
            FormField::AllDay => FormField::Calendar,
            FormField::Calendar => FormField::Title,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            FormField::Title => FormField::Calendar,
            FormField::Date => FormField::Title,
            FormField::StartTime => FormField::Date,
            FormField::EndTime => FormField::StartTime,
            FormField::AllDay => FormField::EndTime,
            FormField::Calendar => FormField::AllDay,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventFormState {
    pub title: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub is_all_day: bool,
    pub calendar_index: usize,
    pub active_field: FormField,
}

impl EventFormState {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            title: String::new(),
            date: date.format("%Y-%m-%d").to_string(),
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
            is_all_day: false,
            calendar_index: 0,
            active_field: FormField::Title,
        }
    }

    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }

    pub fn parsed_start_time(&self) -> Option<chrono::NaiveTime> {
        chrono::NaiveTime::parse_from_str(&self.start_time, "%H:%M").ok()
    }

    pub fn parsed_end_time(&self) -> Option<chrono::NaiveTime> {
        chrono::NaiveTime::parse_from_str(&self.end_time, "%H:%M").ok()
    }

    pub fn input_char(&mut self, c: char) {
        match self.active_field {
            FormField::Title => self.title.push(c),
            FormField::Date => self.date.push(c),
            FormField::StartTime => self.start_time.push(c),
            FormField::EndTime => self.end_time.push(c),
            FormField::AllDay | FormField::Calendar => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.active_field {
            FormField::Title => { self.title.pop(); }
            FormField::Date => { self.date.pop(); }
            FormField::StartTime => { self.start_time.pop(); }
            FormField::EndTime => { self.end_time.pop(); }
            FormField::AllDay | FormField::Calendar => {}
        }
    }

    pub fn toggle_all_day(&mut self) {
        self.is_all_day = !self.is_all_day;
    }

    pub fn next_calendar(&mut self, total: usize) {
        if total > 0 {
            self.calendar_index = (self.calendar_index + 1) % total;
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.title.is_empty()
            && self.parsed_date().is_some()
            && (self.is_all_day
                || (self.parsed_start_time().is_some() && self.parsed_end_time().is_some()))
    }
}

pub struct EventForm;

impl EventForm {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &EventFormState,
        calendars: &[CalendarInfo],
    ) {
        // Center the form popup
        let form_w = area.width.min(50).max(30);
        let form_h = area.height.min(14).max(10);
        let x = area.x + (area.width.saturating_sub(form_w)) / 2;
        let y = area.y + (area.height.saturating_sub(form_h)) / 2;
        let form_area = Rect::new(x, y, form_w, form_h);

        // Clear background
        frame.render_widget(Clear, form_area);

        let block = Block::default()
            .title(" New Event ")
            .title_style(Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ratatui::style::Color::Green));

        let inner = block.inner(form_area);
        frame.render_widget(block, form_area);

        let rows = Layout::vertical([
            Constraint::Length(1), // title
            Constraint::Length(1), // date
            Constraint::Length(1), // start time
            Constraint::Length(1), // end time
            Constraint::Length(1), // all day
            Constraint::Length(1), // calendar
            Constraint::Length(1), // spacer
            Constraint::Length(1), // help
            Constraint::Min(0),
        ])
        .split(inner);

        render_field(frame, rows[0], "Title:", &state.title, state.active_field == FormField::Title);
        render_field(frame, rows[1], "Date:", &state.date, state.active_field == FormField::Date);

        if state.is_all_day {
            render_field(frame, rows[2], "Start:", "--:--", false);
            render_field(frame, rows[3], "End:", "--:--", false);
        } else {
            render_field(frame, rows[2], "Start:", &state.start_time, state.active_field == FormField::StartTime);
            render_field(frame, rows[3], "End:", &state.end_time, state.active_field == FormField::EndTime);
        }

        let all_day_val = if state.is_all_day { "[x] All Day" } else { "[ ] All Day" };
        render_field(frame, rows[4], "", all_day_val, state.active_field == FormField::AllDay);

        let cal_name = calendars
            .get(state.calendar_index)
            .map(|c| c.title.as_str())
            .unwrap_or("Default");
        render_field(frame, rows[5], "Cal:", cal_name, state.active_field == FormField::Calendar);

        let help = Line::from(vec![
            Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(":Next ", theme::current().dim),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(":Save ", theme::current().dim),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(":Cancel", theme::current().dim),
        ]);
        frame.render_widget(Paragraph::new(help), rows[7]);
    }
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool) {
    let label_w = if label.is_empty() { 0 } else { 7 };
    let cursor = if active { "_" } else { "" };

    let style = if active {
        Style::default().fg(ratatui::style::Color::Cyan)
    } else {
        Style::default()
    };

    let mut spans = Vec::new();
    if !label.is_empty() {
        spans.push(Span::styled(
            format!("{:<width$}", label, width = label_w),
            theme::current().dim,
        ));
    }
    spans.push(Span::styled(format!("{}{}", value, cursor), style));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
