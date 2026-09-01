mod delete;
mod editor;
mod favorite_links;
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
                if self.state.active_view == View::Month && self.state.overlay.is_none() {
                    self.toggle_month_day_preview()?;
                    return Ok(());
                }
                let url = self
                    .selected_url()
                    .ok_or_else(|| app_error("no link selected"))?;
                external::open_url(&url)?;
                self.state.status_message = Some("Ссылка открыта".into());
            }
            Action::CopyLink => {
                let url = self
                    .selected_url()
                    .ok_or_else(|| app_error("no link selected"))?;
                external::copy_url(&url)?;
                self.state.status_message = Some("URL скопирован".into());
            }
            Action::OpenDirectory => {
                let directory = self
                    .selected_event_occurrence()
                    .and_then(|event| event.directory)
                    .ok_or_else(|| app_error("no directory attached to the selected event"))?;
                if !directory.is_dir() {
                    return Err(app_error("attached event directory no longer exists"));
                }
                self.pending_directory = Some(directory);
                self.state.status_message = Some("Открывается shell в директории события".into());
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
            Action::EnterField => self.enter_field()?,
            Action::TabField => self.tab_field(),
            Action::AdjustLeft => self.adjust_field(false),
            Action::AdjustRight => self.adjust_field(true),
            Action::OpenLinkBank => self.open_link_bank()?,
            Action::AddFavoriteLink => self.add_favorite_link(),
            Action::EditFavoriteLink => self.edit_favorite_link(),
            Action::ToggleFavoriteLink => self.toggle_favorite_link()?,
            Action::StartLinkSearch => self.start_link_search(),
            Action::OpenFavoriteLink => self.open_favorite_link()?,
            Action::Submit => self.submit()?,
            Action::Confirm(confirmed) => {
                if matches!(self.state.popup, Some(Popup::SaveConfirm { .. })) {
                    self.confirm_save(confirmed)?;
                } else {
                    self.confirm_delete(confirmed)?;
                }
            }
            Action::ChooseOccurrence => self.choose_scope(false)?,
            Action::ChooseSeries => self.choose_scope(true)?,
            Action::CycleDateFilter => {
                if self.state.overlay == Some(Overlay::Agenda) {
                    self.state.agenda.filters.date = match self.state.agenda.filters.date {
                        DateFilter::All => DateFilter::Today,
                        DateFilter::Today => DateFilter::ThisWeek,
                        DateFilter::ThisWeek => DateFilter::ThisMonth,
                        DateFilter::ThisMonth => DateFilter::All,
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
                if let Some(crate::app::Popup::MonthDayPreview { date, selected }) =
                    self.state.popup
                {
                    let occ_count = self
                        .state
                        .occurrences
                        .iter()
                        .filter(|e| e.date == date)
                        .count();
                    let tasks_today: Vec<_> = self
                        .state
                        .tasks
                        .iter()
                        .filter(|t| t.date == Some(date))
                        .collect();
                    if selected >= occ_count && selected < occ_count + tasks_today.len() {
                        let task = tasks_today[selected - occ_count];
                        self.database.toggle_task(task.id)?;
                        self.refresh_calendar()?;
                        return Ok(());
                    }
                }
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
            Some(Popup::SaveConfirm { .. }) => InputMode::Confirm,
            Some(Popup::Confirm { .. }) => InputMode::Confirm,
            Some(Popup::Scope(_)) => InputMode::Scope,
            Some(Popup::GotoDate(_)) => InputMode::GotoDate,
            Some(Popup::LinkBank) => {
                if self
                    .state
                    .link_bank
                    .as_ref()
                    .is_some_and(|bank| bank.searching)
                {
                    InputMode::LinkSearch
                } else {
                    InputMode::LinkBank
                }
            }
            Some(Popup::Help) => InputMode::Normal,
            Some(Popup::MonthDayPreview { .. }) => InputMode::Normal,
            None if self.state.agenda.searching => InputMode::Search,
            None => InputMode::Normal,
        };
    }

    pub(super) fn toggle_month_day_preview(&mut self) -> AppResult<()> {
        if matches!(self.state.popup, Some(Popup::MonthDayPreview { .. })) {
            self.state.popup = None;
        } else {
            self.state.popup = Some(Popup::MonthDayPreview {
                date: self.state.selected_date,
                selected: 0,
            });
        }
        self.sync_input_mode();
        Ok(())
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
    use crate::app::{Editor, EventForm, EventTarget, NoteForm, Popup, state::FocusedPane};
    use crate::config::Config;
    use crate::model::{Event, EventOccurrence, Importance};
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

    #[test]
    fn editor_enter_confirms_at_the_end_and_tab_changes_choices() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        let mut form = EventForm::new(app.state.today);
        form.active = EventForm::IMPORTANCE_FIELD;
        app.state.popup = Some(Popup::Editor(Editor::Event {
            form,
            target: EventTarget::New,
        }));

        app.apply(Action::TabField).unwrap();
        let Some(Popup::Editor(Editor::Event { form, .. })) = app.state.popup.as_mut() else {
            panic!("event editor should stay open");
        };
        assert_eq!(form.importance, Importance::High);
        assert_eq!(form.active, EventForm::IMPORTANCE_FIELD);
        form.active = EventForm::REPEAT_FIELD;
        app.apply(Action::TabField).unwrap();
        let Some(Popup::Editor(Editor::Event { form, .. })) = app.state.popup.as_mut() else {
            panic!("event editor should stay open");
        };
        assert!(form.weekly);
        form.active = EventForm::DIRECTORY_FIELD;

        app.apply(Action::EnterField).unwrap();
        assert!(matches!(app.state.popup, Some(Popup::SaveConfirm { .. })));
        app.apply(Action::Confirm(false)).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::Editor(Editor::Event { .. }))
        ));

        app.state.popup = Some(Popup::Editor(Editor::Note {
            form: NoteForm {
                title: String::new(),
                date: app.state.today.format("%d.%m.%Y").to_string(),
                body: "текст".into(),
                active: 2,
            },
            target: None,
        }));
        app.apply(Action::EnterField).unwrap();
        assert!(matches!(app.state.popup, Some(Popup::SaveConfirm { .. })));
    }

    #[test]
    fn link_bank_keeps_the_event_draft_and_directory_action_is_queued() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        app.state.popup = Some(Popup::Editor(Editor::Event {
            form: EventForm::new(app.state.today),
            target: EventTarget::New,
        }));
        app.apply(Action::OpenLinkBank).unwrap();
        app.apply(Action::AddFavoriteLink).unwrap();
        let Some(Popup::Editor(Editor::FavoriteLink { form, .. })) = app.state.popup.as_mut()
        else {
            panic!("favorite link editor should open");
        };
        form.label = "Условие".into();
        form.url = "https://example.com/task".into();
        form.description = "Домашняя работа".into();
        form.tags = "#дз".into();
        app.apply(Action::Submit).unwrap();
        let link_id = app
            .state
            .link_bank
            .as_ref()
            .unwrap()
            .event_form
            .favorite_link_ids[0];
        app.apply(Action::Back).unwrap();
        let Some(Popup::Editor(Editor::Event { form, .. })) = app.state.popup.as_ref() else {
            panic!("event draft should be restored");
        };
        assert_eq!(form.favorite_link_ids, [link_id]);

        app.state.popup = None;
        app.state.active_view = View::Day;
        app.state.focused_pane = FocusedPane::Events;
        app.state.occurrences = vec![EventOccurrence::from_event(
            &Event {
                id: 1,
                title: "ДЗ".into(),
                description: None,
                start_date: app.state.selected_date,
                start_time: None,
                end_time: None,
                importance: Importance::Normal,
                recurrence_id: None,
                directory: Some("/tmp".into()),
            },
            Vec::new(),
        )];
        app.apply(Action::OpenDirectory).unwrap();
        assert_eq!(
            app.take_directory_request().as_deref(),
            Some(std::path::Path::new("/tmp"))
        );
    }

    #[test]
    fn agenda_date_filter_cycles_day_week_month_all() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        app.state.overlay = Some(Overlay::Agenda);

        for expected in [
            DateFilter::Today,
            DateFilter::ThisWeek,
            DateFilter::ThisMonth,
            DateFilter::All,
        ] {
            app.apply(Action::CycleDateFilter).unwrap();
            assert_eq!(app.state.agenda.filters.date, expected);
        }
    }

    #[test]
    fn directory_autocompletion_in_editor() {
        let temp_dir = std::env::temp_dir().join("rutendar_test_dir_comp");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("subfolder")).unwrap();

        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        app.apply(Action::Create).unwrap();

        if let Some(Popup::Editor(Editor::Event { form, .. })) = app.state.popup.as_mut() {
            form.active = EventForm::DIRECTORY_FIELD;
            form.directory = format!("{}/sub", temp_dir.display());
        }

        app.apply(Action::Input('f')).unwrap();
        assert!(!app.state.path_suggestions.is_empty());
        assert!(app.state.path_suggestions[0].ends_with("/subfolder/"));

        app.apply(Action::TabField).unwrap();
        if let Some(Popup::Editor(Editor::Event { form, .. })) = app.state.popup.as_ref() {
            assert!(form.directory.ends_with("/subfolder/"));
        } else {
            panic!("expected editor popup");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn month_day_preview_toggle_and_navigation() {
        let mut db = Database::in_memory().unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        db.create_event(
            &crate::model::NewEvent {
                title: "Событие 1".into(),
                description: None,
                start_date: today,
                start_time: chrono::NaiveTime::from_hms_opt(10, 0, 0),
                end_time: None,
                importance: Importance::Normal,
                directory: None,
            },
            None,
            &[],
            &[],
        )
        .unwrap();
        db.create_event(
            &crate::model::NewEvent {
                title: "Событие 2".into(),
                description: None,
                start_date: today,
                start_time: chrono::NaiveTime::from_hms_opt(15, 0, 0),
                end_time: None,
                importance: Importance::High,
                directory: None,
            },
            None,
            &[],
            &[],
        )
        .unwrap();

        let mut app = App::new(db, Config::default()).unwrap();
        app.state.selected_date = today;
        app.apply(Action::SwitchView(View::Month)).unwrap();

        // Press 'o' (Action::OpenLink in Month view) to open preview
        app.apply(Action::OpenLink).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::MonthDayPreview { date, selected: 0 }) if date == today
        ));

        // Press MoveDown (j) -> selected becomes 1
        app.apply(Action::MoveDown).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::MonthDayPreview { selected: 1, .. })
        ));

        // Press Back (Esc) -> popup closes
        app.apply(Action::Back).unwrap();
        assert!(app.state.popup.is_none());

        // Press 'o' again to reopen
        app.apply(Action::OpenLink).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::MonthDayPreview { .. })
        ));

        // Press Open (Enter) -> switches to Day view and opens editor
        app.apply(Action::Open).unwrap();
        assert_eq!(app.state.active_view, View::Day);
        assert!(matches!(app.state.popup, Some(Popup::Editor(_))));
    }

    #[test]
    fn month_day_preview_with_tasks_and_space_toggle() {
        let mut db = Database::in_memory().unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        db.create_event(
            &crate::model::NewEvent {
                title: "Событие".into(),
                description: None,
                start_date: today,
                start_time: chrono::NaiveTime::from_hms_opt(10, 0, 0),
                end_time: None,
                importance: Importance::Normal,
                directory: None,
            },
            None,
            &[],
            &[],
        )
        .unwrap();

        let task_id = db
            .create_task(&crate::model::NewTask {
                title: "Задание 1".into(),
                description: None,
                date: Some(today),
                importance: Importance::High,
            })
            .unwrap();

        let mut app = App::new(db, Config::default()).unwrap();
        app.state.selected_date = today;
        app.apply(Action::SwitchView(View::Month)).unwrap();

        // Open preview with 'o'
        app.apply(Action::OpenLink).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::MonthDayPreview { date, selected: 0 }) if date == today
        ));

        // Move to task (index 1)
        app.apply(Action::MoveDown).unwrap();
        assert!(matches!(
            app.state.popup,
            Some(Popup::MonthDayPreview { selected: 1, .. })
        ));

        // Press Space (ToggleTagFilter) to toggle completion
        app.apply(Action::ToggleTagFilter).unwrap();
        let task = app.database.get_task(task_id).unwrap().unwrap();
        assert!(task.is_done);

        // Press Space again to toggle back
        app.apply(Action::ToggleTagFilter).unwrap();
        let task = app.database.get_task(task_id).unwrap().unwrap();
        assert!(!task.is_done);

        // Press Delete to remove task
        app.apply(Action::Delete).unwrap();
        assert!(app.database.get_task(task_id).unwrap().is_none());
    }
}
