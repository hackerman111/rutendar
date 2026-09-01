use super::query::SearchQuery;
use crate::model::normalize_tag;

pub fn parse_query(input: &str) -> SearchQuery {
    let mut text = Vec::new();
    let mut tags = Vec::new();
    for token in input.split_whitespace() {
        if token.starts_with('#') {
            let tag = normalize_tag(token);
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        } else {
            text.push(token);
        }
    }
    SearchQuery {
        text: text.join(" ").to_lowercase(),
        tags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_splits_text_and_deduplicates_tags() {
        assert_eq!(
            parse_query("матан #Универ #лекция #универ"),
            SearchQuery {
                text: "матан".into(),
                tags: vec!["универ".into(), "лекция".into()],
            }
        );
    }
}
