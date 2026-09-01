use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::{params, params_from_iter};

use super::{
    Database, StorageResult,
    database::{date_from_row, date_string, invalid_input, now_string, sql_placeholders},
};
use crate::model::{Link, LinkId, NewLink, NewNote, Note, NoteId};

impl Database {
    pub fn create_note(&self, note: &NewNote) -> StorageResult<NoteId> {
        if note.body.trim().is_empty()
            && note
                .title
                .as_deref()
                .is_none_or(|title| title.trim().is_empty())
        {
            return Err(invalid_input("note title and body cannot both be empty"));
        }
        let now = now_string();
        self.connection.execute(
            "INSERT INTO notes(date, title, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                date_string(note.date),
                note.title
                    .as_deref()
                    .filter(|value| !value.trim().is_empty()),
                note.body,
                now,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn update_note(&self, id: NoteId, note: &NewNote) -> StorageResult<()> {
        if note.body.trim().is_empty()
            && note
                .title
                .as_deref()
                .is_none_or(|title| title.trim().is_empty())
        {
            return Err(invalid_input("note title and body cannot both be empty"));
        }
        let changed = self.connection.execute(
            "UPDATE notes SET date = ?1, title = ?2, body = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                date_string(note.date),
                note.title
                    .as_deref()
                    .filter(|value| !value.trim().is_empty()),
                note.body,
                now_string(),
                id,
            ],
        )?;
        if changed == 0 {
            return Err(invalid_input("note does not exist"));
        }
        Ok(())
    }

    pub fn delete_note(&self, id: NoteId) -> StorageResult<()> {
        if self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])?
            == 0
        {
            return Err(invalid_input("note does not exist"));
        }
        Ok(())
    }

    pub fn create_link(&self, link: &NewLink) -> StorageResult<LinkId> {
        validate_link(link)?;
        self.connection.execute(
            "INSERT INTO links(note_id, label, url, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                link.note_id,
                link.label.trim(),
                link.url.trim(),
                now_string()
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn update_link(&self, id: LinkId, link: &NewLink) -> StorageResult<()> {
        validate_link(link)?;
        if self.connection.execute(
            "UPDATE links SET note_id = ?1, label = ?2, url = ?3 WHERE id = ?4",
            params![link.note_id, link.label.trim(), link.url.trim(), id],
        )? == 0
        {
            return Err(invalid_input("link does not exist"));
        }
        Ok(())
    }

    pub fn delete_link(&self, id: LinkId) -> StorageResult<()> {
        if self
            .connection
            .execute("DELETE FROM links WHERE id = ?1", [id])?
            == 0
        {
            return Err(invalid_input("link does not exist"));
        }
        Ok(())
    }

    pub fn notes_between(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> StorageResult<Vec<Note>> {
        let mut statement = self.connection.prepare(
            "SELECT id, date, title, body FROM notes WHERE date BETWEEN ?1 AND ?2 ORDER BY date, id",
        )?;
        let mut notes: Vec<Note> = statement
            .query_map(
                params![date_string(range_start), date_string(range_end)],
                |row| {
                    Ok(Note {
                        id: row.get(0)?,
                        date: date_from_row(row, 1)?,
                        title: row.get(2)?,
                        body: row.get(3)?,
                        links: Vec::new(),
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        let ids: Vec<_> = notes.iter().map(|note| note.id).collect();
        let mut links = self.links_for_notes(&ids)?;
        for note in &mut notes {
            note.links = links.remove(&note.id).unwrap_or_default();
        }
        Ok(notes)
    }

    pub fn all_notes(&self) -> StorageResult<Vec<Note>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, date, title, body FROM notes ORDER BY date, id")?;
        let mut notes: Vec<Note> = statement
            .query_map([], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    date: date_from_row(row, 1)?,
                    title: row.get(2)?,
                    body: row.get(3)?,
                    links: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let ids: Vec<_> = notes.iter().map(|note| note.id).collect();
        let mut links = self.links_for_notes(&ids)?;
        for note in &mut notes {
            note.links = links.remove(&note.id).unwrap_or_default();
        }
        Ok(notes)
    }

    pub(crate) fn links_for_notes(
        &self,
        ids: &[NoteId],
    ) -> rusqlite::Result<HashMap<NoteId, Vec<Link>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT id, note_id, label, url FROM links WHERE note_id IN ({}) ORDER BY id",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(ids), |row| {
            Ok(Link {
                id: row.get(0)?,
                note_id: row.get(1)?,
                label: row.get(2)?,
                url: row.get(3)?,
            })
        })?;
        let mut result: HashMap<_, Vec<_>> = HashMap::new();
        for link in rows {
            let link = link?;
            result.entry(link.note_id).or_default().push(link);
        }
        Ok(result)
    }

    pub fn links_on_dates(
        &self,
        dates: &[NaiveDate],
    ) -> StorageResult<HashMap<NaiveDate, Vec<Link>>> {
        if dates.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT n.date, l.id, l.note_id, l.label, l.url
             FROM notes n JOIN links l ON l.note_id = n.id
             WHERE n.date IN ({}) ORDER BY n.date, n.id, l.id",
            sql_placeholders(dates.len())
        );
        let values: Vec<_> = dates.iter().copied().map(date_string).collect();
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values.iter()), |row| {
            Ok((
                date_from_row(row, 0)?,
                Link {
                    id: row.get(1)?,
                    note_id: row.get(2)?,
                    label: row.get(3)?,
                    url: row.get(4)?,
                },
            ))
        })?;
        let mut result: HashMap<_, Vec<_>> = HashMap::new();
        for row in rows {
            let (date, link) = row?;
            result.entry(date).or_default().push(link);
        }
        Ok(result)
    }
}

fn validate_link(link: &NewLink) -> StorageResult<()> {
    validate_link_fields(&link.label, &link.url)
}

pub(super) fn validate_link_fields(label: &str, url: &str) -> StorageResult<()> {
    let url = url.trim();
    if label.trim().is_empty() || url.is_empty() {
        return Err(invalid_input("link label and URL are required"));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(invalid_input("only http and https links are supported"));
    }
    Ok(())
}
