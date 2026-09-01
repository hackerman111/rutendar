use chrono::NaiveDate;

use crate::model::{EventOccurrence, Note};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemType {
    #[default]
    All,
    Events,
    Notes,
    Recurring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFilter {
    #[default]
    All,
    Today,
    ThisWeek,
    ThisMonth,
    Upcoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagMatching {
    #[default]
    All,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Date,
    Importance,
    Title,
}

#[derive(Debug, Clone)]
pub enum SearchResult {
    Event(EventOccurrence),
    Note(Note),
}

impl SearchResult {
    pub fn date(&self) -> NaiveDate {
        match self {
            Self::Event(item) => item.date,
            Self::Note(item) => item.date,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Event(item) => &item.title,
            Self::Note(item) => item.title.as_deref().unwrap_or("Без названия"),
        }
    }
}
