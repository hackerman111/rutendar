use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use super::widgets::{SELECTED, centered, centered_fixed};
use crate::app::{App, Editor, Popup};

pub fn render_popup(frame: &mut Frame, area: Rect, app: &App, popup: &Popup) {
    match popup {
        Popup::Editor(editor) => render_editor(frame, area, app, editor),
        Popup::Confirm { message, .. } => {
            let popup = centered_fixed(area, 52, 5);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(format!("{message}\n\nEnter/y — да, Esc/n — нет"))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" ПОДТВЕРЖДЕНИЕ "),
                    ),
                popup,
            );
        }
        Popup::Scope(_) => {
            let popup = centered_fixed(area, 58, 5);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new("o — только это occurrence\ns — вся серия\nEsc — отмена")
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" ОБЛАСТЬ ИЗМЕНЕНИЯ "),
                    ),
                popup,
            );
        }
        Popup::GotoDate(value) => {
            let popup = centered_fixed(area, 42, 3);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(value.as_str()).style(SELECTED).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" ДАТА DD.MM.YYYY "),
                ),
                popup,
            );
        }
        Popup::Help => render_help(frame, area),
    }
}

pub fn render_editor(frame: &mut Frame, area: Rect, app: &App, editor: &Editor) {
    let popup = centered(area, 72, 88);
    frame.render_widget(Clear, popup);
    let (title, fields, active) = match editor {
        Editor::Event { form, .. } => (" СОБЫТИЕ ", form.fields().to_vec(), form.active),
        Editor::Note { form, .. } => (
            " ЗАМЕТКА ",
            vec![
                ("TITLE", form.title.clone()),
                ("DATE", form.date.clone()),
                ("BODY", form.body.clone()),
            ],
            form.active,
        ),
        Editor::Link { form, .. } => (
            " ССЫЛКА ",
            vec![("LABEL", form.label.clone()), ("URL", form.url.clone())],
            form.active,
        ),
    };
    let capacity = (popup.height.saturating_sub(2) / 2).max(1) as usize;
    let start = active.saturating_sub(capacity.saturating_sub(1));
    let items = fields
        .into_iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, (label, value))| {
            ListItem::new(vec![
                Line::from(Span::styled(label, Style::new().fg(Color::Cyan))),
                Line::from(value),
            ])
            .style(if index == active {
                SELECTED
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    let footer = if app.state.tag_suggestions.is_empty() {
        " Ctrl-S сохранить · Enter/Tab следующее поле · Esc отмена ".into()
    } else {
        format!(
            " → autocomplete: {} ",
            app.state
                .tag_suggestions
                .iter()
                .map(|tag| format!("#{}", tag.name))
                .collect::<Vec<_>>()
                .join("  ")
        )
    };
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(footer),
        ),
        popup,
    );
}

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_fixed(area, 64, 24);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(
            "h/j/k/l, стрелки  навигация\n\
             Enter              открыть\n\
             Esc                назад / закрыть\n\
             n / e / d          создать / изменить / удалить\n\
             Tab                Events / Notes / Links\n\
             a / t              Agenda / Upcoming\n\
             p                  importance\n\
             o / y              открыть / копировать URL\n\
             w / D / m / Y      Week / Day / Month / Year\n\
             g t / g d          сегодня / перейти к дате\n\
             /                  поиск в Agenda\n\
             f/r/i/s/A          фильтры Agenda\n\
             [ / ] / Space      выбрать / включить тег\n\
             ?                  закрыть help\n\
             q                  выход\n\n\
             В редакторе: Ctrl-S сохранить, ←/→ изменить выбор",
        )
        .block(Block::default().borders(Borders::ALL).title(" HELP ")),
        popup,
    );
}
