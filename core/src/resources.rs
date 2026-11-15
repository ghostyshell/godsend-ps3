//! ps3-resources lookups: title ids, Redump disc keys and official PSN
//! updates. The service is the user's own mirror of aldostools' datasets
//! (`https://ps3.game-debrid.link`), with a JSON API per dataset.
//!
//! All three lookups fail soft: a missing service means the app behaves as if
//! the dataset were empty, never as a connection error.

use serde::{Deserialize, Serialize};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleEntry {
    #[serde(rename = "titleId", alias = "title_id")]
    pub title_id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkeyEntry {
    #[serde(rename = "gameId", alias = "game_id", default)]
    pub game_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub dkey: String,
    #[serde(default)]
    pub edition: String,
    #[serde(rename = "sizeBytes", default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub crc32: String,
    #[serde(default)]
    pub md5: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntry {
    #[serde(rename = "titleId", alias = "title_id")]
    pub title_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "sizeBytes", default)]
    pub size_bytes: u64,
    #[serde(rename = "pkgUrl", alias = "pkg_url", default)]
    pub pkg_url: String,
    #[serde(default)]
    pub firmware: String,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(TIMEOUT).build()
}

/// Fetch `/<dataset>?q=...&limit=...` and return its `docs` array.
pub fn query(base: &str, dataset: &str, text: &str, limit: u32) -> Vec<serde_json::Value> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}/api/{dataset}?q={}&limit={limit}", urlencode(text));
    let body: serde_json::Value = match agent().get(&url).call() {
        Ok(resp) => match resp.into_json() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };
    body.get("docs")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Game name for a title id (`BLES01234` -> "Game Name").
pub fn title_name(base: &str, title_id: &str) -> Option<String> {
    let docs = query(base, "titleid", title_id, 5);
    docs.first()
        .and_then(|d| d.get("name").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

/// Redump disc keys matching a title or filename.
pub fn find_dkey(base: &str, text: &str) -> Vec<DkeyEntry> {
    query(base, "dkey", text, 25)
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .filter(|e: &DkeyEntry| !e.dkey.is_empty())
        .collect()
}

/// Pick the disc key whose title best matches a filename.
pub fn best_dkey(base: &str, filename: &str) -> Option<DkeyEntry> {
    let stem = filename
        .rsplit('/')
        .next()
        .unwrap_or(filename)
        .trim_end_matches(".iso")
        .trim_end_matches(".ISO");
    // Strip the Redump "(USA) (En,Fr)" tails; they vary between rips.
    let main = stem.split('(').next().unwrap_or(stem).trim();
    let mut candidates = find_dkey(base, main);
    if candidates.is_empty() {
        candidates = find_dkey(base, stem);
    }
    candidates.sort_by_key(|e| {
        let same_prefix = !e.title.to_lowercase().starts_with(&main.to_lowercase()) as u8;
        (same_prefix, e.title.len())
    });
    candidates.into_iter().next()
}

/// Official update packages for one title id, newest version first.
pub fn updates(base: &str, title_id: &str) -> Vec<UpdateEntry> {
    let mut rows: Vec<UpdateEntry> = query(base, "updates", title_id, 50)
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .filter(|u: &UpdateEntry| !u.pkg_url.is_empty())
        .collect();
    rows.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
    rows
}

/// `01.23` -> 123, so string versions sort numerically.
pub fn version_key(v: &str) -> u64 {
    let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

fn urlencode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_sort_numerically() {
        assert!(version_key("01.10") > version_key("01.09"));
        assert_eq!(version_key("1.00"), 100);
    }

    #[test]
    fn encodes_queries() {
        assert_eq!(urlencode("God of War"), "God+of+War");
        assert_eq!(urlencode("BLES-01234"), "BLES-01234");
        assert!(urlencode("a&b").contains("%26"));
    }

    #[test]
    fn parses_dataset_rows() {
        let row: DkeyEntry = serde_json::from_value(serde_json::json!({
            "gameId": "BCAS20102",
            "title": "God of War Collection (USA).iso",
            "dkey": "323CE7042E84AF01555853C280760F1F",
            "edition": "v1.00",
            "sizeBytes": 24541396992u64
        }))
        .unwrap();
        assert_eq!(row.game_id, "BCAS20102");
        assert_eq!(row.dkey.len(), 32);
        let upd: UpdateEntry = serde_json::from_value(serde_json::json!({
            "titleId": "BLES01234", "name": "Game", "version": "01.02",
            "sizeBytes": 100, "pkgUrl": "http://x/y.pkg"
        }))
        .unwrap();
        assert_eq!(upd.version, "01.02");
    }
}
