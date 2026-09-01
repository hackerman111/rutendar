use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};

use super::{
    Database, StorageResult,
    database::{date_string, invalid_input, now_string, optional_date_from_row},
};
use crate::model::{Importance, NewTask, Task, TaskFilter};

impl Database {
    pub fn create_task(&self, task: &NewTask) -> StorageResult<i64> {
        let clean_title = task.title.trim();
        if clean_title.is_empty() {
            return Err(invalid_input("название задания не может быть пустым"));
        }
        let now = now_string();
        self.connection.execute(
            "INSERT INTO tasks(title, description, date, is_done, importance, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?5, ?5)",
            params![
                clean_title,
                task.description.as_deref().filter(|d| !d.trim().is_empty()),
                task.date.map(date_string),
                task.importance.to_db(),
                now,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn toggle_task(&self, id: i64) -> StorageResult<bool> {
        let current_done: Option<i64> = self
            .connection
            .query_row(
                "SELECT is_done FROM tasks WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        let Some(current_done) = current_done else {
            return Err(invalid_input("задание с указанным ID не найдено"));
        };

        let new_done = if current_done == 0 { 1 } else { 0 };
        let now = now_string();
        let completed_at = if new_done == 1 {
            Some(now.clone())
        } else {
            None
        };

        self.connection.execute(
            "UPDATE tasks SET is_done = ?1, completed_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_done, completed_at, now, id],
        )?;

        Ok(new_done == 1)
    }

    pub fn delete_task(&self, id: i64) -> StorageResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(invalid_input("задание с указанным ID не найдено"));
        }
        Ok(())
    }

    pub fn get_task(&self, id: i64) -> StorageResult<Option<Task>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
             FROM tasks WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(task_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn tasks_on_date(&self, date: NaiveDate) -> StorageResult<Vec<Task>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
             FROM tasks WHERE date = ?1
             ORDER BY is_done ASC, importance DESC, id ASC",
        )?;
        let rows = stmt.query_map(params![date_string(date)], task_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn tasks_between(&self, start: NaiveDate, end: NaiveDate) -> StorageResult<Vec<Task>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
             FROM tasks WHERE date >= ?1 AND date <= ?2
             ORDER BY date ASC, is_done ASC, importance DESC, id ASC",
        )?;
        let rows = stmt.query_map(params![date_string(start), date_string(end)], task_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn all_tasks(&self, filter: TaskFilter) -> StorageResult<Vec<Task>> {
        let sql = match filter {
            TaskFilter::Active => {
                "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
                 FROM tasks WHERE is_done = 0
                 ORDER BY (date IS NULL) ASC, date ASC, importance DESC, id ASC"
            }
            TaskFilter::Done => {
                "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
                 FROM tasks WHERE is_done = 1
                 ORDER BY completed_at DESC, id DESC"
            }
            TaskFilter::All => {
                "SELECT id, title, description, date, is_done, importance, completed_at, created_at, updated_at
                 FROM tasks
                 ORDER BY is_done ASC, (date IS NULL) ASC, date ASC, importance DESC, id ASC"
            }
        };
        let mut stmt = self.connection.prepare(sql)?;
        let rows = stmt.query_map([], task_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

pub(crate) fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let raw_done: i64 = row.get(4)?;
    let raw_importance: i64 = row.get(5)?;
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        date: optional_date_from_row(row, 3)?,
        is_done: raw_done != 0,
        importance: Importance::from_db(raw_importance)?,
        completed_at: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_crud_and_toggle_completion() -> StorageResult<()> {
        let db = Database::in_memory()?;
        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();

        let task_id = db.create_task(&NewTask {
            title: "Купить билеты".into(),
            description: Some("На поезд в Питер".into()),
            date: Some(date),
            importance: Importance::High,
        })?;

        let task = db.get_task(task_id)?.expect("task must exist");
        assert_eq!(task.title, "Купить билеты");
        assert_eq!(task.description.as_deref(), Some("На поезд в Питер"));
        assert_eq!(task.date, Some(date));
        assert!(!task.is_done);
        assert_eq!(task.importance, Importance::High);
        assert!(task.completed_at.is_none());

        let tasks_today = db.tasks_on_date(date)?;
        assert_eq!(tasks_today.len(), 1);

        // Toggle to completed
        let is_now_done = db.toggle_task(task_id)?;
        assert!(is_now_done);

        let completed_task = db.get_task(task_id)?.unwrap();
        assert!(completed_task.is_done);
        assert!(completed_task.completed_at.is_some());

        // Check filters
        let active = db.all_tasks(TaskFilter::Active)?;
        assert!(active.is_empty());
        let done = db.all_tasks(TaskFilter::Done)?;
        assert_eq!(done.len(), 1);

        // Toggle back to active
        let is_now_done = db.toggle_task(task_id)?;
        assert!(!is_now_done);
        let active = db.all_tasks(TaskFilter::Active)?;
        assert_eq!(active.len(), 1);

        // Delete
        db.delete_task(task_id)?;
        assert!(db.get_task(task_id)?.is_none());
        Ok(())
    }
}
