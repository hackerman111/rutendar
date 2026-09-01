use std::{
    error::Error,
    path::{Path, PathBuf},
};

use chrono::{Duration, Local, NaiveDate, NaiveTime, Weekday};
use rusqlite::{Connection, Row, types::Type};

use super::{StorageResult, migrations};
use crate::{
    model::{Event, EventOccurrence, Frequency, Importance, Recurrence},
    recurrence::MAX_INTERVAL_WEEKS,
    search::{
        SearchFilters, SearchResult, date_range, event_matches, note_matches, parse_query,
        sort_results,
    },
};

pub struct Database {
    pub(super) connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> StorageResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        let mut database = Self { connection };
        database.migrate()?;
        database.delete_unused_tags()?;
        Ok(database)
    }

    #[cfg(test)]
    pub fn in_memory() -> StorageResult<Self> {
        let connection = Connection::open_in_memory()?;
        let mut database = Self { connection };
        database.migrate()?;
        database.delete_unused_tags()?;
        Ok(database)
    }

    fn migrate(&mut self) -> rusqlite::Result<()> {
        migrations::migrate(&mut self.connection)
    }

    pub fn export(&self, destination: &Path) -> StorageResult<(u64, usize)> {
        if let Some(parent) = destination.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            std::fs::remove_file(destination)?;
        }
        let dest_str = destination
            .to_str()
            .ok_or_else(|| invalid_input("путь экспорта содержит недопустимые символы"))?;
        self.connection.execute("VACUUM INTO ?1", [dest_str])?;
        let file_size = std::fs::metadata(destination)?.len();
        let event_count: usize = self
            .connection
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok((file_size, event_count))
    }

    pub fn validate_file(path: &Path) -> StorageResult<usize> {
        if !path.exists() {
            return Err(invalid_input("файл не существует"));
        }
        let connection = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let integrity: String =
            connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(invalid_input(
                "база данных повреждена (integrity check failed)",
            ));
        }
        let table_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('events', 'schema_migrations')",
            [],
            |row| row.get(0),
        )?;
        if table_count < 2 {
            return Err(invalid_input(
                "файл не является корректной базой данных rutendar",
            ));
        }
        let event_count: usize =
            connection.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(event_count)
    }

    pub fn search(
        &self,
        input: &str,
        filters: &SearchFilters,
        today: NaiveDate,
    ) -> StorageResult<Vec<SearchResult>> {
        let (start, end) = date_range(filters.date, today)
            .map(Ok)
            .unwrap_or_else(|| self.data_bounds(today))?;
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

    pub(crate) fn data_bounds(&self, today: NaiveDate) -> StorageResult<(NaiveDate, NaiveDate)> {
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

pub(crate) fn event_from_row(row: &Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        start_date: date_from_row(row, 3)?,
        start_time: optional_time_from_row(row, 4)?,
        end_time: optional_time_from_row(row, 5)?,
        importance: Importance::from_db(row.get(6)?)?,
        recurrence_id: row.get(7)?,
        directory: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
    })
}

pub(crate) fn recurrence_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<Recurrence> {
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

pub(crate) fn date_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<NaiveDate> {
    let value: String = row.get(index)?;
    NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

pub(crate) fn optional_date_from_row(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<NaiveDate>> {
    let value: Option<String> = row.get(index)?;
    value
        .map(|value| {
            NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
            })
        })
        .transpose()
}

pub(crate) fn optional_time_from_row(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<NaiveTime>> {
    let value: Option<String> = row.get(index)?;
    value
        .map(|value| {
            NaiveTime::parse_from_str(&value, "%H:%M:%S").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
            })
        })
        .transpose()
}

pub(crate) fn invalid_column(index: usize, message: &'static str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        std::io::Error::new(std::io::ErrorKind::InvalidData, message).into(),
    )
}

pub(crate) fn date_string(value: NaiveDate) -> String {
    value.format("%Y-%m-%d").to_string()
}

pub(crate) fn time_string(value: Option<NaiveTime>) -> Option<String> {
    value.map(|time| time.format("%H:%M:%S").to_string())
}

pub(crate) fn sql_placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn occurrence_order(
    left: &EventOccurrence,
    right: &EventOccurrence,
) -> std::cmp::Ordering {
    left.date
        .cmp(&right.date)
        .then_with(|| left.start_time.cmp(&right.start_time))
        .then_with(|| right.importance.cmp(&left.importance))
        .then_with(|| left.title.cmp(&right.title))
}

pub(crate) fn encode_weekdays(days: &[Weekday]) -> String {
    let mut values: Vec<_> = days.iter().map(|day| day.num_days_from_monday()).collect();
    values.sort_unstable();
    values.dedup();
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn decode_weekdays(value: &str) -> rusqlite::Result<Vec<Weekday>> {
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

pub(crate) fn now_string() -> String {
    Local::now().to_rfc3339()
}

pub(crate) fn invalid_input(message: &'static str) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::TimeZone;

    use super::*;
    use crate::model::{NewEvent, NewFavoriteLink, NewLink, NewNote, NewRecurrence, UpcomingOrder};

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
            directory: None,
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
        assert_eq!(versions, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn event_tag_crud_normalizes_and_batches() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let first = database.create_event(
            &event(1),
            None,
            &["Универ".into(), "лекция".into(), " универ ".into()],
            &[],
        )?;
        let second =
            database.create_event(&event(3), None, &["универ".into(), "экзамен".into()], &[])?;

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
        database.update_event(first, &updated, None, &["обновлено".into()], &[])?;
        let stored = database.get_event(first)?.unwrap();
        assert_eq!(stored.title, "Семинар");
        assert_eq!(database.event_tags(first)?[0].normalized_name, "обновлено");

        database.delete_event(second)?;
        assert_eq!(database.events_between(date(1), date(30))?.len(), 1);
        Ok(())
    }

    #[test]
    fn favorite_links_are_searchable_and_follow_a_recurring_event() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let link_id = database.create_favorite_link(&NewFavoriteLink {
            label: "Задание".into(),
            url: "https://example.com/homework".into(),
            description: Some("ДЗ по акустике".into()),
            tags: "#универ #звук".into(),
        })?;
        assert_eq!(database.search_favorite_links("ДЗ ЗВУК")?.len(), 1);
        assert!(database.search_favorite_links("химия")?.is_empty());

        let mut recurring_event = event(1);
        recurring_event.directory = Some("/tmp".into());
        let rule = NewRecurrence {
            interval: 1,
            weekdays: vec![Weekday::Tue],
            start_date: date(1),
            end_date: Some(date(8)),
            count: None,
        };
        database.create_event(&recurring_event, Some(&rule), &[], &[link_id])?;

        let occurrences = database.events_between(date(1), date(8))?;
        assert_eq!(occurrences.len(), 2);
        assert!(occurrences.iter().all(|occurrence| {
            occurrence.directory.as_deref() == Some(std::path::Path::new("/tmp"))
                && occurrence.favorite_links[0].id == link_id
        }));
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
        let event_id = database.create_event(&event(1), Some(&rule), &["универ".into()], &[])?;
        let recurrence_id = database
            .get_event(event_id)?
            .unwrap()
            .recurrence_id
            .unwrap();
        database.cancel_occurrence(recurrence_id, date(8))?;
        let mut replacement = event(15);
        replacement.start_time = NaiveTime::from_hms_opt(16, 20, 0);
        replacement.end_time = NaiveTime::from_hms_opt(17, 50, 0);
        database.modify_occurrence(
            recurrence_id,
            date(15),
            &replacement,
            &["универ".into()],
            &[],
        )?;

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
        database.modify_occurrence(recurrence_id, date(22), &moved, &["универ".into()], &[])?;
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
        let event_id = database.create_event(&event(1), Some(&rule), &[], &[])?;
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
        database.modify_occurrence(recurrence_id, october_13, &moved_into_range, &[], &[])?;

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
        let event_id = database.create_event(&event(1), Some(&rule), &[], &[])?;
        let recurrence_id = database
            .get_event(event_id)?
            .unwrap()
            .recurrence_id
            .unwrap();
        database.modify_occurrence(recurrence_id, date(8), &event(8), &[], &[])?;
        database.update_event(event_id, &event(1), None, &[], &[])?;

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
        let event_id = database.create_event(&event(1), None, &["tag".into()], &[])?;
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
        let tags: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
        assert_eq!((event_tags, links, tags), (0, 0, 0));
        Ok(())
    }

    #[test]
    fn tag_cleanup_and_manual_deletion() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        let id1 =
            database.create_event(&event(1), None, &["rust".into(), "calendar".into()], &[])?;
        let id2 = database.create_event(&event(2), None, &["rust".into()], &[])?;

        assert_eq!(database.search_tags("rust", 10)?.len(), 1);
        assert_eq!(database.search_tags("calendar", 10)?.len(), 1);

        // Deleting id1 should remove "calendar" because it's no longer used, but keep "rust"
        database.delete_event(id1)?;
        assert_eq!(database.search_tags("calendar", 10)?.len(), 0);
        assert_eq!(database.search_tags("rust", 10)?.len(), 1);

        // Manual deletion of tag "rust" should remove it from id2 and from tags table
        let rust_tag = database.search_tags("rust", 1)?[0].clone();
        database.delete_tag(rust_tag.id)?;
        assert_eq!(database.search_tags("rust", 10)?.len(), 0);
        assert_eq!(database.event_tags(id2)?.len(), 0);

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
        database.create_event(&future, None, &[], &[])?;
        let now = Local
            .with_ymd_and_hms(2026, 9, 1, 8, 0, 0)
            .single()
            .unwrap();
        let upcoming = database.upcoming_events(now, None, 10, UpcomingOrder::Time)?;
        assert_eq!(upcoming.items.len(), 1);
        assert_eq!(upcoming.items[0].date, future.start_date);
        Ok(())
    }

    #[test]
    fn upcoming_can_be_limited_to_the_current_week() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        database.create_event(&event(2), None, &[], &[])?;
        database.create_event(&event(7), None, &[], &[])?;
        let now = Local
            .with_ymd_and_hms(2026, 9, 1, 8, 0, 0)
            .single()
            .unwrap();

        let upcoming = database.upcoming_events(now, Some(date(6)), 10, UpcomingOrder::Time)?;

        assert_eq!(upcoming.total, 1);
        assert_eq!(upcoming.items[0].date, date(2));
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
            database.create_event(&low, None, &[], &[])?;
        }
        let mut high = event(1);
        high.title = "High".into();
        high.start_date = date(1) + Duration::days(400);
        high.importance = Importance::High;
        database.create_event(&high, None, &[], &[])?;
        let now = Local
            .with_ymd_and_hms(2026, 9, 1, 8, 0, 0)
            .single()
            .unwrap();

        let upcoming = database.upcoming_events(now, None, 200, UpcomingOrder::Importance)?;
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

        assert!(
            database
                .create_event(&event(1), Some(&rule), &[], &[])
                .is_err()
        );
        let count: i64 =
            database
                .connection
                .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn database_export_and_validate_file() -> StorageResult<()> {
        let mut database = Database::in_memory()?;
        database.create_event(&event(1), None, &["тест".into()], &[])?;
        let temp_dir = std::env::temp_dir().join("rutendar_test_export");
        let _ = std::fs::create_dir_all(&temp_dir);
        let export_path = temp_dir.join("test_calendar.db");
        if export_path.exists() {
            let _ = std::fs::remove_file(&export_path);
        }

        let (file_size, count) = database.export(&export_path)?;
        assert_eq!(count, 1);
        assert!(file_size > 0);
        assert!(export_path.exists());

        let validated_count = Database::validate_file(&export_path)?;
        assert_eq!(validated_count, 1);

        // Invalid non-sqlite file validation test
        let invalid_path = temp_dir.join("invalid.db");
        std::fs::write(&invalid_path, b"not a sqlite database")?;
        assert!(Database::validate_file(&invalid_path).is_err());

        // Cleanup
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&invalid_path);
        let _ = std::fs::remove_dir(&temp_dir);
        Ok(())
    }
}
