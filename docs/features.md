# Features

GODsend PS3 manages a CFW or HEN PlayStation 3 over FTP and webMAN MOD.

## Consoles

- Multiple consoles, one active at a time.
- FTP login with the anonymous or `ftp` convention, automatic fallback.
- webMAN MOD detection with version, firmware, CFW name and model.
- Internal HDD free and total space from webMAN's system page.
- LAN discovery on port 21.
- System actions: unmount, refresh XMB, rebuild database, screenshot,
  notification, browser, quick restart, power off.

## Library

- Digital games and homebrew from `/dev_hdd0/game`.
- JB folders from `/dev_hdd0/GAMES`, with or without PARAM.SFO.
- PS3, PS2, PS1 and PSP ISOs, plus ROM folders.
- Packages waiting in `pkg` and `packages`.
- PARAM.SFO title, title id, category and version.
- ICON0 art fetched over FTP and cached, with an R2 cover fallback by title id.
- Mount, mount and play, updates, delete.

## Virtual controller

- On-screen DualShock with press and hold.
- Keyboard mapping for every button.
- Combos and text macros.
- Analog stick directions.
- PS button, screenshot and restart shortcuts.

## Downloads

- Game index search and Internet Archive search, with file lists.
- PS3 homebrew catalog from GitHub releases.
- aria2c with 16 connections when available, plain HTTP otherwise.
- Cancellable job queue with progress and a journal across restarts.

## Installs

- PKG to `/dev_hdd0/packages`, then webMAN install.
- ISO to its system folder, with FAT32 splitting on USB.
- Game folders to `/dev_hdd0/GAMES`.
- ZIP unpacking with game-root detection.
- Official updates by title id from ps3-resources.

## Conversions

- Folder to ISO: ISO9660 plus Joliet, multi-extent over 4 GiB.
- ISO to folder, reading Joliet when present.
- Split for FAT32 and merge.
- Redump dkey decryption, with automatic key lookup.

## Files

- Local and console panes.
- Upload, download, create folder, delete.
- Progress in the shared job queue.

## MCP server

- Loopback HTTP endpoint that starts with the app, default
  `http://127.0.0.1:8743/mcp`.
- Tools for status, library, mount, install, pad, system, files, conversions,
  search, homebrew and updates.
- Standalone stdio binary `godsend-ps3-mcp` for clients that spawn a server.

## Out of scope

- PKG decryption and NPDRM, which need console keys.
- PS2 Classics and PS1 EBOOT packaging, which need Sony's tools.
