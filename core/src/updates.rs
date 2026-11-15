//! Official PSN game updates: look a title up in ps3-resources and download
//! the newest package, then install it like any other PKG.

use crate::config::{Config, Host};
use crate::error::Result;
use crate::install::{install_local, InstallReport};
use crate::resources::{self, UpdateEntry};
use crate::transfer::JobContext;
use std::path::PathBuf;

pub fn list(config: &Config, title_id: &str) -> Vec<UpdateEntry> {
    if title_id.trim().is_empty() {
        return Vec::new();
    }
    resources::updates(&config.ps3_resources_base, title_id.trim())
}

/// Download the newest update and push it to the console.
pub fn download_and_install(
    host: &Host,
    config: &Config,
    update: &UpdateEntry,
    ctx: &JobContext,
) -> Result<InstallReport> {
    let name = update
        .pkg_url
        .split('?')
        .next()
        .and_then(|u| u.rsplit('/').next())
        .filter(|n| n.to_lowercase().ends_with(".pkg"))
        .map(|n| n.to_string())
        .unwrap_or_else(|| format!("{}-{}.pkg", update.title_id, update.version));
    ctx.message(format!("downloading update {}", update.version));
    let local: PathBuf = crate::store::download_url(&update.pkg_url, &name, config, ctx)?;
    let result = install_local(host, &local, config, ctx)?;
    Ok(result)
}
