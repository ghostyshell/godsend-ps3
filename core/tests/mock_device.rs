//! End-to-end test against a fake PS3: a minimal FTP server plus a webMAN HTTP
//! server. Exercises library scanning, upload/download and the pad/mount paths
//! exactly as a real console would be driven.

use godsend_core::config::Host;
use godsend_core::ftp;
use godsend_core::homebrew;
use godsend_core::install;
use godsend_core::library;
use godsend_core::pad;
use godsend_core::pkg;
use godsend_core::sfo::{build_sfo, SfoValue};
use godsend_core::transfer::JobManager;
use godsend_core::webman::WebMan;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

thread_local! {
    static PASSIVE: std::cell::RefCell<Option<TcpListener>> = std::cell::RefCell::new(None);
}

struct FakeFtp {
    port: u16,
}

impl FakeFtp {
    fn start(root: PathBuf) -> FakeFtp {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ftp");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let root = root.clone();
                std::thread::spawn(move || serve_control(stream, root));
            }
        });
        FakeFtp { port }
    }

    fn host(&self, http_port: u16) -> Host {
        let mut host = Host::new("test ps3", "127.0.0.1");
        host.ftp_port = self.port;
        host.http_port = http_port;
        host
    }
}

fn map(root: &Path, remote: &str) -> PathBuf {
    root.join(remote.trim_start_matches('/'))
}

fn serve_control(stream: TcpStream, root: PathBuf) {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    let mut cwd = "/".to_string();
    fn reply(writer: &mut TcpStream, text: &str) {
        let _ = writer.write_all(format!("{text}\r\n").as_bytes());
    }
    reply(&mut writer, "220 mock ftp ready");
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        let (cmd, arg) = match line.split_once(' ') {
            Some((c, a)) => (c.to_uppercase(), a.trim().to_string()),
            None => (line.to_uppercase(), String::new()),
        };
        match cmd.as_str() {
            "USER" => reply(&mut writer, "331 password required"),
            "PASS" => reply(&mut writer, "230 logged in"),
            "TYPE" | "NOOP" | "OPTS" | "MODE" | "STRU" | "SITE" | "ABOR" => {
                reply(&mut writer, "200 ok")
            }
            "SYST" => reply(&mut writer, "215 UNIX Type: L8"),
            "FEAT" => {
                let _ = writer.write_all(b"211-Features:\r\n UTF8\r\n211 End\r\n");
            }
            "PWD" => reply(&mut writer, &format!("257 \"{cwd}\"")),
            "CWD" => {
                let target = if arg.starts_with('/') {
                    arg.clone()
                } else {
                    format!("{}/{}", cwd.trim_end_matches('/'), arg)
                };
                if map(&root, &target).is_dir() {
                    cwd = target;
                    reply(&mut writer, "250 ok");
                } else {
                    reply(&mut writer, "550 no such directory");
                }
            }
            "CDUP" => {
                if let Some((parent, _)) = cwd.trim_end_matches('/').rsplit_once('/') {
                    cwd = if parent.is_empty() {
                        "/".into()
                    } else {
                        parent.into()
                    };
                }
                reply(&mut writer, "250 ok");
            }
            "PASV" | "EPSV" => {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let port = listener.local_addr().unwrap().port();
                if cmd == "EPSV" {
                    reply(
                        &mut writer,
                        &format!("229 Entering Extended Passive Mode (|||{port}|)"),
                    );
                } else {
                    reply(
                        &mut writer,
                        &format!(
                            "227 Entering Passive Mode (127,0,0,1,{},{})",
                            port / 256,
                            port % 256
                        ),
                    );
                }
                PASSIVE.with(|slot| *slot.borrow_mut() = Some(listener));
            }
            "MLSD" => reply(&mut writer, "502 MLSD not supported"),
            "LIST" => {
                let target = if arg.is_empty() || arg.starts_with('-') {
                    cwd.clone()
                } else {
                    arg.clone()
                };
                let path = map(&root, &target);
                reply(&mut writer, "150 here it comes");
                if let Some(listener) = PASSIVE.with(|slot| slot.borrow_mut().take()) {
                    if let Ok((mut data, _)) = listener.accept() {
                        if let Ok(entries) = std::fs::read_dir(&path) {
                            for entry in entries.flatten() {
                                let meta = entry.metadata().unwrap();
                                let name = entry.file_name().to_string_lossy().to_string();
                                let perm = if meta.is_dir() {
                                    "drwxr-xr-x"
                                } else {
                                    "-rw-r--r--"
                                };
                                let _ = data.write_all(
                                    format!(
                                        "{perm} 1 root root {} Jan 01 12:00 {name}\r\n",
                                        meta.len()
                                    )
                                    .as_bytes(),
                                );
                            }
                        }
                        let _ = data.flush();
                    }
                }
                reply(&mut writer, "226 transfer complete");
            }
            "SIZE" => {
                let path = map(&root, &arg);
                match std::fs::metadata(&path) {
                    Ok(meta) => reply(&mut writer, &format!("213 {}", meta.len())),
                    Err(_) => reply(&mut writer, "550 no such file"),
                }
            }
            "RETR" => {
                let path = map(&root, &arg);
                reply(&mut writer, "150 sending");
                if let Some(listener) = PASSIVE.with(|slot| slot.borrow_mut().take()) {
                    if let Ok((mut data, _)) = listener.accept() {
                        if let Ok(mut file) = std::fs::File::open(&path) {
                            let _ = std::io::copy(&mut file, &mut data);
                        }
                        let _ = data.flush();
                    }
                }
                reply(&mut writer, "226 transfer complete");
            }
            "STOR" => {
                let path = map(&root, &arg);
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                reply(&mut writer, "150 receiving");
                if let Some(listener) = PASSIVE.with(|slot| slot.borrow_mut().take()) {
                    if let Ok((mut data, _)) = listener.accept() {
                        let mut file = std::fs::File::create(&path).unwrap();
                        let _ = std::io::copy(&mut data, &mut file);
                    }
                }
                reply(&mut writer, "226 transfer complete");
            }
            "MKD" => {
                let _ = std::fs::create_dir_all(map(&root, &arg));
                reply(&mut writer, &format!("257 \"{arg}\" created"));
            }
            "DELE" => {
                let _ = std::fs::remove_file(map(&root, &arg));
                reply(&mut writer, "250 deleted");
            }
            "RMD" => {
                let _ = std::fs::remove_dir_all(map(&root, &arg));
                reply(&mut writer, "250 removed");
            }
            "QUIT" => {
                reply(&mut writer, "221 bye");
                return;
            }
            _ => reply(&mut writer, "502 not implemented"),
        }
    }
}

fn fake_webman() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_copy = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let seen = seen_copy.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                let _ = reader.read_line(&mut request);
                if let Some(path) = request.split_whitespace().nth(1) {
                    seen.lock().unwrap().push(path.to_string());
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            });
        }
    });
    (port, seen)
}

fn pkg_bytes() -> Vec<u8> {
    let mut head = vec![0u8; 512];
    head[0..4].copy_from_slice(&pkg::MAGIC);
    head[4..8].copy_from_slice(&1u32.to_be_bytes());
    head[0x30..0x30 + 36].copy_from_slice(b"UP0001-NPUB30698_00-0000000000000000");
    head
}

fn fixture_tree() -> PathBuf {
    let root = std::env::temp_dir().join(format!("godsend-ps3-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("dev_hdd0/game/BLUS30443")).unwrap();
    std::fs::write(
        root.join("dev_hdd0/game/BLUS30443/PARAM.SFO"),
        build_sfo(&[
            ("TITLE", SfoValue::Utf8("Demon's Souls".into())),
            ("TITLE_ID", SfoValue::Utf8("BLUS30443".into())),
            ("CATEGORY", SfoValue::Utf8("DG".into())),
            ("VERSION", SfoValue::Utf8("01.00".into())),
        ]),
    )
    .unwrap();
    std::fs::write(
        root.join("dev_hdd0/game/BLUS30443/ICON0.PNG"),
        b"\x89PNG\r\n\x1a\nfake",
    )
    .unwrap();

    std::fs::create_dir_all(root.join("dev_hdd0/PS3ISO")).unwrap();
    std::fs::write(
        root.join("dev_hdd0/PS3ISO/God of War Collection (USA).iso"),
        vec![0u8; 4096],
    )
    .unwrap();

    std::fs::create_dir_all(root.join("dev_hdd0/GAMES/My JB Game/PS3_GAME")).unwrap();
    std::fs::write(
        root.join("dev_hdd0/GAMES/My JB Game/PS3_GAME/PARAM.SFO"),
        build_sfo(&[
            ("TITLE", SfoValue::Utf8("My JB Game".into())),
            ("TITLE_ID", SfoValue::Utf8("BLES01234".into())),
        ]),
    )
    .unwrap();

    std::fs::create_dir_all(root.join("dev_hdd0/packages")).unwrap();
    std::fs::write(root.join("dev_hdd0/packages/update.pkg"), pkg_bytes()).unwrap();
    root
}

#[test]
fn drives_a_fake_console_end_to_end() {
    let root = fixture_tree();
    let ftp_server = FakeFtp::start(root.clone());
    let (http_port, seen) = fake_webman();
    let host = ftp_server.host(http_port);

    // 1. Library scan sees digital games, JB folders, ISOs and packages.
    let view = library::scan(&host).expect("library scan");
    let titles: Vec<&str> = view.games.iter().map(|g| g.title.as_str()).collect();
    assert!(titles.contains(&"Demon's Souls"), "titles: {titles:?}");
    assert!(titles.contains(&"My JB Game"), "titles: {titles:?}");
    assert!(titles.iter().any(|t| t.contains("God of War")));
    let demon = view
        .games
        .iter()
        .find(|g| g.title == "Demon's Souls")
        .unwrap();
    assert_eq!(demon.title_id.as_deref(), Some("BLUS30443"));
    assert_eq!(view.packages.len(), 1);
    assert_eq!(view.packages[0].title_id.as_deref(), Some("NPUB30698"));

    // 2. Icon fetch over FTP.
    let icon = library::icon_bytes(&host, "/dev_hdd0/game/BLUS30443/ICON0.PNG").expect("icon");
    assert!(icon.starts_with(b"\x89PNG"));

    // 3. Upload a local file and download it back.
    let local = root.join("upload.bin");
    std::fs::write(&local, b"hello ps3").unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let host_upload = host.clone();
    let local_upload = local.clone();
    JobManager::new().spawn(
        "upload",
        "u",
        |_| {},
        move |ctx| {
            let mut conn = ftp::connect(&host_upload)?;
            ftp::upload(
                &mut conn,
                &local_upload,
                "/dev_hdd0/packages/upload.bin",
                ctx,
            )?;
            let _ = conn.quit();
            tx.send(()).unwrap();
            Ok(())
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("upload finished");
    assert_eq!(
        std::fs::read(root.join("dev_hdd0/packages/upload.bin")).unwrap(),
        b"hello ps3"
    );

    let mut conn = ftp::connect(&host).unwrap();
    let listing = ftp::list(&mut conn, "/dev_hdd0/packages").unwrap();
    assert!(listing
        .iter()
        .any(|e| e.name == "upload.bin" && e.size_bytes == 9));
    let _ = conn.quit();

    let downloaded = root.join("downloaded.bin");
    let (tx2, rx2) = std::sync::mpsc::channel();
    let host_download = host.clone();
    JobManager::new().spawn(
        "download",
        "d",
        |_| {},
        move |ctx| {
            let mut conn = ftp::connect(&host_download)?;
            ftp::download(&mut conn, "/dev_hdd0/packages/upload.bin", &downloaded, ctx)?;
            let _ = conn.quit();
            tx2.send(()).unwrap();
            Ok(())
        },
    );
    rx2.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(
        std::fs::read(root.join("downloaded.bin")).unwrap(),
        b"hello ps3"
    );

    // 4. Virtual pad reaches webMAN with the right commands.
    pad::press(&WebMan::new(&host), "cross").expect("pad press");
    pad::release(&WebMan::new(&host)).expect("pad release");
    pad::combo(&WebMan::new(&host), &["l3", "r3"], 60).expect("combo");
    let paths = seen.lock().unwrap().clone();
    assert!(
        paths.iter().any(|p| p == "/pad.ps3?cross"),
        "paths: {paths:?}"
    );
    assert!(paths.iter().any(|p| p == "/pad.ps3?release"));
    assert!(paths.iter().any(|p| p == "/pad.ps3?l3_r3_hold"));

    // 5. Mount percent-encodes spaces in the ISO name.
    install::mount_and_play(
        &host,
        "/dev_hdd0/PS3ISO/God of War Collection (USA).iso",
        false,
    )
    .expect("mount");
    let paths = seen.lock().unwrap().clone();
    assert!(
        paths
            .iter()
            .any(|p| p == "/mount.ps3/dev_hdd0/PS3ISO/God%20of%20War%20Collection%20(USA).iso"),
        "paths: {paths:?}"
    );

    // 6. Installing a PKG uploads it and asks webMAN to install.
    let pkg_local = root.join("new.pkg");
    std::fs::write(&pkg_local, pkg_bytes()).unwrap();
    let (tx3, rx3) = std::sync::mpsc::channel();
    let host_install = host.clone();
    let report: Arc<Mutex<Option<install::InstallReport>>> = Arc::new(Mutex::new(None));
    let report_copy = report.clone();
    JobManager::new().spawn(
        "install",
        "i",
        |_| {},
        move |ctx| {
            let cfg = godsend_core::Config {
                auto_install_pkg: true,
                ..godsend_core::Config::default()
            };
            let result = install::install_local(&host_install, &pkg_local, &cfg, ctx)?;
            *report_copy.lock().unwrap() = Some(result);
            tx3.send(()).unwrap();
            Ok(())
        },
    );
    rx3.recv_timeout(Duration::from_secs(10))
        .expect("install finished");
    let report = report.lock().unwrap().clone().expect("report");
    assert_eq!(report.remote_path, "/dev_hdd0/packages/new.pkg");
    assert!(report.install_requested);
    let paths = seen.lock().unwrap().clone();
    assert!(
        paths
            .iter()
            .any(|p| p == "/install.ps3/dev_hdd0/packages/new.pkg"),
        "paths: {paths:?}"
    );

    // 7. Homebrew rows are installable.
    assert!(homebrew::bundled_catalog()
        .iter()
        .all(|e| !e.asset.is_empty()));

    let _ = std::fs::remove_dir_all(&root);
}
