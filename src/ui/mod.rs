pub mod agenda;
pub mod day;
pub mod month;
pub mod popup;
pub mod upcoming;
pub mod week;
pub mod widgets;
pub mod year;

use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

use self::{
    agenda::render_agenda,
    day::render_day,
    month::render_month,
    popup::render_popup,
    upcoming::render_upcoming,
    week::render_week,
    widgets::{month_name, relative_event, weekday_long},
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
    frame.render_widget(
        Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(Style::new().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_next(frame: &mut Frame, area: Rect, app: &App) {
    let mut text = String::from("NEXT  ");
    let mut shown = 0;
    for event in &app.state.next {
        if shown >= app.config.agenda.next_events {
            break;
        }
        let part = relative_event(app, event);
        let separator = if shown == 0 { "" } else { " · " };
        let reserve = 8;
        if text.chars().count() + separator.chars().count() + part.chars().count() + reserve
            > area.width as usize
        {
            break;
        }
        text.push_str(separator);
        text.push_str(&part);
        shown += 1;
    }
    let remaining = app.state.next_total.saturating_sub(shown);
    if remaining > 0 {
        text.push_str(&format!(" · +{remaining}"));
    }
    frame.render_widget(
        Paragraph::new(text).style(Style::new().fg(Color::Cyan)),
        area,
    );
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let mode = match app.state.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editor => "EDIT",
        InputMode::Search => "SEARCH",
        InputMode::Confirm => "CONFIRM",
        InputMode::Scope => "SCOPE",
        InputMode::GotoDate => "DATE",
    };
    let mut text = format!(
        " {mode} │ {} │ {} │ a events │ t upcoming │ ? help ",
        app.state.active_view.label(),
        app.state.selected_date.format("%d.%m.%Y")
    );
    if let Some(status) = &app.state.status_message {
        text.push_str("│ ");
        text.push_str(status);
    }
    frame.render_widget(
        Paragraph::new(text).style(Style::new().bg(Color::DarkGray)),
        area,
    );
}
