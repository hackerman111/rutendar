use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::widgets::{FOCUSED, KEY_BADGE, KEY_LABEL, SELECTED, UNFOCUSED, centered};
use crate::app::App;

pub fn render_link_bank(frame: &mut Frame, area: Rect, app: &App) {
    let Some(bank) = app.state.link_bank.as_ref() else {
        return;
    };
    let popup = centered(area, 92, 86);
    frame.render_widget(Clear, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(popup);

    let search_content = if bank.searching {
        Line::from(vec![
            Span::styled(
                "› ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &bank.query,
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::new().fg(Color::Cyan)),
        ])
    } else if bank.query.is_empty() {
        Line::from(Span::styled(
            "› Нажмите / для поиска по тегам и описанию...",
            Style::new().fg(Color::DarkGray),
        ))
    } else {
        Line::from(vec![
            Span::styled("› ", Style::new().fg(Color::Cyan)),
            Span::styled(&bank.query, Style::new().fg(Color::White)),
        ])
    };
    frame.render_widget(
        Paragraph::new(search_content).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    if bank.searching {
                        " ПОИСК: АКТИВЕН "
                    } else {
                        " ПОИСК "
                    },
                    if bank.searching {
                        SELECTED
                    } else {
                        Style::new().fg(Color::DarkGray)
                    },
                ))
                .border_style(if bank.searching { FOCUSED } else { UNFOCUSED }),
        ),
        rows[0],
    );

    let capacity = rows[1].height.saturating_sub(3).max(1) as usize;
    let start = bank.selected.saturating_sub(capacity.saturating_sub(1));
    let selected_ids = &bank.event_form.favorite_link_ids;
    let table_rows = bank
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, link)| {
            let row_selected = index == bank.selected;
            let style = if row_selected {
                SELECTED
            } else {
                Style::default()
            };
            Row::new([
                Cell::from(if selected_ids.contains(&link.id) {
                    " ✓ "
                } else {
                    "   "
                }),
                Cell::from(link.label.clone()),
                Cell::from(link.tags.clone()),
                Cell::from(link.description.clone().unwrap_or_default()),
                Cell::from(link.url.clone()),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();

    let footer = Line::from(vec![
        Span::styled("[Enter/Space]", KEY_BADGE),
        Span::styled(" LINK  ", KEY_LABEL),
        Span::styled("[a]", KEY_BADGE),
        Span::styled(" NEW  ", KEY_LABEL),
        Span::styled("[e]", KEY_BADGE),
        Span::styled(" EDIT  ", KEY_LABEL),
        Span::styled(format!("[{}]", app.config.keys.open_link), KEY_BADGE),
        Span::styled(" OPEN  ", KEY_LABEL),
        Span::styled("[/]", KEY_BADGE),
        Span::styled(" SEARCH  ", KEY_LABEL),
        Span::styled("[Esc]", KEY_BADGE),
        Span::styled(" BACK", KEY_LABEL),
    ]);
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(3),
                Constraint::Percentage(20),
                Constraint::Percentage(18),
                Constraint::Percentage(27),
                Constraint::Percentage(35),
            ],
        )
        .header(
            Row::new(["", "ССЫЛКА", "ТЕГИ", "ОПИСАНИЕ", "URL"])
                .style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    format!(" БАНК ИЗБРАННЫХ ССЫЛОК // RESULTS: {} ", bank.items.len()),
                    SELECTED,
                ))
                .title_bottom(footer)
                .border_style(FOCUSED),
        ),
        rows[1],
    );
}
