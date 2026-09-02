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
    Neo,
    Light,
    Plain,
}

impl Theme {
    pub fn cycle(self) -> Self {
        match self {
            Self::Neo => Self::Light,
            Self::Light => Self::Plain,
            Self::Plain => Self::Neo,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Neo => "Neo",
            Self::Light => "Light",
            Self::Plain => "Plain",
        }
    }

    pub fn border_type(self) -> BorderType {
        match self {
            Self::Neo | Self::Light => BorderType::Rounded,
            Self::Plain => BorderType::Plain,
        }
    }

    pub fn border_color(self, is_warning: bool) -> Color {
        if is_warning {
            match self {
                Self::Neo => Color::LightRed,
                Self::Light => Color::Red,
                Self::Plain => Color::Reset,
            }
        } else {
            match self {
                Self::Neo => Color::Cyan,
                Self::Light => Color::DarkGray,
                Self::Plain => Color::Reset,
            }
        }
    }

    pub fn selection_style(self) -> Style {
        match self {
            Self::Neo => Style::default().bg(Color::Rgb(24, 34, 52)),
            Self::Light => Style::default().bg(Color::Rgb(220, 225, 235)),
            Self::Plain => Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    pub fn cursor_marker(self, is_selected: bool) -> Span<'static> {
        if is_selected {
            match self {
                Self::Neo => Span::styled(
                    " ▸ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Self::Light => Span::styled(
                    " > ",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Self::Plain => Span::styled(">  ", Style::default().add_modifier(Modifier::BOLD)),
            }
        } else {
            Span::raw("   ")
        }
    }

    pub fn date_icon(self) -> &'static str {
        match self {
            Self::Neo | Self::Light => "📅 ",
            Self::Plain => "[D] ",
        }
    }

    pub fn search_icon(self) -> &'static str {
        match self {
            Self::Neo | Self::Light => "🔍 ",
            Self::Plain => "[?] ",
        }
    }

    pub fn add_icon(self) -> &'static str {
        match self {
            Self::Neo | Self::Light => "➕ ",
            Self::Plain => "+ ",
        }
    }

    pub fn edit_icon(self) -> &'static str {
        match self {
            Self::Neo | Self::Light => "✏️ ",
            Self::Plain => "* ",
        }
    }

    pub fn task_icon(self) -> &'static str {
        match self {
            Self::Neo | Self::Light => "☑️ ",
            Self::Plain => "[T] ",
        }
    }

    pub fn importance_span(self, imp: Importance) -> Span<'static> {
        match self {
            Self::Neo => match imp {
                Importance::High => Span::styled(
                    "▲ ! ",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Importance::Normal => Span::styled("● ", Style::default().fg(Color::Cyan)),
                Importance::Low => Span::styled("· ", Style::default().fg(Color::Blue)),
                Importance::None => Span::raw("  "),
            },
            Self::Light => match imp {
                Importance::High => Span::styled(
                    "! ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Importance::Normal => Span::styled(
                    "* ",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Importance::Low => Span::styled("- ", Style::default().fg(Color::DarkGray)),
                Importance::None => Span::raw("  "),
            },
            Self::Plain => match imp {
                Importance::High => {
                    Span::styled("[!] ", Style::default().add_modifier(Modifier::BOLD))
                }
                Importance::Normal => Span::raw("[.] "),
                Importance::Low => Span::raw("[-] "),
                Importance::None => Span::raw("    "),
            },
        }
    }

    pub fn task_checkbox_span(self, is_done: bool) -> Span<'static> {
        match self {
            Self::Neo => {
                if is_done {
                    Span::styled("✔ [x] ", Style::default().fg(Color::Green))
                } else {
                    Span::styled("☐ [ ] ", Style::default().fg(Color::DarkGray))
                }
            }
            Self::Light => {
                if is_done {
                    Span::styled(
                        "[x] ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
                }
            }
            Self::Plain => {
                if is_done {
                    Span::styled("[X] ", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Span::raw("[ ] ")
                }
            }
        }
    }

    pub fn time_style(self) -> Style {
        match self {
            Self::Neo => Style::default().fg(Color::Yellow),
            Self::Light => Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            Self::Plain => Style::default().add_modifier(Modifier::BOLD),
        }
    }

    pub fn tag_style(self) -> Style {
        match self {
            Self::Neo => Style::default().fg(Color::Cyan),
            Self::Light => Style::default().fg(Color::Blue),
            Self::Plain => Style::default(),
        }
    }

    pub fn title_style(self, is_selected: bool, is_done: bool) -> Style {
        if is_done {
            match self {
                Self::Neo | Self::Light => Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
                Self::Plain => Style::default().add_modifier(Modifier::DIM),
            }
        } else if is_selected {
            match self {
                Self::Neo => Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                Self::Light => Style::default()
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                Self::Plain => Style::default().add_modifier(Modifier::BOLD),
            }
        } else {
            match self {
                Self::Neo => Style::default().fg(Color::White),
                Self::Light => Style::default().fg(Color::Black),
                Self::Plain => Style::default(),
            }
        }
    }

    pub fn active_tab_style(self) -> Style {
        match self {
            Self::Neo => Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Self::Light => Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            Self::Plain => Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    pub fn inactive_tab_style(self) -> Style {
        match self {
            Self::Neo => Style::default().fg(Color::DarkGray),
            Self::Light => Style::default().fg(Color::Gray),
            Self::Plain => Style::default(),
        }
    }

    pub fn key_badge_style(self) -> Style {
        match self {
            Self::Neo => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Self::Light => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            Self::Plain => Style::default().add_modifier(Modifier::BOLD),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_cycle() {
        let t = Theme::Neo;
        assert_eq!(t.cycle(), Theme::Light);
        assert_eq!(t.cycle().cycle(), Theme::Plain);
        assert_eq!(t.cycle().cycle().cycle(), Theme::Neo);
    }

    #[test]
    fn test_plain_theme_properties() {
        let t = Theme::Plain;
        assert_eq!(t.border_type(), BorderType::Plain);
        assert_eq!(t.date_icon(), "[D] ");
        assert_eq!(t.search_icon(), "[?] ");
        assert_eq!(t.importance_span(Importance::High).content, "[!] ");
        assert_eq!(t.task_checkbox_span(true).content, "[X] ");
    }
}
