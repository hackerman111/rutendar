use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::widgets::{
    month_name, theme_border_type, theme_calendar_border_style, theme_selected, theme_today,
    theme_today_badge, theme_unfocused,
};
use crate::{app::App, model::Importance, ui::Theme};

pub fn render_year(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 4); 4])
        .split(area);
    for row in 0..4 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3); 3])
            .split(rows[row]);
        for column in 0..3 {
            let month = (row * 3 + column + 1) as u32;
            let (events, has_high_importance) = app
                .state
                .occurrences
                .iter()
                .filter(|event| event.date.month() == month)
                .fold((0, false), |(count, important), event| {
                    (count + 1, important || event.importance == Importance::High)
                });
            let notes = app
                .state
                .notes
                .iter()
                .filter(|note| note.date.month() == month)
                .count();
            let selected = app.state.selected_date.month() == month;
            let is_today_month = app.state.today.year() == app.state.selected_date.year()
                && app.state.today.month() == month;

            let title_line = if selected {
                Line::from(vec![Span::styled(
                    format!(" {:02} · {} ", month, month_name(month)),
                    theme_selected(theme),
                )])
            } else if is_today_month {
                let today_str = if theme == Theme::Ascii {
                    " [TODAY] "
                } else {
                    " TODAY "
                };

                Line::from(vec![
                    Span::styled(
                        format!(" {:02} · {} ", month, month_name(month)),
                        theme_today_badge(theme),
                    ),
                    Span::styled(today_str, theme_today(theme)),
                ])
            } else {
                Line::from(vec![Span::styled(
                    format!(" [{:02} · {}] ", month, month_name(month)),
                    theme_unfocused(theme),
                )])
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled("  СОБЫТИЯ  ", theme_unfocused(theme)),
                    Span::styled(
                        format!("{events:>3} "),
                        if events > 0 {
                            theme.title_style(true, false)
                        } else {
                            theme_unfocused(theme)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ЗАМЕТКИ  ", theme_unfocused(theme)),
                    Span::styled(
                        format!("{notes:>3} "),
                        if notes > 0 {
                            theme.key_badge_style()
                        } else {
                            theme_unfocused(theme)
                        },
                    ),
                ]),
            ];

            let border_style =
                theme_calendar_border_style(theme, selected, is_today_month, has_high_importance);

            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Left).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme_border_type(theme))
                        .title(title_line)
                        .border_style(border_style),
                ),
                columns[column],
            );
        }
    }
}
