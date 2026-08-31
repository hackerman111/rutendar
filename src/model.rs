use std::fmt;

use chrono::{NaiveDate, NaiveTime, Weekday};

pub type EventId = i64;
pub type RecurrenceId = i64;
pub type NoteId = i64;
pub type LinkId = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Importance {
    None,
    Low,
    Normal,
    High,
}

impl Importance {
    pub fn from_db(value: i64) -> rusqlite::Result<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Low),
            2 => Ok(Self::Normal),
            3 => Ok(Self::High),
            _ => Err(rusqlite::Error::IntegralValueOutOfRange(0, value)),
        }
    }

    pub const fn to_db(self) -> i64 {
        self as i64
    }

    pub const fn next(self) -> Self {
        match self {
            Self::None => Self::Low,
            Self::Low => Self::Normal,
            Self::Normal => Self::High,
            Self::High => Self::None,
        }
    }
}

impl fmt::Display for Importance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "None",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpcomingOrder {
    #[default]
    Time,
    Importance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: EventId,
    pub title: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub importance: Importance,
    pub recurrence_id: Option<RecurrenceId>,
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub title: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub importance: Importance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frequency {
    Weekly,
}

#[derive(Debug, Clone)]
pub struct Recurrence {
    pub id: RecurrenceId,
    pub frequency: Frequency,
    pub interval: u32,
    pub weekdays: Vec<Weekday>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NewRecurrence {
    pub interval: u32,
    pub weekdays: Vec<Weekday>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionKind {
    Cancelled,
    Modified,
}

#[derive(Debug, Clone)]
pub struct RecurrenceException {
    pub recurrence_id: RecurrenceId,
    pub original_date: NaiveDate,
    pub kind: ExceptionKind,
    pub replacement_event_id: Option<EventId>,
}

#[derive(Debug, Clone)]
pub struct EventOccurrence {
    pub event_id: EventId,
    pub recurrence_id: Option<RecurrenceId>,
    pub original_date: NaiveDate,
    pub date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub title: String,
    pub description: Option<String>,
    pub importance: Importance,
    pub tags: Vec<Tag>,
    pub is_recurring: bool,
}

impl EventOccurrence {
    pub fn from_event(event: &Event, tags: Vec<Tag>) -> Self {
        Self {
            event_id: event.id,
            recurrence_id: event.recurrence_id,
            original_date: event.start_date,
            date: event.start_date,
            start_time: event.start_time,
            end_time: event.end_time,
            title: event.title.clone(),
            description: event.description.clone(),
            importance: event.importance,
            tags,
            is_recurring: event.recurrence_id.is_some(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: NoteId,
    pub date: NaiveDate,
    pub title: Option<String>,
    pub body: String,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct NewNote {
    pub date: NaiveDate,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub id: LinkId,
    pub note_id: NoteId,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct NewLink {
    pub note_id: NoteId,
    pub label: String,
    pub url: String,
}

pub fn normalize_tag(value: &str) -> String {
    value.trim().trim_start_matches('#').to_lowercase()
}

pub fn parse_date(value: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(value.trim(), "%d.%m.%Y")
        .or_else(|_| NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d"))
}

pub fn parse_time(value: &str) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization_is_case_and_space_insensitive() {
        assert_eq!(normalize_tag("ML"), "ml");
        assert_eq!(normalize_tag(" ml "), "ml");
        assert_eq!(normalize_tag("#Ml"), "ml");
    }

    #[test]
    fn date_accepts_ui_and_iso_formats() {
        let expected = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(parse_date("01.09.2026").unwrap(), expected);
        assert_eq!(parse_date("2026-09-01").unwrap(), expected);
    }
}
