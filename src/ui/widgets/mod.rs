use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
};

use crate::{app::App, model::EventOccurrence};

pub const SELECTED: Style = Style::new().fg(Color::Black).bg(Color::Yellow);
pub const TODAY: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const FOCUSED: Style = Style::new().fg(Color::Yellow);

pub fn event_line(app: &App, event: &EventOccurrence) -> String {
    let recurring = if event.is_recurring { "↻" } else { "" };
    let time = event
        .start_time
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_else(|| "весь день".into());
    format!(
        "{recurring}{} {time} {}",
        app.config.importance_symbol(event.importance),
        event.title
    )
}

pub fn tags_line(event: &EventOccurrence) -> String {
    event
        .tags
        .iter()
        .map(|tag| format!("#{}", tag.name))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn relative_event(app: &App, event: &EventOccurrence) -> String {
    let date = relative_date(app.state.today, event.date);
    let time = event
        .start_time
        .map(|time| time.format("%H:%M ").to_string())
        .unwrap_or_default();
    format!(
        "{date}{time}{}{}",
        app.config.importance_symbol(event.importance),
        event.title
    )
}

pub fn relative_date(today: NaiveDate, date: NaiveDate) -> String {
    if date == today {
        String::new()
    } else if date == today + Duration::days(1) {
        "завтра ".into()
    } else {
        format!("{} ", date.format("%d.%m"))
    }
}

pub fn centered(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

pub fn centered_fixed(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

pub fn weekday_short(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "ПН",
        chrono::Weekday::Tue => "ВТ",
        chrono::Weekday::Wed => "СР",
        chrono::Weekday::Thu => "ЧТ",
        chrono::Weekday::Fri => "ПТ",
        chrono::Weekday::Sat => "СБ",
        chrono::Weekday::Sun => "ВС",
    }
}

pub fn weekday_long(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "ПОНЕДЕЛЬНИК",
        chrono::Weekday::Tue => "ВТОРНИК",
        chrono::Weekday::Wed => "СРЕДА",
        chrono::Weekday::Thu => "ЧЕТВЕРГ",
        chrono::Weekday::Fri => "ПЯТНИЦА",
        chrono::Weekday::Sat => "СУББОТА",
        chrono::Weekday::Sun => "ВОСКРЕСЕНЬЕ",
    }
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "ЯНВАРЬ",
        2 => "ФЕВРАЛЬ",
        3 => "МАРТ",
        4 => "АПРЕЛЬ",
        5 => "МАЙ",
        6 => "ИЮНЬ",
        7 => "ИЮЛЬ",
        8 => "АВГУСТ",
        9 => "СЕНТЯБРЬ",
        10 => "ОКТЯБРЬ",
        11 => "НОЯБРЬ",
        12 => "ДЕКАБРЬ",
        _ => "",
    }
}
