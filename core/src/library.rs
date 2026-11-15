//! Remote library scan: what is installed on the console, grouped by system.
//!
//! PS3 CFW exposes a stable filesystem layout over FTP:
//!
//! - `/dev_hdd0/game/<ID>` digital games and installed homebrew (PARAM.SFO)
//! - `/dev_hdd0/GAMES/<name>` JB-format folder games (PS3_GAME/PARAM.SFO)
//! - `/dev_hdd0/PS3ISO`, `PS2ISO`, `PSXISO`, `PSPISO`, `ROMS` disc images
//! - `/dev_hdd0/pkg`, `/dev_hdd0/packages` ready-to-install PKGs
//!
//! The scan tolerates missing directories and per-entry failures, so one
//! unreadable PARAM.SFO never empties the library.

use crate::config::Host;
use crate::error::Result;
use crate::ftp;
use crate::pkg;
use crate::sfo::Sfo;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct GameEntry {
    /// Stable id: `<kind>:<remote path>`.
    pub id: String,
    pub title: String,
    pub title_id: Option<String>,
    /// `ps3`, `ps2`, `ps1`, `psp`, `pkg`, `unknown`.
    pub system: String,
    /// `game`, `iso`, `pkg`, `homebrew`.
    pub kind: String,
    /// `disc`, `digital`, `homebrew`, `package`.
    pub category: String,
    pub path: String,
    pub size_bytes: u64,
    pub has_icon: bool,
    pub icon_path: Option<String>,
    pub version: Option<String>,
    /// `hdd0`, `usb000`, ...
    pub device: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LibraryView {
    pub games: Vec<GameEntry>,
    pub packages: Vec<GameEntry>,
    /// Directories that were reachable, for the empty state ("no PS3ISO folder").
    pub roots: Vec<String>,
}

/// Directory names under each device root that hold mountable content.
const ISO_DIRS: &[(&str, &str)] = &[
    ("PS3ISO", "ps3"),
    ("PS2ISO", "ps2"),
    ("PSXISO", "ps1"),
    ("PSPISO", "psp"),
    ("ROMS", "rom"),
    ("ISO", "ps3"),
];

pub fn scan(host: &Host) -> Result<LibraryView> {
    let mut conn = ftp::connect(host)?;
    let mut view = LibraryView::default();

    for device in list_devices(&mut conn) {
        let dev_name = device.trim_start_matches("/dev_").to_string();
        let root = format!("/dev_{}", dev_name);

        // Digital games and homebrew in /game.
        let game_dir = format!("{root}/game");
        if let Ok(entries) = ftp::list(&mut conn, &game_dir) {
            view.roots.push(game_dir.clone());
            for entry in entries.iter().filter(|e| e.is_dir) {
                if let Some(game) = read_game_folder(&mut conn, entry, &dev_name, "digital") {
                    view.games.push(game);
                }
            }
        }

        // JB folder games in /GAMES.
        let games_dir = format!("{root}/GAMES");
        if let Ok(entries) = ftp::list(&mut conn, &games_dir) {
            view.roots.push(games_dir.clone());
            for entry in entries.iter().filter(|e| e.is_dir) {
                if let Some(game) = read_jb_folder(&mut conn, entry, &dev_name) {
                    view.games.push(game);
                }
            }
        }

        // Disc images.
        for (dir, system) in ISO_DIRS {
            let path = format!("{root}/{dir}");
            if let Ok(entries) = ftp::list(&mut conn, &path) {
                view.roots.push(path.clone());
                for entry in entries.iter().filter(|e| !e.is_dir) {
                    if !entry.name.to_lowercase().ends_with(".iso") {
                        continue;
                    }
                    view.games.push(iso_entry(entry, system, &dev_name));
                }
            }
        }

        // Packages waiting to install.
        for dir in ["pkg", "packages"] {
            let path = format!("{root}/{dir}");
            if let Ok(entries) = ftp::list(&mut conn, &path) {
                view.roots.push(path.clone());
                for entry in entries.iter().filter(|e| !e.is_dir) {
                    if !entry.name.to_lowercase().ends_with(".pkg") {
                        continue;
                    }
                    let head = ftp::read_head(&mut conn, &entry.path, 4096).unwrap_or_default();
                    let info = pkg::info_from_head(
                        entry.path.clone(),
                        entry.name.clone(),
                        entry.size_bytes,
                        &head,
                    );
                    view.packages.push(GameEntry {
                        id: format!("pkg:{}", entry.path),
                        title: info
                            .title_id
                            .clone()
                            .unwrap_or_else(|| title_from_filename(&entry.name)),
                        title_id: info.title_id,
                        system: "pkg".into(),
                        kind: "pkg".into(),
                        category: "package".into(),
                        path: entry.path.clone(),
                        size_bytes: entry.size_bytes,
                        has_icon: false,
                        icon_path: None,
                        version: None,
                        device: dev_name.clone(),
                    });
                }
            }
        }
    }

    view.games
        .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    view.packages
        .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    view.games.dedup_by(|a, b| a.path == b.path);
    let _ = conn.quit();
    Ok(view)
}

/// Which `/dev_*` roots exist: always hdd0, plus any USB devices.
fn list_devices(conn: &mut suppaftp::FtpStream) -> Vec<String> {
    let mut devices = vec!["/dev_hdd0".to_string()];
    if let Ok(entries) = ftp::list(conn, "/") {
        for entry in entries {
            let name = entry.name.trim_start_matches('/');
            if entry.is_dir && name.starts_with("dev_usb") {
                devices.push(format!("/{name}"));
            }
        }
    }
    // De-duplicate and keep hdd0 first.
    devices.sort();
    devices.dedup();
    devices
}

fn read_game_folder(
    conn: &mut suppaftp::FtpStream,
    entry: &ftp::RemoteEntry,
    device: &str,
    category: &str,
) -> Option<GameEntry> {
    let sfo_path = format!("{}/PARAM.SFO", entry.path);
    let head = ftp::read_head(conn, &sfo_path, 512 * 1024).ok()?;
    let sfo = Sfo::parse(&head).ok()?;
    let title = sfo
        .title()
        .map(|s| s.to_string())
        .unwrap_or_else(|| title_from_filename(&entry.name));
    let title_id = sfo.title_id().map(|s| s.to_string());
    let category_code = sfo.category().unwrap_or("");
    let is_homebrew = category_code == "HG" || title_id.as_deref().unwrap_or("").starts_with("GDS");
    let has_icon = true; // ICON0 is fetched lazily; a missing one just shows a monogram
    Some(GameEntry {
        id: format!("game:{}", entry.path),
        title,
        title_id,
        system: "ps3".into(),
        kind: if is_homebrew {
            "homebrew".into()
        } else {
            "game".into()
        },
        category: if is_homebrew {
            "homebrew".into()
        } else {
            category.into()
        },
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        has_icon,
        icon_path: Some(format!("{}/ICON0.PNG", entry.path)),
        version: sfo.version().map(|s| s.to_string()),
        device: device.into(),
    })
}

fn read_jb_folder(
    conn: &mut suppaftp::FtpStream,
    entry: &ftp::RemoteEntry,
    device: &str,
) -> Option<GameEntry> {
    for candidate in [
        format!("{}/PS3_GAME/PARAM.SFO", entry.path),
        format!("{}/PARAM.SFO", entry.path),
    ] {
        let Ok(head) = ftp::read_head(conn, &candidate, 512 * 1024) else {
            continue;
        };
        let Ok(sfo) = Sfo::parse(&head) else {
            continue;
        };
        let title = sfo
            .title()
            .map(|s| s.to_string())
            .unwrap_or_else(|| title_from_filename(&entry.name));
        let icon_path = candidate.replace("PARAM.SFO", "ICON0.PNG");
        return Some(GameEntry {
            id: format!("game:{}", entry.path),
            title,
            title_id: sfo.title_id().map(|s| s.to_string()),
            system: "ps3".into(),
            kind: "game".into(),
            category: "disc".into(),
            path: entry.path.clone(),
            size_bytes: entry.size_bytes,
            has_icon: true,
            icon_path: Some(icon_path),
            version: sfo.version().map(|s| s.to_string()),
            device: device.into(),
        });
    }
    // A folder without PARAM.SFO is still mountable; keep it visible.
    Some(GameEntry {
        id: format!("game:{}", entry.path),
        title: title_from_filename(&entry.name),
        title_id: None,
        system: "ps3".into(),
        kind: "game".into(),
        category: "disc".into(),
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        has_icon: false,
        icon_path: None,
        version: None,
        device: device.into(),
    })
}

fn iso_entry(entry: &ftp::RemoteEntry, system: &str, device: &str) -> GameEntry {
    let system = if system == "rom" { "unknown" } else { system };
    GameEntry {
        id: format!("iso:{}", entry.path),
        title: title_from_filename(&entry.name),
        title_id: game_id_from_filename(&entry.name),
        system: system.into(),
        kind: "iso".into(),
        category: "disc".into(),
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        has_icon: false,
        icon_path: None,
        version: None,
        device: device.into(),
    }
}

/// `God of War (USA).iso` -> `God of War`.
pub fn title_from_filename(name: &str) -> String {
    let stem = name.rsplit('/').next().unwrap_or(name);
    let stem = stem
        .trim_end_matches(".iso")
        .trim_end_matches(".ISO")
        .trim_end_matches(".pkg")
        .trim_end_matches(".PKG");
    let before_tag = stem.split(" (").next().unwrap_or(stem);
    let cleaned = before_tag.trim().trim_end_matches('.').to_string();
    if cleaned.is_empty() {
        stem.to_string()
    } else {
        cleaned
    }
}

/// Pull `BLES01234` out of `BLES01234 - Title.iso` or `Title [BLES01234].iso`.
pub fn game_id_from_filename(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    let bytes = upper.as_bytes();
    for index in 0..bytes.len().saturating_sub(9) {
        let window = &bytes[index..index + 9];
        let Ok(text) = std::str::from_utf8(window) else {
            continue;
        };
        if !pkg::is_game_id(text) {
            continue;
        }
        // Make sure it is not part of a longer alphanumeric run.
        let before_ok = index == 0 || !bytes[index - 1].is_ascii_alphanumeric();
        let after_ok = index + 9 >= bytes.len() || !bytes[index + 9].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some(text.to_string());
        }
    }
    None
}

/// Download and cache ICON0.PNG for a library entry.
pub fn icon_bytes(host: &Host, icon_path: &str) -> Option<Vec<u8>> {
    let cache = icon_cache_path(icon_path);
    if let Ok(bytes) = std::fs::read(&cache) {
        return Some(bytes);
    }
    let mut conn = ftp::connect(host).ok()?;
    let bytes = ftp::read_head(&mut conn, icon_path, 2 * 1024 * 1024).ok()?;
    let _ = conn.quit();
    if bytes.is_empty() {
        return None;
    }
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache, &bytes);
    Some(bytes)
}

fn icon_cache_path(icon_path: &str) -> PathBuf {
    crate::config::icons_cache_dir().join(format!("{:016x}.png", fnv1a(icon_path)))
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Delete a game folder or ISO from the console.
pub fn delete_entry(host: &Host, entry_path: &str, is_dir: bool) -> Result<()> {
    let mut conn = ftp::connect(host)?;
    let result = ftp::remove(&mut conn, entry_path, is_dir);
    let _ = conn.quit();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_from_filenames() {
        assert_eq!(title_from_filename("God of War (USA).iso"), "God of War");
        assert_eq!(
            title_from_filename("/dev_hdd0/PS3ISO/Demon's Souls.ISO"),
            "Demon's Souls"
        );
        assert_eq!(
            title_from_filename("BLES01234 - Game.pkg"),
            "BLES01234 - Game"
        );
    }

    #[test]
    fn game_ids_from_filenames() {
        assert_eq!(
            game_id_from_filename("BLES01234 - Game.iso").as_deref(),
            Some("BLES01234")
        );
        assert_eq!(
            game_id_from_filename("Game [NPUB30698].iso").as_deref(),
            Some("NPUB30698")
        );
        assert_eq!(game_id_from_filename("No Id Here.iso"), None);
    }

    #[test]
    fn hashes_are_stable() {
        assert_eq!(fnv1a("abc"), fnv1a("abc"));
        assert_ne!(fnv1a("abc"), fnv1a("abd"));
    }
}
