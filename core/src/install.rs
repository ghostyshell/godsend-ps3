//! Installing things on the console: upload over FTP, then hand PKGs to
//! webMAN for installation.
//!
//! Destination follows the console's own layout:
//!
//! - PKG                  -> `/dev_hdd0/packages/` (+ `/install.ps3` call)
//! - PS3 / PS2 / PS1 ISO  -> `/dev_hdd0/PS3ISO`, `PS2ISO`, `PSXISO`
//! - JB folder game       -> `/dev_hdd0/GAMES/<folder>`
//! - homebrew ZIP         -> unpacked locally, then treated as a folder
//!
//! On a FAT32 USB device a file above the split limit is written as
//! `<name>.0`, `<name>.1`, ... which the console's file managers reassemble.

use crate::config::{Config, Host};
use crate::error::{Error, Result};
use crate::ftp;
use crate::iso;
use crate::pkg;
use crate::split;
use crate::transfer::{dir_size, JobContext};
use crate::webman::WebMan;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const HDD0: &str = "/dev_hdd0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalKind {
    Pkg,
    Iso,
    Folder,
    Zip,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallReport {
    pub remote_path: String,
    pub kind: String,
    pub bytes: u64,
    /// True when webMAN was asked to install the package.
    pub install_requested: bool,
    pub message: String,
}

/// What kind of local content this path is.
pub fn classify(path: &Path) -> LocalKind {
    if path.is_dir() {
        if path.join("PS3_GAME").is_dir()
            || path.join("PS3_DISC.SFB").is_file()
            || path.join("PS3_GAME").join("PARAM.SFO").is_file()
        {
            return LocalKind::Folder;
        }
        if path.join("PARAM.SFO").is_file() || path.join("EBOOT.BIN").is_file() {
            return LocalKind::Folder;
        }
        return LocalKind::Other;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pkg" => LocalKind::Pkg,
        "iso" => LocalKind::Iso,
        "zip" => LocalKind::Zip,
        _ => LocalKind::Other,
    }
}

/// Remote folder for a local ISO, based on its filesystem contents.
pub fn iso_dest(path: &Path) -> &'static str {
    match iso::probe_system(path).as_str() {
        "ps2" => "/dev_hdd0/PS2ISO",
        "ps1" => "/dev_hdd0/PSXISO",
        "psp" => "/dev_hdd0/PSPISO",
        _ => "/dev_hdd0/PS3ISO",
    }
}

/// Upload one file, splitting it when the destination is a FAT32 USB device
/// and the file is above the split limit.
pub fn push_file(
    conn: &mut suppaftp::FtpStream,
    local: &Path,
    remote_dir: &str,
    split_bytes: u64,
    ctx: &JobContext,
) -> Result<String> {
    let size = std::fs::metadata(local)?.len();
    let name = local
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("file has no name".into()))?;
    let target = format!("{}/{}", remote_dir.trim_end_matches('/'), name);
    ftp::mkdir_all(conn, remote_dir)?;
    if remote_dir.starts_with("/dev_usb") && split_bytes > 0 && size > split_bytes {
        ctx.message(format!("splitting {name} for FAT32"));
        let parts = split::split(local, split_bytes, ctx)?;
        for part in &parts {
            let part_name = part.file_name().unwrap_or_default().to_string_lossy();
            let remote = format!("{}/{}", remote_dir.trim_end_matches('/'), part_name);
            ftp::upload(conn, part, &remote, ctx)?;
            let _ = std::fs::remove_file(part);
        }
        return Ok(target);
    }
    ftp::upload(conn, local, &target, ctx)?;
    Ok(target)
}

/// Upload a folder tree under `remote_parent/<folder name>`.
pub fn push_folder(
    conn: &mut suppaftp::FtpStream,
    local: &Path,
    remote_parent: &str,
    ctx: &JobContext,
) -> Result<String> {
    let name = local
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("folder has no name".into()))?;
    let base = format!("{}/{}", remote_parent.trim_end_matches('/'), name);
    let entries = iso::walk_tree(local)?;
    let total = dir_size(local);
    let mut done = 0u64;
    ctx.progress(0, total);
    ftp::mkdir_all(conn, &base)?;
    for (path, is_dir) in entries {
        ctx.check_cancel()?;
        let rel = path.strip_prefix(local).unwrap_or(&path);
        let remote = format!("{}/{}", base, rel.to_string_lossy().replace('\\', "/"));
        if is_dir {
            ftp::mkdir_all(conn, &remote)?;
            continue;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        ftp::upload(conn, &path, &remote, ctx)?;
        done += size;
        ctx.progress(done, total);
    }
    Ok(base)
}

/// Unpack a ZIP into a fresh temp directory and return the directory that
/// looks like the game root (the shallowest folder holding PS3_GAME).
pub fn unpack_zip(zip_path: &Path, ctx: &JobContext) -> Result<PathBuf> {
    let dest = std::env::temp_dir().join(format!(
        "godsend-ps3-unzip-{}-{}",
        std::process::id(),
        chrono_stamp()
    ));
    std::fs::create_dir_all(&dest)?;
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let total: u64 = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.size()))
        .sum();
    let mut done = 0u64;
    for i in 0..archive.len() {
        ctx.check_cancel()?;
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out = dest.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut outfile = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut outfile)?;
        done += entry.size();
        ctx.progress(done, total);
    }
    Ok(find_game_root(&dest).unwrap_or(dest))
}

fn find_game_root(dir: &Path) -> Option<PathBuf> {
    if dir.join("PS3_GAME").is_dir() || dir.join("PARAM.SFO").is_file() {
        return Some(dir.to_path_buf());
    }
    let entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    let dirs: Vec<_> = entries.iter().filter(|e| e.path().is_dir()).collect();
    if dirs.len() == 1 {
        return find_game_root(&dirs[0].path());
    }
    for entry in entries {
        if entry.path().is_dir() {
            if let Some(found) = find_game_root(&entry.path()) {
                return Some(found);
            }
        }
    }
    None
}

/// Full install flow for one local path.
pub fn install_local(
    host: &Host,
    local: &Path,
    config: &Config,
    ctx: &JobContext,
) -> Result<InstallReport> {
    let kind = classify(local);
    let mut conn = ftp::connect(host)?;
    let webman = WebMan::new(host);
    let result = (|| -> Result<InstallReport> {
        match kind {
            LocalKind::Pkg => {
                ctx.message("uploading package");
                let remote = push_file(&mut conn, local, &pkg::INSTALL_DIR, 0, ctx)?;
                let install_requested = if config.auto_install_pkg && webman.available() {
                    ctx.message("asking webMAN to install");
                    webman.install(&remote)?;
                    true
                } else {
                    false
                };
                Ok(InstallReport {
                    remote_path: remote,
                    kind: "pkg".into(),
                    bytes: std::fs::metadata(local)?.len(),
                    install_requested,
                    message: if install_requested {
                        "uploaded and install requested; the console shows a progress bubble".into()
                    } else {
                        "uploaded to /dev_hdd0/packages; install it from webMAN or the XMB".into()
                    },
                })
            }
            LocalKind::Iso => {
                let dest = iso_dest(local);
                ctx.message(format!("uploading to {dest}"));
                let bytes = std::fs::metadata(local)?.len();
                let remote = push_file(&mut conn, local, dest, config.split_bytes, ctx)?;
                Ok(InstallReport {
                    remote_path: remote,
                    kind: "iso".into(),
                    bytes,
                    install_requested: false,
                    message: format!("uploaded to {dest}; mount it from the Library"),
                })
            }
            LocalKind::Folder => {
                ctx.message("uploading game folder");
                let base = push_folder(&mut conn, local, "/dev_hdd0/GAMES", ctx)?;
                Ok(InstallReport {
                    remote_path: base,
                    kind: "folder".into(),
                    bytes: dir_size(local),
                    install_requested: false,
                    message: "uploaded to /dev_hdd0/GAMES; mount it from the Library".into(),
                })
            }
            LocalKind::Zip => {
                ctx.message("unpacking archive");
                let unpacked = unpack_zip(local, ctx)?;
                let result = install_local(host, &unpacked, config, ctx);
                let _ = std::fs::remove_dir_all(&unpacked);
                result
            }
            LocalKind::Other => Err(Error::Invalid(format!(
                "{} is not a PKG, ISO, game folder or ZIP",
                local.display()
            ))),
        }
    })();
    let _ = conn.quit();
    result
}

/// Mount and then launch an entry on the console.
pub fn mount_and_play(host: &Host, path: &str, launch: bool) -> Result<()> {
    let webman = WebMan::new(host);
    webman.mount(path)?;
    if launch {
        // webMAN needs a moment to expose the mounted disc.
        std::thread::sleep(std::time::Duration::from_millis(1200));
        webman.play(path)?;
    }
    Ok(())
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Read a local file header without loading the whole file.
pub fn read_head(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; max];
    let read = file.read(&mut buf)?;
    buf.truncate(read);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_local_paths() {
        let dir = std::env::temp_dir().join(format!("godsend-ps3-kind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("PS3_GAME")).unwrap();
        std::fs::write(dir.join("PS3_GAME/PARAM.SFO"), b"\0PSF").unwrap();
        assert_eq!(classify(&dir), LocalKind::Folder);
        std::fs::write(dir.join("game.pkg"), b"x").unwrap();
        assert_eq!(classify(&dir.join("game.pkg")), LocalKind::Pkg);
        std::fs::write(dir.join("game.iso"), b"x").unwrap();
        assert_eq!(classify(&dir.join("game.iso")), LocalKind::Iso);
        std::fs::write(dir.join("game.rar"), b"x").unwrap();
        assert_eq!(classify(&dir.join("game.rar")), LocalKind::Other);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
