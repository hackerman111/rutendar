use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::widgets::{
    FOCUSED, KEY_BADGE, KEY_LABEL, TIME_STYLE, centered, importance_style, relative_date, tags_line,
};
use crate::app::App;

pub fn render_upcoming(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 76, 80);
    frame.render_widget(Clear, popup);
    let capacity = (popup.height.saturating_sub(2) / 2).max(1) as usize;
    let start = app
        .state
        .upcoming
        .selected
        .saturating_sub(capacity.saturating_sub(1));

    let items = app
        .state
        .upcoming
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, event)| {
            let is_selected = index == app.state.upcoming.selected;
            let sel_style = Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let mut line_spans = Vec::new();

            if is_selected {
                line_spans.push(Span::styled("▸ ", sel_style));
            } else {
                line_spans.push(Span::raw("  "));
            }

            let rel_date = relative_date(app.state.today, event.date);
            if !rel_date.is_empty() {
                let badge_style = if is_selected {
                    sel_style
                } else if event.date == app.state.today {
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(Color::Yellow)
                };
                line_spans.push(Span::styled(format!("[{}]", rel_date.trim()), badge_style));
                line_spans.push(if is_selected {
                    Span::styled(" ", sel_style)
                } else {
                    Span::raw(" ")
                });
            }

            if event.is_recurring {
                let rec_style = if is_selected {
                    sel_style
                } else {
                    Style::new().fg(Color::DarkGray)
                };
                line_spans.push(Span::styled("↻ ", rec_style));
            }

            let sym = app.config.importance_symbol(event.importance);
            if !sym.trim().is_empty() {
                let pri_style = if is_selected {
                    sel_style
                } else {
                    importance_style(event.importance)
                };
                line_spans.push(Span::styled(format!("{sym} "), pri_style));
            }

            let time_str = event
                .start_time
                .map(|time| time.format("%H:%M ").to_string())
                .unwrap_or_default();
            if !time_str.is_empty() {
                let t_style = if is_selected { sel_style } else { TIME_STYLE };
                line_spans.push(Span::styled(time_str, t_style));
            }

            let title_style = if is_selected {
                sel_style
            } else {
                Style::new().fg(Color::White)
            };
            line_spans.push(Span::styled(format!("{} ", event.title), title_style));

            let mut detail_spans = vec![Span::raw("    ")];
            let tags = tags_line(event);
            if !tags.is_empty() {
                detail_spans.push(Span::styled(
                    tags,
                    if is_selected {
                        Style::new().fg(Color::Cyan)
                    } else {
                        Style::new().fg(Color::DarkGray)
                    },
                ));
            }

            if let Some(link) = app
                .state
                .upcoming
                .links_by_date
                .get(&event.date)
                .and_then(|links| links.first())
            {
                if !detail_spans.is_empty() {
                    detail_spans.push(Span::raw("  "));
                }
                detail_spans.push(Span::styled(
                    format!("🔗 {} › {}", link.label, link.url),
                    if is_selected {
                        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(Color::DarkGray)
                    },
                ));
            }

            ListItem::new(vec![Line::from(line_spans), Line::from(detail_spans)])
        })
        .collect::<Vec<_>>();

    let title = Line::from(vec![
        Span::styled(
            " ▌UPCOMING // БЛИЖАЙШИЕ▐ ",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("[s]", KEY_BADGE),
        Span::styled(format!(" SORT: {:?} ", app.state.upcoming.sort), KEY_LABEL),
    ]);

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(FOCUSED),
        ),
        popup,
    );
}
