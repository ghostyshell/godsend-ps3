# Changelog

## [Unreleased]

### Changed

- Downloads are hosted on Cloudflare R2 under versioned, immutable keys, so a
  link stays valid until the next version. The weekly GoFile/file.kiwi re-upload
  is gone: the scheduled job now only checks the live feed for a version bump,
  and each publish prunes R2 versions older than the previous one.

## [0.1.0] - 2026-11-15

First release.

### Added

- Multi-console management with FTP and webMAN MOD probing, LAN discovery,
  firmware, model and storage reporting.
- Remote library scan for digital games, JB folders, PS3/PS2/PS1/PSP ISOs and
  packages, with PARAM.SFO titles, title ids and ICON0 art.
- Mount and launch through webMAN.
- Virtual controller over `/pad.ps3`: on-screen pad, keyboard mapping,
  press-and-hold, combos and macros.
- Downloads from the GameSearch index and Internet Archive, PS3 homebrew
  catalog from GitHub releases, aria2c or plain HTTP, with a cancellable job
  queue.
- Install pipeline: PKG to `/dev_hdd0/packages` plus webMAN install, ISOs to
  their system folder, game folders to `/dev_hdd0/GAMES`, ZIP unpacking, and
  FAT32 splitting on USB destinations.
- Conversions: folder to ISO (ISO9660 plus Joliet, multi-extent), ISO to
  folder, FAT32 split and merge, Redump dkey decryption with ps3-resources
  key lookup.
- Two-pane local and console file manager with upload, download, folder
  creation and delete.
- Official update lookup and install from ps3-resources.
- Headless CLI covering hosts, scan, files, install, mount, pad, conversions,
  homebrew, updates and doctor.
- MCP server that starts with the app on loopback (`http://127.0.0.1:8743/mcp`),
  plus a stdio `godsend-ps3-mcp` binary, exposing status, library, mount,
  install, pad, system, files, conversions, search, homebrew and update tools.
- App mark from the supplied purple download loop, with every platform icon
  regenerated from it.
- Tests: 45 unit tests, a fake-console end-to-end test over FTP and webMAN,
  and 11 Playwright page tests against the mock bridge, plus MCP protocol and HTTP transport tests.
