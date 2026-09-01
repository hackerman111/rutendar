use rusqlite::{Connection, params};

const VERSION_1: &str = r#"
CREATE TABLE IF NOT EXISTS recurrences (
    id INTEGER PRIMARY KEY,
    frequency TEXT NOT NULL CHECK (frequency IN ('weekly')),
    interval INTEGER NOT NULL CHECK (interval > 0),
    weekdays TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT,
    count INTEGER CHECK (count IS NULL OR count > 0)
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT,
    start_date TEXT NOT NULL,
    start_time TEXT,
    end_time TEXT,
    importance INTEGER NOT NULL CHECK (importance BETWEEN 0 AND 3),
    recurrence_id INTEGER REFERENCES recurrences(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recurrence_exceptions (
    recurrence_id INTEGER NOT NULL REFERENCES recurrences(id) ON DELETE CASCADE,
    original_date TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('cancelled', 'modified')),
    replacement_event_id INTEGER REFERENCES events(id) ON DELETE SET NULL,
    PRIMARY KEY (recurrence_id, original_date),
    CHECK ((kind = 'cancelled' AND replacement_event_id IS NULL)
        OR (kind = 'modified' AND replacement_event_id IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY,
    date TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY,
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    url TEXT NOT NULL CHECK (length(trim(url)) > 0),
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE CHECK (length(normalized_name) > 0)
);

CREATE TABLE IF NOT EXISTS event_tags (
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_events_date ON events(start_date);
CREATE INDEX IF NOT EXISTS idx_events_recurrence ON events(recurrence_id);
CREATE INDEX IF NOT EXISTS idx_recurrence_dates ON recurrences(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_exceptions_recurrence ON recurrence_exceptions(recurrence_id);
CREATE INDEX IF NOT EXISTS idx_notes_date ON notes(date);
CREATE INDEX IF NOT EXISTS idx_links_note ON links(note_id);
CREATE INDEX IF NOT EXISTS idx_tags_normalized ON tags(normalized_name);
"#;

const VERSION_2: &str = "CREATE INDEX IF NOT EXISTS idx_exceptions_replacement ON recurrence_exceptions(replacement_event_id);";

const VERSION_3: &str = r#"
ALTER TABLE events ADD COLUMN directory TEXT;

CREATE TABLE favorite_links (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    url TEXT NOT NULL CHECK (length(trim(url)) > 0),
    description TEXT,
    tags TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE event_favorite_links (
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    favorite_link_id INTEGER NOT NULL REFERENCES favorite_links(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, favorite_link_id)
);

CREATE INDEX idx_event_favorite_links_link
    ON event_favorite_links(favorite_link_id);
"#;

pub(super) fn migrate(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    let current: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    for (version, sql) in [(1, VERSION_1), (2, VERSION_2), (3, VERSION_3)] {
        if version <= current {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version) VALUES (?1)",
            params![version],
        )?;
        transaction.commit()?;
    }
    Ok(())
}
