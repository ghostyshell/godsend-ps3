//! GODsend PS3 command line.
//!
//! Every command talks to one configured console over FTP and webMAN MOD.

use godsend_core::config::{Config, Host};
use godsend_core::convert;
use godsend_core::decrypt;
use godsend_core::device;
use godsend_core::doctor;
use godsend_core::fm;
use godsend_core::homebrew;
use godsend_core::install;
use godsend_core::library;
use godsend_core::pad;
use godsend_core::sources;
use godsend_core::split;
use godsend_core::store;
use godsend_core::transfer::JobManager;
use godsend_core::updates;
use godsend_core::webman::WebMan;
use godsend_core::{Error, Result};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        usage();
        return;
    }
    if let Err(err) = run(&args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn usage() {
    println!(
        r#"GODsend PS3 {version}

Usage: godsend-ps3 <command> [options]

Hosts
  hosts                          list configured consoles
  add-host <addr> [--name N] [--user U] [--pass P] [--ftp-port N] [--http-port N]
  rm-host <id>                   remove a console
  use <id>                       make a console active
  discover [--subnet 192.168.1]  scan the LAN for FTP servers
  status [<id>]                  probe FTP, webMAN, firmware and storage
  doctor                         preflight checks

Library and files
  scan                           list installed games and packages
  ls <remote-path>               list a console directory
  mkdir <remote-path>            create a remote directory
  rm <remote-path> [--dir]       delete a remote file or directory
  push <local> <remote-dir>      upload a file or folder
  pull <remote> [<local-dir>]    download a file or folder

Games
  install <local>                upload a PKG/ISO/folder/ZIP and install it
  mount <remote-path> [--play]   mount a game, optionally launch it
  updates <title-id>             list official update packages
  update-install <title-id>      download and install the newest update
  search <query>                 search the game index
  download <slug>                download a game's best link

Controller and system
  pad <button>                   press one button once
  hold <button>                  press and hold
  release                        release held buttons
  combo <a+b+c> [ms]             press buttons together
  macro "cross,down+start"       run a sequence of presses
  analog <L|R> <direction>       tilt a stick
  notify <text>                  popup on the console
  screenshot                     take an XMB screenshot
  reboot [--quick]               restart
  shutdown [--vsh]               power off

Conversions
  inspect <path>                 show format, system, title id, encryption
  iso <folder> <out.iso>         build an ISO9660 + Joliet image
  folder <in.iso> <out-dir>      extract an ISO
  split <file> [--chunk-mb N]    split for FAT32
  merge <base> <out>             join <base>.0, <base>.1, ...
  decrypt <in.iso> <out.iso> [--key HEX]

Homebrew
  homebrew [--refresh]           list PS3 homebrew packages
  homebrew-install <id>          download and install a homebrew package
"#,
        version = godsend_core::VERSION
    );
}

fn run(args: &[String]) -> Result<()> {
    let mut config = Config::load();
    let cmd = args[0].as_str();
    match cmd {
        "hosts" => {
            for host in &config.hosts {
                let marker = if config.active_host.as_deref() == Some(&host.id) {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{marker} {:<28} {:<20} ftp :{} http :{}",
                    host.id, host.name, host.ftp_port, host.http_port
                );
            }
            if config.hosts.is_empty() {
                println!("no consoles yet: godsend-ps3 add-host 192.168.1.50");
            }
            Ok(())
        }
        "add-host" => {
            let addr = args
                .get(1)
                .ok_or_else(|| Error::Invalid("add-host <addr>".into()))?;
            let name = flag_value(args, "--name").unwrap_or_else(|| addr.clone());
            let mut host = Host::new(&name, addr);
            if let Some(user) = flag_value(args, "--user") {
                host.ftp_user = user;
            }
            if let Some(pass) = flag_value(args, "--pass") {
                host.ftp_pass = pass;
            }
            if let Some(port) = flag_value(args, "--ftp-port").and_then(|v| v.parse().ok()) {
                host.ftp_port = port;
            }
            if let Some(port) = flag_value(args, "--http-port").and_then(|v| v.parse().ok()) {
                host.http_port = port;
            }
            config.upsert_host(host.clone());
            config.save()?;
            println!("added {} ({})", host.name, host.id);
            Ok(())
        }
        "rm-host" => {
            let id = args
                .get(1)
                .ok_or_else(|| Error::Invalid("rm-host <id>".into()))?;
            config.remove_host(id);
            config.save()?;
            println!("removed {id}");
            Ok(())
        }
        "use" => {
            let id = args
                .get(1)
                .ok_or_else(|| Error::Invalid("use <id>".into()))?;
            if !config.hosts.iter().any(|h| &h.id == id) {
                return Err(Error::NotFound(format!("no host {id}")));
            }
            config.active_host = Some(id.clone());
            config.save()?;
            println!("active host: {id}");
            Ok(())
        }
        "discover" => {
            let subnet = flag_value(args, "--subnet").unwrap_or_else(|| auto_subnet());
            println!("scanning {subnet}.1-254 for FTP...");
            let found = device::discover(&subnet)?;
            for host in found {
                println!("found {} ({})", host.addr, host.id);
            }
            Ok(())
        }
        "status" => {
            let hosts = match args.get(1) {
                Some(id) => vec![config
                    .hosts
                    .iter()
                    .find(|h| &h.id == id || &h.addr == id)
                    .cloned()
                    .ok_or_else(|| Error::NotFound(format!("no host {id}")))?],
                None => config.hosts.clone(),
            };
            if hosts.is_empty() {
                return Err(Error::NoDevice);
            }
            for status in device::probe_all(&hosts) {
                println!(
                    "{} ({}) ftp={} webman={} firmware={} model={} free={}",
                    status.name,
                    status.addr,
                    if status.ftp_ok { "up" } else { "down" },
                    if status.webman_ok { "up" } else { "down" },
                    status.firmware.unwrap_or_default(),
                    status.model.unwrap_or_default(),
                    status
                        .free_bytes
                        .map(doctor::human_bytes)
                        .unwrap_or_else(|| "?".into()),
                );
            }
            Ok(())
        }
        "doctor" => {
            for check in doctor::run(&config) {
                println!(
                    "{} {} - {}",
                    if check.ok { "[ok]" } else { "[!!]" },
                    check.name,
                    check.detail
                );
            }
            Ok(())
        }
        "scan" => {
            let host = config.require_active()?;
            let view = library::scan(&host)?;
            for game in &view.games {
                println!(
                    "{:<28} {:<10} {:<5} {}",
                    game.title_id.clone().unwrap_or_else(|| "-".into()),
                    game.system,
                    game.kind,
                    game.path
                );
            }
            for pkg in &view.packages {
                println!(
                    "{:<28} pkg       -     {}",
                    pkg.title_id.clone().unwrap_or_else(|| "-".into()),
                    pkg.path
                );
            }
            println!(
                "{} games, {} packages",
                view.games.len(),
                view.packages.len()
            );
            Ok(())
        }
        "ls" => {
            let host = config.require_active()?;
            let path = args
                .get(1)
                .ok_or_else(|| Error::Invalid("ls <remote-path>".into()))?;
            for entry in fm::remote_list(&host, path)? {
                println!(
                    "{} {:>12} {}",
                    if entry.is_dir { "d" } else { "-" },
                    entry.size_bytes,
                    entry.path
                );
            }
            Ok(())
        }
        "mkdir" => {
            let host = config.require_active()?;
            let path = args
                .get(1)
                .ok_or_else(|| Error::Invalid("mkdir <remote-path>".into()))?;
            fm::mkdir_remote(&host, path)?;
            println!("created {path}");
            Ok(())
        }
        "rm" => {
            let host = config.require_active()?;
            let path = args
                .get(1)
                .ok_or_else(|| Error::Invalid("rm <remote-path>".into()))?;
            if !has_flag(args, "--dir") {
                return Err(Error::Invalid(
                    "refusing to delete without --dir (or use File Manager in the app)".into(),
                ));
            }
            fm::delete_remote(&host, path, true)?;
            println!("deleted {path}");
            Ok(())
        }
        "push" => {
            let host = config.require_active()?;
            let local = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("push <local> <remote-dir>".into()))?,
            );
            let remote = args
                .get(2)
                .ok_or_else(|| Error::Invalid("push <local> <remote-dir>".into()))?;
            let remote = remote.clone();
            run_job("upload", move |ctx| {
                let path = fm::upload(&host, &local, &remote, config.split_bytes, ctx)?;
                ctx.message(format!("uploaded to {path}"));
                Ok(())
            })
        }
        "pull" => {
            let host = config.require_active()?;
            let remote = args
                .get(1)
                .ok_or_else(|| Error::Invalid("pull <remote> [local-dir]".into()))?
                .clone();
            let local = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            }));
            run_job("download", move |ctx| {
                let path = fm::download(&host, &remote, &local, ctx)?;
                ctx.message(format!("saved to {}", path.display()));
                Ok(())
            })
        }
        "install" => {
            let host = config.require_active()?;
            let local = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("install <local>".into()))?,
            );
            run_job("install", move |ctx| {
                let report = install::install_local(&host, &local, &config_for_job(), ctx)?;
                ctx.message(report.message);
                Ok(())
            })
        }
        "mount" => {
            let host = config.require_active()?;
            let path = args
                .get(1)
                .ok_or_else(|| Error::Invalid("mount <remote-path>".into()))?;
            install::mount_and_play(&host, path, has_flag(args, "--play"))?;
            println!("mounted {path}");
            Ok(())
        }
        "updates" => {
            let id = args
                .get(1)
                .ok_or_else(|| Error::Invalid("updates <title-id>".into()))?;
            let rows = updates::list(&config, id);
            for row in rows {
                println!("{:<8} {:<40} {}", row.version, row.name, row.pkg_url);
            }
            Ok(())
        }
        "update-install" => {
            let host = config.require_active()?;
            let id = args
                .get(1)
                .ok_or_else(|| Error::Invalid("update-install <title-id>".into()))?
                .clone();
            let rows = updates::list(&config, &id);
            let newest = rows
                .into_iter()
                .next()
                .ok_or_else(|| Error::NotFound(format!("no updates for {id}")))?;
            let cfg = config.clone();
            run_job("update", move |ctx| {
                let report = updates::download_and_install(&host, &cfg, &newest, ctx)?;
                ctx.message(report.message);
                Ok(())
            })
        }
        "search" => {
            let query = args
                .get(1)
                .ok_or_else(|| Error::Invalid("search <query>".into()))?;
            for hit in sources::gamesearch(&config.gamesearch_base, query, 20)? {
                println!(
                    "{:<50} {:<8} {} links",
                    hit.title, hit.platform, hit.link_count
                );
            }
            Ok(())
        }
        "download" => {
            let slug = args
                .get(1)
                .ok_or_else(|| Error::Invalid("download <slug>".into()))?
                .clone();
            let hit = sources::gamesearch_detail(&config.gamesearch_base, &slug)?;
            let cfg = config.clone();
            run_job("download", move |ctx| {
                let path = store::download_hit(&hit, &cfg, ctx)?;
                ctx.message(format!("downloaded to {}", path.display()));
                Ok(())
            })
        }
        "pad" => {
            let host = config.require_active()?;
            let name = args
                .get(1)
                .ok_or_else(|| Error::Invalid("pad <button>".into()))?;
            pad::press(&WebMan::new(&host), name)?;
            println!("pressed {name}");
            Ok(())
        }
        "hold" => {
            let host = config.require_active()?;
            let name = args
                .get(1)
                .ok_or_else(|| Error::Invalid("hold <button>".into()))?;
            pad::hold(&WebMan::new(&host), name)?;
            println!("holding {name}");
            Ok(())
        }
        "release" => {
            let host = config.require_active()?;
            pad::release(&WebMan::new(&host))?;
            println!("released");
            Ok(())
        }
        "combo" => {
            let host = config.require_active()?;
            let combo = args
                .get(1)
                .ok_or_else(|| Error::Invalid("combo <a+b>".into()))?;
            let parts: Vec<&str> = combo
                .split('+')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            let ms = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(120);
            pad::combo(&WebMan::new(&host), &parts, ms)?;
            println!("pressed {combo}");
            Ok(())
        }
        "macro" => {
            let host = config.require_active()?;
            let script = args
                .get(1)
                .ok_or_else(|| Error::Invalid("macro \"cross,down+start\"".into()))?;
            pad::macro_run(&WebMan::new(&host), script, 120)?;
            println!("macro done");
            Ok(())
        }
        "analog" => {
            let host = config.require_active()?;
            let side = args
                .get(1)
                .ok_or_else(|| Error::Invalid("analog <L|R> <direction>".into()))?;
            let dir = args
                .get(2)
                .ok_or_else(|| Error::Invalid("analog <L|R> <direction>".into()))?;
            WebMan::new(&host).analog(side, dir)?;
            println!("{side} stick {dir}");
            Ok(())
        }
        "notify" => {
            let host = config.require_active()?;
            let text = args
                .get(1)
                .ok_or_else(|| Error::Invalid("notify <text>".into()))?;
            WebMan::new(&host).notify(text)?;
            println!("sent");
            Ok(())
        }
        "screenshot" => {
            let host = config.require_active()?;
            WebMan::new(&host).screenshot()?;
            println!("screenshot requested");
            Ok(())
        }
        "reboot" => {
            let host = config.require_active()?;
            WebMan::new(&host).reboot(has_flag(args, "--quick"))?;
            println!("restarting");
            Ok(())
        }
        "shutdown" => {
            let host = config.require_active()?;
            WebMan::new(&host).shutdown(has_flag(args, "--vsh"))?;
            println!("shutting down");
            Ok(())
        }
        "inspect" => {
            let path = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("inspect <path>".into()))?,
            );
            let info = convert::inspect(&path)?;
            println!(
                "{}\n  kind={} system={} title_id={} size={} encrypted={} parts={}\n  installs to {}",
                info.path,
                info.kind,
                info.system,
                info.title_id.unwrap_or_else(|| "-".into()),
                doctor::human_bytes(info.size_bytes),
                info.encrypted,
                info.parts,
                convert::install_destination(&path)
            );
            Ok(())
        }
        "iso" => {
            let src = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("iso <folder> <out.iso>".into()))?,
            );
            let out = PathBuf::from(
                args.get(2)
                    .ok_or_else(|| Error::Invalid("iso <folder> <out.iso>".into()))?,
            );
            run_job("build iso", move |ctx| {
                let bytes = convert::folder_to_iso(&src, &out, ctx)?;
                ctx.message(format!("wrote {bytes} bytes"));
                Ok(())
            })
        }
        "folder" => {
            let src = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("folder <in.iso> <out-dir>".into()))?,
            );
            let out = PathBuf::from(
                args.get(2)
                    .ok_or_else(|| Error::Invalid("folder <in.iso> <out-dir>".into()))?,
            );
            run_job("extract iso", move |ctx| {
                let bytes = convert::iso_to_folder(&src, &out, ctx)?;
                ctx.message(format!("extracted {bytes} bytes"));
                Ok(())
            })
        }
        "split" => {
            let src = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("split <file>".into()))?,
            );
            let chunk = flag_value(args, "--chunk-mb")
                .and_then(|v| v.parse::<u64>().ok())
                .map(|mb| mb * 1024 * 1024)
                .unwrap_or(split::DEFAULT_CHUNK);
            run_job("split", move |ctx| {
                let parts = convert::split_file(&src, chunk, ctx)?;
                ctx.message(format!("{} parts", parts.len()));
                Ok(())
            })
        }
        "merge" => {
            let base = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("merge <base> <out>".into()))?,
            );
            let out = PathBuf::from(
                args.get(2)
                    .ok_or_else(|| Error::Invalid("merge <base> <out>".into()))?,
            );
            run_job("merge", move |ctx| {
                let bytes = convert::merge_file(&base, &out, ctx)?;
                ctx.message(format!("wrote {bytes} bytes"));
                Ok(())
            })
        }
        "decrypt" => {
            let src =
                PathBuf::from(args.get(1).ok_or_else(|| {
                    Error::Invalid("decrypt <in.iso> <out.iso> [--key HEX]".into())
                })?);
            let out =
                PathBuf::from(args.get(2).ok_or_else(|| {
                    Error::Invalid("decrypt <in.iso> <out.iso> [--key HEX]".into())
                })?);
            let key = flag_value(args, "--key").unwrap_or_default();
            run_job("decrypt", move |ctx| {
                let bytes = convert::decrypt_iso(&config_for_job(), &src, &out, &key, ctx)?;
                ctx.message(format!("wrote {bytes} bytes"));
                Ok(())
            })
        }
        "homebrew" => {
            for entry in homebrew::catalog(&config, has_flag(args, "--refresh")) {
                println!(
                    "{:<16} {:<22} {:<8} {}",
                    entry.id,
                    entry.name,
                    entry.latest_tag.unwrap_or_else(|| "-".into()),
                    entry.pkg_url.unwrap_or_default()
                );
            }
            Ok(())
        }
        "homebrew-install" => {
            let host = config.require_active()?;
            let id = args
                .get(1)
                .ok_or_else(|| Error::Invalid("homebrew-install <id>".into()))?
                .clone();
            let entries = homebrew::catalog(&config, false);
            let entry = entries.into_iter().find(|e| e.id == id).ok_or_else(|| {
                Error::NotFound(format!("no homebrew {id}; run `homebrew --refresh`"))
            })?;
            let cfg = config.clone();
            run_job("homebrew", move |ctx| {
                let report = homebrew::install(&host, &cfg, &entry, ctx)?;
                ctx.message(report.message);
                Ok(())
            })
        }
        "decrypt-scan" => {
            let path = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| Error::Invalid("decrypt-scan <iso>".into()))?,
            );
            let scan = decrypt::scan(&path)?;
            println!(
                "{}: {} sectors, {} encrypted in {} ranges",
                path.display(),
                scan.total_sectors,
                scan.encrypted_sectors,
                scan.encrypted_ranges.len()
            );
            Ok(())
        }
        other => Err(Error::Invalid(format!(
            "unknown command '{other}' (run with --help)"
        ))),
    }
}

/// Jobs run on a thread, so they load their own config snapshot.
fn config_for_job() -> Config {
    Config::load()
}

fn run_job<F>(label: &str, work: F) -> Result<()>
where
    F: FnOnce(&godsend_core::transfer::JobContext) -> Result<()> + Send + 'static,
{
    let manager = JobManager::new();
    let id = manager.spawn("cli", label, |_| {}, work);
    let mut last_pct = 0u64;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let Some(job) = manager.get(id) else { continue };
        let pct = if job.total_bytes > 0 {
            job.done_bytes * 100 / job.total_bytes
        } else {
            0
        };
        if pct >= last_pct + 10 {
            last_pct = pct;
            eprintln!("{label}: {pct}%");
        }
        match job.state {
            godsend_core::JobState::Running => {}
            godsend_core::JobState::Done => {
                println!("{label}: done {}", job.message);
                return Ok(());
            }
            godsend_core::JobState::Failed => {
                return Err(Error::Other(job.message));
            }
            godsend_core::JobState::Cancelled => {
                return Err(Error::Other("cancelled".into()));
            }
        }
    }
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let index = args.iter().position(|a| a == flag)?;
    args.get(index + 1).cloned()
}

fn auto_subnet() -> String {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok();
    if let Some(socket) = socket {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                if let std::net::IpAddr::V4(ip) = addr.ip() {
                    let octets = ip.octets();
                    return format!("{}.{}.{}", octets[0], octets[1], octets[2]);
                }
            }
        }
    }
    "192.168.1".into()
}
