use chrono::NaiveDate;

use super::link::Link;

pub type NoteId = i64;

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
