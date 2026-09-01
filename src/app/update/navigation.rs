use chrono::Duration;

use super::move_index;
use crate::{
    app::{App, AppResult, FocusedPane, Overlay, View},
    calendar::move_month,
    search::SearchResult,
};

impl App {
    pub(super) fn move_horizontal(&mut self, delta: i32) -> AppResult<()> {
        if self.state.popup.is_some() || self.state.overlay.is_some() {
            return Ok(());
        }
        if self.state.active_view == View::Day {
            if delta < 0 {
                self.state.focused_pane = FocusedPane::Events;
            } else if self.state.focused_pane == FocusedPane::Events {
                self.state.focused_pane = FocusedPane::Notes;
            }
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

    pub(super) fn navigate_day(&mut self, delta: i32) -> AppResult<()> {
        if self.state.popup.is_some() || self.state.overlay.is_some() {
            return Ok(());
        }
        self.state.selected_date += Duration::days(i64::from(delta));
        self.state.selected_event = 0;
        self.state.selected_note = 0;
        self.state.selected_link = 0;
        self.refresh_calendar()
    }

    pub(super) fn move_vertical(&mut self, delta: i32) -> AppResult<()> {
        if matches!(self.state.popup, Some(crate::app::Popup::LinkBank)) {
            self.move_link_bank(delta);
            return Ok(());
        }
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
                View::Week => {
                    self.state.selected_event = move_index(
                        self.state.selected_event,
                        self.events_on_selected_date().count(),
                        delta,
                    );
                }
                View::Day => match self.state.focused_pane {
                    FocusedPane::Events => {
                        self.state.selected_event = move_index(
                            self.state.selected_event,
                            self.events_on_selected_date().count(),
                            delta,
                        );
                    }
                    FocusedPane::Notes => {
                        let notes_count = self.notes_on_selected_date().count();
                        if delta > 0
                            && (notes_count == 0 || self.state.selected_note + 1 >= notes_count)
                        {
                            self.state.focused_pane = FocusedPane::Links;
                            self.state.selected_link = 0;
                        } else {
                            self.state.selected_note =
                                move_index(self.state.selected_note, notes_count, delta);
                            self.state.selected_link = 0;
                        }
                    }
                    FocusedPane::Links => {
                        let links_count = self.day_links_count();
                        if delta < 0 && self.state.selected_link == 0 {
                            self.state.focused_pane = FocusedPane::Notes;
                            let notes_count = self.notes_on_selected_date().count();
                            if notes_count > 0 {
                                self.state.selected_note = notes_count - 1;
                            }
                        } else {
                            self.state.selected_link =
                                move_index(self.state.selected_link, links_count, delta);
                        }
                    }
                },
            },
        }
        Ok(())
    }

    pub(super) fn go_to_top(&mut self) -> AppResult<()> {
        if self.state.popup.is_some() {
            return Ok(());
        }
        match self.state.overlay {
            Some(Overlay::Agenda) => {
                self.state.agenda.selected = 0;
            }
            Some(Overlay::Upcoming) => {
                self.state.upcoming.selected = 0;
                self.state.selected_link = 0;
            }
            None => match self.state.active_view {
                View::Week => {
                    self.state.selected_event = 0;
                }
                View::Day => match self.state.focused_pane {
                    FocusedPane::Events => self.state.selected_event = 0,
                    FocusedPane::Notes => {
                        self.state.selected_note = 0;
                        self.state.selected_link = 0;
                    }
                    FocusedPane::Links => self.state.selected_link = 0,
                },
                View::Month => {
                    self.state.selected_date =
                        crate::calendar::month_start(self.state.selected_date);
                    self.refresh_calendar()?;
                }
                View::Year => {
                    if let Some(first_day) = chrono::NaiveDate::from_ymd_opt(
                        chrono::Datelike::year(&self.state.selected_date),
                        1,
                        1,
                    ) {
                        self.state.selected_date = first_day;
                        self.refresh_calendar()?;
                    }
                }
            },
        }
        Ok(())
    }

    pub(super) fn go_to_bottom(&mut self) -> AppResult<()> {
        if self.state.popup.is_some() {
            return Ok(());
        }
        match self.state.overlay {
            Some(Overlay::Agenda) => {
                self.state.agenda.selected = self.state.agenda.items.len().saturating_sub(1);
            }
            Some(Overlay::Upcoming) => {
                self.state.upcoming.selected = self.state.upcoming.items.len().saturating_sub(1);
                self.state.selected_link = 0;
            }
            None => match self.state.active_view {
                View::Week => {
                    let count = self.events_on_selected_date().count();
                    self.state.selected_event = count.saturating_sub(1);
                }
                View::Day => match self.state.focused_pane {
                    FocusedPane::Events => {
                        let count = self.events_on_selected_date().count();
                        self.state.selected_event = count.saturating_sub(1);
                    }
                    FocusedPane::Notes => {
                        let count = self.notes_on_selected_date().count();
                        self.state.selected_note = count.saturating_sub(1);
                        self.state.selected_link = 0;
                    }
                    FocusedPane::Links => {
                        let count = self.selected_note().map_or(0, |n| n.links.len());
                        self.state.selected_link = count.saturating_sub(1);
                    }
                },
                View::Month => {
                    self.state.selected_date = crate::calendar::month_end(self.state.selected_date);
                    self.refresh_calendar()?;
                }
                View::Year => {
                    if let Some(last_day) = chrono::NaiveDate::from_ymd_opt(
                        chrono::Datelike::year(&self.state.selected_date),
                        12,
                        31,
                    ) {
                        self.state.selected_date = last_day;
                        self.refresh_calendar()?;
                    }
                }
            },
        }
        Ok(())
    }

    pub(super) fn open_selected(&mut self) -> AppResult<()> {
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

    pub(super) fn back(&mut self) -> AppResult<()> {
        if self
            .state
            .link_bank
            .as_ref()
            .is_some_and(|bank| bank.searching)
        {
            self.finish_link_search();
        } else if matches!(self.state.popup, Some(crate::app::Popup::LinkBank)) {
            self.close_link_bank();
        } else if matches!(
            self.state.popup,
            Some(crate::app::Popup::Editor(
                crate::app::Editor::FavoriteLink { .. }
            ))
        ) && self.state.link_bank.is_some()
        {
            self.state.popup = Some(crate::app::Popup::LinkBank);
            self.sync_input_mode();
        } else if self.state.popup.is_some() {
            self.state.popup = None;
            self.state.tag_suggestions.clear();
            self.state.path_suggestions.clear();
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

    pub(super) fn toggle_overlay(&mut self, overlay: Overlay) -> AppResult<()> {
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

    pub(super) fn toggle_focus(&mut self) {
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

    pub(super) fn toggle_focus_prev(&mut self) {
        if self.state.active_view == View::Day
            && self.state.overlay.is_none()
            && self.state.popup.is_none()
        {
            self.state.focused_pane = match self.state.focused_pane {
                FocusedPane::Events => FocusedPane::Links,
                FocusedPane::Notes => FocusedPane::Events,
                FocusedPane::Links => FocusedPane::Notes,
            };
        }
    }
}
