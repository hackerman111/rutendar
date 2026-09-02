use chrono::{Datelike, Duration};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    app::App,
    calendar::{month_start, week_start},
    model::Importance,
    ui::Theme,
};

pub fn render_month(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(area);
    let headers = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 7); 7])
        .split(rows[0]);
    for (index, header) in ["ПН", "ВТ", "СР", "ЧТ", "ПТ", "СБ", "ВС"]
        .iter()
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(Span::styled(
                *header,
                Style::new()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            headers[index],
        );
    }
    let first = week_start(month_start(app.state.selected_date));
    for week in 0..6 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 7); 7])
            .split(rows[week + 1]);
        for day in 0..7 {
            let date = first + Duration::days((week * 7 + day) as i64);
            let day_events = app
                .state
                .occurrences
                .iter()
                .filter(|item| item.date == date);
            let has_high_importance = day_events
                .clone()
                .any(|event| event.importance == Importance::High);
            let note_count = app
                .state
                .notes
                .iter()
                .filter(|item| item.date == date)
                .count();

            let selected = date == app.state.selected_date;
            let is_today = date == app.state.today;
            let is_curr_month = date.month() == app.state.selected_date.month();

            let theme = app.config.ui.theme;
            let mut header_spans = Vec::new();
            if selected {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    super::widgets::theme_selected(theme),
                ));
            } else if is_today {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    super::widgets::theme_today_badge(theme),
                ));
            } else if is_curr_month {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    theme.title_style(false, false),
                ));
            } else {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    super::widgets::theme_unfocused(theme),
                ));
            }

            if is_today && !selected {
                let dot = if theme == Theme::Ascii { " *" } else { " •" };
                header_spans.push(Span::styled(dot, super::widgets::theme_today(theme)));
            }

            let mut lines = vec![Line::from(header_spans)];

            let mut metric_spans = Vec::new();
            for event in day_events {
                metric_spans.push(month_event_marker(
                    event.importance,
                    event.is_recurring,
                    is_curr_month,
                    theme,
                ));
            }
            if note_count > 0 {
                let note_icon = if theme == Theme::Ascii {
                    " [N]"
                } else {
                    " ◆"
                };
                metric_spans.push(Span::styled(
                    format!("{note_icon}{note_count}"),
                    if is_curr_month {
                        theme.key_badge_style()
                    } else {
                        super::widgets::theme_unfocused(theme)
                    },
                ));
            }

            if !metric_spans.is_empty() {
                lines.push(Line::from(metric_spans));
            }

            let border_style = super::widgets::theme_calendar_border_style(
                theme,
                selected,
                is_today,
                has_high_importance,
            );

            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(super::widgets::theme_border_type(theme))
                        .border_style(border_style),
                ),
                columns[day],
            );
        }
    }
}

fn month_event_marker(
    importance: Importance,
    recurring: bool,
    is_current_month: bool,
    theme: Theme,
) -> Span<'static> {
    let marker = match (recurring, importance) {
        (true, Importance::High) => {
            if theme == Theme::Ascii {
                "!(R)"
            } else {
                "▲↻"
            }
        }
        (true, Importance::Normal) => {
            if theme == Theme::Ascii {
                "*(R)"
            } else {
                "●↻"
            }
        }
        (true, Importance::Low) => {
            if theme == Theme::Ascii {
                "-(R)"
            } else {
                "·↻"
            }
        }
        (true, Importance::None) => {
            if theme == Theme::Ascii {
                "(R)"
            } else {
                "↻"
            }
        }
        (false, Importance::High) => {
            if theme == Theme::Ascii {
                "! "
            } else {
                "▲ "
            }
        }
        (false, Importance::Normal) => {
            if theme == Theme::Ascii {
                "* "
            } else {
                "● "
            }
        }
        (false, Importance::Low) => {
            if theme == Theme::Ascii {
                "- "
            } else {
                "· "
            }
        }
        (false, Importance::None) => "  ",
    };
    let style = if !is_current_month {
        super::widgets::theme_unfocused(theme)
    } else {
        super::widgets::theme_importance_style(theme, importance)
    };
    Span::styled(marker, style)
}

pub fn month_day_cell(area: Rect, selected_date: chrono::NaiveDate) -> Option<Rect> {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(area);
    let first = week_start(month_start(selected_date));
    for week in 0..6 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 7); 7])
            .split(rows[week + 1]);
        for day in 0..7 {
            let date = first + Duration::days((week * 7 + day) as i64);
            if date == selected_date {
                return Some(columns[day]);
            }
        }
    }
    None
}

pub fn render_month_day_preview(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    date: chrono::NaiveDate,
    selected_index: usize,
) {
    let month_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area)[1];

    let cell = month_day_cell(month_area, date).unwrap_or(month_area);

    let occurrences: Vec<_> = app
        .state
        .occurrences
        .iter()
        .filter(|e| e.date == date)
        .collect();
    let tasks: Vec<_> = app
        .state
        .tasks
        .iter()
        .filter(|t| t.date == Some(date))
        .collect();
    let notes: Vec<_> = app.state.notes.iter().filter(|n| n.date == date).collect();
    let total_items = occurrences.len() + tasks.len() + notes.len();

    let popup_width = (38.max(cell.width + 8)).min(area.width.saturating_sub(4));
    let content_height = if total_items == 0 {
        1
    } else {
        total_items as u16
    };
    let popup_height = (content_height + 2).min(12);

    let mut x = cell.x;
    if x + popup_width > area.right() {
        x = area.right().saturating_sub(popup_width);
    }

    let mut y = cell.y + cell.height;
    if y + popup_height > area.bottom() {
        y = cell.y.saturating_sub(popup_height);
    }

    let popup_rect = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_rect);

    let theme = app.config.ui.theme;
    let mut lines = Vec::new();
    if total_items == 0 {
        lines.push(Line::from(vec![Span::styled(
            "   (нет событий)",
            super::widgets::theme_unfocused(theme),
        )]));
    } else {
        use chrono::Timelike;
        for (i, occ) in occurrences.iter().enumerate() {
            let is_sel = i == selected_index;
            let cursor = if is_sel {
                match theme {
                    Theme::Default => " ▸ ",
                    Theme::Ascii => " > ",
                }
            } else {
                "   "
            };
            let time_str = if let Some(t) = occ.start_time {
                format!("{:02}:{:02} ", t.hour(), t.minute())
            } else {
                "--:-- ".to_string()
            };
            let imp_symbol = app.config.importance_symbol(occ.importance);
            let rec_symbol = if occ.is_recurring {
                if theme == Theme::Ascii {
                    "(R) "
                } else {
                    "↻ "
                }
            } else {
                "  "
            };

            let cursor_span = Span::styled(
                cursor,
                if is_sel {
                    super::widgets::theme_selected(theme)
                } else {
                    Style::default()
                },
            );
            let time_span = Span::styled(time_str, theme.time_style());
            let imp_span = Span::styled(
                format!("{imp_symbol} "),
                super::widgets::theme_importance_style(theme, occ.importance),
            );
            let rec_span = Span::styled(rec_symbol, super::widgets::theme_unfocused(theme));
            let title_span = Span::styled(
                &occ.title,
                if is_sel {
                    super::widgets::theme_selected(theme)
                } else {
                    theme.title_style(false, false)
                },
            );

            lines.push(Line::from(vec![
                cursor_span,
                time_span,
                imp_span,
                rec_span,
                title_span,
            ]));
        }

        for (k, task) in tasks.iter().enumerate() {
            let idx = occurrences.len() + k;
            let is_sel = idx == selected_index;
            let cursor = if is_sel {
                match theme {
                    Theme::Default => " ▸ ",
                    Theme::Ascii => " > ",
                }
            } else {
                "   "
            };
            let cursor_span = Span::styled(
                cursor,
                if is_sel {
                    super::widgets::theme_selected(theme)
                } else {
                    Style::default()
                },
            );
            let checkbox = theme.task_checkbox_span(task.is_done);
            let imp_symbol = app.config.importance_symbol(task.importance);
            let imp_span = Span::styled(
                format!("{imp_symbol} "),
                super::widgets::theme_importance_style(theme, task.importance),
            );
            let title_style = if is_sel {
                super::widgets::theme_selected(theme)
            } else {
                theme.title_style(false, task.is_done)
            };
            let title_span = Span::styled(&task.title, title_style);

            lines.push(Line::from(vec![
                cursor_span,
                checkbox,
                imp_span,
                title_span,
            ]));
        }

        for (j, note) in notes.iter().enumerate() {
            let idx = occurrences.len() + tasks.len() + j;
            let is_sel = idx == selected_index;
            let cursor = if is_sel {
                match theme {
                    Theme::Default => " ▸ ",
                    Theme::Ascii => " > ",
                }
            } else {
                "   "
            };
            let cursor_span = Span::styled(
                cursor,
                if is_sel {
                    super::widgets::theme_selected(theme)
                } else {
                    Style::default()
                },
            );
            let icon_str = if theme == Theme::Ascii {
                "[N] "
            } else {
                "◆ "
            };
            let icon_span = Span::styled(icon_str, theme.key_badge_style());
            let title_text = note.title.as_deref().unwrap_or(&note.body);
            let title_span = Span::styled(
                title_text,
                if is_sel {
                    super::widgets::theme_selected(theme)
                } else {
                    theme.key_badge_style()
                },
            );

            lines.push(Line::from(vec![cursor_span, icon_span, title_span]));
        }
    }

    let title = format!(" {} [{}] ", date.format("%d.%m.%Y"), total_items);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(super::widgets::theme_border_type(theme))
        .title(Span::styled(title, super::widgets::theme_selected(theme)))
        .border_style(super::widgets::theme_focused(theme));

    frame.render_widget(Paragraph::new(lines).block(block), popup_rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_markers_distinguish_priority_and_recurrence() {
        let low = month_event_marker(Importance::Low, false, true, Theme::Default);
        assert_eq!(low.content, "· ");

        let normal = month_event_marker(Importance::Normal, true, true, Theme::Default);
        assert_eq!(normal.content, "●↻");

        let high = month_event_marker(Importance::High, false, true, Theme::Default);
        assert_eq!(high.content, "▲ ");

        // Plain theme ASCII markers
        let low_plain = month_event_marker(Importance::Low, false, true, Theme::Ascii);
        assert_eq!(low_plain.content, "- ");

        let normal_plain = month_event_marker(Importance::Normal, true, true, Theme::Ascii);
        assert_eq!(normal_plain.content, "*(R)");

        let high_plain = month_event_marker(Importance::High, false, true, Theme::Ascii);
        assert_eq!(high_plain.content, "! ");
    }

    #[test]
    fn month_day_cell_calculates_bounds() {
        let area = Rect::new(0, 0, 100, 30);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let cell = month_day_cell(area, date);
        assert!(cell.is_some());
        let rect = cell.unwrap();
        assert!(rect.width > 0);
        assert!(rect.height > 0);
    }
}
