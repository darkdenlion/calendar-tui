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

const DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub struct MonthView;

impl MonthView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        selected_date: NaiveDate,
        today: NaiveDate,
        days_with_events: &HashSet<u32>,
    ) {
        let year = selected_date.year();
        let month = selected_date.month();

        let title = format!(
            " {} {} ",
            month_name(month),
            year
        );

        let block = Block::default()
            .title(title)
            .title_style(theme::HEADER_STYLE)
            .borders(Borders::ALL)
            .border_style(theme::BORDER_STYLE);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Header row
        let header_cells: Vec<Span> = DAY_NAMES
            .iter()
            .map(|d| Span::styled(format!("{:^5}", d), theme::HEADER_STYLE))
            .collect();
        let header = Line::from(header_cells);

        // Calculate grid
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first_day.weekday().num_days_from_sunday() as usize;
        let days_in_month = days_in_month(year, month);

        // Build weeks
        let mut weeks: Vec<Line> = Vec::new();
        let mut current_day: i32 = 1 - first_weekday as i32;

        while current_day <= days_in_month as i32 {
            let mut cells: Vec<Span> = Vec::new();
            for _ in 0..7 {
                if current_day < 1 || current_day > days_in_month as i32 {
                    cells.push(Span::raw("     "));
                } else {
                    let day = current_day as u32;
                    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                    let has_event = days_with_events.contains(&day);

                    let day_str = if has_event {
                        format!("{:>2}* ", day)
                    } else {
                        format!("{:>2}  ", day)
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
                    } else if date.month() != month {
                        theme::DIM_STYLE
                    } else {
                        Style::default()
                    };

                    cells.push(Span::styled(format!(" {}", day_str), style));
                }
                current_day += 1;
            }
            weeks.push(Line::from(cells));
        }

        // Layout: header + weeks
        let mut constraints = vec![Constraint::Length(1)]; // header
        for _ in &weeks {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(0)); // fill remaining

        let rows = Layout::vertical(constraints).split(inner);

        frame.render_widget(Paragraph::new(header), rows[0]);
        for (i, week) in weeks.iter().enumerate() {
            frame.render_widget(Paragraph::new(week.clone()), rows[i + 1]);
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
