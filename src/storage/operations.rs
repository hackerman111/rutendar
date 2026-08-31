use super::*;
use crate::{calendar::week_start, search::DateFilter};

impl Database {
    pub fn create_event(
        &mut self,
        event: &NewEvent,
        recurrence: Option<&NewRecurrence>,
        tags: &[String],
    ) -> StorageResult<EventId> {
        validate_event(event, recurrence)?;
        let transaction = self.connection.transaction()?;
        let recurrence_id = recurrence
            .map(|rule| insert_recurrence(&transaction, rule))
            .transpose()?;
        let event_id = insert_event(&transaction, event, recurrence_id)?;
        set_event_tags_tx(&transaction, event_id, tags)?;
        transaction.commit()?;
        Ok(event_id)
    }

    pub fn update_event(
        &mut self,
        event_id: EventId,
        event: &NewEvent,
        recurrence: Option<&NewRecurrence>,
        tags: &[String],
    ) -> StorageResult<()> {
        validate_event(event, recurrence)?;
        let old_recurrence_id: Option<RecurrenceId> = self
            .connection
            .query_row(
                "SELECT recurrence_id FROM events WHERE id = ?1",
                [event_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let transaction = self.connection.transaction()?;
        let recurrence_id = match (old_recurrence_id, recurrence) {
            (Some(id), Some(rule)) => {
                update_recurrence(&transaction, id, rule)?;
                Some(id)
            }
            (Some(id), None) => {
                transaction.execute(
                    "UPDATE events SET recurrence_id = NULL WHERE id = ?1",
                    [event_id],
                )?;
                delete_recurrence(&transaction, id)?;
                None
            }
            (None, Some(rule)) => Some(insert_recurrence(&transaction, rule)?),
            (None, None) => None,
        };
        let changed = transaction.execute(
            "UPDATE events SET title = ?1, description = ?2, start_date = ?3,
                 start_time = ?4, end_time = ?5, importance = ?6, recurrence_id = ?7,
                 updated_at = ?8 WHERE id = ?9",
            params![
                event.title.trim(),
                event
                    .description
                    .as_deref()
                    .filter(|value| !value.trim().is_empty()),
                date_string(event.start_date),
                time_string(event.start_time),
                time_string(event.end_time),
                event.importance.to_db(),
                recurrence_id,
                now_string(),
                event_id,
            ],
        )?;
        if changed == 0 {
            return Err(invalid_input("event does not exist"));
        }
        set_event_tags_tx(&transaction, event_id, tags)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_event(&mut self, event_id: EventId) -> StorageResult<()> {
        let recurrence_id: Option<RecurrenceId> = self
            .connection
            .query_row(
                "SELECT recurrence_id FROM events WHERE id = ?1",
                [event_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let transaction = self.connection.transaction()?;
        let changed = if let Some(recurrence_id) = recurrence_id {
            delete_recurrence(&transaction, recurrence_id)?
        } else {
            transaction.execute("DELETE FROM events WHERE id = ?1", [event_id])?
        };
        if changed == 0 {
            return Err(invalid_input("event does not exist"));
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_recurrence(&mut self, recurrence_id: RecurrenceId) -> StorageResult<()> {
        let transaction = self.connection.transaction()?;
        if delete_recurrence(&transaction, recurrence_id)? == 0 {
            return Err(invalid_input("recurrence does not exist"));
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn set_event_importance(
        &self,
        event_id: EventId,
        importance: Importance,
    ) -> StorageResult<()> {
        let changed = self.connection.execute(
            "UPDATE events SET importance = ?1, updated_at = ?2 WHERE id = ?3",
            params![importance.to_db(), now_string(), event_id],
        )?;
        if changed == 0 {
            return Err(invalid_input("event does not exist"));
        }
        Ok(())
    }

    pub fn cancel_occurrence(
        &mut self,
        recurrence_id: RecurrenceId,
        original_date: NaiveDate,
    ) -> StorageResult<()> {
        let transaction = self.connection.transaction()?;
        let old_replacement: Option<EventId> = transaction
            .query_row(
                "SELECT replacement_event_id FROM recurrence_exceptions
                 WHERE recurrence_id = ?1 AND original_date = ?2",
                params![recurrence_id, date_string(original_date)],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        transaction.execute(
            "INSERT INTO recurrence_exceptions(recurrence_id, original_date, kind, replacement_event_id)
             VALUES (?1, ?2, 'cancelled', NULL)
             ON CONFLICT(recurrence_id, original_date)
             DO UPDATE SET kind = 'cancelled', replacement_event_id = NULL",
            params![recurrence_id, date_string(original_date)],
        )?;
        if let Some(event_id) = old_replacement {
            transaction.execute("DELETE FROM events WHERE id = ?1", [event_id])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn modify_occurrence(
        &mut self,
        recurrence_id: RecurrenceId,
        original_date: NaiveDate,
        replacement: &NewEvent,
        tags: &[String],
    ) -> StorageResult<EventId> {
        validate_event(replacement, None)?;
        let transaction = self.connection.transaction()?;
        let old_replacement: Option<EventId> = transaction
            .query_row(
                "SELECT replacement_event_id FROM recurrence_exceptions
                 WHERE recurrence_id = ?1 AND original_date = ?2",
                params![recurrence_id, date_string(original_date)],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let replacement_id = insert_event(&transaction, replacement, None)?;
        set_event_tags_tx(&transaction, replacement_id, tags)?;
        transaction.execute(
            "INSERT INTO recurrence_exceptions(recurrence_id, original_date, kind, replacement_event_id)
             VALUES (?1, ?2, 'modified', ?3)
             ON CONFLICT(recurrence_id, original_date)
             DO UPDATE SET kind = 'modified', replacement_event_id = excluded.replacement_event_id",
            params![recurrence_id, date_string(original_date), replacement_id],
        )?;
        if let Some(old_id) = old_replacement {
            transaction.execute("DELETE FROM events WHERE id = ?1", [old_id])?;
        }
        transaction.commit()?;
        Ok(replacement_id)
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

    fn links_for_notes(&self, ids: &[NoteId]) -> rusqlite::Result<HashMap<NoteId, Vec<Link>>> {
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

    pub fn search(
        &self,
        input: &str,
        filters: &SearchFilters,
        today: NaiveDate,
    ) -> StorageResult<Vec<SearchResult>> {
        let (start, end) = if filters.date == DateFilter::Upcoming {
            let (_, end) = self.data_bounds(today)?;
            (today, end)
        } else {
            date_range(filters.date, today)
                .map(Ok)
                .unwrap_or_else(|| self.data_bounds(today))?
        };
        let query = parse_query(input);
        let mut results: Vec<SearchResult> = self
            .events_between(start, end)?
            .into_iter()
            .filter(|event| event_matches(event, &query, filters))
            .map(SearchResult::Event)
            .collect();
        results.extend(
            self.notes_between(start, end)?
                .into_iter()
                .filter(|note| note_matches(note, &query, filters))
                .map(SearchResult::Note),
        );
        sort_results(&mut results, filters.sort);
        Ok(results)
    }

    pub(super) fn data_bounds(&self, today: NaiveDate) -> StorageResult<(NaiveDate, NaiveDate)> {
        let (minimum, maximum): (Option<String>, Option<String>) = self.connection.query_row(
            "SELECT MIN(value), MAX(value) FROM (
                 SELECT start_date AS value FROM events
                 UNION ALL SELECT date FROM notes
                 UNION ALL SELECT start_date FROM recurrences
                 UNION ALL SELECT end_date FROM recurrences WHERE end_date IS NOT NULL
             )",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let parse = |value: Option<String>| -> StorageResult<Option<NaiveDate>> {
            value
                .map(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(Into::into))
                .transpose()
        };
        let start = parse(minimum)?.unwrap_or(today);
        let stored_end = parse(maximum)?.unwrap_or(today);
        let max_open_interval: Option<i64> = self.connection.query_row(
            "SELECT MAX(interval) FROM recurrences WHERE end_date IS NULL",
            [],
            |row| row.get(0),
        )?;
        let end = if let Some(interval) = max_open_interval {
            if !(1..=i64::from(MAX_INTERVAL_WEEKS)).contains(&interval) {
                return Err(invalid_input("stored recurrence interval is invalid"));
            }
            let horizon = Duration::days(366).max(Duration::weeks(interval + 1));
            stored_end.max(today.checked_add_signed(horizon).unwrap_or(NaiveDate::MAX))
        } else {
            stored_end
        };
        Ok((start, end.max(start)))
    }
}

fn validate_event(event: &NewEvent, recurrence: Option<&NewRecurrence>) -> StorageResult<()> {
    if event.title.trim().is_empty() {
        return Err(invalid_input("event title cannot be empty"));
    }
    if let (Some(start), Some(end)) = (event.start_time, event.end_time)
        && end <= start
    {
        return Err(invalid_input("event end time must be after start time"));
    }
    if event.start_time.is_none() && event.end_time.is_some() {
        return Err(invalid_input("event end time requires a start time"));
    }
    if let Some(rule) = recurrence {
        if rule.interval == 0 || rule.interval > MAX_INTERVAL_WEEKS || rule.weekdays.is_empty() {
            return Err(invalid_input(
                "weekly recurrence interval must be 1..=5200 weeks and needs weekdays",
            ));
        }
        if week_start(rule.start_date)
            .checked_add_signed(Duration::weeks(i64::from(rule.interval)))
            .is_none()
        {
            return Err(invalid_input("recurrence interval exceeds the date range"));
        }
        if rule.end_date.is_some_and(|end| end < rule.start_date) {
            return Err(invalid_input("recurrence end date precedes start date"));
        }
        if rule.count == Some(0) {
            return Err(invalid_input("recurrence count must be positive"));
        }
    }
    Ok(())
}

fn validate_link(link: &NewLink) -> StorageResult<()> {
    if link.label.trim().is_empty() || link.url.trim().is_empty() {
        return Err(invalid_input("link label and URL are required"));
    }
    if !(link.url.starts_with("http://") || link.url.starts_with("https://")) {
        return Err(invalid_input("only http and https links are supported"));
    }
    Ok(())
}

fn insert_recurrence(
    transaction: &Transaction<'_>,
    recurrence: &NewRecurrence,
) -> rusqlite::Result<RecurrenceId> {
    transaction.execute(
        "INSERT INTO recurrences(frequency, interval, weekdays, start_date, end_date, count)
         VALUES ('weekly', ?1, ?2, ?3, ?4, ?5)",
        params![
            recurrence.interval,
            encode_weekdays(&recurrence.weekdays),
            date_string(recurrence.start_date),
            recurrence.end_date.map(date_string),
            recurrence.count,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn update_recurrence(
    transaction: &Transaction<'_>,
    id: RecurrenceId,
    recurrence: &NewRecurrence,
) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE recurrences SET interval = ?1, weekdays = ?2, start_date = ?3,
             end_date = ?4, count = ?5 WHERE id = ?6",
        params![
            recurrence.interval,
            encode_weekdays(&recurrence.weekdays),
            date_string(recurrence.start_date),
            recurrence.end_date.map(date_string),
            recurrence.count,
            id,
        ],
    )?;
    Ok(())
}

fn insert_event(
    transaction: &Transaction<'_>,
    event: &NewEvent,
    recurrence_id: Option<RecurrenceId>,
) -> rusqlite::Result<EventId> {
    let now = now_string();
    transaction.execute(
        "INSERT INTO events(title, description, start_date, start_time, end_time, importance,
                            recurrence_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            event.title.trim(),
            event
                .description
                .as_deref()
                .filter(|value| !value.trim().is_empty()),
            date_string(event.start_date),
            time_string(event.start_time),
            time_string(event.end_time),
            event.importance.to_db(),
            recurrence_id,
            now,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn delete_recurrence(
    transaction: &Transaction<'_>,
    recurrence_id: RecurrenceId,
) -> rusqlite::Result<usize> {
    let replacement_ids: Vec<EventId> = {
        let mut statement = transaction.prepare(
            "SELECT replacement_event_id FROM recurrence_exceptions
             WHERE recurrence_id = ?1 AND replacement_event_id IS NOT NULL",
        )?;
        statement
            .query_map([recurrence_id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let changed = transaction.execute("DELETE FROM recurrences WHERE id = ?1", [recurrence_id])?;
    if !replacement_ids.is_empty() {
        let sql = format!(
            "DELETE FROM events WHERE id IN ({})",
            sql_placeholders(replacement_ids.len())
        );
        transaction.execute(&sql, params_from_iter(&replacement_ids))?;
    }
    Ok(changed)
}

pub(super) fn set_event_tags_tx(
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
    Ok(())
}
