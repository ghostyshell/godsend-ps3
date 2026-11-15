//! Preflight checks shown on the Home page.

use crate::config::{Config, Host};
use crate::device;
use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

pub fn run(config: &Config) -> Vec<Check> {
    let mut checks = Vec::new();

    match config.active() {
        Some(host) => checks.push(Check {
            name: "Console configured".into(),
            ok: true,
            detail: format!("{} at {}", host.name, host.addr),
        }),
        None => checks.push(Check {
            name: "Console configured".into(),
            ok: false,
            detail: "Add a PS3 host on the Devices page".into(),
        }),
    }

    if let Some(host) = config.active() {
        let status = device::probe(&host);
        checks.push(Check {
            name: "FTP reachable".into(),
            ok: status.ftp_ok,
            detail: if status.ftp_ok {
                format!("{} ms", status.ftp_ms)
            } else {
                "start the FTP server on the console (webMAN MOD, multiMAN, IRISMAN)".into()
            },
        });
        checks.push(Check {
            name: "webMAN MOD".into(),
            ok: status.webman_ok,
            detail: if status.webman_ok {
                status
                    .webman_version
                    .map(|v| format!("v{v}"))
                    .unwrap_or_else(|| "answered on the HTTP port".into())
            } else {
                "remote control, mounting and the virtual pad need webMAN MOD".into()
            },
        });
        checks.push(Check {
            name: "Firmware".into(),
            ok: status.firmware.is_some(),
            detail: status
                .firmware
                .clone()
                .or(status.model.clone())
                .unwrap_or_else(|| "not reported".into()),
        });
        if let Some(bytes) = status.free_bytes {
            checks.push(Check {
                name: "Internal HDD space".into(),
                ok: bytes > 5 * 1024 * 1024 * 1024,
                detail: format!("{} free", human_bytes(bytes)),
            });
        }
    }

    let download_dir = crate::aria2::download_dir(config);
    let writable = std::fs::create_dir_all(&download_dir).is_ok()
        && std::fs::write(download_dir.join(".probe"), b"ok").is_ok();
    if writable {
        let _ = std::fs::remove_file(download_dir.join(".probe"));
    }
    checks.push(Check {
        name: "Download folder".into(),
        ok: writable,
        detail: download_dir.display().to_string(),
    });

    checks.push(Check {
        name: "aria2c".into(),
        ok: crate::aria2::find_aria2(&config.aria2_path).is_some(),
        detail: match crate::aria2::find_aria2(&config.aria2_path) {
            Some(path) => path.display().to_string(),
            None => "optional: downloads fall back to a single connection".into(),
        },
    });

    checks.push(Check {
        name: "Debrid".into(),
        ok: !config.rd_api_key.is_empty() || !config.tb_api_key.is_empty(),
        detail: match (config.rd_api_key.is_empty(), config.tb_api_key.is_empty()) {
            (false, false) => "Real-Debrid and TorBox connected".into(),
            (false, true) => "Real-Debrid connected".into(),
            (true, false) => "TorBox connected".into(),
            (true, true) => "optional: keys unlock debrid download links".into(),
        },
    });

    checks.push(Check {
        name: "MCP server".into(),
        ok: config.mcp_port > 0,
        detail: if config.mcp_port == 0 {
            "disabled (set an MCP port to enable it)".into()
        } else {
            format!(
                "http://127.0.0.1:{}/mcp, started with the app",
                config.mcp_port
            )
        },
    });

    checks.push(Check {
        name: "ps3-resources".into(),
        ok: reachable(&config.ps3_resources_base),
        detail: if reachable(&config.ps3_resources_base) {
            format!(
                "{} (title ids, updates, disc keys)",
                config.ps3_resources_base
            )
        } else {
            format!("{} is not answering", config.ps3_resources_base)
        },
    });

    checks
}

pub fn host_checks(host: &Host) -> Result<Vec<Check>> {
    let status = device::probe(host);
    Ok(vec![
        Check {
            name: "FTP".into(),
            ok: status.ftp_ok,
            detail: format!("{} ms", status.ftp_ms),
        },
        Check {
            name: "webMAN".into(),
            ok: status.webman_ok,
            detail: status.webman_version.unwrap_or_default(),
        },
        Check {
            name: "Firmware".into(),
            ok: status.firmware.is_some(),
            detail: status.firmware.unwrap_or_default(),
        },
    ])
}

fn reachable(base: &str) -> bool {
    let url = format!("{}/api/titleid?limit=1", base.trim_end_matches('/'));
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .get(&url)
        .call()
        .is_ok()
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
    }
}
