# Convert images

The Convert page inspects whatever you pick and offers only the operations
that fit.

## PS3 game folder to ISO

Pick the folder that contains `PS3_GAME` and `PS3_DISC.SFB`. Build ISO writes
an ISO9660 image with a Joliet tree, so long and lowercase filenames survive,
and multi-extent records for any file over 4 GiB. The result mounts from the
Library like any other ISO.

## ISO to game folder

Pick the ISO and choose an output folder. The reader prefers the Joliet tree,
falls back to ISO9660 names, and follows multi-extent chains.

## Split for FAT32

Pick an ISO or PKG and press Split for FAT32. Parts are written next to the
source as `name.0`, `name.1` and so on at the configured chunk size. Merge
takes the `.0` part and writes the original file back.

## Decrypt a Redump ISO

A retail dump keeps its directory tree readable but scrambles file data in
encrypted regions. The page shows `encrypted` when it finds any. Decrypt:

1. Paste a 32-character Redump key, or leave the field empty.
2. With an empty field the app asks ps3-resources for the key whose title
   matches the ISO filename.
3. Choose an output path.

The output is written as a `.part` file and renamed only after a complete run,
and the region map count is zeroed so a second attempt refuses instead of
scrambling the image.

The algorithm is AES-128-CBC per 2048-byte sector, IV is twelve zero bytes
plus the big-endian sector index, and only the odd-indexed regions are
encrypted. It is ported from PS3 Quick Disc Decryptor and cross-checked
against ps3dec.

## Installs

Every non-split result can be sent straight to the console with Install on
PS3. The destination follows the table in the install guide.
