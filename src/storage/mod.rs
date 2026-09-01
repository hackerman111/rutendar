use std::error::Error;

use crate::model::EventOccurrence;

pub mod database;
pub mod events;
pub mod favorite_links;
pub mod migrations;
pub mod notes;
pub mod tags;
pub mod tasks;

pub use database::Database;

pub type StorageResult<T> = Result<T, Box<dyn Error>>;

pub struct UpcomingEvents {
    pub items: Vec<EventOccurrence>,
    pub total: usize,
}
