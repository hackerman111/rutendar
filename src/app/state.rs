use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};

use super::AppResult;
use crate::{
    model::{
        Event, EventId, EventOccurrence, FavoriteLink, FavoriteLinkId, Importance, Link, LinkId,
        NewEvent, NewFavoriteLink, NewRecurrence, Note, NoteId, Recurrence, RecurrenceId, Tag,
        parse_date, parse_time,
    },
    search::{SearchFilters, SearchResult},
};

pub use crate::model::UpcomingOrder as UpcomingSort;

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

    pub const fn next(self) -> Self {
        match self {
            Self::Week => Self::Day,
            Self::Day => Self::Month,
            Self::Month => Self::Year,
            Self::Year => Self::Week,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Week => Self::Year,
            Self::Day => Self::Week,
            Self::Month => Self::Day,
            Self::Year => Self::Month,
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
    LinkBank,
    LinkSearch,
    Confirm,
    Scope,
    GotoDate,
}

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

#[derive(Debug, Clone)]
pub struct EventForm {
    pub title: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub importance: Importance,
    pub tags: String,
    pub weekly: bool,
    pub interval: String,
    pub weekdays: String,
    pub ends: String,
    pub description: String,
    pub favorite_link_ids: Vec<FavoriteLinkId>,
    pub favorite_links: String,
    pub directory: String,
    pub active: usize,
}

pub type EventFormValues = (
    NewEvent,
    Option<NewRecurrence>,
    Vec<String>,
    Vec<FavoriteLinkId>,
);

impl EventForm {
    pub const FIELD_COUNT: usize = 13;
    pub const IMPORTANCE_FIELD: usize = 4;
    pub const TAGS_FIELD: usize = 5;
    pub const REPEAT_FIELD: usize = 6;
    pub const LINKS_FIELD: usize = 11;
    pub const DIRECTORY_FIELD: usize = 12;

    pub fn new(date: NaiveDate) -> Self {
        Self {
            title: String::new(),
            date: date.format("%d.%m.%Y").to_string(),
            start_time: String::new(),
            end_time: String::new(),
            importance: Importance::Normal,
            tags: String::new(),
            weekly: false,
            interval: "1".into(),
            weekdays: weekday_name(date.weekday()).into(),
            ends: String::new(),
            description: String::new(),
            favorite_link_ids: Vec::new(),
            favorite_links: String::new(),
            directory: String::new(),
            active: 0,
        }
    }

    pub fn from_event(
        event: &Event,
        tags: &[Tag],
        favorite_links: &[FavoriteLink],
        recurrence: Option<&Recurrence>,
    ) -> Self {
        let mut form = Self {
            title: event.title.clone(),
            date: event.start_date.format("%d.%m.%Y").to_string(),
            start_time: event
                .start_time
                .map(|time| time.format("%H:%M").to_string())
                .unwrap_or_default(),
            end_time: event
                .end_time
                .map(|time| time.format("%H:%M").to_string())
                .unwrap_or_default(),
            importance: event.importance,
            tags: tags
                .iter()
                .map(|tag| format!("#{}", tag.name))
                .collect::<Vec<_>>()
                .join(" "),
            weekly: recurrence.is_some(),
            interval: recurrence.map_or_else(|| "1".into(), |rule| rule.interval.to_string()),
            weekdays: recurrence.map_or_else(
                || weekday_name(event.start_date.weekday()).into(),
                |rule| {
                    rule.weekdays
                        .iter()
                        .map(|day| weekday_name(*day))
                        .collect::<Vec<_>>()
                        .join(",")
                },
            ),
            ends: recurrence
                .and_then(|rule| rule.end_date)
                .map(|date| date.format("%d.%m.%Y").to_string())
                .unwrap_or_default(),
            description: event.description.clone().unwrap_or_default(),
            favorite_link_ids: Vec::new(),
            favorite_links: String::new(),
            directory: event
                .directory
                .as_deref()
                .and_then(std::path::Path::to_str)
                .unwrap_or_default()
                .to_owned(),
            active: 0,
        };
        form.set_favorite_links(favorite_links);
        form
    }

    pub fn from_occurrence(event: &EventOccurrence) -> Self {
        let synthetic = Event {
            id: event.event_id,
            title: event.title.clone(),
            description: event.description.clone(),
            start_date: event.date,
            start_time: event.start_time,
            end_time: event.end_time,
            importance: event.importance,
            recurrence_id: None,
            directory: event.directory.clone(),
        };
        let mut form = Self::from_event(&synthetic, &event.tags, &event.favorite_links, None);
        form.weekly = false;
        form
    }

    pub fn fields(&self) -> [(&'static str, &str); Self::FIELD_COUNT] {
        [
            ("TITLE", &self.title),
            ("DATE", &self.date),
            ("TIME", &self.start_time),
            ("END TIME", &self.end_time),
            ("IMPORTANCE", self.importance.as_str()),
            ("TAGS", &self.tags),
            ("REPEAT", if self.weekly { "Weekly" } else { "Never" }),
            ("INTERVAL", &self.interval),
            ("WEEKDAYS", &self.weekdays),
            ("ENDS", &self.ends),
            ("DESCRIPTION", &self.description),
            ("LINKS", &self.favorite_links),
            ("DIRECTORY", &self.directory),
        ]
    }

    pub fn push(&mut self, character: char) {
        match self.active {
            0 => self.title.push(character),
            1 => self.date.push(character),
            2 => self.start_time.push(character),
            3 => self.end_time.push(character),
            5 => self.tags.push(character),
            7 => self.interval.push(character),
            8 => self.weekdays.push(character),
            9 => self.ends.push(character),
            10 => self.description.push(character),
            Self::DIRECTORY_FIELD => self.directory.push(character),
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.active {
            0 => _ = self.title.pop(),
            1 => _ = self.date.pop(),
            2 => _ = self.start_time.pop(),
            3 => _ = self.end_time.pop(),
            5 => _ = self.tags.pop(),
            7 => _ = self.interval.pop(),
            8 => _ = self.weekdays.pop(),
            9 => _ = self.ends.pop(),
            10 => _ = self.description.pop(),
            Self::DIRECTORY_FIELD => _ = self.directory.pop(),
            _ => {}
        }
    }

    pub fn adjust(&mut self, forward: bool) {
        match self.active {
            Self::IMPORTANCE_FIELD => {
                self.importance = if forward {
                    self.importance.next()
                } else {
                    match self.importance {
                        Importance::None => Importance::High,
                        Importance::Low => Importance::None,
                        Importance::Normal => Importance::Low,
                        Importance::High => Importance::Normal,
                    }
                }
            }
            Self::REPEAT_FIELD => self.weekly = !self.weekly,
            _ => {}
        }
    }

    pub fn values(&self) -> AppResult<EventFormValues> {
        let date = parse_date(&self.date)?;
        let start_time = optional_time(&self.start_time)?;
        let end_time = optional_time(&self.end_time)?;
        let directory = if self.directory.trim().is_empty() {
            None
        } else {
            let directory = std::fs::canonicalize(self.directory.trim())?;
            if !directory.is_dir() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "event directory is not a directory",
                )
                .into());
            }
            Some(directory)
        };
        let event = NewEvent {
            title: self.title.trim().into(),
            description: (!self.description.trim().is_empty())
                .then(|| self.description.trim().into()),
            start_date: date,
            start_time,
            end_time,
            importance: self.importance,
            directory,
        };
        let recurrence = if self.weekly {
            let weekdays = parse_weekdays(&self.weekdays)?;
            Some(NewRecurrence {
                interval: self.interval.trim().parse()?,
                weekdays,
                start_date: date,
                end_date: (!self.ends.trim().is_empty())
                    .then(|| parse_date(&self.ends))
                    .transpose()?,
                count: None,
            })
        } else {
            None
        };
        let tags = self
            .tags
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|tag| !tag.trim_matches('#').is_empty())
            .map(str::to_owned)
            .collect();
        Ok((event, recurrence, tags, self.favorite_link_ids.clone()))
    }

    pub fn set_favorite_links(&mut self, links: &[FavoriteLink]) {
        self.favorite_link_ids = links.iter().map(|link| link.id).collect();
        self.favorite_links = links
            .iter()
            .map(|link| link.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
    }
}

#[derive(Debug, Clone)]
pub struct NoteForm {
    pub title: String,
    pub date: String,
    pub body: String,
    pub active: usize,
}

#[derive(Debug, Clone)]
pub struct LinkForm {
    pub label: String,
    pub url: String,
    pub note_id: NoteId,
    pub active: usize,
}

#[derive(Debug, Clone)]
pub struct FavoriteLinkForm {
    pub label: String,
    pub url: String,
    pub tags: String,
    pub description: String,
    pub active: usize,
}

impl FavoriteLinkForm {
    pub const FIELD_COUNT: usize = 4;

    pub fn from_link(link: &FavoriteLink) -> Self {
        Self {
            label: link.label.clone(),
            url: link.url.clone(),
            tags: link.tags.clone(),
            description: link.description.clone().unwrap_or_default(),
            active: 0,
        }
    }

    pub fn fields(&self) -> [(&'static str, &str); Self::FIELD_COUNT] {
        [
            ("LABEL", &self.label),
            ("URL", &self.url),
            ("TAGS", &self.tags),
            ("DESCRIPTION", &self.description),
        ]
    }

    pub fn push(&mut self, character: char) {
        match self.active {
            0 => self.label.push(character),
            1 => self.url.push(character),
            2 => self.tags.push(character),
            3 => self.description.push(character),
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.active {
            0 => _ = self.label.pop(),
            1 => _ = self.url.pop(),
            2 => _ = self.tags.pop(),
            3 => _ = self.description.pop(),
            _ => {}
        }
    }

    pub fn values(&self) -> NewFavoriteLink {
        NewFavoriteLink {
            label: self.label.clone(),
            url: self.url.clone(),
            description: (!self.description.trim().is_empty())
                .then(|| self.description.trim().to_owned()),
            tags: self.tags.clone(),
        }
    }
}

impl Default for FavoriteLinkForm {
    fn default() -> Self {
        Self {
            label: String::new(),
            url: "https://".into(),
            tags: String::new(),
            description: String::new(),
            active: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Editor {
    Event {
        form: EventForm,
        target: EventTarget,
    },
    Note {
        form: NoteForm,
        target: Option<NoteId>,
    },
    Link {
        form: LinkForm,
        target: Option<LinkId>,
    },
    FavoriteLink {
        form: FavoriteLinkForm,
        target: Option<FavoriteLinkId>,
    },
}

#[derive(Debug, Clone)]
pub enum EventTarget {
    New,
    Event(EventId),
    Occurrence {
        recurrence_id: RecurrenceId,
        original_date: NaiveDate,
    },
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Event(EventId),
    Recurrence(RecurrenceId),
    Occurrence(RecurrenceId, NaiveDate),
    Note(NoteId),
    Link(LinkId),
    Tag(i64),
}

#[derive(Debug, Clone)]
pub enum ScopeOperation {
    Edit(EventOccurrence),
    Delete(EventOccurrence),
    Importance(EventOccurrence),
}

#[derive(Debug, Clone)]
pub enum Popup {
    Editor(Editor),
    SaveConfirm {
        message: String,
        editor: Editor,
    },
    Confirm {
        message: String,
        target: DeleteTarget,
    },
    Scope(ScopeOperation),
    GotoDate(String),
    LinkBank,
    Help,
}

#[derive(Debug)]
pub struct LinkBankState {
    pub event_form: EventForm,
    pub event_target: EventTarget,
    pub query: String,
    pub items: Vec<FavoriteLink>,
    pub selected: usize,
    pub searching: bool,
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
    pub link_bank: Option<LinkBankState>,
    pub status_message: Option<String>,
    pub loaded_range: Option<(NaiveDate, NaiveDate)>,
}

fn optional_time(value: &str) -> AppResult<Option<NaiveTime>> {
    (!value.trim().is_empty())
        .then(|| parse_time(value))
        .transpose()
        .map_err(Into::into)
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "ПН",
        Weekday::Tue => "ВТ",
        Weekday::Wed => "СР",
        Weekday::Thu => "ЧТ",
        Weekday::Fri => "ПТ",
        Weekday::Sat => "СБ",
        Weekday::Sun => "ВС",
    }
}

fn parse_weekdays(value: &str) -> AppResult<Vec<Weekday>> {
    let mut days = Vec::new();
    for token in value.split(|character: char| character.is_whitespace() || character == ',') {
        if token.is_empty() {
            continue;
        }
        let day = match token.to_uppercase().as_str() {
            "ПН" | "MON" | "1" => Weekday::Mon,
            "ВТ" | "TUE" | "2" => Weekday::Tue,
            "СР" | "WED" | "3" => Weekday::Wed,
            "ЧТ" | "THU" | "4" => Weekday::Thu,
            "ПТ" | "FRI" | "5" => Weekday::Fri,
            "СБ" | "SAT" | "6" => Weekday::Sat,
            "ВС" | "SUN" | "7" => Weekday::Sun,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid weekday",
                )
                .into());
            }
        };
        if !days.contains(&day) {
            days.push(day);
        }
    }
    if days.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "at least one weekday is required",
        )
        .into());
    }
    Ok(days)
}
