use chrono::NaiveDate;

use crate::model::Importance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub date: Option<NaiveDate>,
    pub is_done: bool,
    pub importance: Importance,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<NaiveDate>,
    pub importance: Importance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskFilter {
    #[default]
    Active,
    All,
    Done,
}
