use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::widgets::{FOCUSED, KEY_BADGE, KEY_LABEL, UNFOCUSED, centered, importance_style};
use crate::{app::App, search::SearchResult};

pub fn render_agenda(frame: &mut Frame, area: Rect, app: &App) {
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
        " ▌SEARCH: ACTIVE▐ "
    } else {
        " [ SEARCH ] "
    };
    let search_border = if app.state.agenda.searching {
        FOCUSED
    } else {
        UNFOCUSED
    };

    let search_content = if app.state.agenda.searching {
        Line::from(vec![
            Span::styled(
                "› ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.state.agenda.query,
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::new().fg(Color::Cyan)),
        ])
    } else if app.state.agenda.query.is_empty() {
        Line::from(vec![
            Span::styled("› ", Style::new().fg(Color::DarkGray)),
            Span::styled("Нажмите / для поиска...", Style::new().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("› ", Style::new().fg(Color::Cyan)),
            Span::styled(&app.state.agenda.query, Style::new().fg(Color::White)),
        ])
    };

    frame.render_widget(
        Paragraph::new(search_content).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    search_title,
                    if app.state.agenda.searching {
                        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(Color::DarkGray)
                    },
                ))
                .border_style(search_border),
        ),
        rows[0],
    );

    // Filters and Tags bar
    let filters = &app.state.agenda.filters;
    let filter_line = Line::from(vec![
        Span::styled("[f]", KEY_BADGE),
        Span::styled(" DATE:", KEY_LABEL),
        Span::styled(
            format!(" {:?}  ", filters.date),
            Style::new().fg(Color::Cyan),
        ),
        Span::styled("[r]", KEY_BADGE),
        Span::styled(" TYPE:", KEY_LABEL),
        Span::styled(
            format!(" {:?}  ", filters.item_type),
            Style::new().fg(Color::Cyan),
        ),
        Span::styled("[i]", KEY_BADGE),
        Span::styled(" PRI:", KEY_LABEL),
        Span::styled(
            format!(" {:?}  ", filters.importance),
            Style::new().fg(Color::Cyan),
        ),
        Span::styled("[s]", KEY_BADGE),
        Span::styled(" SORT:", KEY_LABEL),
        Span::styled(
            format!(" {:?}  ", filters.sort),
            Style::new().fg(Color::Cyan),
        ),
        Span::styled("[A]", KEY_BADGE),
        Span::styled(" TAGS:", KEY_LABEL),
        Span::styled(
            format!(" {:?}", filters.tag_matching),
            Style::new().fg(Color::Cyan),
        ),
    ]);

    let tag_capacity = (popup.width / 14).max(1) as usize;
    let tag_start = app
        .state
        .agenda
        .tag_cursor
        .saturating_sub(tag_capacity.saturating_sub(1));

    let mut tag_spans = vec![
        Span::styled("[[/]]", KEY_BADGE),
        Span::styled(" TAGS  ", KEY_LABEL),
        Span::styled("[X]", KEY_BADGE),
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

        if selected {
            tag_spans.push(Span::styled(
                format!(" [✓ #{}] ", tag.name),
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if is_cursor {
            tag_spans.push(Span::styled(
                format!(" [#{}] ", tag.name),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        } else {
            tag_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                Style::new().fg(Color::DarkGray),
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
                SearchResult::Event(event) => (
                    event
                        .start_time
                        .map(|time| time.format("%H:%M").to_string())
                        .unwrap_or_else(|| "день".into()),
                    app.config.importance_symbol(event.importance).to_owned(),
                    event.importance,
                    if event.is_recurring {
                        "↻ REPEAT"
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
                ),
                SearchResult::Note(note) => (
                    "-".into(),
                    String::new(),
                    crate::model::Importance::None,
                    "NOTE".to_owned(),
                    note.title.clone().unwrap_or_else(|| "Без названия".into()),
                    String::new(),
                ),
            };

            let sel_style = Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);

            let (date_span, time_span, pri_span, kind_span, title_span, tags_span) =
                if is_row_selected {
                    (
                        Span::styled(format!("▸ {}", item.date().format("%d.%m.%Y")), sel_style),
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
                            Style::new().fg(Color::White),
                        ),
                        Span::styled(
                            time.clone(),
                            if time == "день" || time == "-" {
                                Style::new().fg(Color::DarkGray)
                            } else {
                                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                            },
                        ),
                        Span::styled(importance_sym, importance_style(importance_val)),
                        Span::styled(kind, Style::new().fg(Color::DarkGray)),
                        Span::styled(title, Style::new().fg(Color::White)),
                        Span::styled(tags, Style::new().fg(Color::DarkGray)),
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
                .style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    format!(" ▌AGENDA // RESULTS: {total_results}▐ "),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
                .border_style(FOCUSED),
        ),
        rows[2],
    );
}
