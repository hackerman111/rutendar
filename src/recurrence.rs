use std::collections::HashMap;

use chrono::{Duration, NaiveDate};

use crate::{
    calendar::week_start,
    model::{Event, EventOccurrence, ExceptionKind, Recurrence, RecurrenceException, Tag},
};

pub const MAX_INTERVAL_WEEKS: u32 = 5_200;

pub fn expand_weekly(
    event: &Event,
    rule: &Recurrence,
    exceptions: &[RecurrenceException],
    replacements: &HashMap<i64, (Event, Vec<Tag>)>,
    tags: &[Tag],
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<EventOccurrence> {
    if rule.interval == 0 || rule.interval > MAX_INTERVAL_WEEKS || range_start > range_end {
        return Vec::new();
    }

    let exception_by_date: HashMap<_, _> = exceptions
        .iter()
        .map(|exception| (exception.original_date, exception))
        .collect();
    let first_week = week_start(rule.start_date);
    let last_date = rule.end_date.unwrap_or(range_end).min(range_end);
    let mut week = if rule.count.is_none() && range_start > rule.start_date {
        let range_week = week_start(range_start);
        let elapsed_weeks = (range_week - first_week).num_weeks().max(0);
        let aligned_weeks = elapsed_weeks / i64::from(rule.interval) * i64::from(rule.interval);
        let Some(mut candidate) = first_week.checked_add_signed(Duration::weeks(aligned_weeks))
        else {
            return Vec::new();
        };
        if candidate
            .checked_add_signed(Duration::days(6))
            .is_some_and(|end| end < range_start)
        {
            let Some(next) =
                candidate.checked_add_signed(Duration::weeks(i64::from(rule.interval)))
            else {
                return Vec::new();
            };
            candidate = next;
        }
        candidate
    } else {
        first_week
    };
    let mut ordinal = 0_u32;
    let mut occurrences = Vec::new();
    let mut added_replacements = std::collections::HashSet::new();
    let mut weekdays = rule.weekdays.clone();
    weekdays.sort_by_key(|day| day.num_days_from_monday());
    weekdays.dedup();

    'weeks: while week <= last_date {
        for weekday in &weekdays {
            let Some(date) =
                week.checked_add_signed(Duration::days(i64::from(weekday.num_days_from_monday())))
            else {
                continue;
            };
            if date < rule.start_date || rule.end_date.is_some_and(|end| date > end) {
                continue;
            }
            if rule.count.is_some_and(|count| ordinal >= count) {
                break 'weeks;
            }
            ordinal += 1;
            if date < range_start || date > range_end {
                continue;
            }

            match exception_by_date.get(&date).map(|value| value.kind) {
                Some(ExceptionKind::Cancelled) => continue,
                Some(ExceptionKind::Modified) => {
                    let Some(replacement_id) = exception_by_date[&date].replacement_event_id else {
                        continue;
                    };
                    if let Some((replacement, replacement_tags)) = replacements.get(&replacement_id)
                        && (range_start..=range_end).contains(&replacement.start_date)
                    {
                        let mut occurrence =
                            EventOccurrence::from_event(replacement, replacement_tags.clone());
                        occurrence.recurrence_id = Some(rule.id);
                        occurrence.original_date = date;
                        occurrence.is_recurring = true;
                        added_replacements.insert(replacement_id);
                        occurrences.push(occurrence);
                    }
                    continue;
                }
                None => {}
            }

            let mut occurrence = EventOccurrence::from_event(event, tags.to_vec());
            occurrence.original_date = date;
            occurrence.date = date;
            occurrence.recurrence_id = Some(rule.id);
            occurrence.is_recurring = true;
            occurrences.push(occurrence);
        }
        let Some(next_week) = week.checked_add_signed(Duration::weeks(i64::from(rule.interval)))
        else {
            break;
        };
        week = next_week;
    }

    for exception in exceptions {
        let Some(replacement_id) = exception.replacement_event_id else {
            continue;
        };
        if added_replacements.contains(&replacement_id) {
            continue;
        }
        if let Some((replacement, replacement_tags)) = replacements.get(&replacement_id)
            && (range_start..=range_end).contains(&replacement.start_date)
        {
            let mut occurrence = EventOccurrence::from_event(replacement, replacement_tags.clone());
            occurrence.recurrence_id = Some(rule.id);
            occurrence.original_date = exception.original_date;
            occurrence.is_recurring = true;
            occurrences.push(occurrence);
        }
    }

    occurrences
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveTime, Timelike, Weekday};

    use crate::model::{Frequency, Importance};

    use super::*;

    fn event() -> Event {
        Event {
            id: 1,
            title: "Лекция".into(),
            description: None,
            start_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            start_time: NaiveTime::from_hms_opt(14, 40, 0),
            end_time: None,
            importance: Importance::Normal,
            recurrence_id: Some(1),
        }
    }

    fn rule() -> Recurrence {
        Recurrence {
            id: 1,
            frequency: Frequency::Weekly,
            interval: 1,
            weekdays: vec![Weekday::Tue],
            start_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            end_date: Some(NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()),
            count: None,
        }
    }

    #[test]
    fn expands_weekly_interval_and_multiple_weekdays() {
        let mut rule = rule();
        rule.interval = 2;
        rule.weekdays = vec![Weekday::Tue, Weekday::Thu];
        let dates: Vec<_> = expand_weekly(
            &event(),
            &rule,
            &[],
            &HashMap::new(),
            &[],
            rule.start_date,
            rule.end_date.unwrap(),
        )
        .into_iter()
        .map(|item| item.date)
        .collect();
        assert_eq!(
            dates,
            [
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 3).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
            ]
        );

        let later_dates: Vec<_> = expand_weekly(
            &event(),
            &rule,
            &[],
            &HashMap::new(),
            &[],
            NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(),
            rule.end_date.unwrap(),
        )
        .into_iter()
        .map(|item| item.date)
        .collect();
        assert_eq!(later_dates, dates[2..]);
    }

    #[test]
    fn applies_cancelled_and_modified_exceptions() {
        let mut replacement = event();
        replacement.id = 2;
        replacement.start_date = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        replacement.start_time = NaiveTime::from_hms_opt(16, 20, 0);
        replacement.recurrence_id = None;
        let exceptions = vec![
            RecurrenceException {
                recurrence_id: 1,
                original_date: NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
                kind: ExceptionKind::Cancelled,
                replacement_event_id: None,
            },
            RecurrenceException {
                recurrence_id: 1,
                original_date: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
                kind: ExceptionKind::Modified,
                replacement_event_id: Some(2),
            },
        ];
        let replacements = HashMap::from([(2, (replacement, Vec::new()))]);
        let result = expand_weekly(
            &event(),
            &rule(),
            &exceptions,
            &replacements,
            &[],
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 22).unwrap(),
        );
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].date.day(), 1);
        assert_eq!(result[1].start_time.unwrap().hour(), 16);
        assert_eq!(result[2].date.day(), 22);
    }

    #[test]
    fn overflowing_legacy_interval_stops_without_panicking() {
        let mut event = event();
        event.start_date = NaiveDate::MAX;
        let mut rule = rule();
        rule.start_date = NaiveDate::MAX;
        rule.end_date = None;
        rule.interval = u32::MAX;
        rule.weekdays = vec![NaiveDate::MAX.weekday()];

        let result = expand_weekly(
            &event,
            &rule,
            &[],
            &HashMap::new(),
            &[],
            NaiveDate::MAX,
            NaiveDate::MAX,
        );
        assert!(result.is_empty());
    }
}
