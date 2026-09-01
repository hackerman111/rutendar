use std::collections::HashMap;

use rusqlite::{Row, Transaction, params, params_from_iter};

use super::{
    Database, StorageResult,
    database::{invalid_input, now_string, sql_placeholders},
    notes::validate_link_fields,
};
use crate::model::{EventId, FavoriteLink, FavoriteLinkId, NewFavoriteLink};

impl Database {
    pub fn create_favorite_link(&self, link: &NewFavoriteLink) -> StorageResult<FavoriteLinkId> {
        validate_link_fields(&link.label, &link.url)?;
        let now = now_string();
        self.connection.execute(
            "INSERT INTO favorite_links(label, url, description, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                link.label.trim(),
                link.url.trim(),
                link.description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                link.tags.trim(),
                now,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn update_favorite_link(
        &self,
        id: FavoriteLinkId,
        link: &NewFavoriteLink,
    ) -> StorageResult<()> {
        validate_link_fields(&link.label, &link.url)?;
        let changed = self.connection.execute(
            "UPDATE favorite_links
             SET label = ?1, url = ?2, description = ?3, tags = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                link.label.trim(),
                link.url.trim(),
                link.description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                link.tags.trim(),
                now_string(),
                id,
            ],
        )?;
        if changed == 0 {
            return Err(invalid_input("favorite link does not exist"));
        }
        Ok(())
    }

    pub fn search_favorite_links(&self, query: &str) -> StorageResult<Vec<FavoriteLink>> {
        let mut statement = self.connection.prepare(
            "SELECT id, label, url, description, tags FROM favorite_links ORDER BY label, id",
        )?;
        let mut links = statement
            .query_map([], |row| favorite_link_from_row(row, 0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let needles: Vec<_> = query
            .split_whitespace()
            .map(|part| part.to_lowercase())
            .collect();
        if !needles.is_empty() {
            // ponytail: a local bookmark bank is small; add FTS only after this scan is measured.
            links.retain(|link| {
                let searchable = format!(
                    "{} {} {}",
                    link.label,
                    link.description.as_deref().unwrap_or_default(),
                    link.tags
                )
                .to_lowercase();
                needles.iter().all(|needle| searchable.contains(needle))
            });
        }
        Ok(links)
    }

    pub fn favorite_links_for_event(&self, id: EventId) -> StorageResult<Vec<FavoriteLink>> {
        Ok(self
            .favorite_links_for_events(&[id])?
            .remove(&id)
            .unwrap_or_default())
    }

    pub(crate) fn favorite_links_by_ids(
        &self,
        ids: &[FavoriteLinkId],
    ) -> rusqlite::Result<Vec<FavoriteLink>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT id, label, url, description, tags FROM favorite_links
             WHERE id IN ({}) ORDER BY label, id",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        statement
            .query_map(params_from_iter(ids), |row| favorite_link_from_row(row, 0))?
            .collect()
    }

    pub(crate) fn favorite_links_for_events(
        &self,
        ids: &[EventId],
    ) -> rusqlite::Result<HashMap<EventId, Vec<FavoriteLink>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT efl.event_id, fl.id, fl.label, fl.url, fl.description, fl.tags
             FROM event_favorite_links efl
             JOIN favorite_links fl ON fl.id = efl.favorite_link_id
             WHERE efl.event_id IN ({}) ORDER BY fl.label, fl.id",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(ids), |row| {
            Ok((row.get(0)?, favorite_link_from_row(row, 1)?))
        })?;
        let mut result: HashMap<_, Vec<_>> = HashMap::new();
        for row in rows {
            let (event_id, link) = row?;
            result.entry(event_id).or_default().push(link);
        }
        Ok(result)
    }
}

pub(crate) fn set_event_favorite_links_tx(
    transaction: &Transaction<'_>,
    event_id: EventId,
    link_ids: &[FavoriteLinkId],
) -> StorageResult<()> {
    transaction.execute(
        "DELETE FROM event_favorite_links WHERE event_id = ?1",
        [event_id],
    )?;
    for link_id in link_ids {
        transaction.execute(
            "INSERT OR IGNORE INTO event_favorite_links(event_id, favorite_link_id)
             VALUES (?1, ?2)",
            params![event_id, link_id],
        )?;
    }
    Ok(())
}

fn favorite_link_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<FavoriteLink> {
    Ok(FavoriteLink {
        id: row.get(offset)?,
        label: row.get(offset + 1)?,
        url: row.get(offset + 2)?,
        description: row.get(offset + 3)?,
        tags: row.get(offset + 4)?,
    })
}
