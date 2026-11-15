# Install games and homebrew

The app puts each file where the console expects it.

| Content | Destination | Then |
|---|---|---|
| PKG | `/dev_hdd0/packages` | webMAN install, automatic when enabled |
| PS3 ISO | `/dev_hdd0/PS3ISO` | mount from the Library |
| PS2 ISO | `/dev_hdd0/PS2ISO` | mount with the PS2 Classics launcher |
| PS1 ISO | `/dev_hdd0/PSXISO` | mount |
| PSP ISO | `/dev_hdd0/PSPISO` | mount |
| Game folder | `/dev_hdd0/GAMES` | mount from the Library |
| ZIP | unpacked first | then treated as a game folder |

## From a download

1. Download something on the Game Index, Homebrew or Downloads page.
2. Downloads, then Send to PS3 on the file.
3. Watch the queue. A PKG is uploaded and, if Settings has install PKGs
   immediately on, webMAN is asked to install it. The console shows its own
   progress bubble.

## From your own files

Convert, then Choose file or Choose folder, then Install on PS3. The same
classification applies, so a stray ISO lands in the right folder.

## FAT32 USB drives

A file above the split size cannot sit on FAT32. When the destination is a
USB device the app writes `name.0`, `name.1` and so on. The console's file
managers reassemble them. Set the chunk size in Settings; the default is
4095 MiB.

## Homebrew

Homebrew pulls each project's latest GitHub release: webMAN MOD, prepISO,
PS2 Classics Launcher, PS2CONFIG, IRISMAN, Apollo Save Tool and ManaGunZ.
Install webMAN MOD first, since the app's remote actions depend on it.

## Official updates

Updates takes a title id (for example `BCUS98229`) and lists Sony's update
packages from ps3-resources, newest first. Install downloads the newest and
hands it to webMAN. The Library shows an Updates button on any entry that has
a title id.

## Deleting

Deletes on the Library page remove the folder or file from the console after a
confirmation that names the path. There is no recycle bin.
