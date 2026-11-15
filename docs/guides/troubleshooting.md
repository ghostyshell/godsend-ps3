# Troubleshooting

## FTP offline

- Start the FTP server on the console. webMAN MOD has one; multiMAN and
  IRISMAN have their own.
- Check the IP in Settings, System Settings, System Information.
- Port 21 must be reachable from the computer. Guest networks and AP
  isolation block it.
- Try the user `ftp` if `anonymous` fails; the app does this automatically on
  the second attempt.

## webMAN missing

FTP works but mounting, the pad and system actions fail. Install webMAN MOD
from the Homebrew page, launch it once on the console, then Test on Devices.
The HTTP port is 80 unless changed on the console.

## Mount does nothing

webMAN mounts an ISO or a JB folder, not a digital game already in
`/dev_hdd0/game`; those launch from the XMB. If the console still shows the
old disc, use Unmount in the status bar and mount again.

## A game is not listed

The scan reads `/dev_hdd0/game`, `/dev_hdd0/GAMES`, `PS3ISO`, `PS2ISO`,
`PSXISO`, `PSPISO`, `ROMS`, `pkg`, `packages` and USB devices. A folder under
GAMES without a PARAM.SFO still appears, using its folder name.

## Slow transfers

FTP on a PS3 is single-connection and CPU-bound on the console. Large ISOs
take time. Downloads themselves can use aria2c (16 connections) for speed;
the upload to the console is the slow half.

## PKG install does not start

The app uploads to `/dev_hdd0/packages` and calls webMAN's install command.
If nothing happens, open webMAN on the console and install from its package
manager, or check that the PKG is a real PS3 package and not a split set.

## Decryption says no key found

The lookup matches the ISO filename against the Redump title list. Rename the
file to the Redump name, or paste the 32-character key directly. The key is
for the exact disc revision.

## The app cannot find the console after a reboot

The router gave it a new address. Use Discover, or reserve the address on the
router.

## MCP client cannot connect

The server starts with the app and listens only on loopback at the port in
Settings, default 8743. Check that GODsend PS3 is running and that the client
points at `http://127.0.0.1:8743/mcp`. Set the port to 0 to disable it.
