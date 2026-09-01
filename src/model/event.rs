use std::fmt;

use chrono::{NaiveDate, NaiveTime};

use crate::recurrence::RecurrenceId;

pub type EventId = i64;

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
