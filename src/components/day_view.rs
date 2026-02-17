use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::calendar::CalendarEvent;
use crate::theme;

pub struct DayView;

impl DayView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        date: NaiveDate,
        events: &[CalendarEvent],
    ) {
        let title = format!(" {} ", date.format("%A, %B %d, %Y"));
        let block = Block::default()
            .title(title)
            .title_style(theme::HEADER_STYLE)
            .borders(Borders::ALL)
            .border_style(theme::BORDER_STYLE);

        if events.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let msg = Paragraph::new("No events")
                .style(theme::DIM_STYLE);
            frame.render_widget(msg, inner);
            return;
        }

        let items: Vec<ListItem> = events
            .iter()
            .map(|ev| {
                let time = ev.duration_display();
                let cal_indicator = Span::styled("  ", Style::default().bg(ev.calendar_color));
                let time_span = Span::styled(
                    format!(" {} ", time),
                    Style::default().add_modifier(Modifier::DIM),
                );
                let title_span = Span::styled(&ev.title, Style::default());

                let mut spans = vec![cal_indicator, time_span, title_span];

                if let Some(ref loc) = ev.location {
                    if !loc.is_empty() {
                        spans.push(Span::styled(
                            format!(" @ {}", loc),
                            theme::DIM_STYLE,
                        ));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}
