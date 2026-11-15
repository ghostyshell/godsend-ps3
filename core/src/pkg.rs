//! PKG inspection: Sony's package format used for PSN games, updates and
//! homebrew. The app does not decrypt PKG contents (that needs console keys);
//! it identifies packages, splits them for FAT32, merges split sets and hands
//! them to webMAN for installation.

use crate::error::Result;
use serde::Serialize;
use std::path::Path;

pub const MAGIC: [u8; 4] = [0x7f, 0x43, 0x4e, 0x54]; // "\x7fCNT"
pub const INSTALL_DIR: &str = "/dev_hdd0/packages";

#[derive(Debug, Clone, Serialize)]
pub struct PkgInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub magic_ok: bool,
    /// Package type field at offset 0x04 (1 = game, 2 = patch, ...).
    pub pkg_type: u32,
    /// Nine-character game id (`BLES01234`) found in the content id.
    pub title_id: Option<String>,
    /// Full content id (`UP0001-BLES01234_00-...`) when present.
    pub content_id: Option<String>,
}

pub fn is_pkg(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("pkg"))
        .unwrap_or(false)
}

/// Inspect a PKG using its first 4 KiB plus the file size.
pub fn info(path: &Path) -> Result<PkgInfo> {
    let size = std::fs::metadata(path)?.len();
    let head = std::fs::read(path)?
        .into_iter()
        .take(4096)
        .collect::<Vec<u8>>();
    Ok(info_from_head(
        path.to_string_lossy().to_string(),
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        size,
        &head,
    ))
}

pub fn info_from_head(path: String, name: String, size_bytes: u64, head: &[u8]) -> PkgInfo {
    let magic_ok = head.len() >= 4 && head[0..4] == MAGIC;
    let pkg_type = if head.len() >= 8 {
        u32::from_be_bytes([head[4], head[5], head[6], head[7]])
    } else {
        0
    };
    let (content_id, title_id) = find_content_id(head);
    PkgInfo {
        path,
        name,
        size_bytes,
        magic_ok,
        pkg_type,
        title_id,
        content_id,
    }
}

/// Scan the header for the ASCII content id and extract its game id.
fn find_content_id(head: &[u8]) -> (Option<String>, Option<String>) {
    // Content ids look like `UP0001-BLES01234_00-XXXXXXXXXXXXXXXX` (36 chars).
    let mut run = Vec::new();
    for &byte in head.iter().chain(std::iter::once(&0u8)) {
        if byte.is_ascii_graphic() {
            run.push(byte);
            continue;
        }
        if run.len() >= 24 {
            if let Ok(text) = std::str::from_utf8(&run) {
                let mut parts = text.split('-');
                let _distributor = parts.next();
                if let Some(rest) = parts.next() {
                    let id = rest.split('_').next().unwrap_or(rest).trim();
                    if is_game_id(id) {
                        return (Some(text.to_string()), Some(id.to_string()));
                    }
                }
            }
        }
        run.clear();
    }
    // Fall back to any bare 9-character game id in the header.
    for window in head.windows(9) {
        if let Ok(text) = std::str::from_utf8(window) {
            if is_game_id(text) {
                return (None, Some(text.to_string()));
            }
        }
    }
    (None, None)
}

/// `BLES01234`, `NPUB30698`, `SLUS12345`: four letters then five digits.
pub fn is_game_id(text: &str) -> bool {
    text.len() == 9
        && text[..4].bytes().all(|b| b.is_ascii_uppercase())
        && text[4..].bytes().all(|b| b.is_ascii_digit())
}

/// Where a package is uploaded before installation.
pub fn install_path(name: &str) -> String {
    format!("{INSTALL_DIR}/{}", name.rsplit('/').next().unwrap_or(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_game_ids() {
        assert!(is_game_id("BLES01234"));
        assert!(is_game_id("NPUB30698"));
        assert!(!is_game_id("BLES1234"));
        assert!(!is_game_id("bles01234"));
    }

    #[test]
    fn parses_a_header() {
        let mut head = vec![0u8; 512];
        head[0..4].copy_from_slice(&MAGIC);
        head[4..8].copy_from_slice(&1u32.to_be_bytes());
        head[0x30..0x30 + 36].copy_from_slice(b"UP0001-BLES01234_00-0000000000000000");
        let info = info_from_head("/x.pkg".into(), "x.pkg".into(), 1234, &head);
        assert!(info.magic_ok);
        assert_eq!(info.pkg_type, 1);
        assert_eq!(info.title_id.as_deref(), Some("BLES01234"));
        assert_eq!(
            info.content_id.as_deref(),
            Some("UP0001-BLES01234_00-0000000000000000")
        );
    }

    #[test]
    fn reports_non_pkg_files() {
        let info = info_from_head("/x".into(), "x".into(), 10, b"not a pkg");
        assert!(!info.magic_ok);
        assert_eq!(info.title_id, None);
    }
}
