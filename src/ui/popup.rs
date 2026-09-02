use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use super::link_bank::render_link_bank;
use super::widgets::{
    KEY_LABEL, centered, centered_fixed, theme_border_type, theme_focused, theme_selected,
    theme_unfocused,
};

use crate::{
    app::{App, Editor, Popup},
    ui::Theme,
};

pub fn render_popup(frame: &mut Frame, area: Rect, app: &App, popup: &Popup) {
    let theme = app.config.ui.theme;
    match popup {
        Popup::Editor(editor) => render_editor(frame, area, app, editor),
        Popup::SaveConfirm { message, .. } | Popup::Confirm { message, .. } => {
            let popup = centered_fixed(area, 56, 7);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {message}"),
                    theme.title_style(true, false),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[ Enter / y ]", theme.key_badge_style()),
                    Span::styled(" ДА    ", theme.title_style(false, false)),
                    Span::styled("[ Esc / n ]", theme_unfocused(theme)),
                    Span::styled(" НЕТ", theme_unfocused(theme)),
                ]),
            ];
            let border_color = if theme == Theme::Ascii {
                Color::Reset
            } else {
                Color::LightRed
            };
            let title_style = if theme == Theme::Ascii {
                Style::new().add_modifier(Modifier::BOLD)
            } else {
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::LightRed)
                    .add_modifier(Modifier::BOLD)
            };

            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Center).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme_border_type(theme))
                        .title(Span::styled(" ! ПОДТВЕРЖДЕНИЕ ДЕЙСТВИЯ ", title_style))
                        .border_style(Style::new().fg(border_color)),
                ),
                popup,
            );
        }
        Popup::Scope(_) => {
            let popup = centered_fixed(area, 60, 7);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("[ o ]", theme.key_badge_style()),
                    Span::styled(
                        " ТОЛЬКО ЭТО СОБЫТИЕ (occurrence)",
                        theme.title_style(false, false),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("[ s ]", theme.key_badge_style()),
                    Span::styled(
                        " ВСЯ СЕРИЯ (entire series)",
                        theme.title_style(false, false),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("[ Esc ]", theme_unfocused(theme)),
                    Span::styled(" ОТМЕНА", theme_unfocused(theme)),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Left).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme_border_type(theme))
                        .title(Span::styled(
                            " ОБЛАСТЬ ИЗМЕНЕНИЯ СЕРИИ ",
                            theme_selected(theme),
                        ))
                        .border_style(theme_focused(theme)),
                ),
                popup,
            );
        }
        Popup::GotoDate(value) => {
            let popup = centered_fixed(area, 48, 5);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  › [ ", theme.time_style()),
                    Span::styled(
                        if value.is_empty() {
                            "DD.MM.YYYY"
                        } else {
                            value.as_str()
                        },
                        if value.is_empty() {
                            theme_unfocused(theme)
                        } else {
                            theme.title_style(true, false)
                        },
                    ),
                    Span::styled(" █ ]", theme.time_style()),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme_border_type(theme))
                        .title(Span::styled(" ПЕРЕХОД К ДАТЕ ", theme_selected(theme)))
                        .border_style(theme_focused(theme)),
                ),
                popup,
            );
        }
        Popup::LinkBank => render_link_bank(frame, area, app),
        Popup::Help => render_help(frame, area, app),
        Popup::MonthDayPreview { date, selected } => {
            super::month::render_month_day_preview(frame, area, app, *date, *selected);
        }
        Popup::CreateTask(title) => {
            let popup = centered_fixed(area, 54, 5);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  › [ ", theme.time_style()),
                    Span::styled(
                        if title.is_empty() {
                            "Введите название задания..."
                        } else {
                            title.as_str()
                        },
                        if title.is_empty() {
                            theme_unfocused(theme)
                        } else {
                            theme.title_style(true, false)
                        },
                    ),
                    Span::styled(" █ ]", theme.time_style()),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme_border_type(theme))
                        .title(Span::styled(
                            " НОВОЕ ЗАДАНИЕ (To-Do) ",
                            theme_selected(theme),
                        ))
                        .border_style(theme_focused(theme)),
                ),
                popup,
            );
        }
    }
}

pub fn render_editor(frame: &mut Frame, area: Rect, app: &App, editor: &Editor) {
    let theme = app.config.ui.theme;
    let popup = centered(area, 92, 90);
    frame.render_widget(Clear, popup);
    let event_fields;
    let note_fields;
    let link_fields;
    let favorite_link_fields;
    let (title, fields, active): (&str, &[(&'static str, &str)], usize) = match editor {
        Editor::Event { form, .. } => {
            event_fields = form.fields();
            (" РЕДАКТОР: СОБЫТИЕ ", &event_fields, form.active)
        }
        Editor::Note { form, .. } => {
            note_fields = [
                ("TITLE", form.title.as_str()),
                ("DATE", form.date.as_str()),
                ("BODY", form.body.as_str()),
            ];
            (" РЕДАКТОР: ЗАМЕТКА ", &note_fields, form.active)
        }
        Editor::Link { form, .. } => {
            link_fields = [("LABEL", form.label.as_str()), ("URL", form.url.as_str())];
            (" РЕДАКТОР: ССЫЛКА ", &link_fields, form.active)
        }
        Editor::FavoriteLink { form, .. } => {
            favorite_link_fields = form.fields();
            (
                " РЕДАКТОР: ИЗБРАННАЯ ССЫЛКА ",
                &favorite_link_fields,
                form.active,
            )
        }
    };
    let capacity = (popup.height.saturating_sub(2) / 2).max(1) as usize;
    let start = active.saturating_sub(capacity.saturating_sub(1));
    let items = fields
        .iter()
        .copied()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, (label, value))| {
            let is_active = index == active;
            let label_style = if is_active {
                theme_selected(theme)
            } else {
                theme_unfocused(theme)
            };

            let formatted_label = if is_active {
                format!(" {label:<12} ")
            } else {
                format!("[ {label:<12}]")
            };

            let mut value_spans = Vec::new();
            if is_active {
                let cursor = match theme {
                    Theme::Default => "▸ ",
                    Theme::Ascii => "> ",
                };

                value_spans.push(Span::styled(cursor, theme.key_badge_style()));
                if label == "IMPORTANCE" || label == "REPEAT" {
                    value_spans.push(Span::styled(
                        format!("◄  {value}  ►"),
                        theme_selected(theme),
                    ));
                    value_spans.push(Span::styled(
                        "  (Tab/←/→ переключить)",
                        theme_unfocused(theme),
                    ));
                } else if label == "LINKS" {
                    value_spans.push(Span::styled(
                        if value.is_empty() {
                            "(не выбраны)"
                        } else {
                            value
                        },
                        theme.title_style(true, false),
                    ));
                    value_spans.push(Span::styled(
                        "  (Enter/Ctrl+L — банк)",
                        theme_unfocused(theme),
                    ));
                } else {
                    value_spans.push(Span::styled(value, theme.title_style(true, false)));
                    value_spans.push(Span::styled("█", theme.time_style()));
                }
            } else {
                value_spans.push(Span::raw("  "));
                if value.is_empty() {
                    value_spans.push(Span::styled("(пусто)", theme_unfocused(theme)));
                } else {
                    value_spans.push(Span::styled(value, theme.title_style(false, false)));
                }
            }

            ListItem::new(vec![
                Line::from(vec![Span::styled(formatted_label, label_style)]),
                Line::from(value_spans),
            ])
        })
        .collect::<Vec<_>>();

    let mut footer_spans = vec![
        Span::styled("[Ctrl+S]", theme.key_badge_style()),
        Span::styled(" SAVE  ", KEY_LABEL),
        Span::styled("[Enter]", theme.key_badge_style()),
        Span::styled(" NEXT/DONE  ", KEY_LABEL),
        Span::styled("[Tab]", theme.key_badge_style()),
        Span::styled(" CHOICE  ", KEY_LABEL),
        Span::styled("[C-L]", theme.key_badge_style()),
        Span::styled(" LINKS  ", KEY_LABEL),
        Span::styled("[Esc]", theme_unfocused(theme)),
        Span::styled(" BACK", KEY_LABEL),
    ];

    if !app.state.tag_suggestions.is_empty() {
        footer_spans.push(Span::styled(" │ [AUTO]: ", theme_unfocused(theme)));
        for tag in &app.state.tag_suggestions {
            footer_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                theme_selected(theme),
            ));
            footer_spans.push(Span::raw(" "));
        }
    } else if !app.state.path_suggestions.is_empty() {
        footer_spans.push(Span::styled(" │ [AUTO]: ", theme_unfocused(theme)));
        for path in &app.state.path_suggestions {
            footer_spans.push(Span::styled(format!(" {path} "), theme_selected(theme)));
            footer_spans.push(Span::raw(" "));
        }
    }

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(title, theme_selected(theme)))
                .title_bottom(Line::from(footer_spans))
                .border_style(theme_focused(theme)),
        ),
        popup,
    );
}

pub fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
    let popup = centered_fixed(area, 72, 30);
    frame.render_widget(Clear, popup);

    let rows = [
        ("── НАВИГАЦИЯ (VIM MOTIONS) ─────────────", ""),
        (
            "h / j / k / l ,  стрелки",
            "навигация (в дне: h=события, l=заметки)",
        ),
        ("n  /  N", "следующий / предыдущий день (+1 / -1)"),
        ("gg  /  G", "в начало / в конец списка"),
        ("Ctrl+d  /  Ctrl+u", "страница вниз / вверх"),
        ("Tab  /  Shift+Tab", "панели (в дне) / режимы календаря"),
        ("g t  /  g d", "перейти к Сегодня / к конкретной дате"),
        ("w  /  D  /  m  /  Y", "режимы: Week / Day / Month / Year"),
        ("── ДЕЙСТВИЯ ─────────────────────────────", ""),
        ("a", "создать событие / заметку"),
        ("T", "создать задание (To-Do) на этот день"),
        ("Space", "отметить задание ([ ] ↔ [x])"),
        ("e  /  r", "изменить выбранный элемент"),
        ("d  /  x", "удалить выбранный элемент"),
        ("p", "изменить важность (None / Low / Normal / High)"),
        ("o  /  y", "превью дня (в месяце) / открыть ссылку"),
        ("c", "открыть shell в директории выбранного события"),
        ("Enter  /  Esc", "открыть / закрыть или назад"),
        ("── ПАНЕЛИ И ПОИСК ───────────────────────", ""),
        ("/", "открыть Agenda / поиск по событиям"),
        ("t", "открыть Ближайшие (Upcoming)"),
        (
            "f / R / i / s / A",
            "фильтры: дата, тип, важность, сортировка, теги",
        ),
        (
            "[  /  ]  /  Space",
            "выбрать тег / переключить тег в фильтре",
        ),
        ("X", "удалить выбранный тег из базы"),
        ("── ОБЩИЕ ────────────────────────────────", ""),
        ("F5  /  M", "переключить тему оформления (Default / ASCII)"),
        ("?  /  q", "закрыть справку / выйти из приложения"),
        ("Ctrl+S (в редакторе)", "сохранить изменения"),
        ("Ctrl+L (в событии)", "открыть банк избранных ссылок"),
    ];

    let lines = rows
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                Line::from(Span::styled(*key, theme_unfocused(theme)))
            } else {
                Line::from(vec![
                    Span::styled(format!("  {:<24}", key), theme.key_badge_style()),
                    Span::styled(*desc, theme.title_style(false, false)),
                ])
            }
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme_border_type(theme))
                .title(Span::styled(
                    " СПРАВКА ПО КЛАВИШАМ // QUICK REFERENCE ",
                    theme_selected(theme),
                ))
                .border_style(theme_focused(theme)),
        ),
        popup,
    );
}
