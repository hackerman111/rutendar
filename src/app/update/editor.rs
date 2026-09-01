use super::app_error;
use crate::{
    app::{
        App, AppResult, Editor, EventForm, EventTarget, FocusedPane, LinkForm, NoteForm, Overlay,
        Popup, ScopeOperation,
    },
    model::{EventId, NewLink, NewNote, parse_date},
};

impl App {
    pub(super) fn create_selected(&mut self) -> AppResult<()> {
        if self.state.popup.is_some() {
            return Ok(());
        }
        let pane = if self.state.overlay.is_some() {
            FocusedPane::Events
        } else {
            self.state.focused_pane
        };
        match pane {
            FocusedPane::Events => {
                self.state.popup = Some(Popup::Editor(Editor::Event {
                    form: EventForm::new(self.state.selected_date),
                    target: EventTarget::New,
                }));
            }
            FocusedPane::Notes => {
                self.state.popup = Some(Popup::Editor(Editor::Note {
                    form: NoteForm {
                        title: String::new(),
                        date: self.state.selected_date.format("%d.%m.%Y").to_string(),
                        body: String::new(),
                        active: 0,
                    },
                    target: None,
                }));
            }
            FocusedPane::Links => {
                let note_id = self
                    .selected_note()
                    .map(|note| note.id)
                    .ok_or_else(|| app_error("create or select a note first"))?;
                self.state.popup = Some(Popup::Editor(Editor::Link {
                    form: LinkForm {
                        label: String::new(),
                        url: "https://".into(),
                        note_id,
                        active: 0,
                    },
                    target: None,
                }));
            }
        }
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn edit_selected(&mut self) -> AppResult<()> {
        if let Some(event) = self.selected_event_occurrence() {
            if event.is_recurring {
                self.state.popup = Some(Popup::Scope(ScopeOperation::Edit(event)));
            } else {
                self.open_event_editor(event.event_id)?;
            }
            self.sync_input_mode();
            return Ok(());
        }
        if self.state.overlay == Some(Overlay::Upcoming) {
            return Err(app_error("no upcoming event selected"));
        }
        let note = self.selected_note_for_action().cloned();
        if self.state.overlay == Some(Overlay::Agenda) {
            let note = note.ok_or_else(|| app_error("no item selected"))?;
            self.state.popup = Some(Popup::Editor(Editor::Note {
                form: NoteForm {
                    title: note.title.unwrap_or_default(),
                    date: note.date.format("%d.%m.%Y").to_string(),
                    body: note.body,
                    active: 0,
                },
                target: Some(note.id),
            }));
            self.sync_input_mode();
            return Ok(());
        }
        match self.state.focused_pane {
            FocusedPane::Notes => {
                let note = note.ok_or_else(|| app_error("no note selected"))?;
                self.state.popup = Some(Popup::Editor(Editor::Note {
                    form: NoteForm {
                        title: note.title.unwrap_or_default(),
                        date: note.date.format("%d.%m.%Y").to_string(),
                        body: note.body,
                        active: 0,
                    },
                    target: Some(note.id),
                }));
            }
            FocusedPane::Links => {
                let link = self
                    .selected_link()
                    .cloned()
                    .ok_or_else(|| app_error("no link selected"))?;
                self.state.popup = Some(Popup::Editor(Editor::Link {
                    form: LinkForm {
                        label: link.label,
                        url: link.url,
                        note_id: link.note_id,
                        active: 0,
                    },
                    target: Some(link.id),
                }));
            }
            FocusedPane::Events => return Err(app_error("no event selected")),
        }
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn open_event_editor(&mut self, event_id: EventId) -> AppResult<()> {
        let event = self
            .database
            .get_event(event_id)?
            .ok_or_else(|| app_error("event no longer exists"))?;
        let tags = self.database.event_tags(event_id)?;
        let recurrence = event
            .recurrence_id
            .map(|id| self.database.get_recurrence(id))
            .transpose()?
            .flatten();
        self.state.popup = Some(Popup::Editor(Editor::Event {
            form: EventForm::from_event(&event, &tags, recurrence.as_ref()),
            target: EventTarget::Event(event.id),
        }));
        Ok(())
    }

    pub(super) fn input_character(&mut self, character: char) -> AppResult<()> {
        match self.state.popup.as_mut() {
            Some(Popup::Editor(Editor::Event { form, .. })) => form.push(character),
            Some(Popup::Editor(Editor::Note { form, .. })) => match form.active {
                0 => form.title.push(character),
                1 => form.date.push(character),
                2 => form.body.push(character),
                _ => {}
            },
            Some(Popup::Editor(Editor::Link { form, .. })) => match form.active {
                0 => form.label.push(character),
                1 => form.url.push(character),
                _ => {}
            },
            Some(Popup::GotoDate(value)) => value.push(character),
            _ if self.state.agenda.searching => {
                self.state.agenda.query.push(character);
                self.refresh_agenda()?;
            }
            _ => {}
        }
        self.refresh_tag_suggestions()?;
        Ok(())
    }

    pub(super) fn backspace(&mut self) -> AppResult<()> {
        match self.state.popup.as_mut() {
            Some(Popup::Editor(Editor::Event { form, .. })) => form.backspace(),
            Some(Popup::Editor(Editor::Note { form, .. })) => match form.active {
                0 => _ = form.title.pop(),
                1 => _ = form.date.pop(),
                2 => _ = form.body.pop(),
                _ => {}
            },
            Some(Popup::Editor(Editor::Link { form, .. })) => match form.active {
                0 => _ = form.label.pop(),
                1 => _ = form.url.pop(),
                _ => {}
            },
            Some(Popup::GotoDate(value)) => _ = value.pop(),
            _ if self.state.agenda.searching => {
                _ = self.state.agenda.query.pop();
                self.refresh_agenda()?;
            }
            _ => {}
        }
        self.refresh_tag_suggestions()?;
        Ok(())
    }

    pub(super) fn move_field(&mut self, delta: i32) {
        let move_active = |active: &mut usize, count: usize| {
            *active = (*active as i32 + delta).rem_euclid(count as i32) as usize;
        };
        match self.state.popup.as_mut() {
            Some(Popup::Editor(Editor::Event { form, .. })) => {
                move_active(&mut form.active, EventForm::FIELD_COUNT)
            }
            Some(Popup::Editor(Editor::Note { form, .. })) => move_active(&mut form.active, 3),
            Some(Popup::Editor(Editor::Link { form, .. })) => move_active(&mut form.active, 2),
            _ => {}
        }
        self.state.tag_suggestions.clear();
    }

    pub(super) fn adjust_field(&mut self, forward: bool) {
        let suggestion = forward
            .then(|| {
                self.state
                    .tag_suggestions
                    .first()
                    .map(|tag| tag.name.clone())
            })
            .flatten();
        if let Some(Popup::Editor(Editor::Event { form, .. })) = self.state.popup.as_mut() {
            if form.active == 5
                && let Some(suggestion) = suggestion
            {
                let start = form
                    .tags
                    .char_indices()
                    .rev()
                    .find(|(_, character)| character.is_whitespace())
                    .map_or(0, |(index, character)| index + character.len_utf8());
                form.tags.truncate(start);
                form.tags.push('#');
                form.tags.push_str(&suggestion);
                form.tags.push(' ');
                self.state.tag_suggestions.clear();
            } else {
                form.adjust(forward);
            }
        }
    }

    pub(super) fn refresh_tag_suggestions(&mut self) -> AppResult<()> {
        let prefix = match self.state.popup.as_ref() {
            Some(Popup::Editor(Editor::Event { form, .. })) if form.active == 5 => form
                .tags
                .split_whitespace()
                .next_back()
                .unwrap_or("")
                .trim_start_matches('#')
                .to_owned(),
            _ => String::new(),
        };
        self.state.tag_suggestions = if prefix.is_empty() {
            Vec::new()
        } else {
            self.database.search_tags(&prefix, 5)?
        };
        Ok(())
    }

    pub(super) fn submit(&mut self) -> AppResult<()> {
        match self.state.popup.clone() {
            Some(Popup::Editor(editor)) => {
                self.save_editor(editor)?;
                self.state.popup = None;
                self.state.tag_suggestions.clear();
                self.state.status_message = Some("Сохранено".into());
                self.refresh_after_change()?;
            }
            Some(Popup::GotoDate(value)) => {
                self.state.selected_date = parse_date(&value)?;
                self.state.popup = None;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
            }
            Some(Popup::Help) => self.state.popup = None,
            _ if self.state.agenda.searching => {
                self.state.agenda.searching = false;
                self.refresh_agenda()?;
            }
            _ => {}
        }
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn save_editor(&mut self, editor: Editor) -> AppResult<()> {
        match editor {
            Editor::Event { form, target } => {
                let (event, recurrence, tags) = form.values()?;
                self.state.selected_date = event.start_date;
                match target {
                    EventTarget::New => {
                        self.database
                            .create_event(&event, recurrence.as_ref(), &tags)?;
                    }
                    EventTarget::Event(id) => {
                        self.database
                            .update_event(id, &event, recurrence.as_ref(), &tags)?;
                    }
                    EventTarget::Occurrence {
                        recurrence_id,
                        original_date,
                    } => {
                        self.database.modify_occurrence(
                            recurrence_id,
                            original_date,
                            &event,
                            &tags,
                        )?;
                    }
                }
            }
            Editor::Note { form, target } => {
                let note = NewNote {
                    date: parse_date(&form.date)?,
                    title: (!form.title.trim().is_empty()).then(|| form.title.trim().into()),
                    body: form.body,
                };
                self.state.selected_date = note.date;
                if let Some(id) = target {
                    self.database.update_note(id, &note)?;
                } else {
                    self.database.create_note(&note)?;
                }
            }
            Editor::Link { form, target } => {
                let link = NewLink {
                    note_id: form.note_id,
                    label: form.label,
                    url: form.url,
                };
                if let Some(id) = target {
                    self.database.update_link(id, &link)?;
                } else {
                    self.database.create_link(&link)?;
                }
            }
        }
        Ok(())
    }
}
