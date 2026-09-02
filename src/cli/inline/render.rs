use chrono::{Datelike, Duration, Weekday};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::state::{InlineApp, InlineTab};
use crate::model::Importance;

pub fn render_inline(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tabs / Header
            Constraint::Length(1), // Subheader / Context
            Constraint::Min(1),    // List content
            Constraint::Length(1), // Footer / Hotkeys
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_subheader(frame, chunks[1], app);
    render_content(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let active_tab_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let inactive_tab_style = Style::default().fg(Color::DarkGray);

    let tab_day = if app.tab == InlineTab::Day {
        Span::styled(" [ 1 ДЕНЬ ] ", active_tab_style)
    } else {
        Span::styled("  1 ДЕНЬ  ", inactive_tab_style)
    };
    let tab_week = if app.tab == InlineTab::Week {
        Span::styled(" [ 2 НЕДЕЛЯ ] ", active_tab_style)
    } else {
        Span::styled("  2 НЕДЕЛЯ  ", inactive_tab_style)
    };
    let tab_search = if app.tab == InlineTab::Search {
        Span::styled(" [ 3 ПОИСК ] ", active_tab_style)
    } else {
        Span::styled("  3 ПОИСК  ", inactive_tab_style)
    };

    let weekday_str = match app.today.weekday() {
        Weekday::Mon => "Понедельник",
        Weekday::Tue => "Вторник",
        Weekday::Wed => "Среда",
        Weekday::Thu => "Четверг",
        Weekday::Fri => "Пятница",
        Weekday::Sat => "Суббота",
        Weekday::Sun => "Воскресенье",
    };

    let today_banner = format!("📅 {}, {}", weekday_str, app.today.format("%d.%m.%Y"));

    let header_line = Line::from(vec![
        Span::styled("Режим: ", Style::default().fg(Color::DarkGray)),
        tab_day,
        Span::raw(" "),
        tab_week,
        Span::raw(" "),
        tab_search,
        Span::raw("   "),
        Span::styled(today_banner, Style::default().fg(Color::White)),
        Span::styled(
            "  (F: Полный TUI · q: Выход)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(header_line), area);
}

fn render_subheader(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let subheader_line = match app.tab {
        InlineTab::Day => {
            let is_today = app.current_date == app.today;
            let day_label = if is_today {
                "Сегодня"
            } else if app.current_date == app.today - Duration::days(1) {
                "Вчера"
            } else if app.current_date == app.today + Duration::days(1) {
                "Завтра"
            } else {
                match app.current_date.weekday() {
                    Weekday::Mon => "Пн",
                    Weekday::Tue => "Вт",
                    Weekday::Wed => "Ср",
                    Weekday::Thu => "Чт",
                    Weekday::Fri => "Пт",
                    Weekday::Sat => "Сб",
                    Weekday::Sun => "Вс",
                }
            };

            Line::from(vec![
                Span::styled("Дата: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "← ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ({})", app.current_date.format("%d.%m.%Y"), day_label),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " →",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "  ·  {} событий, {} задач",
                        app.day_events.len(),
                        app.day_tasks.len()
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "  [←/→: сменить день · t: сегодня]",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        InlineTab::Week => {
            let weekday_offset = app.current_date.weekday().num_days_from_monday() as i64;
            let monday = app.current_date - Duration::days(weekday_offset);
            let sunday = monday + Duration::days(6);
            Line::from(vec![
                Span::styled("Неделя: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(
                        "{} — {}",
                        monday.format("%d.%m"),
                        sunday.format("%d.%m.%Y")
                    ),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ·  {} событий", app.week_events.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        InlineTab::Search => {
            let query_display = if app.query.is_empty() {
                Span::styled(
                    "введите текст для поиска...",
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled(
                    &app.query,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            };

            Line::from(vec![
                Span::styled("🔍 Поиск: ", Style::default().fg(Color::Yellow)),
                query_display,
                Span::styled("█", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("  (найдено: {})", app.search_results.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
    };

    frame.render_widget(Paragraph::new(subheader_line), area);
}

fn render_content(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let list_height = area.height as usize;
    let selected = app.selected_idx;

    let mut lines = Vec::new();

    match app.tab {
        InlineTab::Day => {
            if app.day_events.is_empty() && app.day_tasks.is_empty() {
                lines.push(Line::from(Span::styled(
                    "   (нет событий и задач на этот день)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                // Render events
                for (i, event) in app.day_events.iter().enumerate() {
                    let is_sel = i == selected;
                    lines.push(render_event_line(event, is_sel));
                }

                // Render tasks section
                if !app.day_tasks.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " ── Задачи на день ──────────────────────────────────────────────────────────",
                        Style::default().fg(Color::DarkGray),
                    )));

                    for (j, task) in app.day_tasks.iter().enumerate() {
                        let is_sel = (app.day_events.len() + j) == selected;
                        let marker = if is_sel {
                            Span::styled(
                                "▸ ",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::raw("  ")
                        };

                        let checkbox = if task.is_done {
                            Span::styled("[x] ", Style::default().fg(Color::Green))
                        } else {
                            Span::styled("[ ] ", Style::default().fg(Color::White))
                        };

                        let title_style = if task.is_done {
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::CROSSED_OUT)
                        } else if is_sel {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        let imp_span = match task.importance {
                            Importance::High => Span::styled(
                                " ! ",
                                Style::default()
                                    .fg(Color::LightRed)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            _ => Span::raw(""),
                        };

                        let mut spans = vec![marker, checkbox, Span::styled(&task.title, title_style), imp_span];
                        if is_sel && !task.is_done {
                            spans.push(Span::styled(" (Space: выполнено)", Style::default().fg(Color::DarkGray)));
                        }

                        let line = if is_sel {
                            Line::from(spans).style(Style::default().bg(Color::Rgb(20, 30, 45)))
                        } else {
                            Line::from(spans)
                        };
                        lines.push(line);
                    }
                }
            }
        }
        InlineTab::Week => {
            if app.week_events.is_empty() {
                lines.push(Line::from(Span::styled(
                    "   (нет событий на этой неделе)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for (i, event) in app.week_events.iter().enumerate() {
                    let is_sel = i == selected;
                    let marker = if is_sel {
                        Span::styled(
                            "▸ ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("  ")
                    };

                    let weekday_prefix = match event.date.weekday() {
                        Weekday::Mon => "Пн",
                        Weekday::Tue => "Вт",
                        Weekday::Wed => "Ср",
                        Weekday::Thu => "Чт",
                        Weekday::Fri => "Пт",
                        Weekday::Sat => "Сб",
                        Weekday::Sun => "Вс",
                    };

                    let date_span = Span::styled(
                        format!("[{} {}] ", weekday_prefix, event.date.format("%d.%m")),
                        Style::default().fg(Color::Cyan),
                    );

                    let time_str = match (event.start_time, event.end_time) {
                        (Some(s), Some(e)) => {
                            format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M"))
                        }
                        (Some(s), None) => format!("{} ", s.format("%H:%M")),
                        _ => "Весь день ".to_string(),
                    };
                    let time_span = Span::styled(time_str, Style::default().fg(Color::Yellow));

                    let title_style = if is_sel {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let title_span = Span::styled(format!("{} ", event.title), title_style);

                    let tags_str = event
                        .tags
                        .iter()
                        .map(|t| format!("#{}", t.name))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let tags_span = Span::styled(tags_str, Style::default().fg(Color::Cyan));

                    let line_spans = vec![marker, date_span, time_span, title_span, tags_span];
                    if is_sel {
                        lines.push(
                            Line::from(line_spans).style(Style::default().bg(Color::Rgb(20, 30, 45))),
                        );
                    } else {
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        }
        InlineTab::Search => {
            if app.search_results.is_empty() {
                lines.push(Line::from(Span::styled(
                    "   (ничего не найдено)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for (i, event) in app.search_results.iter().enumerate() {
                    let is_sel = i == selected;
                    let marker = if is_sel {
                        Span::styled(
                            "▸ ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("  ")
                    };

                    let date_span = Span::styled(
                        format!("[{}] ", event.date.format("%d.%m.%Y")),
                        Style::default().fg(Color::Cyan),
                    );

                    let time_str = match (event.start_time, event.end_time) {
                        (Some(s), Some(e)) => {
                            format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M"))
                        }
                        (Some(s), None) => format!("{} ", s.format("%H:%M")),
                        _ => String::new(),
                    };
                    let time_span = Span::styled(time_str, Style::default().fg(Color::Yellow));

                    let title_style = if is_sel {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let title_span = Span::styled(format!("{} ", event.title), title_style);

                    let tags_str = event
                        .tags
                        .iter()
                        .map(|t| format!("#{}", t.name))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let tags_span = Span::styled(tags_str, Style::default().fg(Color::DarkGray));

                    let line_spans = vec![marker, date_span, time_span, title_span, tags_span];
                    if is_sel {
                        lines.push(
                            Line::from(line_spans).style(Style::default().bg(Color::Rgb(20, 30, 45))),
                        );
                    } else {
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        }
    }

    // Scrolling logic
    let total_rendered_lines = lines.len();
    let scroll_offset = if selected >= list_height && total_rendered_lines > list_height {
        (selected + 1).saturating_sub(list_height)
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset)
        .take(list_height)
        .collect();

    frame.render_widget(
        Paragraph::new(visible_lines).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

fn render_event_line(event: &crate::model::EventOccurrence, is_sel: bool) -> Line<'static> {
    let marker = if is_sel {
        Span::styled(
            "▸ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };

    let imp_indicator = match event.importance {
        Importance::High => Span::styled(
            "! ",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Importance::Normal => Span::styled("• ", Style::default().fg(Color::Cyan)),
        Importance::Low => Span::styled("· ", Style::default().fg(Color::Blue)),
        Importance::None => Span::raw("  "),
    };

    let time_str = match (event.start_time, event.end_time) {
        (Some(s), Some(e)) => format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M")),
        (Some(s), None) => format!("{} ", s.format("%H:%M")),
        _ => "Весь день ".to_string(),
    };
    let time_span = Span::styled(time_str, Style::default().fg(Color::Yellow));

    let title_style = if is_sel {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let title_span = Span::styled(format!("{} ", event.title), title_style);

    let tags_str = event
        .tags
        .iter()
        .map(|t| format!("#{}", t.name))
        .collect::<Vec<_>>()
        .join(" ");
    let tags_span = Span::styled(tags_str, Style::default().fg(Color::Cyan));

    let line_spans = vec![marker, imp_indicator, time_span, title_span, tags_span];
    if is_sel {
        Line::from(line_spans).style(Style::default().bg(Color::Rgb(20, 30, 45)))
    } else {
        Line::from(line_spans)
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let footer_line = match app.tab {
        InlineTab::Day => Line::from(vec![
            Span::styled(" [↑/↓] ", Style::default().fg(Color::Cyan)),
            Span::styled("Выбор  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter] ", Style::default().fg(Color::Cyan)),
            Span::styled("Карточка  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[p] ", Style::default().fg(Color::Cyan)),
            Span::styled("Сводка дня  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[a] ", Style::default().fg(Color::Cyan)),
            Span::styled("Добавить  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab] ", Style::default().fg(Color::Cyan)),
            Span::styled("Режим", Style::default().fg(Color::DarkGray)),
        ]),
        InlineTab::Week => Line::from(vec![
            Span::styled(" [↑/↓] ", Style::default().fg(Color::Cyan)),
            Span::styled("Выбор  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter] ", Style::default().fg(Color::Cyan)),
            Span::styled("Карточка  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[a] ", Style::default().fg(Color::Cyan)),
            Span::styled("Добавить  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab] ", Style::default().fg(Color::Cyan)),
            Span::styled("Режим  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[F] ", Style::default().fg(Color::Cyan)),
            Span::styled("Полный TUI", Style::default().fg(Color::DarkGray)),
        ]),
        InlineTab::Search => Line::from(vec![
            Span::styled(" [↑/↓] ", Style::default().fg(Color::Cyan)),
            Span::styled("Выбор  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter] ", Style::default().fg(Color::Cyan)),
            Span::styled("Карточка  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc] ", Style::default().fg(Color::Cyan)),
            Span::styled("Очистить  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab] ", Style::default().fg(Color::Cyan)),
            Span::styled("Режим", Style::default().fg(Color::DarkGray)),
        ]),
    };

    frame.render_widget(Paragraph::new(footer_line), area);
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn test_render_inline_headless() {
        let backend = TestBackend::new(90, 13);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let app = InlineApp::new(today, InlineTab::Day);

        terminal
            .draw(|frame| {
                render_inline(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = format!("{:?}", buffer);

        assert!(rendered_text.contains("1 ДЕНЬ"));
        assert!(rendered_text.contains("2 НЕДЕЛЯ"));
        assert!(rendered_text.contains("3 ПОИСК"));
        assert!(rendered_text.contains("03.09.2026"));
    }
}
