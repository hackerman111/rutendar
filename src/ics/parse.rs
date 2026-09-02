use std::{error::Error, fmt, path::PathBuf};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use super::{IcsCalendar, IcsEvent, IcsTask};
use crate::model::{Importance, NewRecurrence};

#[derive(Debug, PartialEq, Eq)]
pub enum IcsParseError {
    EmptyCalendar,
    InvalidFormat(String),
}

impl fmt::Display for IcsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCalendar => write!(f, "календарь пуст или не содержит VCALENDAR"),
            Self::InvalidFormat(msg) => write!(f, "ошибка формата iCalendar: {msg}"),
        }
    }
}

impl Error for IcsParseError {}

/// Unfolds lines according to RFC 5545 section 3.1:
/// A line ending with CRLF immediately followed by a single whitespace (space or tab)
/// is treated as a continuation.
pub fn unfold(input: &str) -> Vec<String> {
    let mut unfolded = Vec::new();
    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if (line.starts_with(' ') || line.starts_with('\t')) && !unfolded.is_empty() {
            let last: &mut String = unfolded.last_mut().unwrap();
            last.push_str(&line[1..]);
        } else if !line.trim().is_empty() {
            unfolded.push(line.to_string());
        }
    }
    unfolded
}

/// Unescapes text characters: `\\` -> `\`, `\;` -> `;`, `\,` -> `,`, `\n`/`\N` -> newline.
pub fn unescape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(';') => out.push(';'),
                Some(',') => out.push(','),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.trim().to_uppercase().as_str() {
        "MO" => Some(Weekday::Mon),
        "TU" => Some(Weekday::Tue),
        "WE" => Some(Weekday::Wed),
        "TH" => Some(Weekday::Thu),
        "FR" => Some(Weekday::Fri),
        "SA" => Some(Weekday::Sat),
        "SU" => Some(Weekday::Sun),
        _ => None,
    }
}

fn parse_priority(val: &str) -> Importance {
    match val.trim().parse::<u32>() {
        Ok(1..=4) => Importance::High,
        Ok(5) => Importance::Normal,
        Ok(6..=9) => Importance::Low,
        _ => Importance::None,
    }
}

/// Parses an RFC 5545 date or datetime string into (NaiveDate, Option<NaiveTime>).
/// Handles:
/// - YYYYMMDD
/// - YYYYMMDDTHHMMSS
/// - YYYYMMDDTHHMMSSZ
pub fn parse_ics_date_time(val: &str) -> Option<(NaiveDate, Option<NaiveTime>)> {
    let clean = val.trim();
    if clean.len() == 8 {
        // Date only: YYYYMMDD
        let date = NaiveDate::parse_from_str(clean, "%Y%m%d").ok()?;
        return Some((date, None));
    }

    // Try parsing YYYYMMDDTHHMMSSZ or YYYYMMDDTHHMMSS
    let without_z = clean.trim_end_matches('Z');
    if let Ok(dt) = NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S") {
        return Some((dt.date(), Some(dt.time())));
    }

    // Fallback: try taking first 8 chars as date
    if clean.len() >= 8
        && let Ok(date) = NaiveDate::parse_from_str(&clean[..8], "%Y%m%d")
    {
        return Some((date, None));
    }

    None
}

/// Parses an RRULE string into `NewRecurrence`.
fn parse_rrule(val: &str, default_date: NaiveDate) -> Option<NewRecurrence> {
    let mut freq = None;
    let mut interval: u32 = 1;
    let mut weekdays = Vec::new();
    let mut count = None;
    let mut end_date = None;

    for part in val.split(';') {
        let mut key_val = part.splitn(2, '=');
        let key = key_val.next()?.trim().to_uppercase();
        let value = key_val.next()?.trim();

        match key.as_str() {
            "FREQ" => freq = Some(value.to_uppercase()),
            "INTERVAL" => interval = value.parse::<u32>().unwrap_or(1).max(1),
            "COUNT" => count = value.parse::<u32>().ok(),
            "UNTIL" => {
                if let Some((d, _)) = parse_ics_date_time(value) {
                    end_date = Some(d);
                }
            }
            "BYDAY" => {
                for day_str in value.split(',') {
                    // Ignore numeric prefixes like 1MO, 2TU if present
                    let code = day_str
                        .trim()
                        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '+');
                    if let Some(w) = parse_weekday(code) {
                        weekdays.push(w);
                    }
                }
            }
            _ => {}
        }
    }

    let freq = freq?;
    if freq == "WEEKLY" {
        if weekdays.is_empty() {
            weekdays.push(default_date.weekday());
        }
        weekdays.sort_by_key(|w| w.number_from_monday());
        weekdays.dedup();
        Some(NewRecurrence {
            interval,
            weekdays,
            start_date: default_date,
            end_date,
            count,
        })
    } else if freq == "DAILY" {
        // Daily recurrence represented as weekly with all 7 days if interval is 1
        let all_days = vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ];
        Some(NewRecurrence {
            interval,
            weekdays: all_days,
            start_date: default_date,
            end_date,
            count,
        })
    } else {
        None
    }
}

/// Parses raw RFC 5545 iCalendar content into `IcsCalendar`.
pub fn parse_ics(input: &str) -> Result<IcsCalendar, IcsParseError> {
    let lines = unfold(input);
    if lines.is_empty() {
        return Err(IcsParseError::EmptyCalendar);
    }

    let mut in_calendar = false;
    let mut current_component: Option<&str> = None;
    let mut event_props = Vec::new();
    let mut task_props = Vec::new();

    let mut calendar = IcsCalendar::default();

    for line in &lines {
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        let prop_name = key.split(';').next().unwrap_or(key).to_uppercase();

        if prop_name == "BEGIN" {
            let comp = value.to_uppercase();
            if comp == "VCALENDAR" {
                in_calendar = true;
            } else if in_calendar {
                if comp == "VEVENT" {
                    current_component = Some("VEVENT");
                    event_props.clear();
                } else if comp == "VTODO" {
                    current_component = Some("VTODO");
                    task_props.clear();
                }
            }
            continue;
        }

        if prop_name == "END" {
            let comp = value.to_uppercase();
            if comp == "VEVENT" && current_component == Some("VEVENT") {
                if let Some(event) = parse_vevent(&event_props) {
                    calendar.events.push(event);
                }
                current_component = None;
                event_props.clear();
            } else if comp == "VTODO" && current_component == Some("VTODO") {
                if let Some(task) = parse_vtodo(&task_props) {
                    calendar.tasks.push(task);
                }
                current_component = None;
                task_props.clear();
            } else if comp == "VCALENDAR" {
                in_calendar = false;
            }
            continue;
        }

        match current_component {
            Some("VEVENT") => event_props.push((key.to_string(), value.to_string())),
            Some("VTODO") => task_props.push((key.to_string(), value.to_string())),
            _ => {}
        }
    }

    if !in_calendar && calendar.events.is_empty() && calendar.tasks.is_empty() {
        // Calendar might not have ended cleanly or was empty
        if lines.iter().any(|l| l.contains("BEGIN:VCALENDAR")) {
            return Ok(calendar);
        }
        return Err(IcsParseError::EmptyCalendar);
    }

    Ok(calendar)
}

fn parse_vevent(props: &[(String, String)]) -> Option<IcsEvent> {
    let mut title = None;
    let mut description = None;
    let mut start_date = None;
    let mut start_time = None;
    let mut end_time = None;
    let mut importance = Importance::None;
    let mut tags = Vec::new();
    let mut recurrence = None;
    let mut link = None;
    let mut directory = None;

    for (key, val) in props {
        let name = key.split(';').next().unwrap_or(key).to_uppercase();
        match name.as_str() {
            "SUMMARY" => {
                title = Some(unescape_text(val));
            }
            "DESCRIPTION" => {
                let text = unescape_text(val);
                // Check if folder was saved in description
                if let Some(idx) = text.find("Папка: ") {
                    let dir_part = &text[idx + "Папка: ".len()..];
                    let dir_line = dir_part.lines().next().unwrap_or(dir_part).trim();
                    if !dir_line.is_empty() {
                        directory = Some(PathBuf::from(dir_line));
                    }
                }
                description = Some(text);
            }
            "DTSTART" => {
                if let Some((d, t)) = parse_ics_date_time(val) {
                    start_date = Some(d);
                    start_time = t;
                }
            }
            "DTEND" => {
                if let Some((_, t)) = parse_ics_date_time(val) {
                    end_time = t;
                }
            }
            "PRIORITY" => {
                importance = parse_priority(val);
            }
            "CATEGORIES" => {
                for part in val.split(',') {
                    let tag = unescape_text(part.trim());
                    if !tag.is_empty() {
                        tags.push(tag);
                    }
                }
            }
            "URL" => {
                let url_val = val.trim();
                if !url_val.is_empty() {
                    link = Some(url_val.to_string());
                }
            }
            "RRULE" => {
                // We will parse RRULE after finding start_date
            }
            _ => {}
        }
    }

    let start_date = start_date?;
    let title = title.unwrap_or_else(|| "Без названия".to_string());

    // Parse RRULE if present
    for (key, val) in props {
        let name = key.split(';').next().unwrap_or(key).to_uppercase();
        if name == "RRULE" {
            recurrence = parse_rrule(val, start_date);
            break;
        }
    }

    Some(IcsEvent {
        title,
        description,
        start_date,
        start_time,
        end_time,
        importance,
        tags,
        recurrence,
        link,
        directory,
    })
}

fn parse_vtodo(props: &[(String, String)]) -> Option<IcsTask> {
    let mut title = None;
    let mut description = None;
    let mut date = None;
    let mut is_done = false;
    let mut importance = Importance::None;

    for (key, val) in props {
        let name = key.split(';').next().unwrap_or(key).to_uppercase();
        match name.as_str() {
            "SUMMARY" => {
                title = Some(unescape_text(val));
            }
            "DESCRIPTION" => {
                description = Some(unescape_text(val));
            }
            "DUE" | "DTSTART" => {
                if let Some((d, _)) = parse_ics_date_time(val) {
                    date = Some(d);
                }
            }
            "STATUS" if val.trim().eq_ignore_ascii_case("COMPLETED") => {
                is_done = true;
            }
            "PERCENT-COMPLETE" if val.trim() == "100" => {
                is_done = true;
            }

            "PRIORITY" => {
                importance = parse_priority(val);
            }
            _ => {}
        }
    }

    let title = title.unwrap_or_else(|| "Задание".to_string());

    Some(IcsTask {
        title,
        description,
        date,
        is_done,
        importance,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_unfold() {
        let folded =
            "SUMMARY:This is a long\r\n  line that was folded\r\nDESCRIPTION:Another\r\n line";
        let lines = unfold(folded);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "SUMMARY:This is a long line that was folded");
        assert_eq!(lines[1], "DESCRIPTION:Anotherline");
    }

    #[test]
    fn test_unescape_text() {
        let escaped = r"Встреча\, презентация\; план\\отчет\nвторая строка";
        let unescaped = unescape_text(escaped);
        assert_eq!(
            unescaped,
            "Встреча, презентация; план\\отчет\nвторая строка"
        );
    }

    #[test]
    fn test_parse_google_calendar_sample() {
        let sample = r#"BEGIN:VCALENDAR
PRODID:-//Google Inc//Google Calendar 70.9054//EN
VERSION:2.0
CALSCALE:GREGORIAN
METHOD:PUBLISH
X-WR-CALNAME:Work
BEGIN:VEVENT
DTSTART;TZID=Europe/Moscow:20260904T103000
DTEND;TZID=Europe/Moscow:20260904T113000
DTSTAMP:20260903T120000Z
UID:google-event-12345@google.com
CREATED:20260901T080000Z
DESCRIPTION:Обсуждение деталей релиза v0.8
LAST-MODIFIED:20260902T100000Z
LOCATION:Online
SEQUENCE:0
STATUS:CONFIRMED
SUMMARY:Daily Standup
CATEGORIES:work,meetings
RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20261231T235959Z;INTERVAL=1;BYDAY=MO,WE,FR
END:VEVENT
BEGIN:VTODO
UID:google-task-999@google.com
SUMMARY:Сдать отчет
DUE;VALUE=DATE:20260910
STATUS:COMPLETED
PRIORITY:1
END:VTODO
END:VCALENDAR"#;

        let cal = parse_ics(sample).expect("failed to parse sample");
        assert_eq!(cal.events.len(), 1);
        let ev = &cal.events[0];
        assert_eq!(ev.title, "Daily Standup");
        assert_eq!(
            ev.description.as_deref(),
            Some("Обсуждение деталей релиза v0.8")
        );
        assert_eq!(ev.start_date, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap());
        assert_eq!(
            ev.start_time,
            Some(NaiveTime::from_hms_opt(10, 30, 0).unwrap())
        );
        assert_eq!(
            ev.end_time,
            Some(NaiveTime::from_hms_opt(11, 30, 0).unwrap())
        );
        assert_eq!(ev.tags, vec!["work", "meetings"]);

        let rrule = ev.recurrence.as_ref().expect("expected recurrence");
        assert_eq!(rrule.interval, 1);
        assert_eq!(
            rrule.weekdays,
            vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
        );
        assert_eq!(
            rrule.end_date,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())
        );

        assert_eq!(cal.tasks.len(), 1);
        let task = &cal.tasks[0];
        assert_eq!(task.title, "Сдать отчет");
        assert_eq!(
            task.date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 10).unwrap())
        );
        assert!(task.is_done);
        assert_eq!(task.importance, Importance::High);
    }
}
