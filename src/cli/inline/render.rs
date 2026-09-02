use chrono::{Datelike, Duration, Weekday};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::state::{InlineApp, InlineTab};
use crate::ui::Theme;

pub fn render_inline(frame: &mut Frame<'_>, area: Rect, app: &InlineApp) {
    let header_line = build_header(app);
    let footer_line = build_footer(app);

    let border_color = app.theme.border_color(app.pending_delete.is_some());

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.theme.border_type())
        .border_style(Style::default().fg(border_color))
        .title(header_line)
        .title_bottom(footer_line);

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if inner_area.height == 0 || inner_area.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Subheader / Context line
            Constraint::Min(1),    // List content
        ])
        .split(inner_area);

    render_subheader(frame, chunks[0], app);
    render_content(frame, chunks[1], app);
}

fn build_header(app: &InlineApp) -> Line<'static> {
    let brand = Span::styled(" rutendar ", app.theme.key_badge_style());

    let active_tab_style = app.theme.active_tab_style();
    let inactive_tab_style = app.theme.inactive_tab_style();

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

    let weekday_short = match app.today.weekday() {
        Weekday::Mon => "Пн",
        Weekday::Tue => "Вт",
        Weekday::Wed => "Ср",
        Weekday::Thu => "Чт",
        Weekday::Fri => "Пт",
        Weekday::Sat => "Сб",
        Weekday::Sun => "Вс",
    };

    let today_banner = format!(
        " {} {}, {} ",
        app.theme.date_icon(),
        weekday_short,
        app.today.format("%d.%m")
    );

    let sep = if app.theme == Theme::Plain {
        "- "
    } else {
        "─ "
    };
    let sep_end = if app.theme == Theme::Plain {
        " -"
    } else {
        " ─"
    };

    let theme_label = format!("[m: {}]", app.theme.name());

    Line::from(vec![
        brand,
        Span::styled(sep, inactive_tab_style),
        tab_day,
        Span::raw(" "),
        tab_week,
        Span::raw(" "),
        tab_search,
        Span::styled(sep_end, inactive_tab_style),
        Span::styled(today_banner, app.theme.title_style(false, false)),
        Span::styled(
            format!("(F: TUI · {theme_label} · q: Выход) "),
            inactive_tab_style,
        ),
    ])
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

            let (arrow_l, arrow_r) = if app.theme == Theme::Plain {
                ("<- ", " ->")
            } else {
                ("← ", " →")
            };

            Line::from(vec![
                Span::raw(" "),
                Span::styled(app.theme.date_icon(), app.theme.key_badge_style()),
                Span::styled(arrow_l, app.theme.key_badge_style()),
                Span::styled(
                    format!("{} ({})", app.current_date.format("%d.%m.%Y"), day_label),
                    app.theme.time_style(),
                ),
                Span::styled(arrow_r, app.theme.key_badge_style()),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled(
                    format!("{} событий", app.day_events.len()),
                    app.theme.title_style(false, false),
                ),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled(
                    format!("{} задач", app.day_tasks.len()),
                    app.theme.title_style(false, false),
                ),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled("[←/→: день · t: сегодня]", app.theme.inactive_tab_style()),
            ])
        }
        InlineTab::Week => {
            let weekday_offset = app.current_date.weekday().num_days_from_monday() as i64;
            let monday = app.current_date - Duration::days(weekday_offset);
            let sunday = monday + Duration::days(6);
            Line::from(vec![
                Span::raw(" "),
                Span::styled(app.theme.date_icon(), app.theme.key_badge_style()),
                Span::styled(
                    format!(
                        "Неделя {} ({} — {})",
                        monday.iso_week().week(),
                        monday.format("%d.%m"),
                        sunday.format("%d.%m.%Y")
                    ),
                    app.theme.time_style(),
                ),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled(
                    format!("{} событий за неделю", app.week_events.len()),
                    app.theme.title_style(false, false),
                ),
            ])
        }
        InlineTab::Search => {
            let query_display = if app.query.is_empty() {
                Span::styled(
                    "введите текст для поиска...",
                    app.theme.inactive_tab_style(),
                )
            } else {
                Span::styled(&app.query, app.theme.title_style(true, false))
            };

            let cursor_sym = if app.theme == Theme::Plain {
                "_"
            } else {
                "█"
            };

            Line::from(vec![
                Span::raw(" "),
                Span::styled(app.theme.search_icon(), app.theme.key_badge_style()),
                query_display,
                Span::styled(cursor_sym, app.theme.key_badge_style()),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled(
                    format!("Найдено: {}", app.search_results.len()),
                    app.theme.title_style(false, false),
                ),
                Span::styled("  │  ", app.theme.inactive_tab_style()),
                Span::styled("(Esc: стереть)", app.theme.inactive_tab_style()),
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
                    app.theme.inactive_tab_style(),
                )));
            } else {
                // Render events
                for (i, event) in app.day_events.iter().enumerate() {
                    let is_sel = i == selected;
                    lines.push(render_event_line(event, is_sel, app.theme));
                }

                // Render tasks section
                if !app.day_tasks.is_empty() {
                    let task_divider = if app.theme == Theme::Plain {
                        "  -- Задачи -------------------------------------------------------------"
                    } else {
                        "  ── Задачи ─────────────────────────────────────────────────────────────"
                    };
                    lines.push(Line::from(Span::styled(
                        task_divider,
                        app.theme.inactive_tab_style(),
                    )));

                    for (j, task) in app.day_tasks.iter().enumerate() {
                        let is_sel = (app.day_events.len() + j) == selected;
                        let marker = app.theme.cursor_marker(is_sel);
                        let checkbox = app.theme.task_checkbox_span(task.is_done);
                        let title_style = app.theme.title_style(is_sel, task.is_done);
                        let imp_span = app.theme.importance_span(task.importance);

                        let mut spans = vec![
                            marker,
                            checkbox,
                            Span::styled(&task.title, title_style),
                            imp_span,
                        ];
                        if is_sel && !task.is_done {
                            spans.push(Span::styled(
                                " (Space: выполнено)",
                                app.theme.inactive_tab_style(),
                            ));
                        }

                        let line = if is_sel {
                            Line::from(spans).style(app.theme.selection_style())
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
                    app.theme.inactive_tab_style(),
                )));
            } else {
                for (i, event) in app.week_events.iter().enumerate() {
                    let is_sel = i == selected;
                    let marker = app.theme.cursor_marker(is_sel);

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
                        app.theme.key_badge_style(),
                    );

                    let time_str = match (event.start_time, event.end_time) {
                        (Some(s), Some(e)) => {
                            format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M"))
                        }
                        (Some(s), None) => format!("{} ", s.format("%H:%M")),
                        _ => "Весь день ".to_string(),
                    };
                    let time_span = Span::styled(time_str, app.theme.time_style());

                    let title_style = app.theme.title_style(is_sel, false);
                    let title_span = Span::styled(format!("{} ", event.title), title_style);

                    let tags_str = event
                        .tags
                        .iter()
                        .map(|t| format!("#{:<0}", t.name))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let tags_span = Span::styled(tags_str, app.theme.tag_style());

                    let line_spans = vec![marker, date_span, time_span, title_span, tags_span];
                    if is_sel {
                        lines.push(Line::from(line_spans).style(app.theme.selection_style()));
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
                    app.theme.inactive_tab_style(),
                )));
            } else {
                for (i, event) in app.search_results.iter().enumerate() {
                    let is_sel = i == selected;
                    let marker = app.theme.cursor_marker(is_sel);

                    let date_span = Span::styled(
                        format!("[{}] ", event.date.format("%d.%m.%Y")),
                        app.theme.key_badge_style(),
                    );

                    let time_str = match (event.start_time, event.end_time) {
                        (Some(s), Some(e)) => {
                            format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M"))
                        }
                        (Some(s), None) => format!("{} ", s.format("%H:%M")),
                        _ => String::new(),
                    };
                    let time_span = Span::styled(time_str, app.theme.time_style());

                    let title_style = app.theme.title_style(is_sel, false);
                    let title_span = Span::styled(format!("{} ", event.title), title_style);

                    let tags_str = event
                        .tags
                        .iter()
                        .map(|t| format!("#{}", t.name))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let tags_span = Span::styled(tags_str, app.theme.tag_style());

                    let line_spans = vec![marker, date_span, time_span, title_span, tags_span];
                    if is_sel {
                        lines.push(Line::from(line_spans).style(app.theme.selection_style()));
                    } else {
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        }
    }

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

    frame.render_widget(Paragraph::new(visible_lines), area);
}

fn render_event_line(
    event: &crate::model::EventOccurrence,
    is_sel: bool,
    theme: Theme,
) -> Line<'static> {
    let marker = theme.cursor_marker(is_sel);
    let imp_indicator = theme.importance_span(event.importance);

    let time_str = match (event.start_time, event.end_time) {
        (Some(s), Some(e)) => format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M")),
        (Some(s), None) => format!("{} ", s.format("%H:%M")),
        _ => "Весь день ".to_string(),
    };
    let time_span = Span::styled(time_str, theme.time_style());

    let title_style = theme.title_style(is_sel, false);
    let title_span = Span::styled(format!("{} ", event.title), title_style);

    let tags_str = event
        .tags
        .iter()
        .map(|t| format!("#{}", t.name))
        .collect::<Vec<_>>()
        .join(" ");
    let tags_span = Span::styled(tags_str, theme.tag_style());

    let line_spans = vec![marker, imp_indicator, time_span, title_span, tags_span];
    if is_sel {
        Line::from(line_spans).style(theme.selection_style())
    } else {
        Line::from(line_spans)
    }
}

fn build_footer(app: &InlineApp) -> Line<'static> {
    if let Some(del) = &app.pending_delete {
        let (kind, title) = match del {
            crate::cli::inline::state::PendingDelete::Event { title, .. } => {
                ("событие", title.as_str())
            }
            crate::cli::inline::state::PendingDelete::Task { title, .. } => {
                ("задачу", title.as_str())
            }
        };
        return Line::from(vec![
            Span::styled(
                " ⚠ ",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Удалить {kind} \"{title}\"? "),
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("[y] ", app.theme.key_badge_style()),
            Span::styled("Да · ", app.theme.title_style(false, false)),
            Span::styled("[Любая клавиша] ", app.theme.inactive_tab_style()),
            Span::styled("Отмена ", app.theme.inactive_tab_style()),
        ]);
    }

    match app.tab {
        InlineTab::Day => Line::from(vec![
            Span::styled(" [↑/↓] ", app.theme.key_badge_style()),
            Span::styled("Выбор · ", app.theme.inactive_tab_style()),
            Span::styled("[Enter] ", app.theme.key_badge_style()),
            Span::styled("Карта · ", app.theme.inactive_tab_style()),
            Span::styled("[e] ", app.theme.key_badge_style()),
            Span::styled("Изм · ", app.theme.inactive_tab_style()),
            Span::styled("[a/A] ", app.theme.key_badge_style()),
            Span::styled("Доб · ", app.theme.inactive_tab_style()),
            Span::styled("[x] ", app.theme.key_badge_style()),
            Span::styled("Удал · ", app.theme.inactive_tab_style()),
            Span::styled("[m] ", app.theme.key_badge_style()),
            Span::styled("Тема · ", app.theme.inactive_tab_style()),
            Span::styled("[Space] ", app.theme.key_badge_style()),
            Span::styled("Статус · ", app.theme.inactive_tab_style()),
            Span::styled("[Tab] ", app.theme.key_badge_style()),
            Span::styled("Режим ", app.theme.inactive_tab_style()),
        ]),
        InlineTab::Week => Line::from(vec![
            Span::styled(" [↑/↓] ", app.theme.key_badge_style()),
            Span::styled("Выбор · ", app.theme.inactive_tab_style()),
            Span::styled("[Enter] ", app.theme.key_badge_style()),
            Span::styled("Карта · ", app.theme.inactive_tab_style()),
            Span::styled("[e] ", app.theme.key_badge_style()),
            Span::styled("Изм · ", app.theme.inactive_tab_style()),
            Span::styled("[a] ", app.theme.key_badge_style()),
            Span::styled("Доб · ", app.theme.inactive_tab_style()),
            Span::styled("[x] ", app.theme.key_badge_style()),
            Span::styled("Удал · ", app.theme.inactive_tab_style()),
            Span::styled("[m] ", app.theme.key_badge_style()),
            Span::styled("Тема · ", app.theme.inactive_tab_style()),
            Span::styled("[Tab] ", app.theme.key_badge_style()),
            Span::styled("Режим · ", app.theme.inactive_tab_style()),
            Span::styled("[F] ", app.theme.key_badge_style()),
            Span::styled("TUI ", app.theme.inactive_tab_style()),
        ]),
        InlineTab::Search => Line::from(vec![
            Span::styled(" [↑/↓] ", app.theme.key_badge_style()),
            Span::styled("Выбор · ", app.theme.inactive_tab_style()),
            Span::styled("[Enter] ", app.theme.key_badge_style()),
            Span::styled("Карта · ", app.theme.inactive_tab_style()),
            Span::styled("[e] ", app.theme.key_badge_style()),
            Span::styled("Изм · ", app.theme.inactive_tab_style()),
            Span::styled("[x] ", app.theme.key_badge_style()),
            Span::styled("Удал · ", app.theme.inactive_tab_style()),
            Span::styled("[m] ", app.theme.key_badge_style()),
            Span::styled("Тема · ", app.theme.inactive_tab_style()),
            Span::styled("[Esc] ", app.theme.key_badge_style()),
            Span::styled("Сброс · ", app.theme.inactive_tab_style()),
            Span::styled("[Tab] ", app.theme.key_badge_style()),
            Span::styled("Режим ", app.theme.inactive_tab_style()),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn test_render_inline_headless_neo_and_plain() {
        let backend = TestBackend::new(90, 13);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        // 1. Test Neo Theme
        let app_neo = InlineApp::new(today, InlineTab::Day).with_theme(Theme::Neo);
        terminal
            .draw(|frame| {
                render_inline(frame, frame.area(), &app_neo);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text_neo = format!("{:?}", buffer);
        assert!(text_neo.contains("1 ДЕНЬ"));
        assert!(text_neo.contains("2 НЕДЕЛЯ"));
        assert!(text_neo.contains("3 ПОИСК"));
        assert!(text_neo.contains("Neo"));

        // 2. Test Plain Theme (ASCII)
        let app_plain = InlineApp::new(today, InlineTab::Day).with_theme(Theme::Plain);
        terminal
            .draw(|frame| {
                render_inline(frame, frame.area(), &app_plain);
            })
            .unwrap();

        let buffer_plain = terminal.backend().buffer();
        let text_plain = format!("{:?}", buffer_plain);
        assert!(text_plain.contains("[D]"));
        assert!(text_plain.contains("Plain"));
    }
}
