# Connect your PS3

GODsend PS3 needs two services on the console:

1. FTP, for files. webMAN MOD, multiMAN, IRISMAN and the standalone PS3 FTP
   Server all provide one. webMAN MOD's server is the usual choice.
2. webMAN MOD on HTTP port 80, for mounting, launching, the virtual pad,
   screenshots and system actions. Without it the app still reads and writes
   files, but the Controller page and mount buttons report that remote
   control is unavailable.

## Find the console's address

On the PS3, open Settings, then System Settings, then System Information. The
IP address is listed there. Reserve it on your router so it does not move.

## Add it in the app

1. Open Devices.
2. Choose Add console.
3. Enter a name and the IP. Leave FTP on 21 and HTTP on 80 unless you changed
   them on the console.
4. Leave the FTP user as `anonymous` with no password. Most PS3 FTP servers
   accept any user; the app retries with `ftp` if the first login fails.
5. Choose Save console, then Test.

A healthy console shows FTP up with a latency, webMAN ready with a version,
and firmware, model and free space in the inspector.

## Discovery

Devices, then Discover, scans port 21 across your subnet and keeps only hosts
that answer as a PS3 FTP server. Use it when you do not know the address.

## More than one console

Add each console and use Use to make it active. Every page, the virtual pad
and the job queue work against the active console.

## Security

PS3 FTP servers are plaintext and usually unauthenticated. Keep the console
and the computer on your own network; do not expose port 21 to the internet.
