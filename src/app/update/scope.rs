use super::app_error;
use crate::{
    app::{
        App, AppResult, DeleteTarget, Editor, EventForm, EventTarget, FocusedPane, Overlay, Popup,
        ScopeOperation,
    },
    model::{EventOccurrence, NewEvent, Note},
    search::SearchResult,
};

impl App {
    pub(super) fn change_importance(&mut self) -> AppResult<()> {
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

    pub(super) fn choose_scope(&mut self, series: bool) -> AppResult<()> {
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
                let favorite_link_ids = event
                    .favorite_links
                    .iter()
                    .map(|link| link.id)
                    .collect::<Vec<_>>();
                let replacement = NewEvent {
                    title: event.title,
                    description: event.description,
                    start_date: event.date,
                    start_time: event.start_time,
                    end_time: event.end_time,
                    importance: event.importance.next(),
                    directory: event.directory,
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
                    &favorite_link_ids,
                )?;
                self.state.popup = None;
                self.refresh_after_change()?;
            }
        }
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn selected_event_occurrence(&self) -> Option<EventOccurrence> {
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

    pub(super) fn selected_note_for_action(&self) -> Option<&Note> {
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

    pub(super) fn selected_url(&self) -> Option<String> {
        self.selected_event_occurrence()
            .and_then(|event| event.favorite_links.first().map(|link| link.url.clone()))
            .or_else(|| self.selected_link().map(|link| link.url.clone()))
    }
}
