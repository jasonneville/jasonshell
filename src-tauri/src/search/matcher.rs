#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MatchTier {
    Exact,
    Prefix,
    Acronym,
    TokenPrefix,
    Subsequence,
    EditDistance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatchField {
    Title,
    Subtitle,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MatchData {
    pub(crate) tier: MatchTier,
    pub(crate) score: i32,
    pub(crate) reason: &'static str,
    pub(crate) field: MatchField,
    pub(crate) highlight_data: Vec<usize>,
}

impl MatchTier {
    pub(crate) fn score(self) -> i32 {
        match self {
            Self::Exact => 2_000,
            Self::Prefix => 1_650,
            Self::Acronym => 1_500,
            Self::TokenPrefix => 1_200,
            Self::Subsequence => 920,
            Self::EditDistance => 760,
        }
    }

    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prefix => "prefix",
            Self::Acronym => "acronym",
            Self::TokenPrefix => "tokenPrefix",
            Self::Subsequence => "subsequence",
            Self::EditDistance => "editDistance",
        }
    }
}

#[derive(Clone, Debug)]
struct WordPart {
    start: usize,
    end: usize,
    normalized: String,
}

pub(crate) fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['_', '-', '.', '/', '\\', ':'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn query_tokens(query: &str) -> Vec<String> {
    normalize(query)
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn full_highlight(value: &str) -> Vec<usize> {
    let len = value.chars().count();
    if len == 0 {
        Vec::new()
    } else {
        vec![0, len]
    }
}

pub(crate) fn acronym(value: &str) -> String {
    word_parts(value)
        .iter()
        .filter_map(|word| word.normalized.chars().next())
        .collect()
}

pub(crate) fn best_match(
    title: &str,
    subtitle: Option<&str>,
    hidden_values: &[String],
    query: &str,
    tokens: &[String],
    allow_fuzzy: bool,
) -> Option<MatchData> {
    let query = normalize(query);
    if query.is_empty() || tokens.is_empty() {
        return None;
    }

    let title_matchers = [
        (MatchField::Title, title),
        (MatchField::Subtitle, subtitle.unwrap_or_default()),
    ];

    for tier in [
        MatchTier::Exact,
        MatchTier::Prefix,
        MatchTier::Acronym,
        MatchTier::TokenPrefix,
    ] {
        for (field, value) in title_matchers {
            if value.is_empty() {
                continue;
            }
            if let Some(highlight_data) = match_display_tier(value, &query, tokens, tier) {
                return Some(MatchData {
                    tier,
                    score: tier.score(),
                    reason: tier.reason(),
                    field,
                    highlight_data,
                });
            }
        }
        if hidden_values
            .iter()
            .any(|value| hidden_matches_tier(value, &query, tokens, tier))
        {
            return Some(MatchData {
                tier,
                score: tier.score(),
                reason: tier.reason(),
                field: MatchField::Hidden,
                highlight_data: Vec::new(),
            });
        }
    }

    if !allow_fuzzy {
        return None;
    }

    for tier in [MatchTier::Subsequence, MatchTier::EditDistance] {
        for (field, value) in title_matchers {
            if value.is_empty() {
                continue;
            }
            if let Some(highlight_data) = match_display_tier(value, &query, tokens, tier) {
                return Some(MatchData {
                    tier,
                    score: tier.score(),
                    reason: tier.reason(),
                    field,
                    highlight_data,
                });
            }
        }
        if hidden_values
            .iter()
            .any(|value| hidden_matches_tier(value, &query, tokens, tier))
        {
            return Some(MatchData {
                tier,
                score: tier.score(),
                reason: tier.reason(),
                field: MatchField::Hidden,
                highlight_data: Vec::new(),
            });
        }
    }

    None
}

fn hidden_matches_tier(value: &str, query: &str, tokens: &[String], tier: MatchTier) -> bool {
    let normalized = normalize(value);
    if normalized.is_empty() {
        return false;
    }
    match tier {
        MatchTier::Exact => normalized == query,
        MatchTier::Prefix => normalized.starts_with(query),
        MatchTier::Acronym => !acronym(value).is_empty() && acronym(value) == query,
        MatchTier::TokenPrefix => token_prefix_positions(value, tokens).is_some(),
        MatchTier::Subsequence => subsequence_positions(value, query).is_some(),
        MatchTier::EditDistance => bounded_edit_distance_match(value, query),
    }
}

fn match_display_tier(
    value: &str,
    query: &str,
    tokens: &[String],
    tier: MatchTier,
) -> Option<Vec<usize>> {
    match tier {
        MatchTier::Exact => {
            if normalize(value) == query {
                Some(vec![0, value.chars().count()])
            } else {
                None
            }
        }
        MatchTier::Prefix => prefix_positions(value, query),
        MatchTier::Acronym => acronym_positions(value, query),
        MatchTier::TokenPrefix => token_prefix_positions(value, tokens),
        MatchTier::Subsequence => subsequence_positions(value, query),
        MatchTier::EditDistance => {
            if bounded_edit_distance_match(value, query) {
                Some(vec![0, value.chars().count()])
            } else {
                None
            }
        }
    }
}

fn prefix_positions(value: &str, query: &str) -> Option<Vec<usize>> {
    let chars = value.chars().collect::<Vec<_>>();
    let query_chars = query
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<Vec<_>>();
    let mut start = None;
    let mut matched = 0usize;

    for (index, ch) in chars.iter().enumerate() {
        if !ch.is_alphanumeric() {
            if matched == 0 {
                continue;
            }
            break;
        }
        let normalized = ch.to_ascii_lowercase();
        if matched >= query_chars.len() || normalized != query_chars[matched] {
            if matched == 0 {
                return None;
            }
            break;
        }
        start.get_or_insert(index);
        matched += 1;
        if matched == query_chars.len() {
            break;
        }
    }

    match (start, matched == query_chars.len()) {
        (Some(start), true) => Some(vec![start, matched]),
        _ => None,
    }
}

fn acronym_positions(value: &str, query: &str) -> Option<Vec<usize>> {
    let words = word_parts(value);
    if words.is_empty() {
        return None;
    }
    let query_chars = query.chars().collect::<Vec<_>>();
    if query_chars.len() != words.len().min(query_chars.len()) {
        let actual = words
            .iter()
            .filter_map(|word| word.normalized.chars().next())
            .collect::<String>();
        if actual != query {
            return None;
        }
    }
    let actual = words
        .iter()
        .filter_map(|word| word.normalized.chars().next())
        .collect::<String>();
    if actual != query {
        return None;
    }
    Some(
        words
            .iter()
            .flat_map(|word| [word.start, 1usize])
            .collect::<Vec<_>>(),
    )
}

fn token_prefix_positions(value: &str, tokens: &[String]) -> Option<Vec<usize>> {
    let words = word_parts(value);
    if words.is_empty() {
        return None;
    }
    let mut cursor = 0usize;
    let mut positions = Vec::new();

    for token in tokens {
        let mut matched = None;
        for start_index in cursor..words.len() {
            if words[start_index].normalized.starts_with(token) {
                matched = Some((start_index + 1, vec![words[start_index].start, token.len()]));
                break;
            }

            if let Some(group_match) = initials_group_positions(&words, start_index, token) {
                matched = Some((group_match.0, group_match.1));
                break;
            }
        }

        let Some((next_cursor, segment_positions)) = matched else {
            return None;
        };
        cursor = next_cursor;
        positions.extend(segment_positions);
    }

    Some(merge_highlight_pairs(positions))
}

fn initials_group_positions(
    words: &[WordPart],
    start_index: usize,
    token: &str,
) -> Option<(usize, Vec<usize>)> {
    if token.len() < 2 {
        return None;
    }
    let mut initials = String::new();
    let mut positions = Vec::new();

    for (index, word) in words.iter().enumerate().skip(start_index) {
        let Some(ch) = word.normalized.chars().next() else {
            continue;
        };
        initials.push(ch);
        positions.extend([word.start, 1usize]);
        if initials == token {
            return Some((index + 1, positions));
        }
        if !token.starts_with(&initials) {
            return None;
        }
    }

    None
}

fn subsequence_positions(value: &str, query: &str) -> Option<Vec<usize>> {
    let query_chars = query
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<Vec<_>>();
    if query_chars.len() < 3 {
        return None;
    }

    let mut matched_positions = Vec::new();
    let mut query_index = 0usize;
    for (index, ch) in value.chars().enumerate() {
        if !ch.is_alphanumeric() {
            continue;
        }
        if query_index < query_chars.len() && ch.to_ascii_lowercase() == query_chars[query_index] {
            matched_positions.push(index);
            query_index += 1;
            if query_index == query_chars.len() {
                break;
            }
        }
    }

    if query_index != query_chars.len() {
        return None;
    }

    Some(merge_highlight_pairs(
        matched_positions
            .into_iter()
            .flat_map(|index| [index, 1usize])
            .collect(),
    ))
}

fn bounded_edit_distance_match(value: &str, query: &str) -> bool {
    let left = compact(value);
    let right = compact(query);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let threshold = max_edit_distance(right.chars().count());
    if left.len().abs_diff(right.len()) > threshold {
        return false;
    }
    bounded_levenshtein(&left, &right, threshold).is_some()
}

fn max_edit_distance(query_len: usize) -> usize {
    if query_len >= 7 {
        2
    } else {
        1
    }
}

fn bounded_levenshtein(left: &str, right: &str, threshold: usize) -> Option<usize> {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0usize; right_chars.len() + 1];

    for (left_index, left_char) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
            row_min = row_min.min(current[right_index + 1]);
        }
        if row_min > threshold {
            return None;
        }
        previous.clone_from_slice(&current);
    }

    let distance = previous[right_chars.len()];
    if distance <= threshold {
        Some(distance)
    } else {
        None
    }
}

fn compact(value: &str) -> String {
    normalize(value)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn merge_highlight_pairs(pairs: Vec<usize>) -> Vec<usize> {
    let mut ranges = pairs
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[0] + pair[1]))
        .collect::<Vec<_>>();
    if ranges.is_empty() {
        return Vec::new();
    }
    ranges.sort_unstable_by_key(|range| range.0);

    let mut merged = Vec::new();
    let mut current = ranges[0];
    for range in ranges.into_iter().skip(1) {
        if range.0 <= current.1 {
            current.1 = current.1.max(range.1);
        } else {
            merged.extend([current.0, current.1 - current.0]);
            current = range;
        }
    }
    merged.extend([current.0, current.1 - current.0]);
    merged
}

fn word_parts(value: &str) -> Vec<WordPart> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        while index < chars.len() && !chars[index].is_alphanumeric() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        let start = index;
        let mut normalized = String::new();
        while index < chars.len() && chars[index].is_alphanumeric() {
            normalized.push(chars[index].to_ascii_lowercase());
            index += 1;
        }
        parts.push(WordPart {
            start,
            end: index,
            normalized,
        });
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_prefix_matches_multi_word_abbreviation_groups() {
        let tokens = query_tokens("vs code");
        let matched = best_match("Visual Studio Code", None, &[], "vs code", &tokens, true)
            .expect("token-prefix match");

        assert_eq!(matched.tier, MatchTier::TokenPrefix);
        assert_eq!(matched.highlight_data, vec![0, 1, 7, 1, 14, 4]);
    }

    #[test]
    fn subsequence_matches_short_launcher_style_abbreviation() {
        let tokens = query_tokens("sptfy");
        let matched =
            best_match("Spotify", None, &[], "sptfy", &tokens, true).expect("subsequence match");

        assert_eq!(matched.tier, MatchTier::Subsequence);
        assert_eq!(matched.highlight_data, vec![0, 2, 3, 1, 5, 2]);
    }

    #[test]
    fn small_edit_distance_is_bounded() {
        let tokens = query_tokens("spitify");
        let matched = best_match("Spotify", None, &[], "spitify", &tokens, true)
            .expect("edit-distance match");

        assert_eq!(matched.tier, MatchTier::EditDistance);
    }
}
