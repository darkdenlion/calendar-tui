use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::calendar::Reminder;
use crate::theme;

pub struct ReminderList;

impl ReminderList {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        reminders: &[Reminder],
        selected_index: usize,
        focused: bool,
    ) {
        let w = area.width as usize;

        let title = if w >= 25 {
            format!(" Reminders ({}) ", reminders.len())
        } else {
            " Reminders ".to_string()
        };

        let border_style = if focused {
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            theme::BORDER_STYLE
        };

        let block = Block::default()
            .title(title)
            .title_style(theme::HEADER_STYLE)
            .borders(Borders::ALL)
            .border_style(border_style);

        if reminders.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let msg = Paragraph::new("No reminders").style(theme::DIM_STYLE);
            frame.render_widget(msg, inner);
            return;
        }

        let inner_w = area.width.saturating_sub(2) as usize;

        // Group reminders by calendar
        let mut current_calendar = String::new();
        let mut items: Vec<ListItem> = Vec::new();

        for (i, reminder) in reminders.iter().enumerate() {
            // Calendar header
            if reminder.calendar_name != current_calendar {
                if !current_calendar.is_empty() {
                    items.push(ListItem::new(Line::from("")));
                }
                current_calendar = reminder.calendar_name.clone();
                items.push(ListItem::new(Line::from(Span::styled(
                    format!(" {}", current_calendar),
                    Style::default()
                        .fg(reminder.calendar_color)
                        .add_modifier(Modifier::BOLD),
                ))));
            }

            let checkbox = if reminder.is_completed { "[x]" } else { "[ ]" };
            let title_style = if reminder.is_completed {
                Style::default()
                    .add_modifier(Modifier::DIM | Modifier::CROSSED_OUT)
            } else {
                Style::default()
            };

            let is_selected = i == selected_index && focused;

            let mut spans = vec![
                Span::styled(
                    format!(" {} ", checkbox),
                    if is_selected {
                        theme::SELECTED_STYLE
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    truncate(&reminder.title, inner_w.saturating_sub(6)),
                    if is_selected {
                        theme::SELECTED_STYLE
                    } else {
                        title_style
                    },
                ),
            ];

            // Due date if there's room
            if let Some(ref due) = reminder.due_date {
                let due_str = format!(" {}", due.format("%m/%d"));
                if spans.iter().map(|s| s.width()).sum::<usize>() + due_str.len() < inner_w {
                    spans.push(Span::styled(due_str, theme::DIM_STYLE));
                }
            }

            items.push(ListItem::new(Line::from(spans)));
        }

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}
