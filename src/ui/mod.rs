pub mod agenda;
pub mod day;
pub mod link_bank;
pub mod month;
pub mod popup;
pub mod theme;
pub mod upcoming;
pub mod week;
pub mod widgets;
pub mod year;

pub use theme::Theme;

use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use self::{
    agenda::render_agenda,
    day::render_day,
    month::render_month,
    popup::render_popup,
    upcoming::render_upcoming,
    week::render_week,
    widgets::{SELECTED, month_name, weekday_long},
    year::render_year,
};
use crate::{
    app::{App, InputMode, Overlay, View},
    calendar::{week_end, week_start},
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, rows[0], app);
    match app.state.active_view {
        View::Week => render_week(frame, rows[1], app),
        View::Day => render_day(frame, rows[1], app),
        View::Month => render_month(frame, rows[1], app),
        View::Year => render_year(frame, rows[1], app),
    }
    render_next(frame, rows[2], app);
    render_status(frame, rows[3], app);

    match app.state.overlay {
        Some(Overlay::Agenda) => render_agenda(frame, area, app),
        Some(Overlay::Upcoming) => render_upcoming(frame, area, app),
        None => {}
    }
    if let Some(popup) = &app.state.popup {
        render_popup(frame, area, app, popup);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.state.active_view {
        View::Week => {
            let range = format!(
                "{} — {}",
                week_start(app.state.selected_date).format("%d.%m.%Y"),
                week_end(app.state.selected_date).format("%d.%m.%Y")
            );
            if app.config.ui.show_week_numbers {
                format!(
                    "W{:02} · {range}",
                    app.state.selected_date.iso_week().week()
                )
            } else {
                range
            }
        }
        View::Day => format!(
            "{} {}{}",
            weekday_long(app.state.selected_date),
            app.state.selected_date.format("%d.%m.%Y"),
            if app.state.selected_date == app.state.today {
                " · TODAY"
            } else {
                ""
            }
        ),
        View::Month => format!(
            "{} {}",
            month_name(app.state.selected_date.month()),
            app.state.selected_date.year()
        ),
        View::Year => app.state.selected_date.year().to_string(),
    };

    let mut spans = vec![
        Span::styled(
            " RUTENDAR ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    // View tabs
    for view in [View::Week, View::Day, View::Month, View::Year] {
        if app.state.active_view == view {
            spans.push(Span::styled(format!(" {} ", view.label()), SELECTED));
        } else {
            spans.push(Span::styled(
                format!(" {} ", view.label()),
                Style::new().fg(Color::DarkGray),
            ));
        }
    }

    spans.push(Span::styled(" │ ", Style::new().fg(Color::DarkGray)));
    spans.push(Span::styled(
        title,
        Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
    ));

    // Right-aligned today indicator if width permits
    let today_str = format!(" TODAY: {} ", app.state.today.format("%d.%m"));
    let current_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if (area.width as usize).saturating_sub(4) > current_len + today_str.len() {
        let padding = (area.width as usize).saturating_sub(4) - current_len - today_str.len();
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(today_str, Style::new().fg(Color::DarkGray)));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(Color::DarkGray)),
            ),
        area,
    );
}

fn render_next(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(
            " NEXT ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    let mut shown = 0;
    let mut current_len = 7; // " NEXT " + " "

    for event in &app.state.next {
        if shown >= app.config.agenda.next_events {
            break;
        }
        let event_spans = widgets::styled_relative_event_spans(app, event);
        let event_char_len: usize = event_spans.iter().map(|s| s.content.chars().count()).sum();
        let sep_len = if shown == 0 { 0 } else { 3 }; // " · "
        let reserve = 10;

        if current_len + sep_len + event_char_len + reserve > area.width as usize {
            break;
        }

        if shown > 0 {
            spans.push(Span::styled(" · ", Style::new().fg(Color::DarkGray)));
            current_len += 3;
        }
        spans.extend(event_spans);
        current_len += event_char_len;
        shown += 1;
    }

    let remaining = app.state.next_total.saturating_sub(shown);
    if remaining > 0 {
        spans.push(Span::styled(
            format!(" [+{remaining}]"),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let (mode_text, mode_style) = match app.state.input_mode {
        InputMode::Normal => (
            " NORMAL ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Editor => (
            " EDIT ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Search => (
            " SEARCH ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::LinkBank => (
            " LINKS ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::LinkSearch => (
            " LINK SEARCH ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Confirm => (
            " CONFIRM ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Scope => (
            " SCOPE ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::GotoDate => (
            " DATE ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::CreateTask => (
            " TASK ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let mut spans = vec![
        Span::styled(mode_text, mode_style),
        Span::styled(
            format!(
                " {} › {} ",
                app.state.active_view.label(),
                app.state.selected_date.format("%d.%m.%Y")
            ),
            Style::new().fg(Color::White),
        ),
    ];

    if let Some(status) = &app.state.status_message {
        spans.push(Span::styled("│ ", Style::new().fg(Color::DarkGray)));
        spans.push(Span::styled(
            status.clone(),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    let hints = [
        ("a", "ADD"),
        ("n", "NEXT DAY"),
        ("Tab", "PANES"),
        ("/", "AGENDA"),
        ("?", "HELP"),
        ("q", "QUIT"),
    ];

    let mut hints_spans = Vec::new();
    for (key, label) in hints {
        hints_spans.push(Span::styled(
            format!("[{key}]"),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        hints_spans.push(Span::styled(
            format!(" {label} "),
            Style::new().fg(Color::DarkGray),
        ));
    }

    let left_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let hints_len: usize = hints_spans.iter().map(|s| s.content.chars().count()).sum();

    if area.width as usize > left_len + hints_len + 1 {
        let padding = (area.width as usize) - left_len - hints_len;
        spans.push(Span::raw(" ".repeat(padding)));
        spans.extend(hints_spans);
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
