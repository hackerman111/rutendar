use std::{collections::HashMap, error::Error};

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Weekday};

use crate::{
    calendar::{month_end, month_start, move_month, week_end, week_start, year_end, year_start},
    config::Config,
    external,
    model::{
        Event, EventId, EventOccurrence, Importance, Link, LinkId, NewEvent, NewLink, NewNote,
        NewRecurrence, Note, NoteId, RecurrenceId, Tag, parse_date, parse_time,
    },
    search::{DateFilter, ItemType, SearchFilters, SearchResult, SortBy, TagMatching},
    storage::Database,
};

pub type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Week,
    Day,
    Month,
    Year,
}

impl View {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Week => "WEEK",
            Self::Day => "DAY",
            Self::Month => "MONTH",
            Self::Year => "YEAR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Events,
    Notes,
    Links,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Agenda,
    Upcoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editor,
    Search,
    Confirm,
    Scope,
    GotoDate,
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Open,
    Back,
    Create,
    Edit,
    Delete,
    ChangeImportance,
    OpenAgenda,
    OpenUpcoming,
    SwitchView(View),
    GoToToday,
    StartGotoDate,
    Help,
    OpenLink,
    CopyLink,
    ToggleFocus,
    StartSearch,
    Input(char),
    Backspace,
    NextField,
    PreviousField,
    AdjustLeft,
    AdjustRight,
    Submit,
    Confirm(bool),
    ChooseOccurrence,
    ChooseSeries,
    CycleDateFilter,
    CycleItemType,
    CycleImportanceFilter,
    CycleSort,
    ToggleTagMatching,
    PreviousTagFilter,
    NextTagFilter,
    ToggleTagFilter,
    Noop,
}

mod form;
mod update;

pub use crate::model::UpcomingOrder as UpcomingSort;
pub use form::{
    DeleteTarget, Editor, EventForm, EventTarget, LinkForm, NoteForm, Popup, ScopeOperation,
};

#[derive(Debug, Default)]
pub struct AgendaState {
    pub query: String,
    pub filters: SearchFilters,
    pub items: Vec<SearchResult>,
    pub selected: usize,
    pub searching: bool,
    pub available_tags: Vec<Tag>,
    pub tag_cursor: usize,
}

#[derive(Debug, Default)]
pub struct UpcomingState {
    pub items: Vec<EventOccurrence>,
    pub selected: usize,
    pub sort: UpcomingSort,
    pub links_by_date: HashMap<NaiveDate, Vec<Link>>,
}

#[derive(Debug)]
pub struct AppState {
    pub today: NaiveDate,
    pub selected_date: NaiveDate,
    pub active_view: View,
    pub focused_pane: FocusedPane,
    pub selected_event: usize,
    pub selected_note: usize,
    pub selected_link: usize,
    pub overlay: Option<Overlay>,
    pub popup: Option<Popup>,
    pub input_mode: InputMode,
    pub agenda: AgendaState,
    pub upcoming: UpcomingState,
    pub occurrences: Vec<EventOccurrence>,
    pub notes: Vec<Note>,
    pub next: Vec<EventOccurrence>,
    pub next_total: usize,
    pub tag_suggestions: Vec<Tag>,
    pub status_message: Option<String>,
    loaded_range: Option<(NaiveDate, NaiveDate)>,
}

pub struct App {
    pub state: AppState,
    pub config: Config,
    database: Database,
    last_clock_minute: i64,
}

impl App {
    pub fn new(database: Database, config: Config) -> AppResult<Self> {
        let now = Local::now();
        let today = now.date_naive();
        let mut app = Self {
            state: AppState {
                today,
                selected_date: today,
                active_view: View::Week,
                focused_pane: FocusedPane::Events,
                selected_event: 0,
                selected_note: 0,
                selected_link: 0,
                overlay: None,
                popup: None,
                input_mode: InputMode::Normal,
                agenda: AgendaState::default(),
                upcoming: UpcomingState::default(),
                occurrences: Vec::new(),
                notes: Vec::new(),
                next: Vec::new(),
                next_total: 0,
                tag_suggestions: Vec::new(),
                status_message: None,
                loaded_range: None,
            },
            config,
            database,
            last_clock_minute: now.timestamp() / 60,
        };
        app.refresh_calendar()?;
        app.refresh_upcoming()?;
        Ok(app)
    }

    pub fn tick(&mut self) {
        let now = Local::now();
        let minute = now.timestamp() / 60;
        if now.date_naive() != self.state.today {
            self.state.today = now.date_naive();
            self.state.loaded_range = None;
            if let Err(error) = self.refresh_calendar() {
                self.set_error(error);
            }
        }
        if minute != self.last_clock_minute {
            self.last_clock_minute = minute;
            if let Err(error) = self.refresh_upcoming() {
                self.set_error(error);
            }
        }
    }

    pub fn input_mode(&self) -> InputMode {
        self.state.input_mode
    }

    fn view_range(&self) -> (NaiveDate, NaiveDate) {
        match self.state.active_view {
            View::Week => (
                week_start(self.state.selected_date),
                week_end(self.state.selected_date),
            ),
            View::Day => (self.state.selected_date, self.state.selected_date),
            View::Month => (
                month_start(self.state.selected_date),
                month_end(self.state.selected_date),
            ),
            View::Year => (
                year_start(self.state.selected_date),
                year_end(self.state.selected_date),
            ),
        }
    }

    fn refresh_calendar(&mut self) -> AppResult<()> {
        let range = self.view_range();
        if self.state.loaded_range != Some(range) {
            self.state.occurrences = self.database.events_between(range.0, range.1)?;
            self.state.notes = self.database.notes_between(range.0, range.1)?;
            self.state.loaded_range = Some(range);
        }
        self.clamp_selections();
        Ok(())
    }

    fn refresh_upcoming(&mut self) -> AppResult<()> {
        let now = Local::now();
        let time_page = self.database.upcoming_events(
            now,
            200.max(self.config.agenda.next_events),
            UpcomingSort::Time,
        )?;
        self.state.next_total = time_page.total;
        self.state.next = time_page
            .items
            .iter()
            .take(self.config.agenda.next_events)
            .cloned()
            .collect();
        self.state.upcoming.items = if self.state.upcoming.sort == UpcomingSort::Importance {
            self.database
                .upcoming_events(now, 200, UpcomingSort::Importance)?
                .items
        } else {
            time_page.items.into_iter().take(200).collect()
        };
        let mut dates: Vec<_> = self
            .state
            .upcoming
            .items
            .iter()
            .map(|event| event.date)
            .collect();
        dates.sort_unstable();
        dates.dedup();
        self.state.upcoming.links_by_date = self.database.links_on_dates(&dates)?;
        self.state.upcoming.selected = self
            .state
            .upcoming
            .selected
            .min(self.state.upcoming.items.len().saturating_sub(1));
        let link_count = self
            .state
            .upcoming
            .items
            .get(self.state.upcoming.selected)
            .and_then(|event| self.state.upcoming.links_by_date.get(&event.date))
            .map_or(0, Vec::len);
        self.state.selected_link = self.state.selected_link.min(link_count.saturating_sub(1));
        Ok(())
    }

    fn refresh_agenda(&mut self) -> AppResult<()> {
        self.state.agenda.available_tags = self.database.search_tags("", 1_000)?;
        self.state.agenda.tag_cursor = self
            .state
            .agenda
            .tag_cursor
            .min(self.state.agenda.available_tags.len().saturating_sub(1));
        self.state.agenda.items = self.database.search(
            &self.state.agenda.query,
            &self.state.agenda.filters,
            self.state.today,
        )?;
        self.state.agenda.selected = self
            .state
            .agenda
            .selected
            .min(self.state.agenda.items.len().saturating_sub(1));
        Ok(())
    }

    fn refresh_after_change(&mut self) -> AppResult<()> {
        self.state.loaded_range = None;
        self.refresh_calendar()?;
        self.refresh_upcoming()?;
        if self.state.overlay == Some(Overlay::Agenda) {
            self.refresh_agenda()?;
        }
        Ok(())
    }

    fn clamp_selections(&mut self) {
        let event_count = self.events_on_selected_date().count();
        let note_count = self.notes_on_selected_date().count();
        self.state.selected_event = self.state.selected_event.min(event_count.saturating_sub(1));
        self.state.selected_note = self.state.selected_note.min(note_count.saturating_sub(1));
        let link_count = self.selected_note().map_or(0, |note| note.links.len());
        self.state.selected_link = self.state.selected_link.min(link_count.saturating_sub(1));
    }

    pub fn events_on_selected_date(&self) -> impl Iterator<Item = &EventOccurrence> {
        self.state
            .occurrences
            .iter()
            .filter(|event| event.date == self.state.selected_date)
    }

    pub fn notes_on_selected_date(&self) -> impl Iterator<Item = &Note> {
        self.state
            .notes
            .iter()
            .filter(|note| note.date == self.state.selected_date)
    }

    pub fn selected_note(&self) -> Option<&Note> {
        self.notes_on_selected_date().nth(self.state.selected_note)
    }

    pub fn selected_link(&self) -> Option<&Link> {
        match self.state.overlay {
            Some(Overlay::Upcoming) => self
                .state
                .upcoming
                .items
                .get(self.state.upcoming.selected)
                .and_then(|event| self.state.upcoming.links_by_date.get(&event.date))
                .and_then(|links| links.get(self.state.selected_link)),
            Some(Overlay::Agenda) => {
                match self.state.agenda.items.get(self.state.agenda.selected) {
                    Some(SearchResult::Note(note)) => note.links.first(),
                    _ => None,
                }
            }
            None => self
                .selected_note()
                .and_then(|note| note.links.get(self.state.selected_link)),
        }
    }
}
