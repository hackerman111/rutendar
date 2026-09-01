use std::fmt::Write;

use chrono::{Datelike, Weekday};
use unicode_width::UnicodeWidthStr;

use crate::model::{EventOccurrence, Importance};

fn strip_ansi(s: &str) -> String {
    let mut clean = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            clean.push(c);
        }
    }
    clean
}

fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

fn push_card_line(out: &mut String, content: &str, inner_width: usize) {
    let vis_len = visible_width(content);
    if vis_len <= inner_width {
        let pad = " ".repeat(inner_width - vis_len);
        let _ = writeln!(out, "\x1b[36m│\x1b[0m {content}{pad} \x1b[36m│\x1b[0m");
    } else {
        let clean = strip_ansi(content);
        let mut truncated = String::new();
        let mut cur_width = 0;
        for c in clean.chars() {
            let cw = c.to_string().width();
            if cur_width + cw + 1 > inner_width {
                break;
            }
            truncated.push(c);
            cur_width += cw;
        }
        let pad = " ".repeat(inner_width.saturating_sub(cur_width + 1));
        let _ = writeln!(out, "\x1b[36m│\x1b[0m {truncated}…{pad} \x1b[36m│\x1b[0m");
    }
}

/// Format an event occurrence into a beautiful terminal card with a fully closed frame.
pub fn format_event_card(event: &EventOccurrence) -> String {
    let mut out = String::new();

    let (badge_text, badge_style) = match event.importance {
        Importance::High => (" ВЫСОКАЯ ВАЖНОСТЬ ", "\x1b[41;97;1m"),
        Importance::Normal => (" ОБЫЧНАЯ ВАЖНОСТЬ ", "\x1b[46;30;1m"),
        Importance::Low => (" НИЗКАЯ ВАЖНОСТЬ ", "\x1b[44;97;1m"),
        Importance::None => (" СОБЫТИЕ ", "\x1b[100;97;1m"),
    };
    let badge_vis_width = badge_text.width();
    let badge_formatted = format!("{badge_style}{badge_text}\x1b[0m");

    let weekday_str = match event.date.weekday() {
        Weekday::Mon => "Понедельник",
        Weekday::Tue => "Вторник",
        Weekday::Wed => "Среда",
        Weekday::Thu => "Четверг",
        Weekday::Fri => "Пятница",
        Weekday::Sat => "Суббота",
        Weekday::Sun => "Воскресенье",
    };

    let time_str = match (event.start_time, event.end_time) {
        (Some(start), Some(end)) => {
            format!("{} — {}", start.format("%H:%M"), end.format("%H:%M"))
        }
        (Some(start), None) => format!("{}", start.format("%H:%M")),
        _ => "Весь день".to_string(),
    };

    let recurrence_str = if event.is_recurring {
        " \x1b[35m↻ (повторяющееся)\x1b[0m"
    } else {
        ""
    };

    let card_width = 72usize;
    let inner_width = card_width.saturating_sub(4); // 1 left bar + 1 space + inner + 1 space + 1 right bar
    let divider = format!("\x1b[36m├{}┤\x1b[0m", "─".repeat(card_width - 2));

    // Top border: ╭── <badge> ──────...──╮
    let prefix_width = 4; // "╭── "
    let suffix_margin = 1; // " " after badge
    let corner_width = 1; // "╮"
    let used_width = prefix_width + badge_vis_width + suffix_margin + corner_width;
    let remaining_dashes = card_width.saturating_sub(used_width);

    let _ = writeln!(
        out,
        "\x1b[36m╭──\x1b[0m {badge_formatted} \x1b[36m{}╮\x1b[0m",
        "─".repeat(remaining_dashes)
    );

    // Date & Time line
    let date_line = format!(
        "\x1b[1m📅 {weekday_str}, {}\x1b[0m \x1b[33m· {time_str}\x1b[0m{recurrence_str}",
        event.date.format("%d.%m.%Y")
    );
    push_card_line(&mut out, &date_line, inner_width);

    // Title line
    let title_line = format!("\x1b[1;32m📌 {}\x1b[0m", event.title);
    push_card_line(&mut out, &title_line, inner_width);

    // Tags
    if !event.tags.is_empty() {
        let tags_formatted = event
            .tags
            .iter()
            .map(|t| format!("\x1b[36m#{}\x1b[0m", t.name))
            .collect::<Vec<_>>()
            .join("  ");
        let tags_line = format!("🏷  {tags_formatted}");
        push_card_line(&mut out, &tags_line, inner_width);
    }

    // Directory
    if let Some(dir) = &event.directory {
        let dir_line = format!("📁 \x1b[34m{}\x1b[0m", dir.display());
        push_card_line(&mut out, &dir_line, inner_width);
    }

    // Description
    if let Some(desc) = &event.description
        && !desc.trim().is_empty()
    {
        let _ = writeln!(out, "{divider}");
        push_card_line(&mut out, "\x1b[1mОписание:\x1b[0m", inner_width);
        for line in desc.lines() {
            let desc_line = format!("  {line}");
            push_card_line(&mut out, &desc_line, inner_width);
        }
    }

    // Links
    if !event.favorite_links.is_empty() {
        let _ = writeln!(out, "{divider}");
        push_card_line(&mut out, "\x1b[1mСсылки:\x1b[0m", inner_width);
        for link in &event.favorite_links {
            let link_line = format!(
                "  🔗 \x1b[1m{}\x1b[0m \x1b[90m›\x1b[0m \x1b[4;36m{}\x1b[0m",
                link.label, link.url
            );
            push_card_line(&mut out, &link_line, inner_width);
            if let Some(desc) = &link.description
                && !desc.trim().is_empty()
            {
                let sub_line = format!("     \x1b[90m↳ {desc}\x1b[0m");
                push_card_line(&mut out, &sub_line, inner_width);
            }
        }
    }

    // Bottom border
    let bottom = format!("\x1b[36m╰{}╯\x1b[0m", "─".repeat(card_width - 2));
    let _ = write!(out, "{bottom}");
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{NaiveDate, NaiveTime};

    use super::*;
    use crate::model::{FavoriteLink, Tag};

    #[test]
    fn format_card_includes_essential_fields_and_is_closed() {
        let event = EventOccurrence {
            event_id: 1,
            recurrence_id: None,
            original_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            start_time: Some(NaiveTime::from_hms_opt(14, 40, 0).unwrap()),
            end_time: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            title: "Коллоквиум".into(),
            description: Some("Билеты 1-15".into()),
            importance: Importance::High,
            tags: vec![Tag {
                id: 1,
                name: "матан".into(),
                normalized_name: "матан".into(),
            }],
            favorite_links: vec![FavoriteLink {
                id: 1,
                label: "Вопросы".into(),
                url: "https://example.com/q".into(),
                description: Some("список".into()),
                tags: String::new(),
            }],
            directory: Some(PathBuf::from("/home/user/study")),
            is_recurring: false,
        };

        let card = format_event_card(&event);
        assert!(card.contains("ВЫСОКАЯ ВАЖНОСТЬ"));
        assert!(!card.contains("[ ВЫСОКАЯ ВАЖНОСТЬ ]")); // Full background fill, no brackets
        assert!(card.contains("Коллоквиум"));
        assert!(card.contains("14:40 — 16:00"));
        assert!(card.contains("01.09.2026"));
        assert!(card.contains("#матан"));
        assert!(card.contains("Билеты 1-15"));
        assert!(card.contains("Вопросы"));
        assert!(card.contains("https://example.com/q"));
        assert!(card.contains("/home/user/study"));

        // Verify closed borders
        assert!(card.contains('╭') && card.contains('╮'));
        assert!(card.contains('╰') && card.contains('╯'));
        assert!(card.contains('├') && card.contains('┤'));

        for line in card.lines() {
            let clean = strip_ansi(line);
            assert_eq!(
                clean.width(),
                72,
                "line visual width should be 72: '{clean}'"
            );
        }
    }
}
