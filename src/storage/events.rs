use std::collections::HashMap;

use chrono::{DateTime, Duration, Local, NaiveDate};
use rusqlite::{OptionalExtension, Transaction, params, params_from_iter};

use super::{
    Database, StorageResult, UpcomingEvents,
    database::{
        date_from_row, date_string, encode_weekdays, event_from_row, invalid_column, invalid_input,
        now_string, occurrence_order, recurrence_from_row, sql_placeholders, time_string,
    },
    favorite_links::set_event_favorite_links_tx,
    tags::{cleanup_unused_tags_tx, set_event_tags_tx},
};
use crate::{
    calendar::week_start,
    model::{
        Event, EventId, EventOccurrence, ExceptionKind, FavoriteLinkId, Importance, NewEvent,
        NewRecurrence, Recurrence, RecurrenceException, RecurrenceId, UpcomingOrder,
    },
    recurrence::{MAX_INTERVAL_WEEKS, expand_weekly},
};

#[derive(Debug, Clone)]
pub struct EventExportData {
    pub event: Event,
    pub tags: Vec<String>,
    pub recurrence: Option<Recurrence>,
    pub favorite_links: Vec<String>,
}

impl Database {
    pub fn all_events_for_export(&self) -> StorageResult<Vec<EventExportData>> {
        let mut direct_statement = self.connection.prepare(
            "SELECT id, title, description, start_date, start_time, end_time, importance,
                    recurrence_id, directory
             FROM events
             WHERE recurrence_id IS NULL
               AND id NOT IN (
                   SELECT replacement_event_id FROM recurrence_exceptions
                   WHERE replacement_event_id IS NOT NULL
               )
             ORDER BY start_date, start_time",
        )?;
        let direct_events: Vec<Event> = direct_statement
            .query_map([], event_from_row)?
            .collect::<rusqlite::Result<_>>()?;

        let mut recurring_statement = self.connection.prepare(
            "SELECT e.id, e.title, e.description, e.start_date, e.start_time, e.end_time,
                    e.importance, e.recurrence_id, e.directory,
                    r.id, r.frequency, r.interval, r.weekdays, r.start_date, r.end_date, r.count
             FROM events e
             JOIN recurrences r ON r.id = e.recurrence_id
             ORDER BY e.start_date, e.start_time",
        )?;
        let recurring: Vec<(Event, Recurrence)> = recurring_statement
            .query_map([], |row| {
                Ok((event_from_row(row)?, recurrence_from_row(row, 9)?))
            })?
            .collect::<rusqlite::Result<_>>()?;

        let mut all_event_ids: Vec<_> = direct_events.iter().map(|e| e.id).collect();
        all_event_ids.extend(recurring.iter().map(|(e, _)| e.id));
        all_event_ids.sort_unstable();
        all_event_ids.dedup();

        let tags_map = self.tags_for_events(&all_event_ids)?;
        let fav_links_map = self.favorite_links_for_events(&all_event_ids)?;

        let mut result = Vec::with_capacity(direct_events.len() + recurring.len());

        for event in direct_events {
            let tags = tags_map
                .get(&event.id)
                .map(|tags| tags.iter().map(|t| t.name.clone()).collect())
                .unwrap_or_default();
            let favorite_links = fav_links_map
                .get(&event.id)
                .map(|links| links.iter().map(|l| l.url.clone()).collect())
                .unwrap_or_default();
            result.push(EventExportData {
                event,
                tags,
                recurrence: None,
                favorite_links,
            });
        }

        for (event, recurrence) in recurring {
            let tags = tags_map
                .get(&event.id)
                .map(|tags| tags.iter().map(|t| t.name.clone()).collect())
                .unwrap_or_default();
            let favorite_links = fav_links_map
                .get(&event.id)
                .map(|links| links.iter().map(|l| l.url.clone()).collect())
                .unwrap_or_default();
            result.push(EventExportData {
                event,
                tags,
                recurrence: Some(recurrence),
                favorite_links,
            });
        }

        result.sort_by(|a, b| {
            a.event
                .start_date
                .cmp(&b.event.start_date)
                .then_with(|| a.event.start_time.cmp(&b.event.start_time))
        });

        Ok(result)
    }
    pub fn events_between(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> StorageResult<Vec<EventOccurrence>> {
        if range_start > range_end {
            return Ok(Vec::new());
        }

        let mut direct_statement = self.connection.prepare(
            "SELECT id, title, description, start_date, start_time, end_time, importance,
                    recurrence_id, directory
             FROM events
             WHERE recurrence_id IS NULL
               AND id NOT IN (
                   SELECT replacement_event_id FROM recurrence_exceptions
                   WHERE replacement_event_id IS NOT NULL
               )
               AND start_date BETWEEN ?1 AND ?2",
        )?;
        let direct_events: Vec<Event> = direct_statement
            .query_map(
                params![date_string(range_start), date_string(range_end)],
                event_from_row,
            )?
            .collect::<rusqlite::Result<_>>()?;

        let mut recurring_statement = self.connection.prepare(
            "SELECT e.id, e.title, e.description, e.start_date, e.start_time, e.end_time,
                    e.importance, e.recurrence_id, e.directory,
                    r.id, r.frequency, r.interval, r.weekdays, r.start_date, r.end_date, r.count
             FROM events e
             JOIN recurrences r ON r.id = e.recurrence_id
             WHERE (r.start_date <= ?2 AND (r.end_date IS NULL OR r.end_date >= ?1))
                OR EXISTS (
                    SELECT 1 FROM recurrence_exceptions x
                    JOIN events replacement ON replacement.id = x.replacement_event_id
                    WHERE x.recurrence_id = r.id
                      AND replacement.start_date BETWEEN ?1 AND ?2
                )",
        )?;
        let recurring: Vec<(Event, Recurrence)> = recurring_statement
            .query_map(
                params![date_string(range_start), date_string(range_end)],
                |row| Ok((event_from_row(row)?, recurrence_from_row(row, 9)?)),
            )?
            .collect::<rusqlite::Result<_>>()?;

        let recurrence_ids: Vec<_> = recurring.iter().map(|(_, rule)| rule.id).collect();
        let exceptions = self.exceptions_for(&recurrence_ids, range_start, range_end)?;
        let replacement_ids: Vec<_> = exceptions
            .values()
            .flatten()
            .filter_map(|exception| exception.replacement_event_id)
            .collect();
        let replacement_events = self.events_by_ids(&replacement_ids)?;

        let mut all_event_ids: Vec<_> = direct_events.iter().map(|event| event.id).collect();
        all_event_ids.extend(recurring.iter().map(|(event, _)| event.id));
        all_event_ids.extend(replacement_ids.iter().copied());
        all_event_ids.sort_unstable();
        all_event_ids.dedup();
        let tags = self.tags_for_events(&all_event_ids)?;
        let favorite_links = self.favorite_links_for_events(&all_event_ids)?;

        let replacements: HashMap<_, _> = replacement_events
            .into_iter()
            .map(|event| {
                let event_tags = tags.get(&event.id).cloned().unwrap_or_default();
                (event.id, (event, event_tags))
            })
            .collect();
        let mut occurrences: Vec<_> = direct_events
            .iter()
            .map(|event| {
                EventOccurrence::from_event(event, tags.get(&event.id).cloned().unwrap_or_default())
            })
            .collect();
        for (event, rule) in recurring {
            occurrences.extend(expand_weekly(
                &event,
                &rule,
                exceptions
                    .get(&rule.id)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                &replacements,
                tags.get(&event.id).map(Vec::as_slice).unwrap_or_default(),
                range_start,
                range_end,
            ));
        }
        for occurrence in &mut occurrences {
            occurrence.favorite_links = favorite_links
                .get(&occurrence.event_id)
                .cloned()
                .unwrap_or_default();
        }
        occurrences.sort_by(occurrence_order);
        Ok(occurrences)
    }

    pub(crate) fn exceptions_for(
        &self,
        recurrence_ids: &[RecurrenceId],
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> rusqlite::Result<HashMap<RecurrenceId, Vec<RecurrenceException>>> {
        if recurrence_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = sql_placeholders(recurrence_ids.len());
        let sql = format!(
            "SELECT x.recurrence_id, x.original_date, x.kind, x.replacement_event_id
             FROM recurrence_exceptions x
             LEFT JOIN events replacement ON replacement.id = x.replacement_event_id
             WHERE x.recurrence_id IN ({placeholders})
               AND (x.original_date BETWEEN ? AND ?
                    OR replacement.start_date BETWEEN ? AND ?)"
        );
        let start = date_string(range_start);
        let end = date_string(range_end);
        let mut parameters: Vec<rusqlite::types::Value> = recurrence_ids
            .iter()
            .copied()
            .map(rusqlite::types::Value::Integer)
            .collect();
        parameters.extend([
            rusqlite::types::Value::Text(start.clone()),
            rusqlite::types::Value::Text(end.clone()),
            rusqlite::types::Value::Text(start),
            rusqlite::types::Value::Text(end),
        ]);
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(parameters.iter()), |row| {
            let kind: String = row.get(2)?;
            Ok(RecurrenceException {
                recurrence_id: row.get(0)?,
                original_date: date_from_row(row, 1)?,
                kind: match kind.as_str() {
                    "cancelled" => ExceptionKind::Cancelled,
                    "modified" => ExceptionKind::Modified,
                    _ => return Err(invalid_column(2, "invalid recurrence exception kind")),
                },
                replacement_event_id: row.get(3)?,
            })
        })?;
        let mut result: HashMap<_, Vec<_>> = HashMap::new();
        for exception in rows {
            let exception = exception?;
            result
                .entry(exception.recurrence_id)
                .or_default()
                .push(exception);
        }
        Ok(result)
    }

    pub(crate) fn events_by_ids(&self, ids: &[EventId]) -> rusqlite::Result<Vec<Event>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT id, title, description, start_date, start_time, end_time, importance,
                    recurrence_id, directory
             FROM events WHERE id IN ({})",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        statement
            .query_map(params_from_iter(ids), event_from_row)?
            .collect()
    }

    pub fn get_event(&self, id: EventId) -> StorageResult<Option<Event>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, start_date, start_time, end_time, importance,
                        recurrence_id, directory
                 FROM events WHERE id = ?1",
                [id],
                event_from_row,
            )
            .optional()?)
    }

    pub fn get_recurrence(&self, id: RecurrenceId) -> StorageResult<Option<Recurrence>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, frequency, interval, weekdays, start_date, end_date, count
                 FROM recurrences WHERE id = ?1",
                [id],
                |row| recurrence_from_row(row, 0),
            )
            .optional()?)
    }

    pub fn event_for_recurrence(
        &self,
        recurrence_id: RecurrenceId,
    ) -> StorageResult<Option<Event>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, start_date, start_time, end_time, importance,
                        recurrence_id, directory
                 FROM events WHERE recurrence_id = ?1",
                [recurrence_id],
                event_from_row,
            )
            .optional()?)
    }

    pub fn create_event(
        &mut self,
        event: &NewEvent,
        recurrence: Option<&NewRecurrence>,
        tags: &[String],
        favorite_link_ids: &[FavoriteLinkId],
    ) -> StorageResult<EventId> {
        validate_event(event, recurrence)?;
        let transaction = self.connection.transaction()?;
        let recurrence_id = recurrence
            .map(|rule| insert_recurrence(&transaction, rule))
            .transpose()?;
        let event_id = insert_event(&transaction, event, recurrence_id)?;
        set_event_tags_tx(&transaction, event_id, tags)?;
        set_event_favorite_links_tx(&transaction, event_id, favorite_link_ids)?;
        transaction.commit()?;
        Ok(event_id)
    }

    pub fn update_event(
        &mut self,
        event_id: EventId,
        event: &NewEvent,
        recurrence: Option<&NewRecurrence>,
        tags: &[String],
        favorite_link_ids: &[FavoriteLinkId],
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
                 directory = ?8, updated_at = ?9 WHERE id = ?10",
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
                event.directory.as_deref().and_then(std::path::Path::to_str),
                now_string(),
                event_id,
            ],
        )?;
        if changed == 0 {
            return Err(invalid_input("event does not exist"));
        }
        set_event_tags_tx(&transaction, event_id, tags)?;
        set_event_favorite_links_tx(&transaction, event_id, favorite_link_ids)?;
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
        cleanup_unused_tags_tx(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_recurrence(&mut self, recurrence_id: RecurrenceId) -> StorageResult<()> {
        let transaction = self.connection.transaction()?;
        if delete_recurrence(&transaction, recurrence_id)? == 0 {
            return Err(invalid_input("recurrence does not exist"));
        }
        cleanup_unused_tags_tx(&transaction)?;
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
        cleanup_unused_tags_tx(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn modify_occurrence(
        &mut self,
        recurrence_id: RecurrenceId,
        original_date: NaiveDate,
        replacement: &NewEvent,
        tags: &[String],
        favorite_link_ids: &[FavoriteLinkId],
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
        set_event_favorite_links_tx(&transaction, replacement_id, favorite_link_ids)?;
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
        cleanup_unused_tags_tx(&transaction)?;
        transaction.commit()?;
        Ok(replacement_id)
    }

    pub fn upcoming_events(
        &self,
        now: DateTime<Local>,
        through: Option<NaiveDate>,
        limit: usize,
        order: UpcomingOrder,
    ) -> StorageResult<UpcomingEvents> {
        let today = now.date_naive();
        let current_time = now.time();
        let (_, available_end) = self.data_bounds(today)?;
        let range_end = through.map_or(available_end, |end| end.min(available_end));
        let mut events = self.events_between(today, range_end)?;
        events.retain(|event| {
            event.date > today || event.start_time.is_none_or(|time| time >= current_time)
        });
        if order == UpcomingOrder::Importance {
            events.sort_by(|left, right| {
                right
                    .importance
                    .cmp(&left.importance)
                    .then_with(|| occurrence_order(left, right))
            });
        }
        let total = events.len();
        events.truncate(limit);
        Ok(UpcomingEvents {
            items: events,
            total,
        })
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
    if event
        .directory
        .as_ref()
        .is_some_and(|directory| !directory.is_absolute())
    {
        return Err(invalid_input("event directory must be an absolute path"));
    }
    if event
        .directory
        .as_ref()
        .is_some_and(|directory| directory.to_str().is_none())
    {
        return Err(invalid_input("event directory must be valid UTF-8"));
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
                            recurrence_id, directory, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
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
            event.directory.as_deref().and_then(std::path::Path::to_str),
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
