#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchQuery {
    pub original_query: String,
    pub trimmed_query: String,
    pub search: String,
    pub search_terms: Vec<String>,
    pub action_keyword: Option<String>,
    pub is_home_query: bool,
    pub is_requery: bool,
}

pub(crate) fn parse_search_query(
    original_query: &str,
    action_keywords: &[&str],
    is_requery: bool,
) -> SearchQuery {
    let trimmed_query = original_query.trim().to_string();
    if trimmed_query.is_empty() {
        return SearchQuery {
            original_query: String::new(),
            trimmed_query: String::new(),
            search: String::new(),
            search_terms: Vec::new(),
            action_keyword: None,
            is_home_query: true,
            is_requery,
        };
    }

    let terms = trimmed_query
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let first = terms.first().map(String::as_str).unwrap_or_default();
    let action_keyword = action_keywords
        .iter()
        .any(|keyword| *keyword == first)
        .then(|| first.to_string());

    let search_terms = if action_keyword.is_some() {
        terms.iter().skip(1).cloned().collect::<Vec<_>>()
    } else {
        terms.clone()
    };
    let search = search_terms.join(" ");

    SearchQuery {
        original_query: original_query.to_string(),
        trimmed_query,
        search,
        search_terms,
        action_keyword,
        is_home_query: false,
        is_requery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flow_style_action_keyword_and_terms() {
        let query = parse_search_query("  f   quarterly   plan  ", &["f", "folder"], true);

        assert_eq!(query.original_query, "  f   quarterly   plan  ");
        assert_eq!(query.trimmed_query, "f   quarterly   plan");
        assert_eq!(query.action_keyword.as_deref(), Some("f"));
        assert_eq!(query.search, "quarterly plan");
        assert_eq!(query.search_terms, ["quarterly", "plan"]);
        assert!(query.is_requery);
        assert!(!query.is_home_query);
    }

    #[test]
    fn parses_home_query_without_action_keyword() {
        let query = parse_search_query("   ", &["f"], false);

        assert!(query.is_home_query);
        assert!(query.search_terms.is_empty());
        assert_eq!(query.search, "");
        assert_eq!(query.action_keyword, None);
    }
}
