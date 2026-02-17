use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

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
    current_date: NaiveDate,
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

    // Show due date context
    if let Some(due) = &rem.due_date {
        let due_date = due.date_naive();
        if due_date < current_date {
            spans.push(Span::styled(
                format!(" (overdue: {})", due_date.format("%m/%d")),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::DIM),
            ));
        }
    } else {
        spans.push(Span::styled(" (no date)", theme::DIM_STYLE));
    }

    ListItem::new(Line::from(spans))
}
