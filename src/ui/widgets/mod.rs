use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    app::App,
    model::{EventOccurrence, Importance},
    ui::Theme,
};
use ratatui::widgets::BorderType;

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

pub fn theme_selected(theme: Theme) -> Style {
    theme.selection_style()
}

pub fn theme_focused(theme: Theme) -> Style {
    match theme {
        Theme::Default => FOCUSED,
        Theme::Ascii => Style::new().add_modifier(Modifier::BOLD),
    }
}

pub fn theme_unfocused(theme: Theme) -> Style {
    match theme {
        Theme::Default => UNFOCUSED,
        Theme::Ascii => Style::new(),
    }
}

pub fn theme_border_type(theme: Theme) -> BorderType {
    theme.border_type()
}

pub fn theme_border_color(theme: Theme, is_focused: bool) -> Color {
    if is_focused {
        match theme {
            Theme::Default => Color::Cyan,
            Theme::Ascii => Color::Reset,
        }
    } else {
        match theme {
            Theme::Default => Color::DarkGray,
            Theme::Ascii => Color::Reset,
        }
    }
}

pub fn theme_today(theme: Theme) -> Style {
    match theme {
        Theme::Default => TODAY,
        Theme::Ascii => Style::new().add_modifier(Modifier::BOLD),
    }
}

pub fn theme_today_badge(theme: Theme) -> Style {
    theme.active_tab_style()
}

pub fn theme_importance_style(theme: Theme, importance: Importance) -> Style {
    match theme {
        Theme::Ascii => match importance {
            Importance::High => Style::new().add_modifier(Modifier::BOLD),
            _ => Style::new(),
        },
        Theme::Default => importance_style(importance),
    }
}

pub fn theme_calendar_border_style(
    theme: Theme,
    selected: bool,
    current: bool,
    has_high_importance: bool,
) -> Style {
    match theme {
        Theme::Ascii => {
            if selected {
                theme_selected(theme)
            } else if current || has_high_importance {
                Style::new().add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            }
        }
        Theme::Default => calendar_border_style(selected, current, has_high_importance),
    }
}

pub fn importance_style(importance: Importance) -> Style {
    match importance {
        Importance::High => Style::new().fg(COLOR_CRIT).add_modifier(Modifier::BOLD),
        Importance::Normal => Style::new().fg(COLOR_ACCENT),
        Importance::Low => Style::new().fg(Color::Gray),
        Importance::None => Style::new().fg(Color::DarkGray),
    }
}

pub fn calendar_border_style(selected: bool, current: bool, has_high_importance: bool) -> Style {
    if has_high_importance {
        importance_style(Importance::High)
    } else if current {
        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else if selected {
        FOCUSED
    } else {
        UNFOCUSED
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
    let theme = app.config.ui.theme;
    let mut spans = Vec::new();
    let sel_style = theme_selected(theme);

    if is_selected {
        let marker = match theme {
            Theme::Default => " ▸ ",
            Theme::Ascii => "> ",
        };
        spans.push(Span::styled(marker, sel_style));
    } else {
        spans.push(Span::raw("  "));
    }

    if event.is_recurring {
        let rec_sym = if theme == Theme::Ascii {
            "(R) "
        } else {
            "↻ "
        };
        let style = if is_selected {
            sel_style
        } else {
            theme_unfocused(theme)
        };
        spans.push(Span::styled(rec_sym, style));
    }

    let sym = app.config.importance_symbol(event.importance);
    if !sym.trim().is_empty() {
        let style = if is_selected {
            sel_style
        } else {
            theme_importance_style(theme, event.importance)
        };
        let disp_sym = if theme == Theme::Ascii {
            match event.importance {
                Importance::High => "[!] ",
                Importance::Normal => "[.] ",
                Importance::Low => "[-] ",
                Importance::None => "    ",
            }
        } else {
            sym
        };
        spans.push(Span::styled(disp_sym.to_string(), style));
        if theme != Theme::Ascii {
            spans.push(Span::raw(" "));
        }
    }

    let time_str = event
        .start_time
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_else(|| "день".into());

    let time_style = if is_selected {
        sel_style
    } else if event.start_time.is_some() {
        theme.time_style()
    } else {
        theme_unfocused(theme)
    };
    spans.push(Span::styled(format!("{time_str} "), time_style));

    let title_style = if is_selected {
        sel_style
    } else {
        theme.title_style(false, false)
    };
    spans.push(Span::styled(format!("{} ", event.title), title_style));

    if !event.favorite_links.is_empty() {
        let link_icon = if theme == Theme::Ascii { "[L]" } else { "🔗" };
        spans.push(Span::styled(
            format!("{link_icon}{} ", event.favorite_links.len()),
            if is_selected {
                sel_style
            } else {
                Style::new().fg(COLOR_MUTED)
            },
        ));
    }
    if event.directory.is_some() {
        let dir_icon = if theme == Theme::Ascii {
            "[dir] "
        } else {
            "📁 "
        };
        spans.push(Span::styled(
            dir_icon,
            if is_selected {
                sel_style
            } else {
                Style::new().fg(COLOR_MUTED)
            },
        ));
    }

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
    let theme = app.config.ui.theme;
    let mut spans = Vec::new();
    let date = relative_date(app.state.today, event.date);
    if !date.is_empty() {
        spans.push(Span::styled(date, theme.key_badge_style()));
    }
    if let Some(time) = event.start_time {
        spans.push(Span::styled(
            time.format("%H:%M ").to_string(),
            theme.time_style(),
        ));
    }
    let sym = app.config.importance_symbol(event.importance);
    if !sym.trim().is_empty() {
        let imp_span = theme.importance_span(event.importance);
        spans.push(Span::styled(imp_span.content, imp_span.style));
    }
    spans.push(Span::styled(
        event.title.clone(),
        theme.title_style(false, false),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_border_prioritizes_important_then_current_then_selected() {
        assert_eq!(
            calendar_border_style(true, true, true),
            importance_style(Importance::High)
        );
        assert_eq!(
            calendar_border_style(true, true, false),
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
        );
        assert_eq!(calendar_border_style(true, false, false), FOCUSED);
        assert_eq!(calendar_border_style(false, false, false), UNFOCUSED);
    }
}
