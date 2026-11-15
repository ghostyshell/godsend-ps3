//! PS3 homebrew catalog: GitHub projects that publish ready-to-install PKGs.
//!
//! Each entry names a repository and the asset filename fragment to pick from
//! its latest release, so a refresh is one API call per project and never
//! depends on release ordering.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::install::{install_local, InstallReport};
use crate::transfer::JobContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    /// `cfw` | `backup` | `save` | `tool` | `media`
    pub category: String,
    pub repo: String,
    /// Asset filename fragment, e.g. `webMAN_MOD_`.
    pub asset: String,
    pub note: Option<String>,
    #[serde(default)]
    pub latest_tag: Option<String>,
    #[serde(default)]
    pub pkg_url: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub size_bytes: u64,
}

pub fn bundled_catalog() -> Vec<CatalogEntry> {
    let entry = |id: &str,
                 name: &str,
                 description: &str,
                 category: &str,
                 repo: &str,
                 asset: &str,
                 note: Option<&str>| CatalogEntry {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        category: category.into(),
        repo: repo.into(),
        asset: asset.into(),
        note: note.map(|n| n.to_string()),
        latest_tag: None,
        pkg_url: None,
        published_at: None,
        size_bytes: 0,
    };
    vec![
        entry(
            "webman-mod",
            "webMAN MOD",
            "The standard CFW web server, FTP server and game launcher. Required for mounting games, the virtual pad and remote actions from this app.",
            "cfw",
            "aldostools/webMAN-MOD",
            "webMAN_MOD_",
            Some("Install this first: the Devices, Controller and Library actions talk to webMAN on port 80."),
        ),
        entry(
            "prepiso",
            "prepISO",
            "Mounts games stored on NTFS, exFAT and network shares by preparing the USB device.",
            "tool",
            "aldostools/webMAN-MOD",
            "prepISO_",
            None,
        ),
        entry(
            "ps2-classics",
            "PS2 Classics Launcher",
            "Launches PS2 ISO images as PS2 Classics on CFW.",
            "tool",
            "aldostools/webMAN-MOD",
            "PS2_Classics_Launcher.pkg",
            Some("Pair with a PS2 config package for best compatibility."),
        ),
        entry(
            "ps2config",
            "PS2 Config",
            "Compatibility configuration database for the PS2 Classics emulator.",
            "tool",
            "aldostools/webMAN-MOD",
            "PS2CONFIG.pkg",
            None,
        ),
        entry(
            "irisman",
            "IRISMAN",
            "All-in-one backup manager: copy discs, mount images, browse files and launch games.",
            "backup",
            "aldostools/IRISMAN",
            "IRISMAN_",
            Some("Useful on HEN consoles where webMAN cannot mount every format."),
        ),
        entry(
            "apollo",
            "Apollo Save Tool",
            "Save data manager: back up, restore and resign saves, and download saves from the community database.",
            "save",
            "bucanero/apollo-ps3",
            "apollo-ps3.pkg",
            None,
        ),
        entry(
            "managunz",
            "ManaGunZ",
            "Backup manager and game launcher with a file manager and ISO tools.",
            "backup",
            "Zarh/ManaGunZ",
            "ManaGunZ_",
            None,
        ),
        entry(
            "managunz-fm",
            "ManaGunZ File Manager",
            "Standalone file manager from the ManaGunZ project.",
            "tool",
            "Zarh/ManaGunZ",
            "FileManager_",
            None,
        ),
    ]
}

/// Refreshed catalog rows for the UI, cached on disk between refreshes.
pub fn catalog(config: &Config, refresh: bool) -> Vec<CatalogEntry> {
    let mut entries = bundled_catalog();
    let cache = cache_path();
    if !refresh {
        if let Ok(bytes) = std::fs::read(&cache) {
            if let Ok(cached) = serde_json::from_slice::<Vec<CatalogEntry>>(&bytes) {
                if cached.len() == entries.len() {
                    return cached;
                }
            }
        }
    }
    for entry in &mut entries {
        if let Ok(release) = latest_release(&entry.repo) {
            if let Some((tag, url, published, size)) = pick_asset(&release, &entry.asset) {
                entry.latest_tag = Some(tag);
                entry.pkg_url = Some(url);
                entry.published_at = Some(published);
                entry.size_bytes = size;
            }
        }
    }
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&entries) {
        let _ = std::fs::write(&cache, bytes);
    }
    let _ = config;
    entries
}

fn cache_path() -> PathBuf {
    crate::config::cache_dir().join("homebrew.json")
}

fn latest_release(repo: &str) -> Result<serde_json::Value> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(20))
        .build();
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    agent
        .get(&url)
        .set("User-Agent", "godsend-ps3")
        .set("Accept", "application/vnd.github+json")
        .call()?
        .into_json()
        .map_err(|e| Error::Http(format!("github {repo}: {e}")))
}

/// Choose the release asset matching `fragment`. Returns tag, url, date, size.
fn pick_asset(
    release: &serde_json::Value,
    fragment: &str,
) -> Option<(String, String, String, u64)> {
    let tag = release.get("tag_name")?.as_str()?.to_string();
    let published = release
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let assets = release.get("assets")?.as_array()?;
    let candidates: Vec<&serde_json::Value> = assets
        .iter()
        .filter(|a| {
            let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
            name.contains(fragment)
                && name.to_lowercase().ends_with(".pkg")
                && !name.to_lowercase().contains("older")
        })
        .collect();
    let asset = candidates.first()?;
    Some((
        tag,
        asset.get("browser_download_url")?.as_str()?.to_string(),
        published,
        asset.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
    ))
}

/// Download a catalog entry's package, then install it on the console.
pub fn install(
    host: &crate::config::Host,
    config: &Config,
    entry: &CatalogEntry,
    ctx: &JobContext,
) -> Result<InstallReport> {
    let url = entry.pkg_url.as_ref().ok_or_else(|| {
        Error::NotFound(format!(
            "{} has no package in its latest release",
            entry.name
        ))
    })?;
    let name = url.rsplit('/').next().unwrap_or("homebrew.pkg").to_string();
    ctx.message(format!("downloading {}", entry.name));
    let local = crate::store::download_url(url, &name, config, ctx)?;
    ctx.message(format!("installing {}", entry.name));
    let report = install_local(host, &local, config, ctx)?;
    // Keep the package in the downloads folder; a retry should not re-download.
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_complete_and_unique() {
        let entries = bundled_catalog();
        assert!(entries.len() >= 5);
        let mut ids: Vec<_> = entries.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), entries.len());
        assert!(entries.iter().all(|e| e.repo.contains('/')));
        assert!(entries.iter().any(|e| e.id == "webman-mod"));
    }

    #[test]
    fn picks_the_matching_asset() {
        let release = serde_json::json!({
            "tag_name": "1.47.50",
            "published_at": "2026-10-10T20:18:40Z",
            "assets": [
                {"name": "prepISO_1.33.pkg", "browser_download_url": "https://x/prep.pkg", "size": 889360},
                {"name": "webMAN_MOD_1.47.50_Installer.pkg", "browser_download_url": "https://x/webman.pkg", "size": 14909424},
                {"name": "source.zip", "browser_download_url": "https://x/src.zip", "size": 10}
            ]
        });
        let (tag, url, _, size) = pick_asset(&release, "webMAN_MOD_").expect("asset");
        assert_eq!(tag, "1.47.50");
        assert!(url.ends_with("webman.pkg"));
        assert_eq!(size, 14909424);
        assert!(pick_asset(&release, "missing").is_none());
    }
}
