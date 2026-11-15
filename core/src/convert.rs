//! Conversions between the formats PS3 tools care about.
//!
//! The UI and CLI call these inside a job, so each takes a `JobContext` for
//! progress and cancellation.

use crate::config::{Config, Host};
use crate::decrypt;
use crate::error::{Error, Result};
use crate::install;
use crate::iso;
use crate::isobuild;
use crate::library;
use crate::pkg;
use crate::resources;
use crate::split;
use crate::transfer::JobContext;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ConvertInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    /// `iso` | `folder` | `pkg` | `zip` | `split` | `other`
    pub kind: String,
    /// `ps3` | `ps2` | `ps1` | `psp` | `pkg` | `unknown`
    pub system: String,
    pub title_id: Option<String>,
    /// ISO with encrypted regions (needs a Redump key before RPCS3 can read it).
    pub encrypted: bool,
    /// For split sets: number of parts found.
    pub parts: usize,
}

/// Inspect any local path the Convert page can act on.
pub fn inspect(path: &Path) -> Result<ConvertInfo> {
    let metadata = std::fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut info = ConvertInfo {
        path: path.to_string_lossy().to_string(),
        name: name.clone(),
        size_bytes: metadata.len(),
        kind: "other".into(),
        system: "unknown".into(),
        title_id: None,
        encrypted: false,
        parts: 0,
    };

    // A `.0` file is the head of a split set: report the joined size.
    let parts = split::find_parts(path);
    if parts.len() > 1 {
        info.kind = "split".into();
        info.parts = parts.len();
        info.size_bytes = parts
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
            .sum();
        let head = split::read_head(&parts, 4096).unwrap_or_default();
        if head.len() >= 4 && head[0..4] == pkg::MAGIC {
            info.system = "pkg".into();
        }
        return Ok(info);
    }

    if path.is_dir() {
        info.kind = "folder".into();
        info.system = if path.join("PS3_GAME").is_dir() || path.join("PS3_DISC.SFB").is_file() {
            "ps3".into()
        } else if path.join("PARAM.SFO").is_file() {
            "ps3".into()
        } else {
            "unknown".into()
        };
        let sfo_path = if path.join("PS3_GAME/PARAM.SFO").is_file() {
            path.join("PS3_GAME/PARAM.SFO")
        } else {
            path.join("PARAM.SFO")
        };
        if let Ok(bytes) = std::fs::read(&sfo_path) {
            if let Ok(sfo) = crate::sfo::Sfo::parse(&bytes) {
                info.title_id = sfo.title_id().map(|s| s.to_string());
            }
        }
        return Ok(info);
    }

    let lower = name.to_lowercase();
    if lower.ends_with(".pkg") {
        info.kind = "pkg".into();
        info.system = "pkg".into();
        if let Ok(p) = pkg::info(path) {
            info.title_id = p.title_id;
        }
    } else if lower.ends_with(".iso") {
        info.kind = "iso".into();
        if let Ok(scan) = decrypt::scan(path) {
            info.encrypted = scan.encrypted;
        }
        if let Ok(mut image) = iso::Iso::open(path) {
            info.system = image.system();
            info.title_id = image.title_id();
        } else {
            info.system = "unknown".into();
        }
    } else if lower.ends_with(".zip") {
        info.kind = "zip".into();
    }
    info.title_id = info
        .title_id
        .or_else(|| library::game_id_from_filename(&name));
    Ok(info)
}

pub fn folder_to_iso(src: &Path, out: &Path, ctx: &JobContext) -> Result<u64> {
    if !src.is_dir() {
        return Err(Error::Invalid(format!("{} is not a folder", src.display())));
    }
    ctx.message("building ISO9660 + Joliet image");
    isobuild::build_iso(src, out, "PS3VOLUME", ctx)
}

pub fn iso_to_folder(src: &Path, out: &Path, ctx: &JobContext) -> Result<u64> {
    let mut image = iso::Iso::open(src)?;
    ctx.message(format!(
        "extracting {} entries{}",
        image.nodes.len(),
        if image.joliet { " (Joliet names)" } else { "" }
    ));
    std::fs::create_dir_all(out)?;
    image.extract(out, ctx)
}

pub fn split_file(src: &Path, chunk: u64, ctx: &JobContext) -> Result<Vec<PathBuf>> {
    ctx.message(format!(
        "splitting into {} MiB parts",
        chunk / (1024 * 1024)
    ));
    split::split(src, chunk, ctx)
}

pub fn merge_file(base: &Path, out: &Path, ctx: &JobContext) -> Result<u64> {
    let parts = split::find_parts(base);
    if parts.is_empty() {
        return Err(Error::NotFound(format!(
            "no parts found for {} (expected {}.0, {}.1, ...)",
            base.display(),
            base.display(),
            base.display()
        )));
    }
    ctx.message(format!("merging {} parts", parts.len()));
    split::merge(&parts, out, ctx)
}

/// Decrypt an ISO using a supplied hex key or the matching Redump key from
/// ps3-resources.
pub fn decrypt_iso(
    config: &Config,
    src: &Path,
    out: &Path,
    key_hex: &str,
    ctx: &JobContext,
) -> Result<u64> {
    let key = if key_hex.trim().is_empty() {
        let entry = resources::best_dkey(&config.ps3_resources_base, &src.to_string_lossy())
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "no Redump key found for {}; paste one or add the dkey folder",
                    src.display()
                ))
            })?;
        ctx.message(format!("using key for {}", entry.title));
        entry.dkey
    } else {
        key_hex.to_string()
    };
    decrypt::decrypt(src, out, &key, ctx)
}

/// One-shot: make a local file installable and push it to the console.
/// This is what the Library "Install" and Downloads "Send to PS3" buttons use.
pub fn push_to_console(
    host: &Host,
    local: &Path,
    config: &Config,
    ctx: &JobContext,
) -> Result<install::InstallReport> {
    install::install_local(host, local, config, ctx)
}

/// Destination folder on the console for a local image, for the UI preview.
pub fn install_destination(local: &Path) -> String {
    match install::classify(local) {
        install::LocalKind::Pkg => pkg::INSTALL_DIR.to_string(),
        install::LocalKind::Iso => install::iso_dest(local).to_string(),
        install::LocalKind::Folder => "/dev_hdd0/GAMES".to_string(),
        install::LocalKind::Zip => "(unpacked, then its contents)".to_string(),
        install::LocalKind::Other => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_a_folder() {
        let dir = std::env::temp_dir().join(format!("godsend-ps3-inspect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("PS3_GAME")).unwrap();
        std::fs::write(
            dir.join("PS3_GAME/PARAM.SFO"),
            crate::sfo::build_sfo(&[
                ("TITLE", crate::sfo::SfoValue::Utf8("Demo".into())),
                ("TITLE_ID", crate::sfo::SfoValue::Utf8("BLES01234".into())),
            ]),
        )
        .unwrap();
        let info = inspect(&dir).unwrap();
        assert_eq!(info.kind, "folder");
        assert_eq!(info.system, "ps3");
        assert_eq!(info.title_id.as_deref(), Some("BLES01234"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_destination_matches_the_console_layout() {
        let dir = std::env::temp_dir().join(format!("godsend-ps3-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = dir.join("a.pkg");
        std::fs::write(&pkg, b"x").unwrap();
        assert_eq!(install_destination(&pkg), "/dev_hdd0/packages");
        let iso = dir.join("b.iso");
        std::fs::write(&iso, b"x").unwrap();
        assert_eq!(install_destination(&iso), "/dev_hdd0/PS3ISO");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
