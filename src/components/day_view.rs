use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::DayAction;
use crate::calendar::{CalendarEvent, Reminder};
use crate::theme;

pub struct DayView;

impl DayView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        date: NaiveDate,
        events: &[CalendarEvent],
        reminders: &[Reminder],
        scroll: usize,
    ) {
        let w = area.width as usize;

        let title = if w >= 30 {
            format!(" {} ", date.format("%A, %B %d, %Y"))
        } else if w >= 18 {
            format!(" {} ", date.format("%b %d, %Y"))
        } else {
            format!(" {} ", date.format("%m/%d"))
        };

        let mut counts = Vec::new();
        if !events.is_empty() {
            let n = events.len();
            counts.push(format!("{} event{}", n, if n == 1 { "" } else { "s" }));
        }
        if !reminders.is_empty() {
            let n = reminders.len();
            counts.push(format!("{} reminder{}", n, if n == 1 { "" } else { "s" }));
        }
        let count_str = if counts.is_empty() {
            String::new()
        } else {
            format!(" {} ", counts.join(", "))
        };

        let block = Block::default()
            .title(title)
            .title_style(theme::HEADER_STYLE)
            .title_bottom(Line::from(Span::styled(count_str, theme::DIM_STYLE)))
            .borders(Borders::ALL)
            .border_style(theme::BORDER_STYLE);

        if events.is_empty() && reminders.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let msg = Paragraph::new("No events or reminders").style(theme::DIM_STYLE);
            frame.render_widget(msg, inner);
            return;
        }

        let inner_w = area.width.saturating_sub(2) as usize;

        let all_day: Vec<&CalendarEvent> = events.iter().filter(|e| e.is_all_day).collect();
        let timed: Vec<&CalendarEvent> = events.iter().filter(|e| !e.is_all_day).collect();

        let mut items: Vec<ListItem> = Vec::new();

        // All-day events section
        if !all_day.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "All Day",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ))));
            for ev in &all_day {
                items.push(format_event(ev, inner_w, true));
            }
            if !timed.is_empty() || !reminders.is_empty() {
                items.push(ListItem::new(Line::from("")));
            }
        }

        // Reminders section
        if !reminders.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Reminders",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ))));
            for rem in reminders {
                items.push(format_reminder(rem, inner_w, date));
            }
            if !timed.is_empty() {
                items.push(ListItem::new(Line::from("")));
            }
        }

        // Timed events
        for ev in &timed {
            items.push(format_event(ev, inner_w, false));
        }

        // Apply scroll
        let visible_items: Vec<ListItem> = items.into_iter().skip(scroll).collect();

        let list = List::new(visible_items).block(block);
        frame.render_widget(list, area);
    }
}

fn format_event(ev: &CalendarEvent, max_width: usize, is_all_day: bool) -> ListItem<'static> {
    let cal_indicator = Span::styled("  ", Style::default().bg(ev.calendar_color));

    let time_str = if is_all_day {
        String::new()
    } else {
        format!(" {} ", ev.duration_display())
    };
    let time_span = Span::styled(
        time_str.clone(),
        Style::default().add_modifier(Modifier::DIM),
    );

    let title_span = Span::styled(ev.title.clone(), Style::default());

    let mut spans = vec![cal_indicator, time_span, title_span];

    // Only show location if there's room
    let used = 2 + time_str.len() + ev.title.len();
    if let Some(ref loc) = ev.location {
        if !loc.is_empty() && used + 4 + loc.len() <= max_width {
            spans.push(Span::styled(format!(" @ {}", loc), theme::DIM_STYLE));
        }
    }

    ListItem::new(Line::from(spans))
}

fn format_reminder(
    rem: &Reminder,
    _max_width: usize,
    _current_date: NaiveDate,
) -> ListItem<'static> {
    let cal_indicator = Span::styled("  ", Style::default().bg(rem.calendar_color));

    let checkbox = if rem.is_completed {
        " [x] "
    } else {
        " [ ] "
    };
    let checkbox_span = Span::styled(checkbox, Style::default());

    let title_style = if rem.is_completed {
        Style::default().add_modifier(Modifier::DIM | Modifier::CROSSED_OUT)
    } else {
        Style::default()
    };
    let title_span = Span::styled(rem.title.clone(), title_style);

    let mut spans = vec![cal_indicator, checkbox_span, title_span];

    // Show calendar name for context
    spans.push(Span::styled(
        format!(" ({})", rem.calendar_name),
        theme::DIM_STYLE,
    ));

    ListItem::new(Line::from(spans))
}

/// Render an event/reminder detail popup overlay.
pub fn render_detail_popup(
    frame: &mut Frame,
    area: Rect,
    detail: &DayAction,
    events: &[CalendarEvent],
    reminders: &[Reminder],
) {
    let popup_w = area.width.min(60).max(30);
    let popup_h = area.height.min(16).max(8);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    match detail {
        DayAction::Event(idx) => {
            if let Some(ev) = events.get(*idx) {
                render_event_detail(frame, popup_area, ev);
            }
        }
        DayAction::Reminder(idx) => {
            if let Some(rem) = reminders.get(*idx) {
                render_reminder_detail(frame, popup_area, rem);
            }
        }
        DayAction::None => {}
    }
}

fn render_event_detail(frame: &mut Frame, area: Rect, ev: &CalendarEvent) {
    let block = Block::default()
        .title(format!(" {} ", ev.title))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Calendar
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(ev.calendar_color)),
        Span::styled(format!(" {}", ev.calendar_name), Style::default()),
    ]));

    // Time
    lines.push(Line::from(""));
    if ev.is_all_day {
        lines.push(Line::from(Span::styled("All day", theme::DIM_STYLE)));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Time: ", theme::DIM_STYLE),
            Span::styled(ev.duration_display(), Style::default()),
        ]));
    }

    // Date
    lines.push(Line::from(vec![
        Span::styled("Date: ", theme::DIM_STYLE),
        Span::styled(
            ev.start.format("%A, %B %d, %Y").to_string(),
            Style::default(),
        ),
    ]));

    // Location
    if let Some(ref loc) = ev.location {
        if !loc.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Location: ", theme::DIM_STYLE),
                Span::styled(loc.clone(), Style::default()),
            ]));
        }
    }

    // Notes
    if let Some(ref notes) = ev.notes {
        if !notes.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Notes:", theme::DIM_STYLE)));
            for line in notes.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }
    }

    // Footer hint
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc to close",
        theme::DIM_STYLE,
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn render_reminder_detail(frame: &mut Frame, area: Rect, rem: &Reminder) {
    let status = if rem.is_completed {
        "Completed"
    } else {
        "Incomplete"
    };
    let title = format!(" {} ", rem.title);

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Calendar
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(rem.calendar_color)),
        Span::styled(format!(" {}", rem.calendar_name), Style::default()),
    ]));

    // Status
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Status: ", theme::DIM_STYLE),
        Span::styled(status, Style::default()),
    ]));

    // Due date
    if let Some(due) = &rem.due_date {
        lines.push(Line::from(vec![
            Span::styled("Due: ", theme::DIM_STYLE),
            Span::styled(
                due.format("%A, %B %d, %Y").to_string(),
                Style::default(),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Due: ", theme::DIM_STYLE),
            Span::styled("No date set", theme::DIM_STYLE),
        ]));
    }

    // Priority
    if rem.priority > 0 {
        let priority_str = match rem.priority {
            1..=4 => "High",
            5 => "Medium",
            6..=9 => "Low",
            _ => "None",
        };
        lines.push(Line::from(vec![
            Span::styled("Priority: ", theme::DIM_STYLE),
            Span::styled(priority_str, Style::default()),
        ]));
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc to close",
        theme::DIM_STYLE,
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
