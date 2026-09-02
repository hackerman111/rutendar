use chrono::{NaiveDate, Weekday};

pub type RecurrenceId = i64;

pub const MAX_INTERVAL_WEEKS: u32 = 5_200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frequency {
    Weekly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recurrence {
    pub id: RecurrenceId,
    pub frequency: Frequency,
    pub interval: u32,
    pub weekdays: Vec<Weekday>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRecurrence {
    pub interval: u32,
    pub weekdays: Vec<Weekday>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub count: Option<u32>,
}
