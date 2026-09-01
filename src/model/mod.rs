pub mod event;
pub mod favorite_link;
pub mod link;
pub mod note;
pub mod occurrence;
pub mod tag;
pub mod task;

use chrono::{NaiveDate, NaiveTime};

pub use event::{Event, EventId, Importance, NewEvent, UpcomingOrder};
pub use favorite_link::{FavoriteLink, FavoriteLinkId, NewFavoriteLink};
pub use link::{Link, LinkId, NewLink};
pub use note::{NewNote, Note, NoteId};
pub use occurrence::EventOccurrence;
pub use tag::{Tag, normalize_tag};
pub use task::{NewTask, Task, TaskFilter};

// Re-export Recurrence types for convenience where model is imported
pub use crate::recurrence::{
    ExceptionKind, Frequency, NewRecurrence, Recurrence, RecurrenceException, RecurrenceId,
};

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
    fn date_accepts_ui_and_iso_formats() {
        let expected = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(parse_date("01.09.2026").unwrap(), expected);
        assert_eq!(parse_date("2026-09-01").unwrap(), expected);
    }
}
