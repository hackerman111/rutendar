mod delete;
mod editor;
mod navigation;
mod scope;

use std::error::Error;

use crate::{
    app::{Action, App, AppResult, FocusedPane, InputMode, Overlay, Popup, UpcomingSort, View},
    external,
    model::Importance,
    search::{DateFilter, ItemType, SortBy, TagMatching},
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
            Action::NextDay => self.navigate_day(1)?,
            Action::PreviousDay => self.navigate_day(-1)?,
            Action::GoToTop => self.go_to_top()?,
            Action::GoToBottom => self.go_to_bottom()?,
            Action::PageUp => self.move_vertical(-5)?,
            Action::PageDown => self.move_vertical(5)?,
            Action::Open => self.open_selected()?,
            Action::Back => self.back()?,
            Action::Create => self.create_selected()?,
            Action::Edit => self.edit_selected()?,
            Action::Delete => self.delete_selected()?,
            Action::ChangeImportance => self.change_importance()?,
            Action::OpenAgenda => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.searching = true;
                    self.state.input_mode = InputMode::Search;
                } else {
                    self.toggle_overlay(Overlay::Agenda)?;
                }
            }
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
            Action::NextView => {
                if self.state.active_view == View::Day
                    && self.state.overlay.is_none()
                    && self.state.popup.is_none()
                {
                    self.toggle_focus();
                } else {
                    let next_view = self.state.active_view.next();
                    self.apply(Action::SwitchView(next_view))?;
                }
            }
            Action::PreviousView => {
                if self.state.active_view == View::Day
                    && self.state.overlay.is_none()
                    && self.state.popup.is_none()
                {
                    self.toggle_focus_prev();
                } else {
                    let prev_view = self.state.active_view.previous();
                    self.apply(Action::SwitchView(prev_view))?;
                }
            }
            Action::DeleteTag => self.delete_tag_selected()?,
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

    pub(super) fn sync_input_mode(&mut self) {
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

    pub(crate) fn set_error(&mut self, error: impl std::fmt::Display) {
        self.state.status_message = Some(format!("Ошибка: {error}"));
    }
}

pub(super) fn move_index(current: usize, length: usize, delta: i32) -> usize {
    if length == 0 {
        return 0;
    }
    (current as i32 + delta).clamp(0, length.saturating_sub(1) as i32) as usize
}

pub(super) fn app_error(message: &'static str) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::FocusedPane;
    use crate::config::Config;
    use crate::storage::Database;
    use chrono::Duration;

    #[test]
    fn tab_cycles_panes_in_day_view_and_views_in_week_view() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();

        // In Day view, NextView (Tab) cycles panes
        app.state.active_view = View::Day;
        assert_eq!(app.state.focused_pane, FocusedPane::Events);

        app.apply(Action::NextView).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Notes);

        app.apply(Action::NextView).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Links);

        app.apply(Action::NextView).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Events);

        app.apply(Action::PreviousView).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Links);

        // In Week view, NextView cycles view to Day
        app.state.active_view = View::Week;
        app.apply(Action::NextView).unwrap();
        assert_eq!(app.state.active_view, View::Day);
    }

    #[test]
    fn vim_motions_in_day_view_and_day_navigation() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        app.state.active_view = View::Day;

        // h / l moves between Events and Notes
        app.state.focused_pane = FocusedPane::Events;
        app.apply(Action::MoveRight).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Notes);

        app.apply(Action::MoveLeft).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Events);

        // j at bottom of empty notes moves to Links
        app.state.focused_pane = FocusedPane::Notes;
        app.apply(Action::MoveDown).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Links);

        // k at top of links moves to Notes
        app.apply(Action::MoveUp).unwrap();
        assert_eq!(app.state.focused_pane, FocusedPane::Notes);

        // n and N navigates days
        let initial_date = app.state.selected_date;
        app.apply(Action::NextDay).unwrap();
        assert_eq!(app.state.selected_date, initial_date + Duration::days(1));

        app.apply(Action::PreviousDay).unwrap();
        assert_eq!(app.state.selected_date, initial_date);
    }
}
