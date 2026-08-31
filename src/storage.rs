use std::{
    collections::{HashMap, HashSet},
    error::Error,
    path::Path,
};

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, Weekday};
use rusqlite::{
    Connection, OptionalExtension, Row, Transaction, params, params_from_iter, types::Type,
};

use crate::{
    model::{
        Event, EventId, EventOccurrence, ExceptionKind, Frequency, Importance, Link, LinkId,
        NewEvent, NewLink, NewNote, NewRecurrence, Note, NoteId, Recurrence, RecurrenceException,
        RecurrenceId, Tag, UpcomingOrder, normalize_tag,
    },
    recurrence::{MAX_INTERVAL_WEEKS, expand_weekly},
    search::{
        SearchFilters, SearchResult, date_range, event_matches, note_matches, parse_query,
        sort_results,
    },
};

mod migrations;
mod operations;
use operations::set_event_tags_tx;

pub type StorageResult<T> = Result<T, Box<dyn Error>>;

pub struct Database {
    connection: Connection,
}

pub struct UpcomingEvents {
    pub items: Vec<EventOccurrence>,
    pub total: usize,
}

impl Database {
    pub fn open(path: &Path) -> StorageResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        let mut database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    #[cfg(test)]
    fn in_memory() -> StorageResult<Self> {
        let connection = Connection::open_in_memory()?;
        let mut database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    fn migrate(&mut self) -> rusqlite::Result<()> {
        migrations::migrate(&mut self.connection)
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
            "SELECT id, title, description, start_date, start_time, end_time, importance, recurrence_id
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
                    e.importance, e.recurrence_id,
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
                |row| Ok((event_from_row(row)?, recurrence_from_row(row, 8)?)),
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
        occurrences.sort_by(occurrence_order);
        Ok(occurrences)
    }

    fn exceptions_for(
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

    fn events_by_ids(&self, ids: &[EventId]) -> rusqlite::Result<Vec<Event>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT id, title, description, start_date, start_time, end_time, importance, recurrence_id
             FROM events WHERE id IN ({})",
            sql_placeholders(ids.len())
        );
        let mut statement = self.connection.prepare(&sql)?;
        statement
            .query_map(params_from_iter(ids), event_from_row)?
            .collect()
    }

    fn tags_for_events(&self, ids: &[EventId]) -> rusqlite::Result<HashMap<EventId, Vec<Tag>>> {
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

    pub fn get_event(&self, id: EventId) -> StorageResult<Option<Event>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, start_date, start_time, end_time, importance, recurrence_id
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
                "SELECT id, title, description, start_date, start_time, end_time, importance, recurrence_id
                 FROM events WHERE recurrence_id = ?1",
                [recurrence_id],
                event_from_row,
            )
            .optional()?)
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

    pub fn upcoming_events(
        &self,
        now: DateTime<Local>,
        limit: usize,
        order: UpcomingOrder,
    ) -> StorageResult<UpcomingEvents> {
        let today = now.date_naive();
        let current_time = now.time();
        let (_, range_end) = self.data_bounds(today)?;
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

fn event_from_row(row: &Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        start_date: date_from_row(row, 3)?,
        start_time: optional_time_from_row(row, 4)?,
        end_time: optional_time_from_row(row, 5)?,
        importance: Importance::from_db(row.get(6)?)?,
        recurrence_id: row.get(7)?,
    })
}

fn recurrence_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<Recurrence> {
    let frequency: String = row.get(offset + 1)?;
    if frequency != "weekly" {
        return Err(invalid_column(
            offset + 1,
            "unsupported recurrence frequency",
        ));
    }
    let weekdays: String = row.get(offset + 3)?;
    let interval = row.get(offset + 2)?;
    if !(1..=MAX_INTERVAL_WEEKS).contains(&interval) {
        return Err(invalid_column(offset + 2, "invalid recurrence interval"));
    }
    Ok(Recurrence {
        id: row.get(offset)?,
        frequency: Frequency::Weekly,
        interval,
        weekdays: decode_weekdays(&weekdays)?,
        start_date: date_from_row(row, offset + 4)?,
        end_date: optional_date_from_row(row, offset + 5)?,
        count: row.get(offset + 6)?,
    })
}

fn date_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<NaiveDate> {
    let value: String = row.get(index)?;
    NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn optional_date_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<NaiveDate>> {
    let value: Option<String> = row.get(index)?;
    value
        .map(|value| {
            NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
            })
        })
        .transpose()
}

fn optional_time_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<NaiveTime>> {
    let value: Option<String> = row.get(index)?;
    value
        .map(|value| {
            NaiveTime::parse_from_str(&value, "%H:%M:%S").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
            })
        })
        .transpose()
}

fn invalid_column(index: usize, message: &'static str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        std::io::Error::new(std::io::ErrorKind::InvalidData, message).into(),
    )
}

fn date_string(value: NaiveDate) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn time_string(value: Option<NaiveTime>) -> Option<String> {
    value.map(|time| time.format("%H:%M:%S").to_string())
}

fn sql_placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

fn occurrence_order(left: &EventOccurrence, right: &EventOccurrence) -> std::cmp::Ordering {
    left.date
        .cmp(&right.date)
        .then_with(|| left.start_time.cmp(&right.start_time))
        .then_with(|| right.importance.cmp(&left.importance))
        .then_with(|| left.title.cmp(&right.title))
}

fn encode_weekdays(days: &[Weekday]) -> String {
    let mut values: Vec<_> = days.iter().map(|day| day.num_days_from_monday()).collect();
    values.sort_unstable();
    values.dedup();
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_weekdays(value: &str) -> rusqlite::Result<Vec<Weekday>> {
    value
        .split(',')
        .filter(|part| !part.is_empty())
        .map(|part| match part.parse::<u32>() {
            Ok(0) => Ok(Weekday::Mon),
            Ok(1) => Ok(Weekday::Tue),
            Ok(2) => Ok(Weekday::Wed),
            Ok(3) => Ok(Weekday::Thu),
            Ok(4) => Ok(Weekday::Fri),
            Ok(5) => Ok(Weekday::Sat),
            Ok(6) => Ok(Weekday::Sun),
            _ => Err(invalid_column(0, "invalid weekday")),
        })
        .collect()
}

fn now_string() -> String {
    Local::now().to_rfc3339()
}

fn invalid_input(message: &'static str) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Weekday};

    use super::*;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap()
    }

    fn event(day: u32) -> NewEvent {
        NewEvent {
            title: "Лекция".into(),
            description: Some("Матан".into()),
            start_date: date(day),
            start_time: NaiveTime::from_hms_opt(14, 40, 0),
            end_time: NaiveTime::from_hms_opt(16, 10, 0),
            importance: Importance::Normal,
        }
    }

    #[test]
    fn migrations_are_recorded_in_order() -> StorageResult<()> {
        let database = Database::in_memory()?;
        let versions: Vec<i64> = database
            .connection
            .prepare("SELECT version FROM schema_migrations ORDER BY version")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        assert_eq!(versions, [1, 2]);
        Ok(())
    }

    #[test]
    fn event_tag_crud_normalizes_and_batches() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let first = database.create_event(
            &event(1),
            None,
            &["Универ".into(), "лекция".into(), " универ ".into()],
        )?;
        let second =
            database.create_event(&event(3), None, &["универ".into(), "экзамен".into()])?;

        let all = database.search("#универ", &SearchFilters::default(), date(1))?;
        assert_eq!(all.len(), 2);
        let tag_as_text = database.search("универ", &SearchFilters::default(), date(1))?;
        assert_eq!(tag_as_text.len(), 2);
        let only_lecture =
            database.search("#универ #лекция", &SearchFilters::default(), date(1))?;
        assert_eq!(only_lecture.len(), 1);
        assert_eq!(database.event_tags(first)?.len(), 2);

        let mut updated = event(2);
        updated.title = "Семинар".into();
        database.update_event(first, &updated, None, &["обновлено".into()])?;
        let stored = database.get_event(first)?.unwrap();
        assert_eq!(stored.title, "Семинар");
        assert_eq!(database.event_tags(first)?[0].normalized_name, "обновлено");

        database.delete_event(second)?;
        assert_eq!(database.events_between(date(1), date(30))?.len(), 1);
        Ok(())
    }

    #[test]
    fn weekly_exceptions_cancel_and_replace_one_occurrence() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let rule = NewRecurrence {
            interval: 1,
            weekdays: vec![Weekday::Tue],
            start_date: date(1),
            end_date: Some(date(22)),
            count: None,
        };
        let event_id = database.create_event(&event(1), Some(&rule), &["универ".into()])?;
        let recurrence_id = database
            .get_event(event_id)?
            .unwrap()
            .recurrence_id
            .unwrap();
        database.cancel_occurrence(recurrence_id, date(8))?;
        let mut replacement = event(15);
        replacement.start_time = NaiveTime::from_hms_opt(16, 20, 0);
        replacement.end_time = NaiveTime::from_hms_opt(17, 50, 0);
        database.modify_occurrence(recurrence_id, date(15), &replacement, &["универ".into()])?;

        let occurrences = database.events_between(date(1), date(22))?;
        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0].date, date(1));
        assert_eq!(
            occurrences[1]
                .start_time
                .unwrap()
                .format("%H:%M")
                .to_string(),
            "16:20"
        );
        assert_eq!(occurrences[2].date, date(22));

        let mut moved = event(22);
        moved.start_date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        database.modify_occurrence(recurrence_id, date(22), &moved, &["универ".into()])?;
        let moved_occurrences = database.events_between(moved.start_date, moved.start_date)?;
        assert_eq!(moved_occurrences.len(), 1);
        assert_eq!(moved_occurrences[0].original_date, date(22));

        database.delete_recurrence(recurrence_id)?;
        let remaining_events: i64 =
            database
                .connection
                .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        assert_eq!(remaining_events, 0);
        Ok(())
    }

    #[test]
    fn exception_loading_is_limited_to_the_requested_range() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let rule = NewRecurrence {
            interval: 1,
            weekdays: vec![Weekday::Tue],
            start_date: date(1),
            end_date: None,
            count: None,
        };
        let event_id = database.create_event(&event(1), Some(&rule), &[])?;
        let recurrence_id = database
            .get_event(event_id)?
            .unwrap()
            .recurrence_id
            .unwrap();
        let october_6 = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let october_13 = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        database.cancel_occurrence(recurrence_id, date(8))?;
        database.cancel_occurrence(recurrence_id, october_6)?;
        let mut moved_into_range = event(5);
        moved_into_range.start_date = date(5);
        database.modify_occurrence(recurrence_id, october_13, &moved_into_range, &[])?;

        let exceptions = database.exceptions_for(&[recurrence_id], date(1), date(8))?;
        let originals: HashSet<_> = exceptions[&recurrence_id]
            .iter()
            .map(|exception| exception.original_date)
            .collect();
        assert_eq!(originals, HashSet::from([date(8), october_13]));
        Ok(())
    }

    #[test]
    fn disabling_recurrence_removes_exception_replacements() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let rule = NewRecurrence {
            interval: 1,
            weekdays: vec![Weekday::Tue],
            start_date: date(1),
            end_date: None,
            count: None,
        };
        let event_id = database.create_event(&event(1), Some(&rule), &[])?;
        let recurrence_id = database
            .get_event(event_id)?
            .unwrap()
            .recurrence_id
            .unwrap();
        database.modify_occurrence(recurrence_id, date(8), &event(8), &[])?;
        database.update_event(event_id, &event(1), None, &[])?;

        let counts: (i64, i64, i64) = database.connection.query_row(
            "SELECT
                 (SELECT COUNT(*) FROM events),
                 (SELECT COUNT(*) FROM recurrences),
                 (SELECT COUNT(*) FROM recurrence_exceptions)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(counts, (1, 0, 0));
        Ok(())
    }

    #[test]
    fn deleting_note_and_event_cascades_children() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let event_id = database.create_event(&event(1), None, &["tag".into()])?;
        let note_id = database.create_note(&NewNote {
            date: date(1),
            title: Some("Домашка".into()),
            body: "Решить задачи".into(),
        })?;
        let link_id = database.create_link(&NewLink {
            note_id,
            label: "Условие".into(),
            url: "https://example.com".into(),
        })?;
        database.update_note(
            note_id,
            &NewNote {
                date: date(2),
                title: Some("Домашка 2".into()),
                body: "Решить задачи 6-10".into(),
            },
        )?;
        database.update_link(
            link_id,
            &NewLink {
                note_id,
                label: "Новая ссылка".into(),
                url: "https://example.org".into(),
            },
        )?;
        let notes = database.notes_between(date(2), date(2))?;
        assert_eq!(notes[0].title.as_deref(), Some("Домашка 2"));
        assert_eq!(notes[0].links[0].url, "https://example.org");
        database.delete_event(event_id)?;
        database.delete_note(note_id)?;

        let event_tags: i64 = database.connection.query_row(
            "SELECT COUNT(*) FROM event_tags WHERE event_id = ?1",
            [event_id],
            |row| row.get(0),
        )?;
        let links: i64 = database.connection.query_row(
            "SELECT COUNT(*) FROM links WHERE note_id = ?1",
            [note_id],
            |row| row.get(0),
        )?;
        assert_eq!((event_tags, links), (0, 0));
        Ok(())
    }

    #[test]
    fn links_for_upcoming_dates_are_loaded_in_one_batch() -> StorageResult<()> {
        let database = Database::in_memory()?;
        let first_note = database.create_note(&NewNote {
            date: date(1),
            title: Some("Первое".into()),
            body: String::new(),
        })?;
        let second_note = database.create_note(&NewNote {
            date: date(2),
            title: Some("Второе".into()),
            body: String::new(),
        })?;
        database.create_link(&NewLink {
            note_id: first_note,
            label: "Лекция".into(),
            url: "https://example.com/lecture".into(),
        })?;
        database.create_link(&NewLink {
            note_id: second_note,
            label: "Лишняя".into(),
            url: "https://example.com/other".into(),
        })?;

        let links = database.links_on_dates(&[date(1)])?;
        assert_eq!(links[&date(1)].len(), 1);
        assert_eq!(links[&date(1)][0].label, "Лекция");
        assert!(!links.contains_key(&date(2)));
        Ok(())
    }

    #[test]
    fn upcoming_includes_events_beyond_one_year() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let mut future = event(1);
        future.start_date = NaiveDate::from_ymd_opt(2028, 9, 1).unwrap();
        database.create_event(&future, None, &[])?;
        let now = Local
            .with_ymd_and_hms(2026, 9, 1, 8, 0, 0)
            .single()
            .unwrap();
        let upcoming = database.upcoming_events(now, 10, UpcomingOrder::Time)?;
        assert_eq!(upcoming.items.len(), 1);
        assert_eq!(upcoming.items[0].date, future.start_date);
        Ok(())
    }

    #[test]
    fn upcoming_importance_sort_happens_before_limit() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        for offset in 0..200 {
            let mut low = event(1);
            low.title = format!("Low {offset}");
            low.start_date = date(1) + Duration::days(offset);
            low.importance = Importance::Low;
            database.create_event(&low, None, &[])?;
        }
        let mut high = event(1);
        high.title = "High".into();
        high.start_date = date(1) + Duration::days(400);
        high.importance = Importance::High;
        database.create_event(&high, None, &[])?;
        let now = Local
            .with_ymd_and_hms(2026, 9, 1, 8, 0, 0)
            .single()
            .unwrap();

        let upcoming = database.upcoming_events(now, 200, UpcomingOrder::Importance)?;
        assert_eq!(upcoming.total, 201);
        assert!(
            upcoming
                .items
                .iter()
                .any(|event| event.importance == Importance::High)
        );
        Ok(())
    }

    #[test]
    fn unsafe_recurrence_interval_is_rejected_before_commit() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let rule = NewRecurrence {
            interval: u32::MAX,
            weekdays: vec![Weekday::Tue],
            start_date: date(1),
            end_date: None,
            count: None,
        };

        assert!(database.create_event(&event(1), Some(&rule), &[]).is_err());
        let count: i64 =
            database
                .connection
                .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }
}
