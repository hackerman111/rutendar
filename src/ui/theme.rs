use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::BorderType,
};
use serde::{Deserialize, Serialize};

use crate::model::Importance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    #[serde(alias = "neo", alias = "dark", alias = "classic")]
    Default,
    #[serde(alias = "plain")]
    Ascii,
}

impl Theme {
    pub fn cycle(self) -> Self {
        match self {
            Self::Default => Self::Ascii,
            Self::Ascii => Self::Default,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Ascii => "ASCII",
        }
    }

    pub fn border_type(self) -> BorderType {
        match self {
            Self::Default => BorderType::Rounded,
            Self::Ascii => BorderType::Plain,
        }
    }

    pub fn border_color(self, is_warning: bool) -> Color {
        if is_warning {
            match self {
                Self::Default => Color::LightRed,
                Self::Ascii => Color::Reset,
            }
        } else {
            match self {
                Self::Default => Color::Cyan,
                Self::Ascii => Color::Reset,
            }
        }
    }

    pub fn selection_style(self) -> Style {
        match self {
            Self::Default => Style::default().bg(Color::Rgb(24, 34, 52)),
            Self::Ascii => Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    pub fn cursor_marker(self, is_selected: bool) -> Span<'static> {
        if is_selected {
            match self {
                Self::Default => Span::styled(
                    " ▸ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Self::Ascii => Span::styled(">  ", Style::default().add_modifier(Modifier::BOLD)),
            }
        } else {
            Span::raw("   ")
        }
    }

    pub fn pin_icon(self) -> &'static str {
        match self {
            Self::Default => "📌 ",
            Self::Ascii => "[P] ",
        }
    }
    pub fn time_icon(self) -> &'static str {
        match self {
            Self::Default => "⏰ ",
            Self::Ascii => "[T] ",
        }
    }
    pub fn important_icon(self) -> &'static str {
        match self {
            Self::Default => "⚡ ",
            Self::Ascii => "[P] ",
        }
    }
    pub fn tag_icon(self) -> &'static str {
        match self {
            Self::Default => "🏷 ",
            Self::Ascii => "[#] ",
        }
    }

    pub fn dir_icon(self) -> &'static str {
        match self {
            Self::Default => "📁 ",
            Self::Ascii => "[D] ",
        }
    }

    pub fn note_icon(self) -> &'static str {
        match self {
            Self::Default => "📝 ",
            Self::Ascii => "[N] ",
        }
    }

    pub fn date_icon(self) -> &'static str {
        match self {
            Self::Default => "📅 ",
            Self::Ascii => "[D] ",
        }
    }

    pub fn search_icon(self) -> &'static str {
        match self {
            Self::Default => "🔍 ",
            Self::Ascii => "[?] ",
        }
    }

    pub fn add_icon(self) -> &'static str {
        match self {
            Self::Default => "➕ ",
            Self::Ascii => "+ ",
        }
    }

    pub fn edit_icon(self) -> &'static str {
        match self {
            Self::Default => "✏️ ",
            Self::Ascii => "* ",
        }
    }

    pub fn task_icon(self) -> &'static str {
        match self {
            Self::Default => "☑️ ",
            Self::Ascii => "[T] ",
        }
    }

    pub fn importance_span(self, imp: Importance) -> Span<'static> {
        match self {
            Self::Default => match imp {
                Importance::High => Span::styled(
                    "▲ ! ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Importance::Normal => Span::styled("• ", Style::default().fg(Color::Yellow)),
                Importance::Low => Span::styled("· ", Style::default().fg(Color::Blue)),
                Importance::None => Span::raw("  "),
            },
            Self::Ascii => match imp {
                Importance::High => {
                    Span::styled("[!] ", Style::default().add_modifier(Modifier::BOLD))
                }
                Importance::Normal => Span::styled("[.] ", Style::default()),
                Importance::Low => Span::styled("[-] ", Style::default().fg(Color::DarkGray)),
                Importance::None => Span::raw("    "),
            },
        }
    }

    pub fn task_checkbox_span(self, is_done: bool) -> Span<'static> {
        match self {
            Self::Default => {
                if is_done {
                    Span::styled("✔ [x] ", Style::default().fg(Color::Green))
                } else {
                    Span::styled("☐ [ ] ", Style::default().fg(Color::DarkGray))
                }
            }
            Self::Ascii => {
                if is_done {
                    Span::styled("[X] ", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("[ ] ", Style::default())
                }
            }
        }
    }

    pub fn time_style(self) -> Style {
        match self {
            Self::Default => Style::default().fg(Color::Yellow),
            Self::Ascii => Style::default().add_modifier(Modifier::BOLD),
        }
    }

    pub fn tag_style(self) -> Style {
        match self {
            Self::Default => Style::default().fg(Color::Cyan),
            Self::Ascii => Style::default(),
        }
    }

    pub fn title_style(self, is_selected: bool, is_done: bool) -> Style {
        if is_done {
            match self {
                Self::Default => Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
                Self::Ascii => Style::default().add_modifier(Modifier::DIM),
            }
        } else if is_selected {
            match self {
                Self::Default => Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                Self::Ascii => Style::default().add_modifier(Modifier::BOLD),
            }
        } else {
            match self {
                Self::Default => Style::default().fg(Color::White),
                Self::Ascii => Style::default(),
            }
        }
    }

    pub fn active_tab_style(self) -> Style {
        match self {
            Self::Default => Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Self::Ascii => Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    pub fn inactive_tab_style(self) -> Style {
        match self {
            Self::Default => Style::default().fg(Color::DarkGray),
            Self::Ascii => Style::default(),
        }
    }

    pub fn key_badge_style(self) -> Style {
        match self {
            Self::Default => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Self::Ascii => Style::default().add_modifier(Modifier::BOLD),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_cycle() {
        let t = Theme::Default;
        assert_eq!(t.cycle(), Theme::Ascii);
        assert_eq!(t.cycle().cycle(), Theme::Default);
    }

    #[test]
    fn test_ascii_theme_properties() {
        let t = Theme::Ascii;
        assert_eq!(t.border_type(), BorderType::Plain);
        assert_eq!(t.date_icon(), "[D] ");
        assert_eq!(t.search_icon(), "[?] ");
        assert_eq!(t.add_icon(), "+ ");
        assert_eq!(t.task_checkbox_span(true).content, "[X] ");
    }
}
