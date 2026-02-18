mod app;
mod calendar;
mod components;
mod event;
mod theme;
mod tui;

use std::time::Duration;

use app::{App, InputMode, ViewMode};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};

fn main() -> Result<()> {
    color_eyre::install()?;

    eprintln!("Connecting to Apple Calendar...");
    let mut app = App::new()?;
    eprintln!("Calendar ready. Launching TUI...");

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = tui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = tui::init()?;
    let result = run(&mut terminal, &mut app);
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

            // Main layout: content + status bar
            let layout = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

            let content_area = layout[0];

            // Render main view
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
                        &app.day_reminders,
                        app.day_scroll,
                    );
                }
            }

            // Render event form overlay
            if let Some(ref form) = app.form_state {
                components::EventForm::render(frame, area, form, &app.calendars);
            }

            // Render detail popup overlay
            if let Some(ref detail) = app.detail_item {
                components::day_view::render_detail_popup(
                    frame, area, detail, &app.day_events, &app.day_reminders,
                );
            }

            // Render help overlay
            if app.show_help {
                render_help(frame, area);
            }

            // Status bar
            render_status_bar(frame, layout[1], app, w);
        })?;

        if let Some(key) = event::next_key_event(Duration::from_millis(100))? {
            // Clear status message on any key
            app.status_message = None;

            // Help overlay takes priority
            if app.show_help {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
                    app.show_help = false;
                }
                continue;
            }

            // Detail popup takes priority
            if app.detail_item.is_some() {
                if key.code == KeyCode::Esc {
                    app.close_detail();
                }
                continue;
            }

            match app.input_mode {
                InputMode::Form => handle_form_input(app, key.code, key.modifiers),
                InputMode::Normal => handle_normal_input(app, key.code, key.modifiers),
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_normal_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match (code, modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.running = false;
        }
        (KeyCode::Char('1'), _) => app.view_mode = ViewMode::Month,
        (KeyCode::Char('2'), _) => app.view_mode = ViewMode::Week,
        (KeyCode::Char('3'), _) => app.view_mode = ViewMode::Day,
        (KeyCode::Char('t'), _) => app.go_to_today(),
        (KeyCode::Char('r'), _) => {
            // Refresh reminders
            app.refresh_reminders();
            app.status_message = Some("Reminders refreshed".to_string());
        }
        (KeyCode::Char('n'), _) => app.open_event_form(),
        (KeyCode::Char('d'), _) => app.delete_selected_event(),
        (KeyCode::Char(' '), _) => app.toggle_day_reminder(),
        (KeyCode::Enter, _) => app.show_detail(),
        (KeyCode::Left, _) | (KeyCode::Char('h'), _) => app.prev_day(),
        (KeyCode::Right, _) | (KeyCode::Char('l'), _) => app.next_day(),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
            if app.view_mode == ViewMode::Day || app.view_mode == ViewMode::Month {
                app.scroll_day_up();
            } else {
                app.prev_week();
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
            if app.view_mode == ViewMode::Day || app.view_mode == ViewMode::Month {
                app.scroll_day_down();
            } else {
                app.next_week();
            }
        }
        (KeyCode::Char('['), _) => app.prev_month(),
        (KeyCode::Char(']'), _) => app.next_month(),
        (KeyCode::Char('?'), _) => app.show_help = true,
        _ => {}
    }
}

fn handle_form_input(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) {
    match code {
        KeyCode::Esc => app.close_event_form(),
        KeyCode::Enter => app.submit_event_form(),
        KeyCode::Tab => app.form_tab(),
        KeyCode::BackTab => app.form_backtab(),
        KeyCode::Backspace => app.form_backspace(),
        KeyCode::Char(' ') => {
            // Space toggles all-day or cycles calendar
            if let Some(ref form) = app.form_state {
                match form.active_field {
                    crate::components::event_form::FormField::AllDay => {
                        if let Some(ref mut f) = app.form_state {
                            f.toggle_all_day();
                        }
                    }
                    crate::components::event_form::FormField::Calendar => {
                        let total = app.calendars.len();
                        if let Some(ref mut f) = app.form_state {
                            f.next_calendar(total);
                        }
                    }
                    _ => app.form_input_char(' '),
                }
            }
        }
        KeyCode::Char(c) => app.form_input_char(c),
        _ => {}
    }
}

fn render_month_layout(frame: &mut ratatui::Frame, area: Rect, app: &App, total_width: u16) {
    if total_width < 60 {
        components::MonthView::render(
            frame, area, app.selected_date, app.today, &app.days_with_events, &app.days_with_reminders,
        );
    } else {
        let month_w = if total_width >= 100 { 44 } else { 30 };
        let content = Layout::horizontal([
            Constraint::Length(month_w),
            Constraint::Min(20),
        ])
        .split(area);

        components::MonthView::render(
            frame, content[0], app.selected_date, app.today, &app.days_with_events, &app.days_with_reminders,
        );

        components::DayView::render(
            frame,
            content[1],
            app.selected_date,
            &app.day_events,
            &app.day_reminders,
            app.day_scroll,
        );
    }
}

fn render_status_bar(frame: &mut ratatui::Frame, area: Rect, app: &App, w: u16) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let w = w as usize;

    let mode_str = match app.view_mode {
        ViewMode::Month => "[1]Month",
        ViewMode::Week => "[2]Week",
        ViewMode::Day => "[3]Day",
    };

    let focus_indicator = match app.input_mode {
        InputMode::Form => " [New Event]",
        InputMode::Normal => "",
        _ => "",
    };

    // Show status message if present, otherwise show context-aware hints
    let right_text = if let Some(ref msg) = app.status_message {
        format!(" {} ", msg)
    } else {
        match app.view_mode {
            ViewMode::Day | ViewMode::Month if w >= 80 => {
                " hjkl:Nav [/]:Mon t:Today Enter:Detail Sp:Toggle n:New d:Del ?:Help q:Quit".to_string()
            }
            ViewMode::Day | ViewMode::Month if w >= 50 => {
                " jk:Scroll Enter:Detail Sp:Toggle n:New q:Quit".to_string()
            }
            ViewMode::Week if w >= 70 => {
                " hl:Day [/]:Mon t:Today n:New ?:Help q:Quit".to_string()
            }
            ViewMode::Week if w >= 50 => {
                " arrows:Nav n:New q:Quit".to_string()
            }
            _ => " ?:Help q:Quit".to_string(),
        }
    };

    let left = format!(" {}{} ", mode_str, focus_indicator);
    let padding_len = w.saturating_sub(left.len() + right_text.len());
    let padding = " ".repeat(padding_len);

    let line = Line::from(vec![
        Span::styled(left, theme::STATUS_STYLE),
        Span::styled(padding, theme::STATUS_STYLE),
        Span::styled(right_text, theme::STATUS_STYLE),
    ]);

    let bar = Paragraph::new(line).style(theme::STATUS_STYLE);
    frame.render_widget(bar, area);
}

fn render_help(frame: &mut ratatui::Frame, area: Rect) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let popup_w = area.width.min(52).max(30);
    let popup_h = area.height.min(22).max(12);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let key_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let desc_style = Style::default();
    let section_style = Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let lines = vec![
        Line::from(Span::styled("Navigation", section_style)),
        Line::from(vec![
            Span::styled("  h/l ", key_style),
            Span::styled("or ", theme::DIM_STYLE),
            Span::styled("\u{2190}/\u{2192}  ", key_style),
            Span::styled("Previous/next day", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  j/k ", key_style),
            Span::styled("or ", theme::DIM_STYLE),
            Span::styled("\u{2191}/\u{2193}  ", key_style),
            Span::styled("Scroll day list", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [/]       ", key_style),
            Span::styled("Previous/next month", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  t         ", key_style),
            Span::styled("Jump to today", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Views", section_style)),
        Line::from(vec![
            Span::styled("  1/2/3     ", key_style),
            Span::styled("Month / Week / Day view", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Actions", section_style)),
        Line::from(vec![
            Span::styled("  Enter     ", key_style),
            Span::styled("View event/reminder details", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Space     ", key_style),
            Span::styled("Toggle reminder completion", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  n         ", key_style),
            Span::styled("Create new event", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d         ", key_style),
            Span::styled("Delete selected event", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  r         ", key_style),
            Span::styled("Refresh reminders", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q", key_style),
            Span::styled(" / ", theme::DIM_STYLE),
            Span::styled("Esc     ", key_style),
            Span::styled("Quit / close popup", desc_style),
        ]),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
