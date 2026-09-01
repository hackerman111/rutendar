use chrono::Duration;

use super::*;
use crate::{
    calendar::move_month,
    external,
    model::{EventId, Importance, NewEvent, NewLink, NewNote, parse_date},
    search::{DateFilter, ItemType, SearchResult, SortBy, TagMatching},
};

impl App {
    pub fn update(&mut self, action: Action) -> bool {
        if matches!(action, Action::Quit) {
            return true;
        }
        if let Err(error) = self.apply(action) {
            self.set_error(error);
        }
        false
    }

    fn apply(&mut self, action: Action) -> AppResult<()> {
        match action {
            Action::Quit | Action::Noop => {}
            Action::MoveLeft => self.move_horizontal(-1)?,
            Action::MoveRight => self.move_horizontal(1)?,
            Action::MoveUp => self.move_vertical(-1)?,
            Action::MoveDown => self.move_vertical(1)?,
            Action::Open => self.open_selected()?,
            Action::Back => self.back()?,
            Action::Create => self.create_selected()?,
            Action::Edit => self.edit_selected()?,
            Action::Delete => self.delete_selected()?,
            Action::ChangeImportance => self.change_importance()?,
            Action::OpenAgenda => self.toggle_overlay(Overlay::Agenda)?,
            Action::OpenUpcoming => self.toggle_overlay(Overlay::Upcoming)?,
            Action::SwitchView(view) => {
                self.state.active_view = view;
                if view != View::Day {
                    self.state.focused_pane = FocusedPane::Events;
                }
                self.state.overlay = None;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
            }
            Action::GoToToday => {
                self.state.selected_date = self.state.today;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
            }
            Action::StartGotoDate => {
                self.state.popup = Some(Popup::GotoDate(
                    self.state.selected_date.format("%d.%m.%Y").to_string(),
                ));
                self.state.input_mode = InputMode::GotoDate;
            }
            Action::Help => {
                self.state.popup = if matches!(self.state.popup, Some(Popup::Help)) {
                    None
                } else {
                    Some(Popup::Help)
                };
                self.sync_input_mode();
            }
            Action::OpenLink => {
                let url = self
                    .selected_link()
                    .map(|link| link.url.clone())
                    .ok_or_else(|| app_error("no link selected"))?;
                external::open_url(&url)?;
                self.state.status_message = Some("Ссылка открыта".into());
            }
            Action::CopyLink => {
                let url = self
                    .selected_link()
                    .map(|link| link.url.clone())
                    .ok_or_else(|| app_error("no link selected"))?;
                external::copy_url(&url)?;
                self.state.status_message = Some("URL скопирован".into());
            }
            Action::ToggleFocus => self.toggle_focus(),
            Action::StartSearch => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.searching = true;
                    self.state.input_mode = InputMode::Search;
                }
            }
            Action::Input(character) => self.input_character(character)?,
            Action::Backspace => self.backspace()?,
            Action::NextField => self.move_field(1),
            Action::PreviousField => self.move_field(-1),
            Action::AdjustLeft => self.adjust_field(false),
            Action::AdjustRight => self.adjust_field(true),
            Action::Submit => self.submit()?,
            Action::Confirm(confirmed) => self.confirm_delete(confirmed)?,
            Action::ChooseOccurrence => self.choose_scope(false)?,
            Action::ChooseSeries => self.choose_scope(true)?,
            Action::CycleDateFilter => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.date = match self.state.agenda.filters.date {
                        DateFilter::All => DateFilter::Today,
                        DateFilter::Today => DateFilter::ThisWeek,
                        DateFilter::ThisWeek => DateFilter::ThisMonth,
                        DateFilter::ThisMonth => DateFilter::Upcoming,
                        DateFilter::Upcoming => DateFilter::All,
                    };
                    self.refresh_agenda()?;
                }
            }
            Action::CycleItemType => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.item_type = match self.state.agenda.filters.item_type
                    {
                        ItemType::All => ItemType::Events,
                        ItemType::Events => ItemType::Notes,
                        ItemType::Notes => ItemType::Recurring,
                        ItemType::Recurring => ItemType::All,
                    };
                    self.refresh_agenda()?;
                }
            }
            Action::CycleImportanceFilter => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.importance =
                        match self.state.agenda.filters.importance {
                            None => Some(Importance::High),
                            Some(Importance::High) => Some(Importance::Normal),
                            Some(Importance::Normal) => Some(Importance::Low),
                            Some(Importance::Low) => Some(Importance::None),
                            Some(Importance::None) => None,
                        };
                    self.refresh_agenda()?;
                }
            }
            Action::CycleSort => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.sort = match self.state.agenda.filters.sort {
                        SortBy::Date => SortBy::Importance,
                        SortBy::Importance => SortBy::Title,
                        SortBy::Title => SortBy::Date,
                    };
                    self.refresh_agenda()?;
                } else if self.state.overlay == Some(Overlay::Upcoming) {
                    self.state.upcoming.sort = match self.state.upcoming.sort {
                        UpcomingSort::Time => UpcomingSort::Importance,
                        UpcomingSort::Importance => UpcomingSort::Time,
                    };
                    self.refresh_upcoming()?;
                }
            }
            Action::ToggleTagMatching => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.tag_matching =
                        match self.state.agenda.filters.tag_matching {
                            TagMatching::All => TagMatching::Any,
                            TagMatching::Any => TagMatching::All,
                        };
                    self.refresh_agenda()?;
                }
            }
            Action::PreviousTagFilter => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.tag_cursor = move_index(
                        self.state.agenda.tag_cursor,
                        self.state.agenda.available_tags.len(),
                        -1,
                    );
                }
            }
            Action::NextTagFilter => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.tag_cursor = move_index(
                        self.state.agenda.tag_cursor,
                        self.state.agenda.available_tags.len(),
                        1,
                    );
                }
            }
            Action::ToggleTagFilter => {
                if self.state.overlay == Some(Overlay::Agenda)
                    && let Some(tag) = self
                        .state
                        .agenda
                        .available_tags
                        .get(self.state.agenda.tag_cursor)
                {
                    if let Some(index) = self
                        .state
                        .agenda
                        .filters
                        .tags
                        .iter()
                        .position(|selected| selected == &tag.normalized_name)
                    {
                        self.state.agenda.filters.tags.remove(index);
                    } else {
                        self.state
                            .agenda
                            .filters
                            .tags
                            .push(tag.normalized_name.clone());
                    }
                    self.refresh_agenda()?;
                }
            }
        }
        Ok(())
    }

    fn move_horizontal(&mut self, delta: i32) -> AppResult<()> {
        if self.state.popup.is_some() || self.state.overlay.is_some() {
            return Ok(());
        }
        self.state.selected_date = if self.state.active_view == View::Year {
            move_month(self.state.selected_date, delta)
        } else {
            self.state.selected_date + Duration::days(i64::from(delta))
        };
        self.state.selected_event = 0;
        self.state.selected_note = 0;
        self.state.selected_link = 0;
        self.refresh_calendar()
    }

    fn move_vertical(&mut self, delta: i32) -> AppResult<()> {
        if self.state.popup.is_some() {
            return Ok(());
        }
        match self.state.overlay {
            Some(Overlay::Agenda) => {
                self.state.agenda.selected = move_index(
                    self.state.agenda.selected,
                    self.state.agenda.items.len(),
                    delta,
                );
            }
            Some(Overlay::Upcoming) => {
                self.state.upcoming.selected = move_index(
                    self.state.upcoming.selected,
                    self.state.upcoming.items.len(),
                    delta,
                );
                self.state.selected_link = 0;
            }
            None => match self.state.active_view {
                View::Month => {
                    self.state.selected_date += Duration::days(i64::from(delta) * 7);
                    self.refresh_calendar()?;
                }
                View::Year => {
                    self.state.selected_date = move_month(self.state.selected_date, delta * 3);
                    self.refresh_calendar()?;
                }
                View::Week | View::Day => match self.state.focused_pane {
                    FocusedPane::Events => {
                        self.state.selected_event = move_index(
                            self.state.selected_event,
                            self.events_on_selected_date().count(),
                            delta,
                        );
                    }
                    FocusedPane::Notes => {
                        self.state.selected_note = move_index(
                            self.state.selected_note,
                            self.notes_on_selected_date().count(),
                            delta,
                        );
                        self.state.selected_link = 0;
                    }
                    FocusedPane::Links => {
                        self.state.selected_link = move_index(
                            self.state.selected_link,
                            self.selected_note().map_or(0, |note| note.links.len()),
                            delta,
                        );
                    }
                },
            },
        }
        Ok(())
    }

    fn open_selected(&mut self) -> AppResult<()> {
        if self.state.popup.is_some() {
            return self.submit();
        }
        match self.state.overlay {
            Some(Overlay::Upcoming) => {
                if let Some(event) = self
                    .state
                    .upcoming
                    .items
                    .get(self.state.upcoming.selected)
                    .cloned()
                {
                    self.state.selected_date = event.date;
                    self.state.active_view = View::Day;
                    self.state.focused_pane = FocusedPane::Events;
                    self.state.overlay = None;
                    self.state.loaded_range = None;
                    self.refresh_calendar()?;
                    let selected = self
                        .events_on_selected_date()
                        .position(|candidate| {
                            candidate.event_id == event.event_id
                                && candidate.original_date == event.original_date
                        })
                        .unwrap_or(0);
                    self.state.selected_event = selected;
                }
            }
            Some(Overlay::Agenda) => {
                if let Some(item) = self
                    .state
                    .agenda
                    .items
                    .get(self.state.agenda.selected)
                    .cloned()
                {
                    self.state.selected_date = item.date();
                    self.state.focused_pane = match &item {
                        SearchResult::Event(_) => FocusedPane::Events,
                        SearchResult::Note(_) => FocusedPane::Notes,
                    };
                    self.state.active_view = View::Day;
                    self.state.overlay = None;
                    self.state.loaded_range = None;
                    self.refresh_calendar()?;
                    match item {
                        SearchResult::Event(event) => {
                            let selected = self
                                .events_on_selected_date()
                                .position(|candidate| {
                                    candidate.event_id == event.event_id
                                        && candidate.original_date == event.original_date
                                })
                                .unwrap_or(0);
                            self.state.selected_event = selected;
                        }
                        SearchResult::Note(note) => {
                            let selected = self
                                .notes_on_selected_date()
                                .position(|candidate| candidate.id == note.id)
                                .unwrap_or(0);
                            self.state.selected_note = selected;
                        }
                    }
                }
            }
            None if self.state.active_view != View::Day => {
                self.state.active_view = View::Day;
                self.state.focused_pane = FocusedPane::Events;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
            }
            None => self.edit_selected()?,
        }
        Ok(())
    }

    fn back(&mut self) -> AppResult<()> {
        if self.state.popup.is_some() {
            self.state.popup = None;
            self.state.tag_suggestions.clear();
            self.sync_input_mode();
        } else if self.state.agenda.searching {
            self.state.agenda.searching = false;
            self.sync_input_mode();
        } else if self.state.overlay.take().is_none() && self.state.active_view != View::Week {
            self.state.active_view = View::Week;
            self.state.focused_pane = FocusedPane::Events;
            self.state.loaded_range = None;
            self.refresh_calendar()?;
        }
        Ok(())
    }

    fn toggle_overlay(&mut self, overlay: Overlay) -> AppResult<()> {
        if self.state.popup.is_some() {
            return Ok(());
        }
        if self.state.overlay == Some(overlay) {
            self.state.overlay = None;
            self.state.agenda.searching = false;
        } else {
            self.state.overlay = Some(overlay);
            match overlay {
                Overlay::Agenda => self.refresh_agenda()?,
                Overlay::Upcoming => {
                    self.state.selected_link = 0;
                    self.refresh_upcoming()?;
                }
            }
        }
        self.sync_input_mode();
        Ok(())
    }

    fn toggle_focus(&mut self) {
        if self.state.active_view == View::Day
            && self.state.overlay.is_none()
            && self.state.popup.is_none()
        {
            self.state.focused_pane = match self.state.focused_pane {
                FocusedPane::Events => FocusedPane::Notes,
                FocusedPane::Notes => FocusedPane::Links,
                FocusedPane::Links => FocusedPane::Events,
            };
        }
    }

    fn create_selected(&mut self) -> AppResult<()> {
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

    fn edit_selected(&mut self) -> AppResult<()> {
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

    fn delete_selected(&mut self) -> AppResult<()> {
        if let Some(event) = self.selected_event_occurrence() {
            if event.is_recurring {
                self.state.popup = Some(Popup::Scope(ScopeOperation::Delete(event)));
            } else {
                self.ask_delete(DeleteTarget::Event(event.event_id), "Удалить событие?");
            }
            self.sync_input_mode();
            return Ok(());
        }
        if self.state.overlay == Some(Overlay::Upcoming) {
            return Err(app_error("no upcoming event selected"));
        }
        if self.state.overlay == Some(Overlay::Agenda) {
            let id = self
                .selected_note_for_action()
                .map(|note| note.id)
                .ok_or_else(|| app_error("no item selected"))?;
            self.ask_delete(DeleteTarget::Note(id), "Удалить заметку и её ссылки?");
            self.sync_input_mode();
            return Ok(());
        }
        match self.state.focused_pane {
            FocusedPane::Notes => {
                let id = self
                    .selected_note_for_action()
                    .map(|note| note.id)
                    .ok_or_else(|| app_error("no note selected"))?;
                self.ask_delete(DeleteTarget::Note(id), "Удалить заметку и её ссылки?");
            }
            FocusedPane::Links => {
                let id = self
                    .selected_link()
                    .map(|link| link.id)
                    .ok_or_else(|| app_error("no link selected"))?;
                self.ask_delete(DeleteTarget::Link(id), "Удалить ссылку?");
            }
            FocusedPane::Events => return Err(app_error("no event selected")),
        }
        self.sync_input_mode();
        Ok(())
    }

    fn change_importance(&mut self) -> AppResult<()> {
        let event = self
            .selected_event_occurrence()
            .ok_or_else(|| app_error("no event selected"))?;
        if event.is_recurring {
            self.state.popup = Some(Popup::Scope(ScopeOperation::Importance(event)));
            self.sync_input_mode();
        } else {
            self.database
                .set_event_importance(event.event_id, event.importance.next())?;
            self.refresh_after_change()?;
        }
        Ok(())
    }

    fn ask_delete(&mut self, target: DeleteTarget, message: &str) {
        self.state.popup = Some(Popup::Confirm {
            message: message.into(),
            target,
        });
    }

    fn selected_event_occurrence(&self) -> Option<EventOccurrence> {
        match self.state.overlay {
            Some(Overlay::Upcoming) => self
                .state
                .upcoming
                .items
                .get(self.state.upcoming.selected)
                .cloned(),
            Some(Overlay::Agenda) => {
                match self.state.agenda.items.get(self.state.agenda.selected) {
                    Some(SearchResult::Event(event)) => Some(event.clone()),
                    _ => None,
                }
            }
            None if self.state.focused_pane == FocusedPane::Events => self
                .events_on_selected_date()
                .nth(self.state.selected_event)
                .cloned(),
            None => None,
        }
    }

    fn selected_note_for_action(&self) -> Option<&Note> {
        match self.state.overlay {
            Some(Overlay::Agenda) => {
                match self.state.agenda.items.get(self.state.agenda.selected) {
                    Some(SearchResult::Note(note)) => Some(note),
                    _ => None,
                }
            }
            _ => self.selected_note(),
        }
    }

    fn open_event_editor(&mut self, event_id: EventId) -> AppResult<()> {
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

    fn input_character(&mut self, character: char) -> AppResult<()> {
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

    fn backspace(&mut self) -> AppResult<()> {
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

    fn move_field(&mut self, delta: i32) {
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

    fn adjust_field(&mut self, forward: bool) {
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

    fn refresh_tag_suggestions(&mut self) -> AppResult<()> {
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

    fn submit(&mut self) -> AppResult<()> {
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

    fn save_editor(&mut self, editor: Editor) -> AppResult<()> {
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

    fn confirm_delete(&mut self, confirmed: bool) -> AppResult<()> {
        let Some(Popup::Confirm { target, .. }) = self.state.popup.clone() else {
            return Ok(());
        };
        if confirmed {
            match target {
                DeleteTarget::Event(id) => self.database.delete_event(id)?,
                DeleteTarget::Recurrence(id) => self.database.delete_recurrence(id)?,
                DeleteTarget::Occurrence(id, date) => self.database.cancel_occurrence(id, date)?,
                DeleteTarget::Note(id) => self.database.delete_note(id)?,
                DeleteTarget::Link(id) => self.database.delete_link(id)?,
            }
            self.state.status_message = Some("Удалено".into());
            self.refresh_after_change()?;
        }
        self.state.popup = None;
        self.sync_input_mode();
        Ok(())
    }

    fn choose_scope(&mut self, series: bool) -> AppResult<()> {
        let Some(Popup::Scope(operation)) = self.state.popup.clone() else {
            return Ok(());
        };
        let occurrence = match &operation {
            ScopeOperation::Edit(event)
            | ScopeOperation::Delete(event)
            | ScopeOperation::Importance(event) => event,
        };
        let recurrence_id = occurrence
            .recurrence_id
            .ok_or_else(|| app_error("event is not recurring"))?;
        match operation {
            ScopeOperation::Edit(_) if series => {
                let base = self
                    .database
                    .event_for_recurrence(recurrence_id)?
                    .ok_or_else(|| app_error("recurring event no longer exists"))?;
                self.open_event_editor(base.id)?;
            }
            ScopeOperation::Edit(event) => {
                self.state.popup = Some(Popup::Editor(Editor::Event {
                    form: EventForm::from_occurrence(&event),
                    target: EventTarget::Occurrence {
                        recurrence_id,
                        original_date: event.original_date,
                    },
                }));
            }
            ScopeOperation::Delete(_) if series => {
                self.ask_delete(
                    DeleteTarget::Recurrence(recurrence_id),
                    "Удалить всю серию?",
                );
            }
            ScopeOperation::Delete(event) => {
                self.ask_delete(
                    DeleteTarget::Occurrence(recurrence_id, event.original_date),
                    "Удалить только это occurrence?",
                );
            }
            ScopeOperation::Importance(_) if series => {
                let base = self
                    .database
                    .event_for_recurrence(recurrence_id)?
                    .ok_or_else(|| app_error("recurring event no longer exists"))?;
                self.database
                    .set_event_importance(base.id, base.importance.next())?;
                self.state.popup = None;
                self.refresh_after_change()?;
            }
            ScopeOperation::Importance(event) => {
                let replacement = NewEvent {
                    title: event.title,
                    description: event.description,
                    start_date: event.date,
                    start_time: event.start_time,
                    end_time: event.end_time,
                    importance: event.importance.next(),
                };
                let tags = event
                    .tags
                    .into_iter()
                    .map(|tag| tag.name)
                    .collect::<Vec<_>>();
                self.database.modify_occurrence(
                    recurrence_id,
                    event.original_date,
                    &replacement,
                    &tags,
                )?;
                self.state.popup = None;
                self.refresh_after_change()?;
            }
        }
        self.sync_input_mode();
        Ok(())
    }

    fn sync_input_mode(&mut self) {
        self.state.input_mode = match self.state.popup {
            Some(Popup::Editor(_)) => InputMode::Editor,
            Some(Popup::Confirm { .. }) => InputMode::Confirm,
            Some(Popup::Scope(_)) => InputMode::Scope,
            Some(Popup::GotoDate(_)) => InputMode::GotoDate,
            Some(Popup::Help) => InputMode::Normal,
            None if self.state.agenda.searching => InputMode::Search,
            None => InputMode::Normal,
        };
    }

    pub(super) fn set_error(&mut self, error: impl std::fmt::Display) {
        self.state.status_message = Some(format!("Ошибка: {error}"));
    }
}

fn move_index(current: usize, length: usize, delta: i32) -> usize {
    if length == 0 {
        return 0;
    }
    (current as i32 + delta).clamp(0, length.saturating_sub(1) as i32) as usize
}

fn app_error(message: &'static str) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}
