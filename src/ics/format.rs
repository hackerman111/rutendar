use chrono::{Duration, Local, Weekday};

use crate::{
    model::{Event, Importance, Recurrence, Task},
    storage::events::EventExportData,
};

/// RFC 5545 specifies that lines SHOULD NOT be longer than 75 octets excluding CRLF.
/// Folding is done by inserting CRLF followed by a single whitespace character (space).
pub fn fold_line(line: &str) -> String {
    const MAX_LEN: usize = 75;
    if line.len() <= MAX_LEN {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len() + (line.len() / 70 + 1) * 3);
    let mut remaining = line;
    let mut is_first = true;

    while !remaining.is_empty() {
        let limit = if is_first { MAX_LEN } else { MAX_LEN - 1 };
        if remaining.len() <= limit {
            out.push_str(remaining);
            break;
        }

        // Find the last valid UTF-8 char boundary <= limit
        let mut split_pos = limit;
        while split_pos > 0 && !remaining.is_char_boundary(split_pos) {
            split_pos -= 1;
        }
        if split_pos == 0 {
            split_pos = remaining
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(remaining.len());
        }

        out.push_str(&remaining[..split_pos]);
        out.push_str("\r\n ");
        remaining = &remaining[split_pos..];
        is_first = false;
    }

    out
}

/// Escapes special characters in text values per RFC 5545 section 3.3.11:
/// `\` -> `\\`, `;` -> `\;`, `,` -> `\,`, newline -> `\n`.
pub fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str(r"\\"),
            ';' => out.push_str(r"\;"),
            ',' => out.push_str(r"\,"),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(r"\n");
            }
            '\n' => out.push_str(r"\n"),
            other => out.push(other),
        }
    }
    out
}

fn weekday_to_ics(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}

fn importance_to_priority(importance: Importance) -> Option<u8> {
    match importance {
        Importance::High => Some(1),
        Importance::Normal => Some(5),
        Importance::Low => Some(9),
        Importance::None => None,
    }
}

/// Serializes events and tasks into a valid RFC 5545 iCalendar string.
pub fn format_ics(events: &[EventExportData], tasks: &[Task]) -> String {
    let now = Local::now();
    let dtstamp = now.format("%Y%m%dT%H%M%SZ").to_string();

    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//Rutendar//NONSGML Rutendar//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
    ];

    // Format events
    for item in events {
        format_event(
            &item.event,
            &item.tags,
            item.recurrence.as_ref(),
            &item.favorite_links,
            &dtstamp,
            &mut lines,
        );
    }

    // Format tasks
    for task in tasks {
        format_task(task, &dtstamp, &mut lines);
    }

    lines.push("END:VCALENDAR".to_string());

    let mut output = String::new();
    for line in lines {
        output.push_str(&fold_line(&line));
        output.push_str("\r\n");
    }
    output
}

fn format_event(
    event: &Event,
    tags: &[String],
    recurrence: Option<&Recurrence>,
    links: &[String],
    dtstamp: &str,
    lines: &mut Vec<String>,
) {
    lines.push("BEGIN:VEVENT".to_string());
    lines.push(format!("UID:rutendar-event-{}@rutendar.local", event.id));
    lines.push(format!("DTSTAMP:{}", dtstamp));

    lines.push(format!("SUMMARY:{}", escape_text(&event.title)));

    let mut desc = event.description.clone().unwrap_or_default();
    if let Some(dir) = &event.directory {
        if !desc.is_empty() {
            desc.push('\n');
        }
        desc.push_str(&format!("Папка: {}", dir.display()));
    }
    if !desc.is_empty() {
        lines.push(format!("DESCRIPTION:{}", escape_text(&desc)));
    }

    if let Some(st) = event.start_time {
        let date_str = event.start_date.format("%Y%m%d");
        let start_time_str = st.format("%H%M%S");
        lines.push(format!("DTSTART:{}T{}", date_str, start_time_str));

        if let Some(et) = event.end_time {
            let end_time_str = et.format("%H%M%S");
            lines.push(format!("DTEND:{}T{}", date_str, end_time_str));
        } else {
            lines.push(format!("DTEND:{}T{}", date_str, start_time_str));
        }
    } else {
        // All-day event
        let date_str = event.start_date.format("%Y%m%d");
        lines.push(format!("DTSTART;VALUE=DATE:{}", date_str));
        let next_day = event.start_date + Duration::days(1);
        lines.push(format!("DTEND;VALUE=DATE:{}", next_day.format("%Y%m%d")));
    }

    if let Some(priority) = importance_to_priority(event.importance) {
        lines.push(format!("PRIORITY:{}", priority));
    }

    if !tags.is_empty() {
        let escaped_tags: Vec<String> = tags.iter().map(|t| escape_text(t)).collect();
        lines.push(format!("CATEGORIES:{}", escaped_tags.join(",")));
    }

    if let Some(link) = links.first() {
        lines.push(format!("URL:{}", link));
    }

    if let Some(r) = recurrence {
        let mut rrule = format!("RRULE:FREQ=WEEKLY;INTERVAL={}", r.interval.max(1));
        if !r.weekdays.is_empty() {
            let days: Vec<&'static str> = r.weekdays.iter().copied().map(weekday_to_ics).collect();
            rrule.push_str(&format!(";BYDAY={}", days.join(",")));
        }
        if let Some(count) = r.count {
            rrule.push_str(&format!(";COUNT={}", count));
        } else if let Some(end_date) = r.end_date {
            rrule.push_str(&format!(";UNTIL={}T235959Z", end_date.format("%Y%m%d")));
        }
        lines.push(rrule);
    }

    lines.push("END:VEVENT".to_string());
}

fn format_task(task: &Task, dtstamp: &str, lines: &mut Vec<String>) {
    lines.push("BEGIN:VTODO".to_string());
    lines.push(format!("UID:rutendar-task-{}@rutendar.local", task.id));
    lines.push(format!("DTSTAMP:{}", dtstamp));
    lines.push(format!("SUMMARY:{}", escape_text(&task.title)));

    if let Some(desc) = &task.description
        && !desc.is_empty()
    {
        lines.push(format!("DESCRIPTION:{}", escape_text(desc)));
    }

    if let Some(due) = task.date {
        lines.push(format!("DUE;VALUE=DATE:{}", due.format("%Y%m%d")));
    }

    if task.is_done {
        lines.push("STATUS:COMPLETED".to_string());
        lines.push("PERCENT-COMPLETE:100".to_string());
    } else {
        lines.push("STATUS:NEEDS-ACTION".to_string());
        lines.push("PERCENT-COMPLETE:0".to_string());
    }

    if let Some(priority) = importance_to_priority(task.importance) {
        lines.push(format!("PRIORITY:{}", priority));
    }

    lines.push("END:VTODO".to_string());
}

#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;

    use chrono::{NaiveDate, NaiveTime};

    use super::*;
    use crate::model::Frequency;

    #[test]
    fn test_escape_text() {
        assert_eq!(
            escape_text("hello, world; test\\one\ntwo"),
            r"hello\, world\; test\\one\ntwo"
        );
    }

    #[test]
    fn test_fold_line_short() {
        let short = "SUMMARY:Short text";
        assert_eq!(fold_line(short), short);
    }

    #[test]
    fn test_fold_line_long() {
        let long = "SUMMARY:This is a very long text that must be folded because it strictly exceeds 75 octets per RFC 5545 requirements.";
        let folded = fold_line(long);
        assert!(folded.contains("\r\n "));
        for segment in folded.split("\r\n") {
            assert!(segment.len() <= 75);
        }
    }

    #[test]
    fn test_format_ics_event_and_task() {
        let event = Event {
            id: 42,
            title: "Встреча команды".into(),
            description: Some("Обсуждение релиза".into()),
            start_date: NaiveDate::from_ymd_opt(2026, 9, 3).unwrap(),
            start_time: Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap()),
            end_time: Some(NaiveTime::from_hms_opt(15, 0, 0).unwrap()),
            importance: Importance::High,
            recurrence_id: None,
            directory: Some(PathBuf::from("/home/user/project")),
        };

        let export_item = EventExportData {
            event,
            tags: vec!["работа".into(), "релиз".into()],
            recurrence: Some(Recurrence {
                id: 1,
                frequency: Frequency::Weekly,
                interval: 1,
                weekdays: vec![Weekday::Mon, Weekday::Thu],
                start_date: NaiveDate::from_ymd_opt(2026, 9, 3).unwrap(),
                end_date: Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
                count: None,
            }),
            favorite_links: vec!["https://meet.google.com/abc".into()],
        };

        let task = Task {
            id: 10,
            title: "Завершить отчет".into(),
            description: Some("Отправить руководству".into()),
            date: Some(NaiveDate::from_ymd_opt(2026, 9, 5).unwrap()),
            is_done: true,
            importance: Importance::Normal,
            completed_at: Some("2026-09-03 12:00:00".into()),
            created_at: "2026-09-03 10:00:00".into(),
            updated_at: "2026-09-03 12:00:00".into(),
        };

        let ics = format_ics(&[export_item], &[task]);

        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("UID:rutendar-event-42@rutendar.local"));
        assert!(ics.contains("SUMMARY:Встреча команды"));
        assert!(ics.contains("DTSTART:20260903T140000"));
        assert!(ics.contains("DTEND:20260903T150000"));
        assert!(ics.contains("PRIORITY:1"));
        assert!(ics.contains("CATEGORIES:работа,релиз"));
        assert!(ics.contains("URL:https://meet.google.com/abc"));
        assert!(ics.contains("RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TH;UNTIL=20261231T235959Z"));
        assert!(ics.contains("DESCRIPTION:"));

        assert!(ics.contains("BEGIN:VTODO"));
        assert!(ics.contains("UID:rutendar-task-10@rutendar.local"));
        assert!(ics.contains("SUMMARY:Завершить отчет"));
        assert!(ics.contains("DUE;VALUE=DATE:20260905"));
        assert!(ics.contains("STATUS:COMPLETED"));
        assert!(ics.contains("PRIORITY:5"));
        assert!(ics.contains("END:VCALENDAR"));

        let parsed = crate::ics::parse_ics(&ics).expect("failed to parse generated ics");
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].title, "Встреча команды");
        assert_eq!(
            parsed.events[0].directory,
            Some(PathBuf::from("/home/user/project"))
        );
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].title, "Завершить отчет");
        assert!(parsed.tasks[0].is_done);
    }
}
