use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::widgets::{
    KEY_LABEL, centered, theme_border_type, theme_focused, theme_importance_style, theme_selected,
    theme_unfocused,
};
use crate::{app::App, search::SearchResult, ui::Theme};

pub fn render_agenda(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
    let popup = centered(area, 92, 86);
    frame.render_widget(Clear, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(popup);

    // Search bar
    let search_title = if app.state.agenda.searching {
        " SEARCH: ACTIVE "
    } else {
        " [ SEARCH ] "
    };
    let search_border = if app.state.agenda.searching {
        theme_focused(theme)
    } else {
        theme_unfocused(theme)
    };

    let search_content = if app.state.agenda.searching {
        Line::from(vec![
            Span::styled("› ", theme.key_badge_style()),
            Span::styled(&app.state.agenda.query, theme.title_style(true, false)),
            Span::styled("█", theme.time_style()),
        ])
    } else if app.state.agenda.query.is_empty() {
        Line::from(vec![
            Span::styled("› ", theme_unfocused(theme)),
            Span::styled("Нажмите / для поиска...", theme_unfocused(theme)),
        ])
    } else {
        Line::from(vec![
            Span::styled("› ", theme.time_style()),
            Span::styled(&app.state.agenda.query, theme.title_style(false, false)),
        ])
    };

    frame.render_widget(
        Paragraph::new(search_content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(
                    search_title,
                    if app.state.agenda.searching {
                        theme_selected(theme)
                    } else {
                        theme_unfocused(theme)
                    },
                ))
                .border_style(search_border),
        ),
        rows[0],
    );

    // Filters and Tags bar
    let filters = &app.state.agenda.filters;
    let filter_line = Line::from(vec![
        Span::styled("[f]", theme.key_badge_style()),
        Span::styled(" DATE:", KEY_LABEL),
        Span::styled(format!(" {}  ", filters.date.label()), theme.time_style()),
        Span::styled("[R]", theme.key_badge_style()),
        Span::styled(" TYPE:", KEY_LABEL),
        Span::styled(format!(" {:?}  ", filters.item_type), theme.time_style()),
        Span::styled("[i]", theme.key_badge_style()),
        Span::styled(" PRI:", KEY_LABEL),
        Span::styled(format!(" {:?}  ", filters.importance), theme.time_style()),
        Span::styled("[s]", theme.key_badge_style()),
        Span::styled(" SORT:", KEY_LABEL),
        Span::styled(format!(" {:?}  ", filters.sort), theme.time_style()),
        Span::styled("[A]", theme.key_badge_style()),
        Span::styled(" TAGS:", KEY_LABEL),
        Span::styled(format!(" {:?}", filters.tag_matching), theme.time_style()),
    ]);

    let tag_capacity = (popup.width / 14).max(1) as usize;
    let tag_start = app
        .state
        .agenda
        .tag_cursor
        .saturating_sub(tag_capacity.saturating_sub(1));

    let mut tag_spans = vec![
        Span::styled("[[/]]", theme.key_badge_style()),
        Span::styled(" TAGS  ", KEY_LABEL),
        Span::styled("[X]", theme.key_badge_style()),
        Span::styled(" DEL TAG  ", KEY_LABEL),
    ];

    for (index, tag) in app
        .state
        .agenda
        .available_tags
        .iter()
        .enumerate()
        .skip(tag_start)
        .take(tag_capacity)
    {
        let selected = filters.tags.contains(&tag.normalized_name);
        let is_cursor = index == app.state.agenda.tag_cursor;

        let check_sym = if theme == Theme::Ascii { "[v]" } else { "✓" };
        if selected {
            tag_spans.push(Span::styled(
                format!(" {check_sym} #{} ", tag.name),
                theme_selected(theme),
            ));
        } else if is_cursor {
            tag_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                theme.key_badge_style(),
            ));
        } else {
            tag_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                theme_unfocused(theme),
            ));
        }
        tag_spans.push(Span::raw(" "));
    }

    let tag_line = Line::from(tag_spans);

    frame.render_widget(Paragraph::new(vec![filter_line, tag_line]), rows[1]);

    // Results table
    let capacity = rows[2].height.saturating_sub(3).max(1) as usize;
    let start = app
        .state
        .agenda
        .selected
        .saturating_sub(capacity.saturating_sub(1));

    let cursor = match theme {
        Theme::Default => "▸ ",
        Theme::Ascii => "> ",
    };

    let table_rows = app
        .state
        .agenda
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, item)| {
            let is_row_selected = index == app.state.agenda.selected;
            let (time, importance_sym, importance_val, kind, title, tags) = match item {
                SearchResult::Event(event) => {
                    let imp_disp = if theme == Theme::Ascii {
                        match event.importance {
                            crate::model::Importance::High => "[!]".into(),
                            crate::model::Importance::Normal => "[.]".into(),
                            crate::model::Importance::Low => "[-]".into(),
                            crate::model::Importance::None => String::new(),
                        }
                    } else {
                        app.config.importance_symbol(event.importance).to_owned()
                    };
                    let rec_label = if theme == Theme::Ascii {
                        "(R) REPEAT"
                    } else {
                        "↻ REPEAT"
                    };

                    (
                        event
                            .start_time
                            .map(|time| time.format("%H:%M").to_string())
                            .unwrap_or_else(|| "день".into()),
                        imp_disp,
                        event.importance,
                        if event.is_recurring {
                            rec_label
                        } else {
                            "EVENT"
                        }
                        .to_owned(),
                        event.title.clone(),
                        event
                            .tags
                            .iter()
                            .map(|tag| format!("#{}", tag.name))
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                }
                SearchResult::Note(note) => (
                    "-".into(),
                    String::new(),
                    crate::model::Importance::None,
                    "NOTE".to_owned(),
                    note.title.clone().unwrap_or_else(|| "Без названия".into()),
                    String::new(),
                ),
            };

            let sel_style = theme_selected(theme);

            let (date_span, time_span, pri_span, kind_span, title_span, tags_span) =
                if is_row_selected {
                    (
                        Span::styled(
                            format!("{cursor}{}", item.date().format("%d.%m.%Y")),
                            sel_style,
                        ),
                        Span::styled(time, sel_style),
                        Span::styled(importance_sym, sel_style),
                        Span::styled(kind, sel_style),
                        Span::styled(title, sel_style),
                        Span::styled(tags, sel_style),
                    )
                } else {
                    (
                        Span::styled(
                            format!("  {}", item.date().format("%d.%m.%Y")),
                            theme.title_style(false, false),
                        ),
                        Span::styled(
                            time.clone(),
                            if time == "день" || time == "-" {
                                theme_unfocused(theme)
                            } else {
                                theme.time_style()
                            },
                        ),
                        Span::styled(
                            importance_sym,
                            theme_importance_style(theme, importance_val),
                        ),
                        Span::styled(kind, theme_unfocused(theme)),
                        Span::styled(title, theme.title_style(false, false)),
                        Span::styled(tags, theme_unfocused(theme)),
                    )
                };

            Row::new([
                Cell::from(date_span),
                Cell::from(time_span),
                Cell::from(pri_span),
                Cell::from(kind_span),
                Cell::from(title_span),
                Cell::from(tags_span),
            ])
            .style(if is_row_selected {
                sel_style
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();

    let total_results = app.state.agenda.items.len();
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(13),
                Constraint::Length(8),
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Percentage(42),
                Constraint::Percentage(23),
            ],
        )
        .header(
            Row::new(["  DATE", "TIME", "PRI", "TYPE", "EVENT / NOTE", "TAGS"])
                .style(theme.time_style()),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(
                    format!(" AGENDA // RESULTS: {total_results} "),
                    theme_selected(theme),
                ))
                .border_style(theme_focused(theme)),
        ),
        rows[2],
    );
}
