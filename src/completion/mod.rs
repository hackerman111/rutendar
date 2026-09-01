use std::path::PathBuf;

use crate::storage::Database;

/// Expand leading `~` or `~/` to the user's home directory.
pub fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        dirs_home().unwrap_or_else(|| PathBuf::from("~"))
    } else if let Some(stripped) = input.strip_prefix("~/") {
        dirs_home()
            .map(|home| home.join(stripped))
            .unwrap_or_else(|| PathBuf::from(input))
    } else {
        PathBuf::from(input)
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Find matching directories for the given path input.
/// Returns completed path strings formatted with trailing `/`.
pub fn complete_directories(input: &str, max_results: usize) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let (parent_display, expanded_parent, prefix) = if input == "~" {
        let home = dirs_home().unwrap_or_else(|| PathBuf::from("~"));
        ("~".to_string(), home, "".to_string())
    } else if let Some(stripped) = input.strip_prefix("~/") {
        let home = dirs_home().unwrap_or_else(|| PathBuf::from("~"));
        if let Some(idx) = stripped.rfind('/') {
            let sub = &stripped[..=idx];
            let rest = &stripped[idx + 1..];
            (
                format!("~/{sub}"),
                home.join(&stripped[..idx]),
                rest.to_string(),
            )
        } else {
            ("~".to_string(), home, stripped.to_string())
        }
    } else if let Some(idx) = input.rfind('/') {
        let parent_str = &input[..=idx];
        let rest = &input[idx + 1..];
        let parent_path = if parent_str == "/" {
            PathBuf::from("/")
        } else {
            PathBuf::from(&input[..idx])
        };
        (parent_str.to_string(), parent_path, rest.to_string())
    } else {
        (String::new(), PathBuf::from("."), input.to_string())
    };

    let entries = match std::fs::read_dir(&expanded_parent) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !prefix.starts_with('.') && file_name.starts_with('.') {
            continue;
        }

        if file_name.starts_with(&prefix) {
            let sep = if parent_display.is_empty() || parent_display.ends_with('/') {
                ""
            } else {
                "/"
            };
            matches.push(format!("{parent_display}{sep}{file_name}/"));
        }
    }

    matches.sort();
    matches.truncate(max_results);
    matches
}

/// Autocomplete tags by searching existing tags in the database.
/// Returns tags formatted with leading `#`.
pub fn complete_tags(database: &Database, prefix: &str, max_results: usize) -> Vec<String> {
    let clean = prefix.trim_start_matches('#').trim();
    if clean.is_empty() {
        return Vec::new();
    }
    match database.search_tags(clean, max_results) {
        Ok(tags) => tags
            .into_iter()
            .map(|tag| format!("#{}", tag.name))
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tilde_correctly() {
        if let Some(home) = dirs_home() {
            assert_eq!(expand_tilde("~"), home);
            assert_eq!(expand_tilde("~/test"), home.join("test"));
        }
        assert_eq!(expand_tilde("/var/log"), PathBuf::from("/var/log"));
    }

    #[test]
    fn complete_directories_finds_subdirs() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("rutendar_test_comp");
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(test_dir.join("alpha")).unwrap();
        std::fs::create_dir_all(test_dir.join("alpine")).unwrap();
        std::fs::create_dir_all(test_dir.join("beta")).unwrap();
        std::fs::write(test_dir.join("file.txt"), "not a dir").unwrap();

        let prefix = format!("{}/al", test_dir.display());
        let results = complete_directories(&prefix, 5);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.ends_with("/alpha/")));
        assert!(results.iter().any(|p| p.ends_with("/alpine/")));

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn complete_tags_returns_formatted_tags() {
        let db = Database::in_memory().unwrap();
        let results = complete_tags(&db, "#", 5);
        assert!(results.is_empty());
    }
}
