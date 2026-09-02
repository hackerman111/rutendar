use std::error::Error;

use chrono::{Datelike, Duration, NaiveDate};

use crate::{
    cli::list::filter_events,
    model::{EventOccurrence, Task},
    storage::Database,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineTab {
    Day,
    Week,
    Search,
}

impl InlineTab {
    pub fn next(self) -> Self {
        match self {
            Self::Day => Self::Week,
            Self::Week => Self::Search,
            Self::Search => Self::Day,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Day => Self::Search,
            Self::Week => Self::Day,
            Self::Search => Self::Week,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineOutcome {
    Exit,
    OpenFullTui { initial_date: Option<NaiveDate> },
}

#[derive(Debug, Clone, Copy)]
pub enum SelectedDayItem<'a> {
    Event(&'a EventOccurrence),
    Task(&'a Task),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingDelete {
    Event { id: i64, title: String },
    Task { id: i64, title: String },
}

pub struct InlineApp {
    pub tab: InlineTab,
    pub today: NaiveDate,
    pub current_date: NaiveDate,
    pub selected_idx: usize,
    pub query: String,
    pub day_events: Vec<EventOccurrence>,
    pub day_tasks: Vec<Task>,
    pub week_events: Vec<EventOccurrence>,
    pub all_search_events: Vec<EventOccurrence>,
    pub search_results: Vec<EventOccurrence>,
    pub pending_delete: Option<PendingDelete>,
    pub theme: crate::ui::Theme,
}

impl InlineApp {
    pub fn new(today: NaiveDate, initial_tab: InlineTab) -> Self {
        Self {
            tab: initial_tab,
            today,
            current_date: today,
            selected_idx: 0,
            query: String::new(),
            day_events: Vec::new(),
            day_tasks: Vec::new(),
            week_events: Vec::new(),
            all_search_events: Vec::new(),
            search_results: Vec::new(),
            pending_delete: None,
            theme: crate::ui::Theme::default(),
        }
    }

    pub fn with_theme(mut self, theme: crate::ui::Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle();
    }

    pub fn reload_all(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        self.reload_day(db)?;
        self.reload_week(db)?;

        // Search across 1 year range by default
        let search_start = self.today - Duration::days(90);
        let search_end = self.today + Duration::days(365);
        self.all_search_events = db.events_between(search_start, search_end)?;
        self.reload_search();

        Ok(())
    }

    pub fn reload_day(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        self.day_events = db.events_between(self.current_date, self.current_date)?;
        self.day_tasks = db.tasks_on_date(self.current_date)?;
        self.clamp_selection();
        Ok(())
    }

    pub fn reload_week(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        let weekday_offset = self.current_date.weekday().num_days_from_monday() as i64;
        let monday = self.current_date - Duration::days(weekday_offset);
        let sunday = monday + Duration::days(6);
        self.week_events = db.events_between(monday, sunday)?;
        self.clamp_selection();
        Ok(())
    }

    pub fn reload_search(&mut self) {
        let matched = filter_events(&self.all_search_events, &self.query);
        self.search_results = matched.into_iter().cloned().collect();
        self.clamp_selection();
    }

    pub fn next_day(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        self.pending_delete = None;
        self.current_date += Duration::days(1);
        self.selected_idx = 0;
        self.reload_day(db)?;
        self.reload_week(db)?;
        Ok(())
    }

    pub fn prev_day(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        self.pending_delete = None;
        self.current_date -= Duration::days(1);
        self.selected_idx = 0;
        self.reload_day(db)?;
        self.reload_week(db)?;
        Ok(())
    }

    pub fn jump_to_today(&mut self, db: &Database) -> Result<(), Box<dyn Error>> {
        self.pending_delete = None;
        self.current_date = self.today;
        self.selected_idx = 0;
        self.reload_day(db)?;
        self.reload_week(db)?;
        Ok(())
    }

    pub fn current_items_count(&self) -> usize {
        match self.tab {
            InlineTab::Day => self.day_events.len() + self.day_tasks.len(),
            InlineTab::Week => self.week_events.len(),
            InlineTab::Search => self.search_results.len(),
        }
    }

    pub fn clamp_selection(&mut self) {
        let count = self.current_items_count();
        if count == 0 {
            self.selected_idx = 0;
        } else if self.selected_idx >= count {
            self.selected_idx = count - 1;
        }
    }

    pub fn select_next(&mut self) {
        let count = self.current_items_count();
        if count > 0 && self.selected_idx + 1 < count {
            self.selected_idx += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    pub fn switch_tab(&mut self, tab: InlineTab) {
        self.pending_delete = None;
        self.tab = tab;
        self.selected_idx = 0;
    }

    pub fn cycle_tab(&mut self) {
        self.switch_tab(self.tab.next());
    }

    pub fn cycle_tab_prev(&mut self) {
        self.switch_tab(self.tab.prev());
    }

    pub fn selected_day_item(&self) -> Option<SelectedDayItem<'_>> {
        if self.tab != InlineTab::Day {
            return None;
        }
        if self.selected_idx < self.day_events.len() {
            Some(SelectedDayItem::Event(&self.day_events[self.selected_idx]))
        } else {
            let task_idx = self.selected_idx - self.day_events.len();
            self.day_tasks.get(task_idx).map(SelectedDayItem::Task)
        }
    }

    pub fn selected_event(&self) -> Option<&EventOccurrence> {
        match self.tab {
            InlineTab::Day => {
                if self.selected_idx < self.day_events.len() {
                    self.day_events.get(self.selected_idx)
                } else {
                    None
                }
            }
            InlineTab::Week => self.week_events.get(self.selected_idx),
            InlineTab::Search => self.search_results.get(self.selected_idx),
        }
    }

    pub fn is_task_selected(&self) -> bool {
        self.tab == InlineTab::Day
            && self.selected_idx >= self.day_events.len()
            && !self.day_tasks.is_empty()
    }

    pub fn request_delete(&mut self) {
        match self.tab {
            InlineTab::Day => match self.selected_day_item() {
                Some(SelectedDayItem::Event(ev)) => {
                    self.pending_delete = Some(PendingDelete::Event {
                        id: ev.event_id,
                        title: ev.title.clone(),
                    });
                }
                Some(SelectedDayItem::Task(t)) => {
                    self.pending_delete = Some(PendingDelete::Task {
                        id: t.id,
                        title: t.title.clone(),
                    });
                }
                None => {}
            },
            InlineTab::Week => {
                if let Some(ev) = self.week_events.get(self.selected_idx) {
                    self.pending_delete = Some(PendingDelete::Event {
                        id: ev.event_id,
                        title: ev.title.clone(),
                    });
                }
            }
            InlineTab::Search => {
                if let Some(ev) = self.search_results.get(self.selected_idx) {
                    self.pending_delete = Some(PendingDelete::Event {
                        id: ev.event_id,
                        title: ev.title.clone(),
                    });
                }
            }
        }
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete = None;
    }

    pub fn confirm_delete(&mut self, db: &mut Database) -> Result<bool, Box<dyn Error>> {
        let deleted = match self.pending_delete.take() {
            Some(PendingDelete::Event { id, .. }) => {
                db.delete_event(id)?;
                self.reload_all(db)?;
                true
            }
            Some(PendingDelete::Task { id, .. }) => {
                db.delete_task(id)?;
                self.reload_all(db)?;
                true
            }
            None => false,
        };
        self.clamp_selection();
        Ok(deleted)
    }

    pub fn toggle_selected_task(&mut self, db: &Database) -> Result<bool, Box<dyn Error>> {
        if self.tab != InlineTab::Day {
            return Ok(false);
        }
        if self.selected_idx >= self.day_events.len() {
            let task_idx = self.selected_idx - self.day_events.len();
            if let Some(task) = self.day_tasks.get(task_idx) {
                let new_done = db.toggle_task(task.id)?;
                self.day_tasks[task_idx].is_done = new_done;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn search_push_char(&mut self, c: char) {
        self.query.push(c);
        self.selected_idx = 0;
        self.reload_search();
    }

    pub fn search_pop_char(&mut self) {
        self.query.pop();
        self.selected_idx = 0;
        self.reload_search();
    }

    pub fn search_clear(&mut self) {
        self.query.clear();
        self.selected_idx = 0;
        self.reload_search();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Importance, NewEvent, NewTask};

    fn setup_test_db(today: NaiveDate) -> Database {
        let mut db = Database::in_memory().unwrap();

        // Create today event
        db.create_event(
            &NewEvent {
                title: "Событие Сегодня".into(),
                description: None,
                start_date: today,
                start_time: None,
                end_time: None,
                importance: Importance::Normal,
                directory: None,
            },
            None,
            &["универ".into()],
            &[],
        )
        .unwrap();

        // Create tomorrow event
        db.create_event(
            &NewEvent {
                title: "Событие Завтра".into(),
                description: None,
                start_date: today + Duration::days(1),
                start_time: None,
                end_time: None,
                importance: Importance::High,
                directory: None,
            },
            None,
            &["физика".into()],
            &[],
        )
        .unwrap();

        // Create task for today
        db.create_task(&NewTask {
            title: "Задание на сегодня".into(),
            description: None,
            date: Some(today),
            importance: Importance::Normal,
        })
        .unwrap();

        db
    }

    #[test]
    fn test_inline_app_initialization_and_day_navigation() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let db = setup_test_db(today);

        let mut app = InlineApp::new(today, InlineTab::Day);
        app.reload_all(&db).unwrap();

        assert_eq!(app.tab, InlineTab::Day);
        assert_eq!(app.current_date, today);
        assert_eq!(app.day_events.len(), 1);
        assert_eq!(app.day_tasks.len(), 1);
        assert_eq!(app.current_items_count(), 2);

        // First item is event
        assert!(matches!(
            app.selected_day_item(),
            Some(SelectedDayItem::Event(_))
        ));

        // Move to next item (task)
        app.select_next();
        assert_eq!(app.selected_idx, 1);
        assert!(matches!(
            app.selected_day_item(),
            Some(SelectedDayItem::Task(_))
        ));

        // Toggle task
        let toggled = app.toggle_selected_task(&db).unwrap();
        assert!(toggled);
        assert!(app.day_tasks[0].is_done);

        // Move to tomorrow
        app.next_day(&db).unwrap();
        assert_eq!(app.current_date, today + Duration::days(1));
        assert_eq!(app.day_events.len(), 1);
        assert_eq!(app.day_events[0].title, "Событие Завтра");
        assert_eq!(app.day_tasks.len(), 0);

        // Jump back to today
        app.jump_to_today(&db).unwrap();
        assert_eq!(app.current_date, today);
        assert_eq!(app.day_events[0].title, "Событие Сегодня");
    }

    #[test]
    fn test_inline_app_tabs_and_search() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let db = setup_test_db(today);

        let mut app = InlineApp::new(today, InlineTab::Day);
        app.reload_all(&db).unwrap();

        // Cycle tabs
        app.cycle_tab();
        assert_eq!(app.tab, InlineTab::Week);
        assert_eq!(app.week_events.len(), 2);

        app.cycle_tab();
        assert_eq!(app.tab, InlineTab::Search);

        // Search query
        app.search_push_char('ф');
        app.search_push_char('и');
        app.search_push_char('з');
        assert_eq!(app.search_results.len(), 1);
        assert_eq!(app.search_results[0].title, "Событие Завтра");

        app.search_clear();
        assert_eq!(app.search_results.len(), 2);
    }

    #[test]
    fn test_inline_app_deletion() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let mut db = setup_test_db(today);

        let mut app = InlineApp::new(today, InlineTab::Day);
        app.reload_all(&db).unwrap();

        // 1. Delete event with cancel
        assert_eq!(app.selected_idx, 0);
        app.request_delete();
        assert!(matches!(
            app.pending_delete,
            Some(PendingDelete::Event { .. })
        ));

        app.cancel_delete();
        assert!(app.pending_delete.is_none());
        assert_eq!(app.day_events.len(), 1);

        // 2. Delete event with confirm
        app.request_delete();
        let deleted = app.confirm_delete(&mut db).unwrap();
        assert!(deleted);
        assert_eq!(app.day_events.len(), 0);
        assert_eq!(app.day_tasks.len(), 1);

        // 3. Delete task with confirm
        assert_eq!(app.selected_idx, 0); // now task is at 0
        app.request_delete();
        assert!(matches!(
            app.pending_delete,
            Some(PendingDelete::Task { .. })
        ));

        let deleted_task = app.confirm_delete(&mut db).unwrap();
        assert!(deleted_task);
        assert_eq!(app.day_tasks.len(), 0);
    }

    #[test]
    fn test_inline_app_theme_cycling() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let mut app = InlineApp::new(today, InlineTab::Day).with_theme(crate::ui::Theme::Neo);
        assert_eq!(app.theme, crate::ui::Theme::Neo);

        app.cycle_theme();
        assert_eq!(app.theme, crate::ui::Theme::Light);

        app.cycle_theme();
        assert_eq!(app.theme, crate::ui::Theme::Plain);

        app.cycle_theme();
        assert_eq!(app.theme, crate::ui::Theme::Neo);
    }
}
