use super::*;

pub(super) fn render_agenda(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 92, 86);
    frame.render_widget(Clear, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(3),
        ])
        .split(popup);
    frame.render_widget(
        Paragraph::new(format!("/ {}", app.state.agenda.query))
            .style(if app.state.agenda.searching {
                SELECTED
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL).title(" AGENDA ")),
        rows[0],
    );
    let filters = &app.state.agenda.filters;
    let tag_capacity = (popup.width / 12).max(1) as usize;
    let tag_start = app
        .state
        .agenda
        .tag_cursor
        .saturating_sub(tag_capacity.saturating_sub(1));
    let tag_filters = app
        .state
        .agenda
        .available_tags
        .iter()
        .enumerate()
        .skip(tag_start)
        .take(tag_capacity)
        .map(|(index, tag)| {
            let selected = filters.tags.contains(&tag.normalized_name);
            let label = format!("{}#{}", if selected { "✓" } else { "" }, tag.name);
            if index == app.state.agenda.tag_cursor {
                format!("[{label}]")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!(
                "f date:{:?}  r type:{:?}  i importance:{:?}  s sort:{:?}  A tags:{:?}",
                filters.date,
                filters.item_type,
                filters.importance,
                filters.sort,
                filters.tag_matching
            )),
            Line::from(format!("[/] tag, Space toggle: {tag_filters}")),
        ]),
        rows[1],
    );
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
            let (time, importance, kind, title, tags) = match item {
                SearchResult::Event(event) => (
                    event
                        .start_time
                        .map(|time| time.format("%H:%M").to_string())
                        .unwrap_or_else(|| "весь день".into()),
                    app.config.importance_symbol(event.importance).to_owned(),
                    if event.is_recurring { "↻" } else { "event" }.to_owned(),
                    event.title.clone(),
                    event
                        .tags
                        .iter()
                        .map(|tag| tag.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                SearchResult::Note(note) => (
                    String::new(),
                    String::new(),
                    "note".to_owned(),
                    note.title.clone().unwrap_or_else(|| "Без названия".into()),
                    String::new(),
                ),
            };
            Row::new([
                Cell::from(item.date().format("%d.%m.%Y").to_string()),
                Cell::from(time),
                Cell::from(importance),
                Cell::from(kind),
                Cell::from(title),
                Cell::from(tags),
            ])
            .style(if index == app.state.agenda.selected {
                SELECTED
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(11),
                Constraint::Length(9),
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Percentage(38),
                Constraint::Percentage(28),
            ],
        )
        .header(
            Row::new(["DATE", "TIME", "PRI", "TYPE", "EVENT / NOTE", "TAGS"])
                .style(Style::new().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL)),
        rows[2],
    );
}

pub(super) fn render_upcoming(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 72, 80);
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
            let line = format!(
                "{}  {}",
                relative_date(app.state.today, event.date),
                event_line(app, event)
            );
            let mut details = tags_line(event);
            if let Some(link) = app
                .state
                .upcoming
                .links_by_date
                .get(&event.date)
                .and_then(|links| links.first())
            {
                details.push_str(&format!("  🔗 {}", link.label));
            }
            ListItem::new(vec![Line::from(line), Line::from(details)]).style(
                if index == app.state.upcoming.selected {
                    SELECTED
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            " БЛИЖАЙШИЕ · s sort:{:?} ",
            app.state.upcoming.sort
        ))),
        popup,
    );
}

pub(super) fn render_popup(frame: &mut Frame, area: Rect, app: &App, popup: &Popup) {
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

fn render_editor(frame: &mut Frame, area: Rect, app: &App, editor: &Editor) {
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

fn render_help(frame: &mut Frame, area: Rect) {
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
