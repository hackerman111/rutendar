pub mod format;
pub mod parse;

use std::path::PathBuf;

use chrono::{NaiveDate, NaiveTime};

use crate::model::{Importance, NewEvent, NewRecurrence, NewTask};

pub use format::format_ics;
pub use parse::{IcsParseError, parse_ics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcsEvent {
    pub title: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub importance: Importance,
    pub tags: Vec<String>,
    pub recurrence: Option<NewRecurrence>,
    pub link: Option<String>,
    pub directory: Option<PathBuf>,
}

impl IcsEvent {
    pub fn to_new_event(&self) -> NewEvent {
        NewEvent {
            title: self.title.clone(),
            description: self.description.clone(),
            start_date: self.start_date,
            start_time: self.start_time,
            end_time: self.end_time,
            importance: self.importance,
            directory: self.directory.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcsTask {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<NaiveDate>,
    pub is_done: bool,
    pub importance: Importance,
}

impl IcsTask {
    pub fn to_new_task(&self) -> NewTask {
        NewTask {
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            importance: self.importance,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IcsCalendar {
    pub events: Vec<IcsEvent>,
    pub tasks: Vec<IcsTask>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportStats {
    pub events_imported: usize,
    pub tasks_imported: usize,
    pub skipped: usize,
}
