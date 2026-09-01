use chrono::NaiveDate;

use super::rule::RecurrenceId;
use crate::model::EventId;

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
