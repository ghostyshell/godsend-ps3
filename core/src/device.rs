//! PS3 host probing: FTP reachability, webMAN MOD detection and system info.

use crate::config::Host;
use crate::error::Result;
use crate::ftp;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeviceStatus {
    pub host_id: String,
    pub name: String,
    pub addr: String,
    pub ftp_ok: bool,
    pub webman_ok: bool,
    pub ftp_ms: u64,
    pub http_ms: u64,
    /// e.g. "4.90 CEX Evilnat"
    pub firmware: Option<String>,
    pub model: Option<String>,
    pub webman_version: Option<String>,
    /// Internal HDD (/dev_hdd0) free and total when webMAN reports them.
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    /// First line of the raw webMAN system page, for the inspector card.
    pub message: String,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build()
}

/// Probe a console. Never errors: unreachable pieces are reported as false.
pub fn probe(host: &Host) -> DeviceStatus {
    let mut status = DeviceStatus {
        host_id: host.id.clone(),
        name: host.name.clone(),
        addr: host.addr.clone(),
        ..Default::default()
    };

    let started = Instant::now();
    match ftp::connect(host) {
        Ok(mut conn) => {
            status.ftp_ok = conn.pwd().is_ok();
            status.ftp_ms = started.elapsed().as_millis() as u64;
            let _ = conn.quit();
        }
        Err(e) => status.message = e.to_string(),
    }

    let started = Instant::now();
    match agent()
        .get(&format!("{}/cpursx.ps3", host.http_base()))
        .call()
    {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            status.http_ms = started.elapsed().as_millis() as u64;
            status.webman_ok = !body.is_empty();
            parse_sysinfo(&body, &mut status);
        }
        Err(e) => {
            if status.message.is_empty() {
                status.message = format!("HTTP: {e}");
            }
        }
    }
    status
}

/// Extract firmware, model, free space and webMAN version from the system page.
/// webMAN's page markup changes between builds, so every field is best effort.
pub fn parse_sysinfo(body: &str, status: &mut DeviceStatus) {
    let text = strip_tags(body);
    let lower = text.to_lowercase();

    if status.message.is_empty() {
        status.message = text.lines().next().unwrap_or("").trim().to_string();
    }

    // Firmware: first `4.xx` / `3.xx` looking token.
    for token in text.split(|c: char| c.is_whitespace() || c == ')' || c == '(') {
        let t = token.trim();
        if (t.starts_with("4.") || t.starts_with("3.")) && t.len() <= 10 {
            status.firmware = Some(t.to_string());
            break;
        }
    }

    for cfw in [
        "Evilnat", "Rebug", "Ferrox", "Overflow", "PS3HEN", "HEN", "Cobra", "DEX", "CEX",
    ] {
        if lower.contains(&cfw.to_lowercase()) {
            let fw = status.firmware.clone().unwrap_or_default();
            if !fw.to_lowercase().contains(&cfw.to_lowercase()) {
                status.firmware = Some(format!("{fw} {cfw}").trim().to_string());
            }
            break;
        }
    }

    if let Some(model) = find_token(&text, |t| {
        t.starts_with("CECH-")
            || t.starts_with("CECHH")
            || t.starts_with("CECHK")
            || t.starts_with("CECHL")
    }) {
        status.model = Some(model);
    }

    if let Some(v) = find_version(&text) {
        status.webman_version = Some(v);
    }

    // Storage: `HDD0: 123.4 GB / 456.7 GB` or `123.4/456.7 GB`.
    if let Some((used, total)) = parse_storage(&text) {
        status.total_bytes = Some(total);
        status.free_bytes = Some(total.saturating_sub(used));
    }
}

fn find_token(text: &str, pred: impl Fn(&str) -> bool) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '<' || c == '>')
        .map(|t| t.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-'))
        .find(|t| pred(t))
        .map(|t| t.to_string())
}

fn find_version(text: &str) -> Option<String> {
    let idx = text.to_lowercase().find("webman")?;
    let rest = &text[idx..];
    rest.split(|c: char| c.is_whitespace() || c == '/' || c == ')')
        .skip(1)
        .find(|t| {
            t.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
        .map(|t| {
            t.trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                .to_string()
        })
}

fn parse_storage(text: &str) -> Option<(u64, u64)> {
    // Look for "<used> / <total> GB" anywhere; webMAN writes this for HDD0.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            let before = &text[..i];
            let after = &text[i + 1..];
            let used = before
                .trim_end()
                .rsplit(|c: char| !(c.is_ascii_digit() || c == '.'))
                .find(|s| !s.is_empty())
                .unwrap_or("");
            let total = after
                .trim_start()
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .find(|s| !s.is_empty())
                .unwrap_or("");
            let unit_gb = after.to_lowercase().contains("gb");
            if let (Ok(u), Ok(t)) = (used.parse::<f64>(), total.parse::<f64>()) {
                if u > 0.0 && t > u {
                    let scale = if unit_gb {
                        1024.0 * 1024.0 * 1024.0
                    } else {
                        1024.0 * 1024.0
                    };
                    return Some(((u * scale) as u64, (t * scale) as u64));
                }
            }
        }
        i += 1;
    }
    None
}

/// Drop HTML tags and collapse whitespace, keeping line breaks between blocks.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Probe every configured host in parallel.
pub fn probe_all(hosts: &[Host]) -> Vec<DeviceStatus> {
    let mut handles = Vec::new();
    for host in hosts {
        let host = host.clone();
        handles.push(std::thread::spawn(move || probe(&host)));
    }
    handles.into_iter().filter_map(|h| h.join().ok()).collect()
}

/// Best-effort LAN discovery: a host answering FTP on 21 is a candidate.
pub fn discover(subnet_prefix: &str) -> Result<Vec<Host>> {
    use std::net::{SocketAddr, TcpStream};
    let mut found = Vec::new();
    let mut handles = Vec::new();
    for last in 1..=254u8 {
        let addr = format!("{subnet_prefix}.{last}");
        handles.push(std::thread::spawn(move || {
            let sock: SocketAddr = format!("{addr}:21").parse().ok()?;
            let conn = TcpStream::connect_timeout(&sock, Duration::from_millis(220)).ok()?;
            let _ = conn.set_read_timeout(Some(Duration::from_millis(220)));
            Some(addr)
        }));
        if handles.len() >= 64 {
            for h in handles.drain(..) {
                if let Ok(Some(addr)) = h.join() {
                    found.push(Host::new("", &addr));
                }
            }
        }
    }
    for h in handles {
        if let Ok(Some(addr)) = h.join() {
            found.push(Host::new("", &addr));
        }
    }
    found.sort_by(|a, b| a.addr.cmp(&b.addr));
    // A bare TCP connect only proves port 21 is open; drop obvious non-consoles
    // (the local machine) and confirm each candidate answers as a PS3.
    found.retain(|h| h.addr != local_ipv4().unwrap_or_default());
    Ok(found
        .into_iter()
        .filter(|h| probe(h).ftp_ok)
        .map(|mut h| {
            h.name = format!("PS3 {}", h.addr);
            h
        })
        .collect())
}

fn local_ipv4() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_webman_page() {
        let body = r#"<html><body><div>webMAN MOD 1.47.45 4.90 CEX Evilnat 4.90</div>
        <div>CECH-2501A HDD0: 120.5 GB / 465.7 GB</div></body></html>"#;
        let mut s = DeviceStatus::default();
        parse_sysinfo(body, &mut s);
        assert!(s.firmware.as_deref().unwrap_or("").contains("4.90"));
        assert!(s.firmware.as_deref().unwrap_or("").contains("Evilnat"));
        assert_eq!(s.model.as_deref(), Some("CECH-2501A"));
        assert_eq!(s.webman_version.as_deref(), Some("1.47.45"));
        let total = s.total_bytes.unwrap();
        let free = s.free_bytes.unwrap();
        assert!(total > 400_000_000_000);
        assert!(free < total);
    }

    #[test]
    fn tolerates_unparseable_pages() {
        let mut s = DeviceStatus::default();
        parse_sysinfo("<b>hello</b>", &mut s);
        assert_eq!(s.firmware, None);
        assert_eq!(s.free_bytes, None);
        assert_eq!(s.message, "hello");
    }
}
