use chrono::{Duration, NaiveDate};

use super::query::{DateFilter, ItemType, SearchQuery, SearchResult, SortBy, TagMatching};
use crate::{
    calendar::{month_end, month_start, week_end, week_start},
    model::{EventOccurrence, Importance, Note, normalize_tag},
};

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub item_type: ItemType,
    pub importance: Option<Importance>,
    pub date: DateFilter,
    pub tags: Vec<String>,
    pub tag_matching: TagMatching,
    pub sort: SortBy,
}

pub fn date_range(filter: DateFilter, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    match filter {
        DateFilter::All => None,
        DateFilter::Today => Some((today, today)),
        DateFilter::ThisWeek => Some((week_start(today), week_end(today))),
        DateFilter::ThisMonth => Some((month_start(today), month_end(today))),
        DateFilter::Upcoming => Some((today, today + Duration::days(366))),
    }
}

pub fn event_matches(
    event: &EventOccurrence,
    query: &SearchQuery,
    filters: &SearchFilters,
) -> bool {
    if matches!(filters.item_type, ItemType::Notes)
        || matches!(filters.item_type, ItemType::Recurring) && !event.is_recurring
        || filters
            .importance
            .is_some_and(|value| event.importance != value)
    {
        return false;
    }
    let mut haystack = format!(
        "{} {}",
        event.title.to_lowercase(),
        event.description.as_deref().unwrap_or("").to_lowercase()
    );
    for tag in &event.tags {
        haystack.push(' ');
        haystack.push_str(&tag.normalized_name);
    }
    if !query.text.is_empty() && !haystack.contains(&query.text) {
        return false;
    }
    let event_tags: Vec<_> = event
        .tags
        .iter()
        .map(|tag| tag.normalized_name.as_str())
        .collect();
    if !query
        .tags
        .iter()
        .all(|tag| event_tags.contains(&tag.as_str()))
    {
        return false;
    }
    let mut wanted: Vec<_> = filters.tags.iter().map(|tag| normalize_tag(tag)).collect();
    wanted.sort();
    wanted.dedup();
    match filters.tag_matching {
        TagMatching::All => wanted.iter().all(|tag| event_tags.contains(&tag.as_str())),
        TagMatching::Any => {
            wanted.is_empty() || wanted.iter().any(|tag| event_tags.contains(&tag.as_str()))
        }
    }
}

pub fn note_matches(note: &Note, query: &SearchQuery, filters: &SearchFilters) -> bool {
    if !matches!(filters.item_type, ItemType::All | ItemType::Notes)
        || !query.tags.is_empty()
        || !filters.tags.is_empty()
        || filters.importance.is_some()
    {
        return false;
    }
    if query.text.is_empty() {
        return true;
    }
    let mut haystack = format!(
        "{} {}",
        note.title.as_deref().unwrap_or("").to_lowercase(),
        note.body.to_lowercase()
    );
    for link in &note.links {
        haystack.push(' ');
        haystack.push_str(&link.label.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&link.url.to_lowercase());
    }
    haystack.contains(&query.text)
}

pub fn sort_results(results: &mut [SearchResult], sort: SortBy) {
    results.sort_by(|left, right| match sort {
        SortBy::Date => left
            .date()
            .cmp(&right.date())
            .then_with(|| left.title().cmp(right.title())),
        SortBy::Importance => {
            let importance = |item: &SearchResult| match item {
                SearchResult::Event(event) => event.importance,
                SearchResult::Note(_) => Importance::None,
            };
            importance(right)
                .cmp(&importance(left))
                .then_with(|| left.date().cmp(&right.date()))
        }
        SortBy::Title => left
            .title()
            .to_lowercase()
            .cmp(&right.title().to_lowercase()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Event, Tag};

    #[test]
    fn query_tags_are_always_and_while_filter_tags_can_be_any() {
        let event = Event {
            id: 1,
            title: "Лекция".into(),
            description: None,
            start_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            start_time: None,
            end_time: None,
            importance: Importance::Normal,
            recurrence_id: None,
        };
        let tags = ["универ", "лекция"]
            .into_iter()
            .enumerate()
            .map(|(id, name)| Tag {
                id: id as i64,
                name: name.into(),
                normalized_name: name.into(),
            })
            .collect();
        let occurrence = EventOccurrence::from_event(&event, tags);

        let query = SearchQuery {
            text: String::new(),
            tags: vec!["универ".into(), "экзамен".into()],
        };
        let mut filters = SearchFilters {
            tags: vec!["экзамен".into(), "лекция".into()],
            tag_matching: TagMatching::Any,
            ..SearchFilters::default()
        };
        assert!(!event_matches(&occurrence, &query, &filters));
        filters.tags = vec!["экзамен".into(), "лекция".into()];
        assert!(event_matches(
            &occurrence,
            &SearchQuery {
                text: String::new(),
                tags: vec!["универ".into()],
            },
            &filters
        ));
    }
}
