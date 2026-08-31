use super::*;

#[derive(Debug, Clone)]
pub struct EventForm {
    pub title: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub importance: Importance,
    pub tags: String,
    pub weekly: bool,
    pub interval: String,
    pub weekdays: String,
    pub ends: String,
    pub description: String,
    pub active: usize,
}

impl EventForm {
    pub const FIELD_COUNT: usize = 11;

    pub(super) fn new(date: NaiveDate) -> Self {
        Self {
            title: String::new(),
            date: date.format("%d.%m.%Y").to_string(),
            start_time: String::new(),
            end_time: String::new(),
            importance: Importance::Normal,
            tags: String::new(),
            weekly: false,
            interval: "1".into(),
            weekdays: weekday_name(date.weekday()).into(),
            ends: String::new(),
            description: String::new(),
            active: 0,
        }
    }

    pub(super) fn from_event(
        event: &Event,
        tags: &[Tag],
        recurrence: Option<&crate::model::Recurrence>,
    ) -> Self {
        Self {
            title: event.title.clone(),
            date: event.start_date.format("%d.%m.%Y").to_string(),
            start_time: event
                .start_time
                .map(|time| time.format("%H:%M").to_string())
                .unwrap_or_default(),
            end_time: event
                .end_time
                .map(|time| time.format("%H:%M").to_string())
                .unwrap_or_default(),
            importance: event.importance,
            tags: tags
                .iter()
                .map(|tag| format!("#{}", tag.name))
                .collect::<Vec<_>>()
                .join(" "),
            weekly: recurrence.is_some(),
            interval: recurrence.map_or_else(|| "1".into(), |rule| rule.interval.to_string()),
            weekdays: recurrence.map_or_else(
                || weekday_name(event.start_date.weekday()).into(),
                |rule| {
                    rule.weekdays
                        .iter()
                        .map(|day| weekday_name(*day))
                        .collect::<Vec<_>>()
                        .join(",")
                },
            ),
            ends: recurrence
                .and_then(|rule| rule.end_date)
                .map(|date| date.format("%d.%m.%Y").to_string())
                .unwrap_or_default(),
            description: event.description.clone().unwrap_or_default(),
            active: 0,
        }
    }

    pub(super) fn from_occurrence(event: &EventOccurrence) -> Self {
        let synthetic = Event {
            id: event.event_id,
            title: event.title.clone(),
            description: event.description.clone(),
            start_date: event.date,
            start_time: event.start_time,
            end_time: event.end_time,
            importance: event.importance,
            recurrence_id: None,
        };
        let mut form = Self::from_event(&synthetic, &event.tags, None);
        form.weekly = false;
        form
    }

    pub fn fields(&self) -> [(&'static str, String); Self::FIELD_COUNT] {
        [
            ("TITLE", self.title.clone()),
            ("DATE", self.date.clone()),
            ("TIME", self.start_time.clone()),
            ("END TIME", self.end_time.clone()),
            ("IMPORTANCE", self.importance.to_string()),
            ("TAGS", self.tags.clone()),
            (
                "REPEAT",
                if self.weekly { "Weekly" } else { "Never" }.into(),
            ),
            ("INTERVAL", self.interval.clone()),
            ("WEEKDAYS", self.weekdays.clone()),
            ("ENDS", self.ends.clone()),
            ("DESCRIPTION", self.description.clone()),
        ]
    }

    pub(super) fn push(&mut self, character: char) {
        match self.active {
            0 => self.title.push(character),
            1 => self.date.push(character),
            2 => self.start_time.push(character),
            3 => self.end_time.push(character),
            5 => self.tags.push(character),
            7 => self.interval.push(character),
            8 => self.weekdays.push(character),
            9 => self.ends.push(character),
            10 => self.description.push(character),
            _ => {}
        }
    }

    pub(super) fn backspace(&mut self) {
        match self.active {
            0 => _ = self.title.pop(),
            1 => _ = self.date.pop(),
            2 => _ = self.start_time.pop(),
            3 => _ = self.end_time.pop(),
            5 => _ = self.tags.pop(),
            7 => _ = self.interval.pop(),
            8 => _ = self.weekdays.pop(),
            9 => _ = self.ends.pop(),
            10 => _ = self.description.pop(),
            _ => {}
        }
    }

    pub(super) fn adjust(&mut self, forward: bool) {
        match self.active {
            4 => {
                self.importance = if forward {
                    self.importance.next()
                } else {
                    match self.importance {
                        Importance::None => Importance::High,
                        Importance::Low => Importance::None,
                        Importance::Normal => Importance::Low,
                        Importance::High => Importance::Normal,
                    }
                }
            }
            6 => self.weekly = !self.weekly,
            _ => {}
        }
    }

    pub(super) fn values(&self) -> AppResult<(NewEvent, Option<NewRecurrence>, Vec<String>)> {
        let date = parse_date(&self.date)?;
        let start_time = optional_time(&self.start_time)?;
        let end_time = optional_time(&self.end_time)?;
        let event = NewEvent {
            title: self.title.trim().into(),
            description: (!self.description.trim().is_empty())
                .then(|| self.description.trim().into()),
            start_date: date,
            start_time,
            end_time,
            importance: self.importance,
        };
        let recurrence = if self.weekly {
            let weekdays = parse_weekdays(&self.weekdays)?;
            Some(NewRecurrence {
                interval: self.interval.trim().parse()?,
                weekdays,
                start_date: date,
                end_date: (!self.ends.trim().is_empty())
                    .then(|| parse_date(&self.ends))
                    .transpose()?,
                count: None,
            })
        } else {
            None
        };
        let tags = self
            .tags
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|tag| !tag.trim_matches('#').is_empty())
            .map(str::to_owned)
            .collect();
        Ok((event, recurrence, tags))
    }
}

#[derive(Debug, Clone)]
pub struct NoteForm {
    pub title: String,
    pub date: String,
    pub body: String,
    pub active: usize,
}

#[derive(Debug, Clone)]
pub struct LinkForm {
    pub label: String,
    pub url: String,
    pub note_id: NoteId,
    pub active: usize,
}

#[derive(Debug, Clone)]
pub enum Editor {
    Event {
        form: EventForm,
        target: EventTarget,
    },
    Note {
        form: NoteForm,
        target: Option<NoteId>,
    },
    Link {
        form: LinkForm,
        target: Option<LinkId>,
    },
}

#[derive(Debug, Clone)]
pub enum EventTarget {
    New,
    Event(EventId),
    Occurrence {
        recurrence_id: RecurrenceId,
        original_date: NaiveDate,
    },
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Event(EventId),
    Recurrence(RecurrenceId),
    Occurrence(RecurrenceId, NaiveDate),
    Note(NoteId),
    Link(LinkId),
}

#[derive(Debug, Clone)]
pub enum ScopeOperation {
    Edit(EventOccurrence),
    Delete(EventOccurrence),
    Importance(EventOccurrence),
}

#[derive(Debug, Clone)]
pub enum Popup {
    Editor(Editor),
    Confirm {
        message: String,
        target: DeleteTarget,
    },
    Scope(ScopeOperation),
    GotoDate(String),
    Help,
}

fn optional_time(value: &str) -> AppResult<Option<NaiveTime>> {
    (!value.trim().is_empty())
        .then(|| parse_time(value))
        .transpose()
        .map_err(Into::into)
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "ПН",
        Weekday::Tue => "ВТ",
        Weekday::Wed => "СР",
        Weekday::Thu => "ЧТ",
        Weekday::Fri => "ПТ",
        Weekday::Sat => "СБ",
        Weekday::Sun => "ВС",
    }
}

fn parse_weekdays(value: &str) -> AppResult<Vec<Weekday>> {
    let mut days = Vec::new();
    for token in value.split(|character: char| character.is_whitespace() || character == ',') {
        if token.is_empty() {
            continue;
        }
        let day = match token.to_uppercase().as_str() {
            "ПН" | "MON" | "1" => Weekday::Mon,
            "ВТ" | "TUE" | "2" => Weekday::Tue,
            "СР" | "WED" | "3" => Weekday::Wed,
            "ЧТ" | "THU" | "4" => Weekday::Thu,
            "ПТ" | "FRI" | "5" => Weekday::Fri,
            "СБ" | "SAT" | "6" => Weekday::Sat,
            "ВС" | "SUN" | "7" => Weekday::Sun,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid weekday",
                )
                .into());
            }
        };
        if !days.contains(&day) {
            days.push(day);
        }
    }
    if days.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "at least one weekday is required",
        )
        .into());
    }
    Ok(days)
}
