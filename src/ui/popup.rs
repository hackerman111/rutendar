use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use super::widgets::{FOCUSED, KEY_BADGE, KEY_LABEL, centered, centered_fixed};
use crate::app::{App, Editor, Popup};

pub fn render_popup(frame: &mut Frame, area: Rect, app: &App, popup: &Popup) {
    match popup {
        Popup::Editor(editor) => render_editor(frame, area, app, editor),
        Popup::Confirm { message, .. } => {
            let popup = centered_fixed(area, 56, 7);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {message}"),
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[ Enter / y ]", KEY_BADGE),
                    Span::styled(" ДА    ", Style::new().fg(Color::White)),
                    Span::styled(
                        "[ Esc / n ]",
                        Style::new()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" НЕТ", Style::new().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Center).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            " ▌! ПОДТВЕРЖДЕНИЕ ДЕЙСТВИЯ▐ ",
                            Style::new()
                                .fg(Color::LightRed)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .border_style(Style::new().fg(Color::LightRed)),
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
                    Span::styled("[ o ]", KEY_BADGE),
                    Span::styled(
                        " ТОЛЬКО ЭТО СОБЫТИЕ (occurrence)",
                        Style::new().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("[ s ]", KEY_BADGE),
                    Span::styled(" ВСЯ СЕРИЯ (entire series)", Style::new().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "[ Esc ]",
                        Style::new()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ОТМЕНА", Style::new().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Left).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            " ▌ОБЛАСТЬ ИЗМЕНЕНИЯ СЕРИИ▐ ",
                            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ))
                        .border_style(FOCUSED),
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
                    Span::styled("  › [ ", Style::new().fg(Color::Cyan)),
                    Span::styled(
                        if value.is_empty() {
                            "DD.MM.YYYY"
                        } else {
                            value.as_str()
                        },
                        if value.is_empty() {
                            Style::new().fg(Color::DarkGray)
                        } else {
                            Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
                        },
                    ),
                    Span::styled(" █ ]", Style::new().fg(Color::Cyan)),
                ]),
            ];
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            " ▌ПЕРЕХОД К ДАТЕ▐ ",
                            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ))
                        .border_style(FOCUSED),
                ),
                popup,
            );
        }
        Popup::Help => render_help(frame, area),
    }
}

pub fn render_editor(frame: &mut Frame, area: Rect, app: &App, editor: &Editor) {
    let popup = centered(area, 76, 90);
    frame.render_widget(Clear, popup);
    let event_fields;
    let note_fields;
    let link_fields;
    let (title, fields, active): (&str, &[(&'static str, &str)], usize) = match editor {
        Editor::Event { form, .. } => {
            event_fields = form.fields();
            (" ▌РЕДАКТОР: СОБЫТИЕ▐ ", &event_fields, form.active)
        }
        Editor::Note { form, .. } => {
            note_fields = [
                ("TITLE", form.title.as_str()),
                ("DATE", form.date.as_str()),
                ("BODY", form.body.as_str()),
            ];
            (" ▌РЕДАКТОР: ЗАМЕТКА▐ ", &note_fields, form.active)
        }
        Editor::Link { form, .. } => {
            link_fields = [("LABEL", form.label.as_str()), ("URL", form.url.as_str())];
            (" ▌РЕДАКТОР: ССЫЛКА▐ ", &link_fields, form.active)
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
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::DarkGray)
            };

            let formatted_label = format!("{label:<12}");

            let mut value_spans = Vec::new();
            if is_active {
                value_spans.push(Span::styled(
                    "› ",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ));
                if label == "IMPORTANCE" || label == "REPEAT" {
                    value_spans.push(Span::styled(
                        format!("◄  {value}  ►"),
                        Style::new()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                    value_spans.push(Span::styled(
                        "  (←/→ переключить)",
                        Style::new().fg(Color::DarkGray),
                    ));
                } else {
                    value_spans.push(Span::styled(
                        value,
                        Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                    value_spans.push(Span::styled("█", Style::new().fg(Color::Cyan)));
                }
            } else {
                value_spans.push(Span::raw("  "));
                if value.is_empty() {
                    value_spans.push(Span::styled("(пусто)", Style::new().fg(Color::DarkGray)));
                } else {
                    value_spans.push(Span::styled(value, Style::new().fg(Color::White)));
                }
            }

            ListItem::new(vec![
                Line::from(vec![Span::styled(
                    format!("[ {formatted_label}]"),
                    label_style,
                )]),
                Line::from(value_spans),
            ])
        })
        .collect::<Vec<_>>();

    let mut footer_spans = vec![
        Span::styled("[Ctrl+S]", KEY_BADGE),
        Span::styled(" СОХРАНИТЬ  ", KEY_LABEL),
        Span::styled("[Tab/Enter]", KEY_BADGE),
        Span::styled(" СЛЕДУЮЩЕЕ ПОЛЕ  ", KEY_LABEL),
        Span::styled(
            "[Esc]",
            Style::new()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ОТМЕНА", KEY_LABEL),
    ];

    if !app.state.tag_suggestions.is_empty() {
        footer_spans.push(Span::styled(
            " │ [AUTO]: ",
            Style::new().fg(Color::DarkGray),
        ));
        for tag in &app.state.tag_suggestions {
            footer_spans.push(Span::styled(
                format!(" [#{}] ", tag.name),
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::raw(" "));
        }
    }

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    title,
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
                .title_bottom(Line::from(footer_spans))
                .border_style(FOCUSED),
        ),
        popup,
    );
}

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_fixed(area, 68, 25);
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
        ("e", "изменить выбранный элемент"),
        ("d  /  x", "удалить выбранный элемент"),
        ("p", "изменить важность (None / Low / Normal / High)"),
        ("o  /  y", "открыть ссылку в браузере / скопировать URL"),
        ("Enter  /  Esc", "открыть / закрыть или назад"),
        ("── ПАНЕЛИ И ПОИСК ───────────────────────", ""),
        ("/", "открыть Agenda / поиск по событиям"),
        ("t", "открыть Ближайшие (Upcoming)"),
        (
            "f / r / i / s / A",
            "фильтры: дата, тип, важность, сортировка, теги",
        ),
        (
            "[  /  ]  /  Space",
            "выбрать тег / переключить тег в фильтре",
        ),
        ("X", "удалить выбранный тег из базы"),
        ("── ОБЩИЕ ────────────────────────────────", ""),
        ("?  /  q", "закрыть справку / выйти из приложения"),
        ("Ctrl+S (в редакторе)", "сохранить изменения"),
    ];

    let lines = rows
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                Line::from(Span::styled(*key, Style::new().fg(Color::DarkGray)))
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("  {:<24}", key),
                        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(*desc, Style::new().fg(Color::White)),
                ])
            }
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " ▌СПРАВКА ПО КЛАВИШАМ // QUICK REFERENCE▐ ",
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
                .border_style(FOCUSED),
        ),
        popup,
    );
}
