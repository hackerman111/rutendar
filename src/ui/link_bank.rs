use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::widgets::{
    KEY_LABEL, centered, theme_border_type, theme_focused, theme_selected, theme_unfocused,
};
use crate::{app::App, ui::Theme};

pub fn render_link_bank(frame: &mut Frame, area: Rect, app: &App) {
    let Some(bank) = app.state.link_bank.as_ref() else {
        return;
    };
    let theme = app.config.ui.theme;
    let popup = centered(area, 92, 86);
    frame.render_widget(Clear, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(popup);

    let search_content = if bank.searching {
        Line::from(vec![
            Span::styled("› ", theme.key_badge_style()),
            Span::styled(&bank.query, theme.title_style(true, false)),
            Span::styled("█", theme.time_style()),
        ])
    } else if bank.query.is_empty() {
        Line::from(Span::styled(
            "› Нажмите / для поиска по тегам и описанию...",
            theme_unfocused(theme),
        ))
    } else {
        Line::from(vec![
            Span::styled("› ", theme.time_style()),
            Span::styled(&bank.query, theme.title_style(false, false)),
        ])
    };
    frame.render_widget(
        Paragraph::new(search_content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(
                    if bank.searching {
                        " ПОИСК: АКТИВЕН "
                    } else {
                        " ПОИСК "
                    },
                    if bank.searching {
                        theme_selected(theme)
                    } else {
                        theme_unfocused(theme)
                    },
                ))
                .border_style(if bank.searching {
                    theme_focused(theme)
                } else {
                    theme_unfocused(theme)
                }),
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
                theme_selected(theme)
            } else {
                Style::default()
            };
            let check_icon = if theme == Theme::Ascii {
                "[v]"
            } else {
                " ✓ "
            };

            Row::new([
                Cell::from(if selected_ids.contains(&link.id) {
                    check_icon
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
        Span::styled("[Enter/Space]", theme.key_badge_style()),
        Span::styled(" LINK  ", KEY_LABEL),
        Span::styled("[a]", theme.key_badge_style()),
        Span::styled(" NEW  ", KEY_LABEL),
        Span::styled("[e]", theme.key_badge_style()),
        Span::styled(" EDIT  ", KEY_LABEL),
        Span::styled(
            format!("[{}]", app.config.keys.open_link),
            theme.key_badge_style(),
        ),
        Span::styled(" OPEN  ", KEY_LABEL),
        Span::styled("[/]", theme.key_badge_style()),
        Span::styled(" SEARCH  ", KEY_LABEL),
        Span::styled("[Esc]", theme_unfocused(theme)),
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
        .header(Row::new(["", "ССЫЛКА", "ТЕГИ", "ОПИСАНИЕ", "URL"]).style(theme.time_style()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(
                    format!(" БАНК ИЗБРАННЫХ ССЫЛОК // RESULTS: {} ", bank.items.len()),
                    theme_selected(theme),
                ))
                .title_bottom(footer)
                .border_style(theme_focused(theme)),
        ),
        rows[1],
    );
}
