use std::path::PathBuf;

use chrono::{NaiveDate, NaiveTime};

use super::{
    event::{Event, EventId, Importance},
    favorite_link::FavoriteLink,
    tag::Tag,
};
use crate::recurrence::RecurrenceId;

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
    pub favorite_links: Vec<FavoriteLink>,
    pub directory: Option<PathBuf>,
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
            favorite_links: Vec::new(),
            directory: event.directory.clone(),
            is_recurring: event.recurrence_id.is_some(),
        }
    }
}
