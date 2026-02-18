use chrono::{Local, NaiveDate, Timelike};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::calendar::CalendarEvent;
use crate::theme;

const HOUR_START: u32 = 6;
const HOUR_END: u32 = 23;

pub struct WeekView;

impl WeekView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        selected_date: NaiveDate,
        today: NaiveDate,
        week_start: NaiveDate,
        events: &[CalendarEvent],
    ) {
        let block = Block::default()
            .title(format!(
                " Week of {} ",
                week_start.format("%b %d, %Y")
            ))
            .title_style(theme::current().header)
            .borders(Borders::ALL)
            .border_style(theme::current().border);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 10 || inner.height < 3 {
            return;
        }

        let inner_w = inner.width as usize;
        let inner_h = inner.height as usize;

        // Time label column width
        let time_col_w: u16 = if inner_w >= 70 { 6 } else { 4 };
        let day_cols_w = inner.width.saturating_sub(time_col_w);
        let col_w = (day_cols_w / 7).max(1);

        // Layout: time label | 7 day columns
        let mut col_constraints = vec![Constraint::Length(time_col_w)];
        for _ in 0..7 {
            col_constraints.push(Constraint::Length(col_w));
        }
        col_constraints.push(Constraint::Min(0)); // absorb remainder

        let cols = Layout::horizontal(col_constraints).split(inner);

        // Determine visible hours based on height
        // Reserve 1 row for day headers
        let content_rows = inner_h.saturating_sub(1);
        let total_hours = (HOUR_END - HOUR_START) as usize;
        let rows_per_hour = (content_rows / total_hours).max(1);
        let visible_hours = (content_rows / rows_per_hour).min(total_hours);
        let hour_start = HOUR_START;
        // Row layout: header + hour rows
        let mut row_constraints = vec![Constraint::Length(1)]; // day header
        for _ in 0..visible_hours {
            row_constraints.push(Constraint::Length(rows_per_hour as u16));
        }
        row_constraints.push(Constraint::Min(0));

        let rows = Layout::vertical(row_constraints).split(inner);

        // Render day headers
        for day_offset in 0..7u32 {
            let date = week_start + chrono::Duration::days(day_offset as i64);
            let col_idx = (day_offset + 1) as usize;
            if col_idx >= cols.len() {
                break;
            }

            let day_label = if col_w >= 10 {
                format!("{}", date.format("%a %d"))
            } else if col_w >= 5 {
                format!("{}", date.format("%a"))
            } else {
                format!("{}", date.format("%d"))
            };

            let t = theme::current();
            let style = if date == today && date == selected_date {
                t.today.add_modifier(Modifier::BOLD)
            } else if date == selected_date {
                t.selected
            } else if date == today {
                t.today
            } else {
                t.header
            };

            let label = Paragraph::new(Line::from(Span::styled(
                format!("{:^width$}", day_label, width = col_w as usize),
                style,
            )));
            frame.render_widget(label, cols[col_idx].intersection(rows[0]));
        }

        // Render time labels and grid
        for hour_idx in 0..visible_hours {
            let hour = hour_start + hour_idx as u32;
            let row_idx = hour_idx + 1;
            if row_idx >= rows.len() {
                break;
            }

            // Time label
            let time_label = if time_col_w >= 6 {
                format!("{:>2}:00 ", hour)
            } else {
                format!("{:>2} ", hour)
            };
            let time_para = Paragraph::new(Line::from(Span::styled(
                time_label,
                theme::current().dim,
            )));
            frame.render_widget(time_para, cols[0].intersection(rows[row_idx]));

            // Render events for each day column
            for day_offset in 0..7u32 {
                let date = week_start + chrono::Duration::days(day_offset as i64);
                let col_idx = (day_offset + 1) as usize;
                if col_idx >= cols.len() {
                    break;
                }

                let cell_area = cols[col_idx].intersection(rows[row_idx]);
                if cell_area.width == 0 || cell_area.height == 0 {
                    continue;
                }

                // Find events that overlap this hour on this day
                let cell_events: Vec<&CalendarEvent> = events
                    .iter()
                    .filter(|ev| {
                        let ev_date = ev.start.date_naive();
                        if ev_date != date && !ev.is_all_day {
                            // Check if multi-day or the event spans into this day
                            let ev_end_date = ev.end.date_naive();
                            if ev_end_date < date || ev_date > date {
                                return false;
                            }
                        } else if ev_date != date {
                            return false;
                        }

                        if ev.is_all_day {
                            return hour == hour_start; // show all-day at top
                        }

                        let ev_start_hour = ev.start.hour();
                        let ev_end_hour = if ev.end.minute() > 0 {
                            ev.end.hour()
                        } else {
                            ev.end.hour().saturating_sub(1)
                        };
                        hour >= ev_start_hour && hour <= ev_end_hour
                    })
                    .collect();

                if !cell_events.is_empty() {
                    let ev = cell_events[0];
                    let max_title_len = cell_area.width as usize;
                    let title: String = ev.title.chars().take(max_title_len).collect();
                    let display = format!("{:<width$}", title, width = max_title_len);

                    let style = Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(ev.calendar_color);

                    let lines: Vec<Line> = vec![Line::from(Span::styled(display, style))];
                    // Fill remaining rows of the cell if rows_per_hour > 1
                    let para = Paragraph::new(lines);
                    frame.render_widget(para, cell_area);
                }
            }
        }

        // ── Current time indicator ──
        let now = Local::now();
        let now_date = now.date_naive();
        let now_hour = now.hour();
        let now_minute = now.minute();

        // Check if today is in this week and current hour is visible
        if now_date >= week_start
            && now_date < week_start + chrono::Duration::days(7)
            && now_hour >= hour_start
            && now_hour < hour_start + visible_hours as u32
        {
            let day_offset = (now_date - week_start).num_days() as usize;
            let hour_idx = (now_hour - hour_start) as usize;
            let row_idx = hour_idx + 1;

            if row_idx < rows.len() {
                let row_area = rows[row_idx];
                // Calculate sub-row position based on minute
                let y_offset = (now_minute as u16 * row_area.height) / 60;
                let line_y = row_area.y + y_offset.min(row_area.height.saturating_sub(1));

                // Draw across the today column
                let col_idx = day_offset + 1;
                if col_idx < cols.len() {
                    let col_area = cols[col_idx];
                    let now_line_area = Rect::new(col_area.x, line_y, col_area.width, 1);
                    let now_indicator = Paragraph::new(Line::from(Span::styled(
                        "\u{2500}".repeat(col_area.width as usize),
                        Style::default().fg(ratatui::style::Color::Red),
                    )));
                    frame.render_widget(now_indicator, now_line_area);
                }

                // Also mark the time label
                let time_area = cols[0];
                let label_area = Rect::new(time_area.x, line_y, time_area.width, 1);
                let now_time_str = format!("{:>2}:{:02}", now_hour, now_minute);
                let padded = format!("{:<width$}", now_time_str, width = time_area.width as usize);
                let time_label = Paragraph::new(Line::from(Span::styled(
                    padded,
                    Style::default()
                        .fg(ratatui::style::Color::Red)
                        .add_modifier(Modifier::BOLD),
                )));
                frame.render_widget(time_label, label_area);
            }
        }
    }
}
