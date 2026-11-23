//! Saved Quick Commands as search rows. Enter runs a row, so matching is deliberately strict: the
//! visible label only, never arguments, command text or paths, and never fuzzy. Rows carry the
//! command ID only.

use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::matcher::normalize;
use crate::settings::{validate_quick_command_entry, QuickCommandEntry};
use std::time::Instant;

pub(crate) const QUICK_COMMAND_ROW_PREFIX: &str = "quick:";
/// A shorter label prefix is only a weak match: one or two letters must not put a launch under Enter.
/// Counted on the normalized (trimmed) query; an exact label matches at any length.
pub(crate) const MIN_PREFIX_QUERY_CHARS: usize = 3;

/// Ordered strongest first. Carried in `match_reason`; ranking and the renderer depend on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum QuickCommandMatch {
    Exact,
    Prefix,
    WordPrefix,
    Substring,
}

impl QuickCommandMatch {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prefix => "prefix",
            Self::WordPrefix => "wordPrefix",
            Self::Substring => "substring",
        }
    }

    /// Weak matches must never become what a plain Enter runs.
    pub(crate) fn is_weak(self) -> bool {
        self >= Self::WordPrefix
    }

    /// Anything unrecognized is treated as the weakest match.
    pub(crate) fn of_row(row: &SearchResult) -> Option<Self> {
        if row.provider_id != SearchProviderId::Commands
            || !row.id.starts_with(QUICK_COMMAND_ROW_PREFIX)
        {
            return None;
        }
        Some(
            [Self::Exact, Self::Prefix, Self::WordPrefix]
                .into_iter()
                .find(|tier| tier.reason() == row.match_reason)
                .unwrap_or(Self::Substring),
        )
    }

    fn of_label(label: &str, query: &str) -> Option<Self> {
        let label = normalize(label);
        if label == query {
            return Some(Self::Exact);
        }
        if label.starts_with(query) {
            return Some(if query.chars().count() >= MIN_PREFIX_QUERY_CHARS {
                Self::Prefix
            } else {
                Self::WordPrefix
            });
        }
        let words = label
            .split(|ch: char| !ch.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        if query
            .split(' ')
            .all(|token| words.iter().any(|word| word.starts_with(token)))
        {
            return Some(Self::WordPrefix);
        }
        label.contains(query).then_some(Self::Substring)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct QuickCommandsSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_quick_commands(
    query: &str,
    limit: usize,
    entries: &[QuickCommandEntry],
) -> QuickCommandsSearchRun {
    let started_at = iso_now();
    let started = Instant::now();
    let results = rank_quick_commands(query, limit, entries);

    QuickCommandsSearchRun {
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Commands,
            started_at,
            ended_at: Some(iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: SearchProviderCacheState::Hit,
            cache_age_ms: None,
            result_count: results.len(),
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::Commands,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: None,
        },
        results,
    }
}

fn rank_quick_commands(
    query: &str,
    limit: usize,
    entries: &[QuickCommandEntry],
) -> Vec<SearchResult> {
    let query = normalize(query);
    if !query.chars().any(char::is_alphanumeric) {
        return Vec::new();
    }
    let mut matches = entries
        .iter()
        .filter_map(|entry| Some((QuickCommandMatch::of_label(&entry.label, &query)?, entry)))
        .filter_map(|(tier, entry)| Some((tier, validate_quick_command_entry(entry).ok()?)))
        .collect::<Vec<_>>();
    matches.sort_by(|(left_tier, left), (right_tier, right)| {
        left_tier
            .cmp(right_tier)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.id.cmp(&right.id))
    });
    matches
        .into_iter()
        .take(limit)
        .map(|(tier, entry)| quick_command_result(tier, &entry))
        .collect()
}

fn quick_command_result(tier: QuickCommandMatch, entry: &QuickCommandEntry) -> SearchResult {
    let key = format!("{QUICK_COMMAND_ROW_PREFIX}{}", entry.id);
    SearchResult {
        id: key.clone(),
        provider_id: SearchProviderId::Commands,
        kind: SearchResultKind::Command,
        title: entry.label.clone(),
        subtitle: Some("Quick command".to_string()),
        path: None,
        action: SearchResultAction::RunCommand {
            command_id: key.clone(),
        },
        terms: Vec::new(),
        aliases: Vec::new(),
        score: 0,
        provider_signal: 0,
        match_reason: tier.reason().to_string(),
        record_key: key,
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}
