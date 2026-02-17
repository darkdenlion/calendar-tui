mod app;
mod calendar;
mod components;
mod event;
mod theme;
mod tui;

use std::time::Duration;

use app::{App, ViewMode};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};

fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize calendar BEFORE entering TUI so permission dialog can appear
    eprintln!("Connecting to Apple Calendar...");
    let mut app = App::new()?;
    eprintln!("Calendar ready. Launching TUI...");

    // Set up panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = tui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = tui::init()?;

    let result = run(&mut terminal, &mut app);

    // Always restore terminal
    tui::restore()?;

    result
}

fn run(terminal: &mut tui::Tui, app: &mut App) -> Result<()> {
    while app.running {
        terminal.draw(|frame| {
            let area = frame.area();
            let w = area.width;

            if !app.access_granted {
                let msg = ratatui::widgets::Paragraph::new(
                    "Calendar access denied.\n\n\
                     Please grant access in:\n\
                     System Settings > Privacy & Security > Calendars\n\n\
                     Press 'q' to quit.",
                )
                .style(theme::HEADER_STYLE);
                frame.render_widget(msg, area);
                return;
            }

            let layout = Layout::vertical([
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area);

            let content_area = layout[0];

            match app.view_mode {
                ViewMode::Month => render_month_layout(frame, content_area, app, w),
                ViewMode::Week => {
                    components::WeekView::render(
                        frame,
                        content_area,
                        app.selected_date,
                        app.today,
                        app.week_start(),
                        &app.week_events,
                    );
                }
                ViewMode::Day => {
                    components::DayView::render(
                        frame,
                        content_area,
                        app.selected_date,
                        &app.day_events,
                        app.day_scroll,
                    );
                }
            }

            components::StatusBar::render(frame, layout[1], &app.view_mode);
        })?;

        if let Some(key) = event::next_key_event(Duration::from_millis(100))? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    app.running = false;
                }
                (KeyCode::Char('1'), _) => app.view_mode = ViewMode::Month,
                (KeyCode::Char('2'), _) => app.view_mode = ViewMode::Week,
                (KeyCode::Char('3'), _) => app.view_mode = ViewMode::Day,
                (KeyCode::Char('t'), _) => app.go_to_today(),
                (KeyCode::Left, _) => app.prev_day(),
                (KeyCode::Right, _) => app.next_day(),
                (KeyCode::Up, _) => {
                    if app.view_mode == ViewMode::Day {
                        app.scroll_day_up();
                    } else {
                        app.prev_week();
                    }
                }
                (KeyCode::Down, _) => {
                    if app.view_mode == ViewMode::Day {
                        app.scroll_day_down();
                    } else {
                        app.next_week();
                    }
                }
                (KeyCode::Char('['), _) => app.prev_month(),
                (KeyCode::Char(']'), _) => app.next_month(),
                _ => {}
            }
        }
    }

    Ok(())
}

/// Responsive month layout:
/// - Narrow (<60): month grid only, no sidebar
/// - Medium (60-99): month grid (fixed) + day agenda sidebar
/// - Wide (100+): month grid (fixed) + spacious day agenda
fn render_month_layout(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &App,
    total_width: u16,
) {
    if total_width < 60 {
        // Narrow: month grid only, full width
        components::MonthView::render(
            frame,
            area,
            app.selected_date,
            app.today,
            &app.days_with_events,
        );
    } else {
        // Side by side: month grid + day agenda
        let month_w = if total_width >= 100 { 44 } else { 30 };
        let content = Layout::horizontal([
            Constraint::Length(month_w),
            Constraint::Min(20),
        ])
        .split(area);

        components::MonthView::render(
            frame,
            content[0],
            app.selected_date,
            app.today,
            &app.days_with_events,
        );

        components::DayView::render(
            frame,
            content[1],
            app.selected_date,
            &app.day_events,
            app.day_scroll,
        );
    }
}
