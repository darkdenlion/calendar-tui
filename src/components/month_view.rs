use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme;

const DAY_NAMES_SHORT: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];
const DAY_NAMES_MED: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub struct MonthView;

impl MonthView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        selected_date: NaiveDate,
        today: NaiveDate,
        days_with_events: &HashSet<u32>,
        days_with_reminders: &HashSet<u32>,
    ) {
        let year = selected_date.year();
        let month = selected_date.month();
        let w = area.width as usize;

        // Adaptive cell width based on available space
        // border takes 2 chars, 7 columns needed
        let inner_w = w.saturating_sub(2);
        let cell_w = (inner_w / 7).max(2);
        let compact = cell_w < 4;

        let title = if w >= 22 {
            format!(" {} {} ", month_name(month), year)
        } else {
            format!(" {}/{} ", month, year)
        };

        let block = Block::default()
            .title(title)
            .title_style(theme::HEADER_STYLE)
            .borders(Borders::ALL)
            .border_style(theme::BORDER_STYLE);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Header row
        let day_names = if compact { &DAY_NAMES_SHORT } else { &DAY_NAMES_MED };
        let header_cells: Vec<Span> = day_names
            .iter()
            .map(|d| {
                let formatted = if compact {
                    format!("{:^width$}", d, width = cell_w)
                } else {
                    format!("{:^width$}", d, width = cell_w)
                };
                Span::styled(formatted, theme::HEADER_STYLE)
            })
            .collect();
        let header = Line::from(header_cells);

        // Calculate grid
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first_day.weekday().num_days_from_sunday() as usize;
        let dim = days_in_month(year, month);

        // Build weeks
        let mut weeks: Vec<Line> = Vec::new();
        let mut current_day: i32 = 1 - first_weekday as i32;

        while current_day <= dim as i32 {
            let mut cells: Vec<Span> = Vec::new();
            for _ in 0..7 {
                if current_day < 1 || current_day > dim as i32 {
                    cells.push(Span::raw(" ".repeat(cell_w)));
                } else {
                    let day = current_day as u32;
                    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                    let has_event = days_with_events.contains(&day);
                    let has_reminder = days_with_reminders.contains(&day);

                    // Marker: * for events, . for reminders, + for both
                    let marker = match (has_event, has_reminder) {
                        (true, true) => "+",
                        (true, false) => "*",
                        (false, true) => ".",
                        (false, false) => " ",
                    };

                    let day_str = if compact {
                        if marker != " " {
                            format!("{:>width$}", format!("{}{}", day, marker), width = cell_w)
                        } else {
                            format!("{:>width$}", day, width = cell_w)
                        }
                    } else {
                        let num = format!("{:>2}{}", day, marker);
                        format!("{:^width$}", num, width = cell_w)
                    };

                    let style = if date == today && date == selected_date {
                        Style::default()
                            .fg(ratatui::style::Color::Black)
                            .bg(ratatui::style::Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else if date == selected_date {
                        theme::SELECTED_STYLE
                    } else if date == today {
                        theme::TODAY_STYLE
                    } else {
                        Style::default()
                    };

                    cells.push(Span::styled(day_str, style));
                }
                current_day += 1;
            }
            weeks.push(Line::from(cells));
        }

        // Layout: header + weeks, adapt row height to fill space
        let available_rows = inner.height as usize;
        let total_rows = 1 + weeks.len(); // header + weeks
        let row_height = if available_rows > total_rows {
            (available_rows / total_rows).max(1)
        } else {
            1
        };

        let mut constraints = vec![Constraint::Length(1)]; // header always 1 row
        for _ in &weeks {
            constraints.push(Constraint::Length(row_height as u16));
        }
        constraints.push(Constraint::Min(0));

        let rows = Layout::vertical(constraints).split(inner);

        frame.render_widget(Paragraph::new(header), rows[0]);
        for (i, week) in weeks.iter().enumerate() {
            if i + 1 < rows.len() {
                frame.render_widget(Paragraph::new(week.clone()), rows[i + 1]);
            }
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

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}
