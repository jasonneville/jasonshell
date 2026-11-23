//! "Open URL" and "Search the web" rows. Builds URLs only; never performs network I/O.

use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::settings::WebSearchEngine;
use std::time::Instant;

pub(crate) const MAX_WEB_QUERY_CHARS: usize = 256;
pub(crate) const MAX_OPEN_URL_CHARS: usize = 2048;
pub(crate) const WEB_OPEN_URL_ID: &str = "web:open-url";
pub(crate) const WEB_SEARCH_ID: &str = "web:search";
/// `match_reason` of an Open URL row; the renderer depends on both values.
pub(crate) const WEB_URL_REASON: &str = "url";
pub(crate) const WEB_BARE_DOMAIN_REASON: &str = "bareDomain";
const BARE_DOMAIN_TLDS: &[&str] = &[
    "com", "org", "net", "io", "dev", "app", "co", "ai", "edu", "gov", "uk", "de", "fr", "nl",
    "info", "xyz", "me", "tv", "us", "ca", "au", "eu", "ru", "jp", "cn", "in", "br", "ch",
];

#[derive(Clone, Debug)]
pub(crate) struct WebSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_web(query: &str, limit: usize, engine: WebSearchEngine) -> WebSearchRun {
    let started_at = iso_now();
    let started = Instant::now();
    let mut results = Vec::new();
    if let Some(url) = normalize_open_url_query(query) {
        let reason = if is_explicit_web_address(query) {
            WEB_URL_REASON
        } else {
            WEB_BARE_DOMAIN_REASON
        };
        results.push(web_result(
            WEB_OPEN_URL_ID,
            format!("Open {url}"),
            "Open URL",
            url,
            reason,
        ));
    }
    if let Some(url) = web_search_url(engine, query) {
        results.push(web_result(
            WEB_SEARCH_ID,
            format!("Search the web for \"{}\"", query.trim()),
            engine_name(engine),
            url,
            "web",
        ));
    }
    results.truncate(limit);

    WebSearchRun {
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Web,
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
            provider_id: SearchProviderId::Web,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: None,
        },
        results,
    }
}

fn web_result(
    id: &str,
    title: String,
    subtitle: &str,
    url: String,
    match_reason: &str,
) -> SearchResult {
    SearchResult {
        // The URL is part of the identity, so a stale activation cannot open a newer row's URL.
        record_key: format!("{id}:{url}"),
        id: id.to_string(),
        provider_id: SearchProviderId::Web,
        kind: SearchResultKind::Web,
        title,
        subtitle: Some(subtitle.to_string()),
        path: None,
        action: SearchResultAction::OpenWebUrl { url },
        terms: Vec::new(),
        aliases: Vec::new(),
        score: 0,
        provider_signal: 0,
        match_reason: match_reason.to_string(),
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}

fn engine_name(engine: WebSearchEngine) -> &'static str {
    match engine {
        WebSearchEngine::Google => "Google",
        WebSearchEngine::DuckDuckGo => "DuckDuckGo",
        WebSearchEngine::Bing => "Bing",
        WebSearchEngine::Brave => "Brave Search",
    }
}

pub(crate) fn web_search_url(engine: WebSearchEngine, query: &str) -> Option<String> {
    if query.chars().any(char::is_control) {
        return None;
    }
    let query = query.trim();
    if query.is_empty() || query.chars().count() > MAX_WEB_QUERY_CHARS {
        return None;
    }
    let base = match engine {
        WebSearchEngine::Google => "https://www.google.com/search?q=",
        WebSearchEngine::DuckDuckGo => "https://duckduckgo.com/?q=",
        WebSearchEngine::Bing => "https://www.bing.com/search?q=",
        WebSearchEngine::Brave => "https://search.brave.com/search?q=",
    };
    let mut url = String::with_capacity(base.len() + query.len() * 3);
    url.push_str(base);
    for byte in query.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            url.push(char::from(byte));
        } else {
            push_percent_encoded(&mut url, byte);
        }
    }
    // A long non-ASCII query can expand past the URL cap; emit nothing the backend would refuse.
    validate_search_open_url(&url).ok()
}

fn push_percent_encoded(output: &mut String, byte: u8) {
    output.push_str(&format!("%{byte:02X}"));
}

/// URL-like query -> normalized http(s) URL, else `None`.
pub(crate) fn normalize_open_url_query(query: &str) -> Option<String> {
    if query.chars().any(char::is_control) {
        return None;
    }
    let query = query.trim();
    if query.chars().any(char::is_whitespace) {
        return None;
    }
    let candidate = match strip_web_scheme(query) {
        Some((scheme, rest)) => format!("{scheme}{rest}"),
        None => {
            let authority = query.split(['/', '?', '#']).next().unwrap_or_default();
            if !is_bare_domain(authority) {
                return None;
            }
            format!("https://{query}")
        }
    };
    // Hosts must be ASCII; only the rest of a typed URL may need encoding.
    let mut url = String::with_capacity(candidate.len());
    for ch in candidate.chars() {
        if ch.is_ascii() {
            url.push(ch);
        } else {
            let mut buffer = [0; 4];
            for byte in ch.encode_utf8(&mut buffer).bytes() {
                push_percent_encoded(&mut url, byte);
            }
        }
    }
    validate_search_open_url(&url).ok()
}

fn is_bare_domain(authority: &str) -> bool {
    let Some(host) = valid_host(authority) else {
        return false;
    };
    let labels = host.split('.').collect::<Vec<_>>();
    let Some(tld) = labels.last().filter(|_| labels.len() >= 2) else {
        return false;
    };
    let www = labels.len() >= 3 && labels[0].eq_ignore_ascii_case("www");
    www || BARE_DOMAIN_TLDS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(tld))
}

fn is_explicit_web_address(query: &str) -> bool {
    let query = query.trim();
    strip_web_scheme(query).is_some()
        || query
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
}

fn strip_web_scheme(url: &str) -> Option<(&'static str, &str)> {
    ["https://", "http://"].into_iter().find_map(|scheme| {
        let prefix = url.get(..scheme.len())?;
        prefix
            .eq_ignore_ascii_case(scheme)
            .then(|| (scheme, &url[scheme.len()..]))
    })
}

/// Revalidated immediately before launch. Mirrored by `isSafeHttpUrl` in `src/lib/searchEngine.ts`;
/// the two must accept the same set because one refused row invalidates a whole response there.
pub(crate) fn validate_search_open_url(url: &str) -> Result<String, String> {
    if url.len() > MAX_OPEN_URL_CHARS {
        return Err("URL is too long".to_string());
    }
    // Printable ASCII only. A quote or backslash could escape a browser launch argument.
    if !url
        .bytes()
        .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\\'))
    {
        return Err("URL contains characters that are not allowed".to_string());
    }
    let (_, rest) =
        strip_web_scheme(url).ok_or_else(|| "URL must use http or https".to_string())?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    valid_host(authority)
        .map(|_| url.to_string())
        .ok_or_else(|| "URL host is invalid or includes credentials".to_string())
}

/// The host of `host[:port]`. Credentials fail because `@` is not a host character.
fn valid_host(authority: &str) -> Option<&str> {
    let (host, port) = match authority.strip_prefix('[') {
        Some(rest) => {
            let (address, tail) = rest.split_once(']')?;
            let valid = address.contains(':')
                && address
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() || matches!(byte, b':' | b'.'));
            if !valid {
                return None;
            }
            let port = match tail {
                "" => None,
                _ => Some(tail.strip_prefix(':')?),
            };
            (address, port)
        }
        None => {
            let (host, port) = match authority.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            };
            let valid = host.split('.').all(|label| {
                !label.is_empty()
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            });
            if !valid {
                return None;
            }
            (host, port)
        }
    };
    let valid_port = port.is_none_or(|port| {
        (1..=5).contains(&port.len()) && port.bytes().all(|byte| byte.is_ascii_digit())
    });
    valid_port.then_some(host)
}
