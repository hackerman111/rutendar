use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::widgets::{
    KEY_LABEL, centered, relative_date, tags_line, theme_border_type, theme_focused,
    theme_importance_style, theme_selected, theme_today_badge, theme_unfocused,
};
use crate::{app::App, ui::Theme};

pub fn render_upcoming(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
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
            let sel_style = theme_selected(theme);
            let mut line_spans = Vec::new();

            if is_selected {
                let cursor = match theme {
                    Theme::Default => "▸ ",
                    Theme::Ascii => "> ",
                };

                line_spans.push(Span::styled(cursor, sel_style));
            } else {
                line_spans.push(Span::raw("  "));
            }

            let rel_date = relative_date(app.state.today, event.date);
            if !rel_date.is_empty() {
                let badge_style = if is_selected {
                    sel_style
                } else if event.date == app.state.today {
                    theme_today_badge(theme)
                } else {
                    theme.key_badge_style()
                };
                let badge = if is_selected || event.date == app.state.today {
                    format!(" {} ", rel_date.trim())
                } else {
                    format!("[{}]", rel_date.trim())
                };
                line_spans.push(Span::styled(badge, badge_style));
                line_spans.push(if is_selected {
                    Span::styled(" ", sel_style)
                } else {
                    Span::raw(" ")
                });
            }

            if event.is_recurring {
                let rec_sym = if theme == Theme::Ascii {
                    "(R) "
                } else {
                    "↻ "
                };
                let rec_style = if is_selected {
                    sel_style
                } else {
                    theme_unfocused(theme)
                };
                line_spans.push(Span::styled(rec_sym, rec_style));
            }

            let sym = app.config.importance_symbol(event.importance);
            if !sym.trim().is_empty() {
                let pri_style = if is_selected {
                    sel_style
                } else {
                    theme_importance_style(theme, event.importance)
                };
                let disp_sym = if theme == Theme::Ascii {
                    match event.importance {
                        crate::model::Importance::High => "[!] ",
                        crate::model::Importance::Normal => "[.] ",
                        crate::model::Importance::Low => "[-] ",
                        crate::model::Importance::None => "    ",
                    }
                } else {
                    sym
                };
                line_spans.push(Span::styled(disp_sym.to_string(), pri_style));
                if theme != Theme::Ascii {
                    line_spans.push(Span::raw(" "));
                }
            }

            let time_str = event
                .start_time
                .map(|time| time.format("%H:%M ").to_string())
                .unwrap_or_default();
            if !time_str.is_empty() {
                let t_style = if is_selected {
                    sel_style
                } else {
                    theme.time_style()
                };
                line_spans.push(Span::styled(time_str, t_style));
            }

            let title_style = if is_selected {
                sel_style
            } else {
                theme.title_style(false, false)
            };
            line_spans.push(Span::styled(format!("{} ", event.title), title_style));

            let mut detail_spans = vec![Span::raw("    ")];
            let tags = tags_line(event);
            if !tags.is_empty() {
                detail_spans.push(Span::styled(
                    tags,
                    if is_selected {
                        theme.time_style()
                    } else {
                        theme_unfocused(theme)
                    },
                ));
            }

            let attached_link = event
                .favorite_links
                .first()
                .map(|link| (link.label.as_str(), link.url.as_str()))
                .or_else(|| {
                    app.state
                        .upcoming
                        .links_by_date
                        .get(&event.date)
                        .and_then(|links| links.first())
                        .map(|link| (link.label.as_str(), link.url.as_str()))
                });
            if let Some((label, url)) = attached_link {
                if !detail_spans.is_empty() {
                    detail_spans.push(Span::raw("  "));
                }
                let link_icon = if theme == Theme::Ascii { "[L]" } else { "🔗" };
                detail_spans.push(Span::styled(
                    format!("{link_icon} {label} › {url}"),
                    if is_selected {
                        theme.time_style()
                    } else {
                        theme_unfocused(theme)
                    },
                ));
            }

            ListItem::new(vec![Line::from(line_spans), Line::from(detail_spans)])
        })
        .collect::<Vec<_>>();

    let title = Line::from(vec![
        Span::styled(" UPCOMING // БЛИЖАЙШИЕ ", theme_selected(theme)),
        Span::styled("[s]", theme.key_badge_style()),
        Span::styled(format!(" SORT: {:?} ", app.state.upcoming.sort), KEY_LABEL),
    ]);

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(title)
                .border_style(theme_focused(theme)),
        ),
        popup,
    );
}
