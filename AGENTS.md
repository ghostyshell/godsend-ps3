# GODsend PS3: project overview for agents

GODsend PS3 is a desktop workshop for a CFW or HEN PlayStation 3. It speaks
FTP for files and webMAN MOD's HTTP commands for remote control, then wraps
both in the GODsend family shell (Rust workspace plus Tauri 2 plus React).

## Components

- **`core/` (`godsend-core`)**: framework-agnostic engine. No Tauri, no CLI
  parsing.
  - `config`: hosts, active host, download folder, debrid keys, service
    bases, split size, pad and install toggles.
  - `ftp`: suppaftp wrapper. Tolerant listing (MLSD, Unix LIST, DOS LIST),
    download and upload with progress, `read_head`, recursive remove,
    `mkdir_all`. PS3 paths are absolute (`/dev_hdd0/...`).
  - `webman`: HTTP client for `/pad.ps3`, `/mount.ps3`, `/play.ps3`,
    `/install.ps3`, `/cp.ps3`, `/popup.ps3`, `/browser.ps3`, `/xmb.ps3`,
    `/cpursx.ps3`, `/restart.ps3`, `/shutdown.ps3`, `/extgd.ps3`,
    `/refresh.ps3`, `/rebuild.ps3`. Commands are concatenated directly
    (`/install.ps3<path>`), with spaces and hashes percent-encoded.
  - `device`: host probing (FTP latency, webMAN version, firmware, model,
    HDD space) and LAN discovery on port 21.
  - `library`: remote scan of `/dev_hdd0/{game,GAMES,PS3ISO,PS2ISO,PSXISO,
    PSPISO,ROMS,pkg,packages}` plus USB devices. PARAM.SFO is read over FTP;
    ICON0 is cached by path hash.
  - `iso` and `isobuild`: ISO9660 reader (Joliet preferred, multi-extent) and
    writer (ISO9660 plus Joliet, multi-extent over 4 GiB).
  - `decrypt`: Redump region map plus AES-128-CBC per sector, IV is 12 zero
    bytes and the big-endian sector index. Ported from the user's
    `~/Code/ps3-qdd-mac`.
  - `pkg`, `split`, `install`, `convert`: classification, FAT32 split and
    merge, destination layout, PKG install handoff, conversion entry points.
  - `homebrew`, `resources`, `updates`: GitHub release catalog, ps3-resources
    title ids and dkeys, official update packages.
  - `sources`, `store`, `debrid`, `aria2`: game index and Internet Archive
    search, link selection, Real-Debrid and TorBox resolution, downloads.
  - `pad`: button vocabulary, combos, macros.
  - `transfer`: JobManager with progress hook, cancellation and a journal.
- **`cli/` (`godsend-ps3`)**: one command per feature area. Run with no
  arguments for the list.
- **`mcp/` (`godsend-ps3-mcp`)**: Model Context Protocol server. One tool
  handler over JSON-RPC, two transports: stdio for clients that spawn the
  binary, and loopback HTTP started by the desktop app on `mcp_port`. Tools
  are thin wrappers over core and block until long jobs finish.
- **`app/`**: Tauri 2 shell. `app/src-tauri/src/lib.rs` has the command layer;
  `app/src` is React 18 plus TypeScript with one hand-rolled `styles.css`
  (see DESIGN.md) and a mock bridge in `mock.ts` behind `?mock=1`.

## Invariants

- `core/` must stay UI-free and process-free. It returns `Result<T>`, and the
  Tauri error serializes as the Display string.
- A console is addressable by FTP; nothing assumes a mounted volume. Every
  remote path starts with `/dev_`.
- FTP servers disagree about listings and the anonymous user. Try MLSD then
  LIST, and `anonymous` then `ftp`.
- webMAN command paths are concatenated, not `?`-joined, except for its own
  query commands (`pad.ps3?cross`, `reboot.ps3?quick`,
  `shutdown.ps3?vsh`, `cpursx.ps3?dn`).
- A decrypted image has its region-map count zeroed, so a later scan reports
  it as plaintext and a second decrypt refuses instead of scrambling it.
- ISO directory records store byte lengths, not sector counts. The writer
  pads every directory to a sector and fails loudly if a record would overflow.
- Multi-extent records chain by identifier with flag `0x80` on every record
  but the last.
- Destructive operations (game, package, file delete, power off, install)
  require an explicit user confirmation in the UI.
- FTP on a PS3 is plaintext; never send credentials anywhere else and never
  log passwords.

## Building and testing

```sh
cargo test --workspace
cargo check --workspace
npm --prefix app install
npm --prefix app run build          # tsc --noEmit plus vite build
npm --prefix app run dev:mock       # browser UI, no hardware
npm --prefix app run test:ui        # Playwright against the mock bridge
```

The MCP HTTP transport is covered by `mcp/tests/http.rs` (initialize,
tools/list, notification, method rejection). The end-to-end core test,
`core/tests/mock_device.rs`, runs a fake FTP server
and a fake webMAN HTTP server, then drives scan, icon fetch, upload, download,
pad commands, mount and PKG install. Extend it, not just unit tests, when a
flow changes.

## Releases and copy

- Version lives in `Cargo.toml` (workspace), `app/package.json`,
  `app/src-tauri/tauri.conf.json`, `app/src/App.tsx` and the Settings about
  line. Use `scripts/bump-version.sh x.y.z "note"`; keep CHANGELOG newest
  header at the top, no Unreleased section.
- Public-facing copy: no em dashes, no en dashes, ASCII hyphen only.
- Icons: `assets/logo-source.png` is the mark. Regenerate with
  `npx --prefix app tauri icon assets/logo-source.png` and copy the same file
  to `app/src/assets/logo.png`.

## Demo videos and guides

GODsend apps carry per-feature demo reels and a guides page, recorded from
the renderer's `?mock=1` bridge by `~/Code/demo-reels`. A new, changed or
removed feature updates its reel and its guides entry in the same change.
The PS3 app's reels are not recorded yet; until they are, text guides live in
`docs/guides/`.
