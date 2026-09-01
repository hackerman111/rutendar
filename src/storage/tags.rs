use std::collections::{HashMap, HashSet};

use rusqlite::{Transaction, params, params_from_iter};

use super::{
    Database, StorageResult,
    database::{invalid_input, sql_placeholders},
};
use crate::model::{EventId, Tag, normalize_tag};

impl Database {
    pub(crate) fn tags_for_events(
        &self,
        ids: &[EventId],
    ) -> rusqlite::Result<HashMap<EventId, Vec<Tag>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT et.event_id, t.id, t.name, t.normalized_name
             FROM event_tags et JOIN tags t ON t.id = et.tag_id
             WHERE et.event_id IN ({}) ORDER BY t.normalized_name",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(ids), |row| {
            Ok((
                row.get(0)?,
                Tag {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    normalized_name: row.get(3)?,
                },
            ))
        })?;
        let mut result: HashMap<_, Vec<_>> = HashMap::new();
        for row in rows {
            let (event_id, tag) = row?;
            result.entry(event_id).or_default().push(tag);
        }
        Ok(result)
    }

    pub fn event_tags(&self, id: EventId) -> StorageResult<Vec<Tag>> {
        Ok(self.tags_for_events(&[id])?.remove(&id).unwrap_or_default())
    }

    pub fn set_event_tags(&mut self, event_id: EventId, names: &[String]) -> StorageResult<()> {
        if self.get_event(event_id)?.is_none() {
            return Err(invalid_input("event does not exist"));
        }
        let transaction = self.connection.transaction()?;
        set_event_tags_tx(&transaction, event_id, names)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn create_tag(&self, name: &str) -> StorageResult<Tag> {
        let normalized = normalize_tag(name);
        if normalized.is_empty() {
            return Err(invalid_input("tag cannot be empty"));
        }
        self.connection.execute(
            "INSERT INTO tags(name, normalized_name) VALUES (?1, ?2)
             ON CONFLICT(normalized_name) DO NOTHING",
            params![name.trim().trim_start_matches('#'), normalized],
        )?;
        Ok(self.connection.query_row(
            "SELECT id, name, normalized_name FROM tags WHERE normalized_name = ?1",
            [normalized],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    normalized_name: row.get(2)?,
                })
            },
        )?)
    }

    pub fn search_tags(&self, prefix: &str, limit: usize) -> StorageResult<Vec<Tag>> {
        let prefix = format!("{}%", normalize_tag(prefix));
        let mut statement = self.connection.prepare(
            "SELECT id, name, normalized_name FROM tags
             WHERE normalized_name LIKE ?1 ORDER BY normalized_name LIMIT ?2",
        )?;
        Ok(statement
            .query_map(params![prefix, limit as i64], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    normalized_name: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_tag(&mut self, tag_id: i64) -> StorageResult<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM event_tags WHERE tag_id = ?1", [tag_id])?;
        transaction.execute("DELETE FROM tags WHERE id = ?1", [tag_id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_unused_tags(&self) -> StorageResult<usize> {
        Ok(self.connection.execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM event_tags)",
            [],
        )?)
    }
}

pub(crate) fn cleanup_unused_tags_tx(transaction: &Transaction<'_>) -> StorageResult<()> {
    transaction.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM event_tags)",
        [],
    )?;
    Ok(())
}

pub(crate) fn set_event_tags_tx(
    transaction: &Transaction<'_>,
    event_id: EventId,
    names: &[String],
) -> StorageResult<()> {
    transaction.execute("DELETE FROM event_tags WHERE event_id = ?1", [event_id])?;
    let mut seen = HashSet::new();
    for name in names {
        let normalized = normalize_tag(name);
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        transaction.execute(
            "INSERT INTO tags(name, normalized_name) VALUES (?1, ?2)
             ON CONFLICT(normalized_name) DO NOTHING",
            params![name.trim().trim_start_matches('#'), normalized],
        )?;
        let tag_id: i64 = transaction.query_row(
            "SELECT id FROM tags WHERE normalized_name = ?1",
            [normalized],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO event_tags(event_id, tag_id) VALUES (?1, ?2)",
            params![event_id, tag_id],
        )?;
    }
    cleanup_unused_tags_tx(transaction)?;
    Ok(())
}
