//! webMAN MOD HTTP remote-control client.
//!
//! A CFW PS3 running webMAN MOD answers a set of `/xxx.ps3` web commands on
//! port 80. The app uses them for the virtual pad, mounting, launching,
//! installing packages and system actions. Paths are concatenated directly
//! (`/mount.ps3/dev_hdd0/PS3ISO/game.iso`), which is the documented form and
//! the form webMAN's own webPAD page uses.
//!
//! Without webMAN the app still works over FTP; only these remote actions
//! report "webMAN not found".

use crate::config::Host;
use crate::error::{Error, Result};
use std::time::Duration;

#[derive(Clone)]
pub struct WebMan {
    base: String,
}

impl WebMan {
    pub fn new(host: &Host) -> Self {
        Self {
            base: host.http_base(),
        }
    }

    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .build()
    }

    /// GET a web command, returning the response body (webMAN answers with a
    /// short HTML page or, for pad presses, an empty body).
    pub fn get(&self, command: &str) -> Result<String> {
        let url = if command.starts_with("http") {
            command.to_string()
        } else {
            format!("{}/{}", self.base, encode(command.trim_start_matches('/')))
        };
        match Self::agent().get(&url).call() {
            Ok(resp) => Ok(resp.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(code, _)) => {
                Err(Error::Http(format!("webMAN {code} for {command}")))
            }
            Err(e) => Err(Error::Http(format!("webMAN unreachable: {e}"))),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base, path.trim_start_matches('/'))
    }

    // ---- virtual pad ----

    /// Press and release one button, e.g. `cross`, `up`, `l1`, `ps`.
    pub fn pad(&self, key: &str) -> Result<()> {
        self.get(&format!("pad.ps3?{}", key)).map(|_| ())
    }

    /// Press and hold a button. Combine with `release` to lift it.
    pub fn pad_hold(&self, key: &str) -> Result<()> {
        self.get(&format!("pad.ps3?{}_hold", key)).map(|_| ())
    }

    /// Press several buttons together and hold them.
    pub fn pad_combo_hold(&self, keys: &[&str]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        self.get(&format!("pad.ps3?{}_hold", keys.join("_")))
            .map(|_| ())
    }

    /// Release every held button.
    pub fn pad_release(&self) -> Result<()> {
        self.get("pad.ps3?release").map(|_| ())
    }

    /// Tilt an analog stick: side `L`/`R`, direction `up`, `down_left`, ...
    pub fn analog(&self, side: &str, direction: &str) -> Result<()> {
        let side = match side.to_ascii_uppercase().as_str() {
            "L" => "AnalogL",
            "R" => "AnalogR",
            _ => return Err(Error::Invalid(format!("unknown stick {side}"))),
        };
        let direction = direction.to_ascii_lowercase();
        if !matches!(
            direction.as_str(),
            "up" | "down" | "left" | "right" | "up_left" | "up_right" | "down_left" | "down_right"
        ) {
            return Err(Error::Invalid(format!("unknown direction {direction}")));
        }
        self.get(&format!("pad.ps3?{}_{}", side, direction))
            .map(|_| ())
    }

    // ---- games ----

    /// Mount a game by its device path (`/dev_hdd0/PS3ISO/x.iso` or a JB folder).
    pub fn mount(&self, path: &str) -> Result<()> {
        self.get(&format!("mount.ps3{}", normalize(path)))
            .map(|_| ())
    }

    pub fn unmount(&self) -> Result<()> {
        self.get("mount.ps3/unmount").map(|_| ())
    }

    /// Launch a mounted game or an entry path.
    pub fn play(&self, path: &str) -> Result<()> {
        self.get(&format!("play.ps3{}", normalize(path)))
            .map(|_| ())
    }

    /// Ask webMAN to install a PKG that already sits on the console.
    pub fn install(&self, path: &str) -> Result<()> {
        self.get(&format!("install.ps3{}", normalize(path)))
            .map(|_| ())
    }

    /// Copy within the console (`/cp.ps3` takes `$`-separated arguments).
    pub fn copy(&self, src: &str, dst: &str) -> Result<()> {
        self.get(&format!("cp.ps3${}${}", src, dst)).map(|_| ())
    }

    // ---- system ----

    pub fn notify(&self, text: &str) -> Result<()> {
        self.get(&format!("popup.ps3@info{}", text)).map(|_| ())
    }

    pub fn browser(&self, url: &str) -> Result<()> {
        self.get(&format!("browser.ps3${}", url)).map(|_| ())
    }

    pub fn xmb(&self, command: &str) -> Result<()> {
        self.get(&format!("xmb.ps3${}", command)).map(|_| ())
    }

    pub fn screenshot(&self) -> Result<()> {
        self.get("xmb.ps3$screenshot?show").map(|_| ())
    }

    pub fn refresh_xml(&self) -> Result<()> {
        self.get("refresh.ps3").map(|_| ())
    }

    pub fn rebuild_db(&self) -> Result<()> {
        self.get("rebuild.ps3").map(|_| ())
    }

    pub fn reboot(&self, quick: bool) -> Result<()> {
        self.get(if quick {
            "restart.ps3?quick"
        } else {
            "restart.ps3"
        })
        .map(|_| ())
    }

    pub fn shutdown(&self, vsh: bool) -> Result<()> {
        self.get(if vsh {
            "shutdown.ps3?vsh"
        } else {
            "shutdown.ps3"
        })
        .map(|_| ())
    }

    pub fn toggle_external_gamedata(&self) -> Result<()> {
        self.get("extgd.ps3").map(|_| ())
    }

    /// Raw `/cpursx.ps3` system page.
    pub fn sysinfo(&self) -> Result<String> {
        self.get("cpursx.ps3")
    }

    /// True when the host answers a webMAN command.
    pub fn available(&self) -> bool {
        self.get("cpursx.ps3").is_ok()
    }
}

/// Percent-encode the characters that would break a URL or the command parser.
fn encode(command: &str) -> String {
    command
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23")
}

/// `/dev_hdd0/x` stays as is; `dev_hdd0/x` gains the leading slash.
fn normalize(path: &str) -> String {
    let p = path.trim();
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize("dev_hdd0/x.iso"), "/dev_hdd0/x.iso");
        assert_eq!(normalize("/dev_hdd0/x.iso"), "/dev_hdd0/x.iso");
    }

    #[test]
    fn rejects_unknown_sticks() {
        let host = Host::new("t", "127.0.0.1");
        let wm = WebMan::new(&host);
        assert!(wm.analog("X", "up").is_err());
        assert!(wm.analog("L", "sideways").is_err());
    }
}
