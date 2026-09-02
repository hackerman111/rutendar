use std::{error::Error, path::PathBuf};

use serde::Deserialize;

use crate::{model::Importance, ui::Theme};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub week_start: String,
    pub agenda: AgendaConfig,
    pub importance: ImportanceConfig,
    pub keys: KeyConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AgendaConfig {
    pub next_events: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ImportanceConfig {
    pub none_symbol: String,
    pub low_symbol: String,
    pub normal_symbol: String,
    pub high_symbol: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub open_link: char,
    pub copy_link: char,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct UiConfig {
    pub show_week_numbers: bool,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            week_start: "monday".into(),
            agenda: AgendaConfig::default(),
            importance: ImportanceConfig::default(),
            keys: KeyConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for AgendaConfig {
    fn default() -> Self {
        Self { next_events: 4 }
    }
}

impl Default for ImportanceConfig {
    fn default() -> Self {
        Self {
            none_symbol: " ".into(),
            low_symbol: "·".into(),
            normal_symbol: "•".into(),
            high_symbol: "!".into(),
        }
    }
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            open_link: 'o',
            copy_link: 'y',
        }
    }
}

impl Config {
    pub fn load() -> Result<(Self, Paths), Box<dyn Error>> {
        let paths = Paths::discover()?;
        let config = if paths.config.exists() {
            toml::from_str(&std::fs::read_to_string(&paths.config)?)?
        } else {
            Self::default()
        };
        config.validate()?;
        Ok((config, paths))
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if !self.week_start.eq_ignore_ascii_case("monday") {
            return Err(invalid("only monday week_start is supported"));
        }
        if self.agenda.next_events == 0 {
            return Err(invalid("agenda.next_events must be positive"));
        }
        if self.keys.open_link == self.keys.copy_link {
            return Err(invalid("open_link and copy_link must differ"));
        }
        const FIXED_KEYS: &str = "qhjklgnedatpwDmYc?/frisA[] ";
        if [self.keys.open_link, self.keys.copy_link]
            .into_iter()
            .any(|key| FIXED_KEYS.contains(key))
        {
            return Err(invalid("link keys conflict with a fixed binding"));
        }
        for symbol in [
            &self.importance.none_symbol,
            &self.importance.low_symbol,
            &self.importance.normal_symbol,
            &self.importance.high_symbol,
        ] {
            if symbol.chars().count() != 1 {
                return Err(invalid("importance symbols must contain one character"));
            }
        }
        Ok(())
    }

    pub fn importance_symbol(&self, importance: Importance) -> &str {
        match importance {
            Importance::None => &self.importance.none_symbol,
            Importance::Low => &self.importance.low_symbol,
            Importance::Normal => &self.importance.normal_symbol,
            Importance::High => &self.importance.high_symbol,
        }
    }
}

pub struct Paths {
    pub database: PathBuf,
    pub config: PathBuf,
}

impl Paths {
    fn discover() -> Result<Self, Box<dyn Error>> {
        let home = std::env::var_os("HOME").ok_or_else(|| invalid("HOME is not set"))?;
        let config_root = std::env::var_os("XDG_CONFIG_HOME")
            .unwrap_or_else(|| PathBuf::from(&home).join(".config").into_os_string());
        let data_root = std::env::var_os("XDG_DATA_HOME")
            .unwrap_or_else(|| PathBuf::from(&home).join(".local/share").into_os_string());
        Ok(Self {
            database: PathBuf::from(data_root).join("rutendar/calendar.db"),
            config: PathBuf::from(config_root).join("rutendar/config.toml"),
        })
    }
}

fn invalid(message: &'static str) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_keys_must_remain_reachable() {
        let mut config = Config::default();
        config.keys.copy_link = config.keys.open_link;
        assert!(config.validate().is_err());
        config.keys.copy_link = 'd';
        assert!(config.validate().is_err());
    }

    #[test]
    fn theme_can_be_configured_from_toml() {
        let toml_str = r#"
            [ui]
            theme = "ascii"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ui.theme, Theme::Ascii);

        let toml_str_plain = r#"
            [ui]
            theme = "plain"
        "#;
        let config_plain: Config = toml::from_str(toml_str_plain).unwrap();
        assert_eq!(config_plain.ui.theme, Theme::Ascii);

        let toml_str_default = r#"
            [ui]
            theme = "default"
        "#;
        let config_default: Config = toml::from_str(toml_str_default).unwrap();
        assert_eq!(config_default.ui.theme, Theme::Default);

        let toml_str_neo = r#"
            [ui]
            theme = "neo"
        "#;
        let config_neo: Config = toml::from_str(toml_str_neo).unwrap();
        assert_eq!(config_neo.ui.theme, Theme::Default);
    }
}
