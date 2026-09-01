use crate::app::{App, AppResult, DeleteTarget, FocusedPane, Overlay, Popup, ScopeOperation};

impl App {
    pub(super) fn delete_selected(&mut self) -> AppResult<()> {
        if let Some(Popup::MonthDayPreview { date, selected }) = self.state.popup {
            let occ_count = self
                .state
                .occurrences
                .iter()
                .filter(|e| e.date == date)
                .count();
            let tasks: Vec<_> = self
                .state
                .tasks
                .iter()
                .filter(|t| t.date == Some(date))
                .collect();
            if selected >= occ_count && selected < occ_count + tasks.len() {
                let task_id = tasks[selected - occ_count].id;
                self.database.delete_task(task_id)?;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
                let total = self.day_preview_items_count(date);
                let new_sel = if total == 0 {
                    0
                } else {
                    selected.min(total - 1)
                };
                self.state.popup = Some(Popup::MonthDayPreview {
                    date,
                    selected: new_sel,
                });
                return Ok(());
            }
        }
        if self.state.popup.is_none()
            && self.state.overlay.is_none()
            && (self.state.active_view == crate::app::View::Week
                || (self.state.active_view == crate::app::View::Day
                    && self.state.focused_pane == FocusedPane::Events))
        {
            let occ_count = self.events_on_selected_date().count();
            let tasks: Vec<_> = self
                .state
                .tasks
                .iter()
                .filter(|t| t.date == Some(self.state.selected_date))
                .collect();
            if self.state.selected_event >= occ_count
                && self.state.selected_event < occ_count + tasks.len()
            {
                let task_id = tasks[self.state.selected_event - occ_count].id;
                self.database.delete_task(task_id)?;
                self.state.loaded_range = None;
                self.refresh_calendar()?;
                let total = self.events_and_tasks_on_selected_date_count();
                self.state.selected_event = self.state.selected_event.min(total.saturating_sub(1));
                return Ok(());
            }
        }
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
            return Ok(());
        }
        if self.state.overlay == Some(Overlay::Agenda) {
            if let Some(note) = self.selected_note_for_action() {
                self.ask_delete(DeleteTarget::Note(note.id), "Удалить заметку и её ссылки?");
                self.sync_input_mode();
                return Ok(());
            }
            if let Some(tag) = self
                .state
                .agenda
                .available_tags
                .get(self.state.agenda.tag_cursor)
            {
                let tag_name = tag.name.clone();
                let tag_id = tag.id;
                self.ask_delete(
                    DeleteTarget::Tag(tag_id),
                    &format!("Удалить тег #{tag_name}?"),
                );
                self.sync_input_mode();
                return Ok(());
            }
            return Ok(());
        }
        match self.state.focused_pane {
            FocusedPane::Notes => {
                if let Some(note) = self.selected_note_for_action() {
                    self.ask_delete(DeleteTarget::Note(note.id), "Удалить заметку и её ссылки?");
                }
            }
            FocusedPane::Links => {
                if let Some(link) = self.selected_link() {
                    self.ask_delete(DeleteTarget::Link(link.id), "Удалить ссылку?");
                }
            }
            FocusedPane::Events => {}
        }
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn delete_tag_selected(&mut self) -> AppResult<()> {
        if self.state.overlay == Some(Overlay::Agenda)
            && let Some(tag) = self
                .state
                .agenda
                .available_tags
                .get(self.state.agenda.tag_cursor)
        {
            let tag_name = tag.name.clone();
            let tag_id = tag.id;
            self.ask_delete(
                DeleteTarget::Tag(tag_id),
                &format!("Удалить тег #{tag_name}?"),
            );
            self.sync_input_mode();
            return Ok(());
        }
        self.delete_selected()
    }

    pub(super) fn ask_delete(&mut self, target: DeleteTarget, message: &str) {
        self.state.popup = Some(Popup::Confirm {
            message: message.into(),
            target,
        });
    }

    pub(super) fn confirm_delete(&mut self, confirmed: bool) -> AppResult<()> {
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
                DeleteTarget::Tag(id) => self.database.delete_tag(id)?,
            }
            self.state.status_message = Some("Удалено".into());
            self.refresh_after_change()?;
        }
        self.state.popup = None;
        self.sync_input_mode();
        Ok(())
    }
}
