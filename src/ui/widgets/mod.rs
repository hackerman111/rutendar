use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    app::App,
    model::{EventOccurrence, Importance},
};

// Neo 80 Theme Palette & Tokens
pub const COLOR_ACCENT: Color = Color::Cyan;
pub const COLOR_ACCENT_ALT: Color = Color::Yellow;
pub const COLOR_MUTED: Color = Color::DarkGray;
pub const COLOR_CRIT: Color = Color::LightRed;
pub const COLOR_TEXT: Color = Color::White;

pub const SELECTED: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

pub const SELECTED_INACTIVE: Style = Style::new().fg(Color::White).bg(Color::DarkGray);

pub const TODAY: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub const TODAY_BADGE: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

pub const FOCUSED: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub const UNFOCUSED: Style = Style::new().fg(Color::DarkGray);

pub const TIME_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub const TIME_DIM: Style = Style::new().fg(Color::DarkGray);

pub const TAG_STYLE: Style = Style::new().fg(Color::DarkGray);

pub const KEY_BADGE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

pub const KEY_LABEL: Style = Style::new().fg(Color::DarkGray);

pub fn importance_style(importance: Importance) -> Style {
    match importance {
        Importance::High => Style::new().fg(COLOR_CRIT).add_modifier(Modifier::BOLD),
        Importance::Normal => Style::new().fg(COLOR_ACCENT),
        Importance::Low => Style::new().fg(COLOR_MUTED),
        Importance::None => Style::new().fg(COLOR_MUTED),
    }
}

pub fn event_line(app: &App, event: &EventOccurrence) -> String {
    let recurring = if event.is_recurring { "↻" } else { "" };
    let time = event
        .start_time
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_else(|| "день".into());
    let symbol = app.config.importance_symbol(event.importance);
    if symbol.trim().is_empty() {
        format!("{recurring}{time} {}", event.title)
    } else {
        format!("{recurring}{symbol} {time} {}", event.title)
    }
}

pub fn styled_event_spans(
    app: &App,
    event: &EventOccurrence,
    is_selected: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let sel_style = Style::new()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    if is_selected {
        spans.push(Span::styled("▸ ", sel_style));
    } else {
        spans.push(Span::raw("  "));
    }

    if event.is_recurring {
        let style = if is_selected {
            sel_style
        } else {
            Style::new().fg(COLOR_MUTED)
        };
        spans.push(Span::styled("↻ ", style));
    }

    let sym = app.config.importance_symbol(event.importance);
    if !sym.trim().is_empty() {
        let style = if is_selected {
            sel_style
        } else {
            importance_style(event.importance)
        };
        spans.push(Span::styled(format!("{sym} "), style));
    }

    let time_str = event
        .start_time
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_else(|| "день".into());

    let time_style = if is_selected {
        sel_style
    } else if event.start_time.is_some() {
        TIME_STYLE
    } else {
        TIME_DIM
    };
    spans.push(Span::styled(format!("{time_str} "), time_style));

    let title_style = if is_selected {
        sel_style
    } else {
        Style::new().fg(COLOR_TEXT)
    };
    spans.push(Span::styled(format!("{} ", event.title), title_style));

    spans
}

pub fn styled_tags_line(event: &EventOccurrence, is_selected: bool) -> Line<'static> {
    let tag_style = if is_selected {
        Style::new().fg(Color::Cyan)
    } else {
        TAG_STYLE
    };
    Line::from(vec![
        Span::raw("    "),
        Span::styled(tags_line(event), tag_style),
    ])
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

pub fn styled_relative_event_spans(app: &App, event: &EventOccurrence) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let date = relative_date(app.state.today, event.date);
    if !date.is_empty() {
        spans.push(Span::styled(
            date,
            Style::new()
                .fg(COLOR_ACCENT_ALT)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(time) = event.start_time {
        spans.push(Span::styled(time.format("%H:%M ").to_string(), TIME_STYLE));
    }
    let sym = app.config.importance_symbol(event.importance);
    if !sym.trim().is_empty() {
        spans.push(Span::styled(
            sym.to_string(),
            importance_style(event.importance),
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        event.title.clone(),
        Style::new().fg(COLOR_TEXT),
    ));
    spans
}

pub fn relative_date(today: NaiveDate, date: NaiveDate) -> String {
    if date == today {
        String::new()
    } else if date == today + Duration::days(1) {
        "ЗАВТРА ".into()
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
