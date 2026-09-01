#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
}

pub fn normalize_tag(value: &str) -> String {
    value.trim().trim_start_matches('#').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization_is_case_and_space_insensitive() {
        assert_eq!(normalize_tag("ML"), "ml");
        assert_eq!(normalize_tag(" ml "), "ml");
        assert_eq!(normalize_tag("#Ml"), "ml");
    }
}
