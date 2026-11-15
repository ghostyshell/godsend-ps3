//! Decrypt Redump PS3 disc images in place so RPCS3 and the app's ISO tools
//! can read them.
//!
//! A retail PS3 image starts with a region map in sector 0: `n = BE_u32(0)`
//! followed by `2n - 1` sector boundaries. Regions alternate plain, encrypted,
//! plain, ... starting with plain. Each encrypted 2048-byte sector is one
//! AES-128-CBC block chain with a zero IV except for the big-endian sector
//! index in the last four bytes. The 16-byte key comes from a Redump `.dkey`
//! file (32 hex characters).
//!
//! The algorithm is ported from the user's PS3 Quick Disc Decryptor
//! (`~/Code/ps3-qdd-mac`), which was cross-checked against al3xtjames/ps3dec.

use crate::error::{Error, Result};
use crate::transfer::JobContext;
use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes128;
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const SECTOR: u64 = 2048;
const CHUNK: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SectorRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscScan {
    pub total_sectors: u64,
    pub size_bytes: u64,
    pub encrypted_ranges: Vec<SectorRange>,
    pub encrypted_sectors: u64,
    pub encrypted: bool,
}

/// Parse the region map from sector 0 and report the encrypted spans.
pub fn scan(path: &Path) -> Result<DiscScan> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    if size < SECTOR || size % SECTOR != 0 {
        return Err(Error::Invalid(format!(
            "{} is not a sector-aligned disc image",
            path.display()
        )));
    }
    let mut header = vec![0u8; SECTOR as usize];
    file.read_exact(&mut header)?;
    let count = be_u32(&header, 0);
    if count == 0 {
        return Ok(DiscScan {
            total_sectors: size / SECTOR,
            size_bytes: size,
            encrypted_ranges: Vec::new(),
            encrypted_sectors: 0,
            encrypted: false,
        });
    }
    let entries = 2 * count as usize - 1;
    if 12 + entries * 4 > SECTOR as usize {
        return Err(Error::Invalid(
            "region map is larger than one sector".into(),
        ));
    }
    let mut ranges = Vec::new();
    let mut lba = 0u64;
    let mut previous: Option<u32> = None;
    for index in 0..entries {
        let boundary = be_u32(&header, 12 + index * 4);
        if let Some(prev) = previous {
            if boundary <= prev {
                return Err(Error::Invalid(format!(
                    "region map boundary {index} is not increasing"
                )));
            }
        }
        previous = Some(boundary);
        let last = if index % 2 == 1 {
            boundary as u64 - 1
        } else {
            boundary as u64
        };
        if index % 2 == 1 && last >= lba {
            ranges.push(SectorRange {
                start: lba,
                end: last + 1,
            });
        }
        lba = last + 1;
    }
    let encrypted_sectors = ranges.iter().map(|r| r.end - r.start).sum();
    Ok(DiscScan {
        total_sectors: size / SECTOR,
        size_bytes: size,
        encrypted_ranges: ranges,
        encrypted_sectors,
        encrypted: encrypted_sectors > 0,
    })
}

/// Parse a 32-character hex key (any whitespace/newlines stripped).
pub fn parse_key(hex: &str) -> Result<[u8; 16]> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() != 32 {
        return Err(Error::Invalid(format!(
            "disc key must be 32 hex characters, got {}",
            cleaned.len()
        )));
    }
    let mut key = [0u8; 16];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error::Invalid("disc key is not hexadecimal".into()))?;
    }
    Ok(key)
}

/// Decrypt `input` into `output`. Writes `output.part` first and renames it
/// on success so a cancelled or failed run never leaves a half-valid image.
pub fn decrypt(input: &Path, output: &Path, key_hex: &str, ctx: &JobContext) -> Result<u64> {
    let key = parse_key(key_hex)?;
    let scan = scan(input)?;
    if !scan.encrypted {
        return Err(Error::Invalid(
            "this image has no encrypted regions; it is already decrypted".into(),
        ));
    }
    if scan.size_bytes > 0 && std::fs::metadata(input)?.len() != scan.size_bytes {
        return Err(Error::Invalid("image changed while scanning".into()));
    }

    // Per-sector mask keeps the hot loop O(1).
    let mut mask = vec![false; scan.total_sectors as usize];
    for range in &scan.encrypted_ranges {
        for sector in range.start..range.end {
            if let Some(slot) = mask.get_mut(sector as usize) {
                *slot = true;
            }
        }
    }

    let part = output.with_extension(format!(
        "{}part",
        output
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}."))
            .unwrap_or_default()
    ));
    let mut infile = File::open(input)?;
    let mut outfile = File::create(&part)?;
    let cipher = Aes128::new_from_slice(&key).map_err(|_| Error::Invalid("bad AES key".into()))?;
    let mut buf = vec![0u8; CHUNK];
    let mut sector_index = 0u64;
    let total = scan.total_sectors;
    let mut done = 0u64;
    ctx.progress(0, scan.size_bytes);
    loop {
        ctx.check_cancel()?;
        let read = infile.read(&mut buf)?;
        if read == 0 {
            break;
        }
        let sectors = read / SECTOR as usize;
        for offset in 0..sectors {
            let sector = sector_index + offset as u64;
            if !mask[sector as usize] {
                continue;
            }
            let start = offset * SECTOR as usize;
            decrypt_sector(&cipher, &mut buf[start..start + SECTOR as usize], sector);
        }
        outfile.write_all(&buf[..read])?;
        done += read as u64;
        sector_index += sectors as u64;
        ctx.progress(done, scan.size_bytes);
    }
    outfile.flush()?;
    // The region map only describes which sectors *were* encrypted. Leaving it
    // in place makes every later scan report the image as still encrypted, so
    // clear the entry count: the resulting sector 0 matches a JB dump.
    {
        use std::io::{Seek, SeekFrom};
        outfile.seek(SeekFrom::Start(0))?;
        outfile.write_all(&0u32.to_be_bytes())?;
    }
    drop(outfile);
    if sector_index != total {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Invalid(
            "image ended before the sector map did".into(),
        ));
    }
    if output.exists() {
        std::fs::remove_file(output)?;
    }
    std::fs::rename(&part, output)?;
    ctx.progress(done, scan.size_bytes);
    Ok(done)
}

fn decrypt_sector(cipher: &Aes128, sector: &mut [u8], index: u64) {
    let mut iv = [0u8; 16];
    iv[12..16].copy_from_slice(&(index as u32).to_be_bytes());
    let mut previous = iv;
    for block in sector.chunks_exact_mut(16) {
        let ciphertext = {
            let mut c = [0u8; 16];
            c.copy_from_slice(block);
            c
        };
        let mut b = *aes::Block::from_slice(block);
        cipher.decrypt_block(&mut b);
        for (i, byte) in b.iter_mut().enumerate() {
            *byte ^= previous[i];
        }
        block.copy_from_slice(&b);
        previous = ciphertext;
    }
}

fn be_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// AES-128-CBC decrypt of one 2048-byte sector, exposed for tests.
pub fn decrypt_sector_bytes(key: &[u8; 16], sector: &mut [u8], index: u64) {
    let cipher = Aes128::new_from_slice(key).expect("16-byte key");
    decrypt_sector(&cipher, sector, index);
}

/// Build a test image with one encrypted region at sectors `start..end`.
pub fn make_test_image(
    path: &Path,
    plain_sectors: u64,
    encrypted_start: u64,
    encrypted_sectors: u64,
) -> Result<()> {
    use aes::cipher::BlockEncrypt;
    let key = [0x11u8; 16];
    let cipher = Aes128::new_from_slice(&key).expect("key");
    let total = plain_sectors.max(encrypted_start + encrypted_sectors);
    let mut out = vec![0u8; (total * SECTOR) as usize];
    for sector in 0..total {
        let start = (sector * SECTOR) as usize;
        for (i, byte) in out[start..start + SECTOR as usize].iter_mut().enumerate() {
            *byte = ((sector as usize * 7 + i) % 251) as u8;
        }
    }
    // Region map: n=2 entries? Boundaries: encrypted_start, encrypted_end+1.
    // Regions alternate plain (0..encrypted_start), encrypted (..end), plain.
    // n is the number of boundaries pairs: 2*n-1 entries = 3 -> n=2.
    let n: u32 = 2;
    out[0..4].copy_from_slice(&n.to_be_bytes());
    out[12..16].copy_from_slice(&((encrypted_start - 1) as u32).to_be_bytes());
    out[16..20].copy_from_slice(&((encrypted_start + encrypted_sectors) as u32).to_be_bytes());
    out[20..24].copy_from_slice(&(total as u32).to_be_bytes());
    // The map lives in sector 0, which must stay readable ("plain").
    // Encrypt the targeted sectors.
    for sector in encrypted_start..encrypted_start + encrypted_sectors {
        let mut iv = [0u8; 16];
        iv[12..16].copy_from_slice(&(sector as u32).to_be_bytes());
        let mut previous = iv;
        let start = (sector * SECTOR) as usize;
        for block in out[start..start + SECTOR as usize].chunks_exact_mut(16) {
            for (i, byte) in block.iter_mut().enumerate() {
                *byte ^= previous[i];
            }
            let mut b = *aes::Block::from_slice(block);
            cipher.encrypt_block(&mut b);
            block.copy_from_slice(&b);
            previous.copy_from_slice(block);
        }
    }
    std::fs::write(path, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::JobManager;

    #[test]
    fn parses_keys() {
        assert!(parse_key("323CE7042E84AF01555853C280760F1F").is_ok());
        assert!(parse_key("323CE7042E84AF01555853C280760F1F\n").is_ok());
        assert!(parse_key("abc").is_err());
        assert!(parse_key("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ").is_err());
    }

    #[test]
    fn region_map_matches_the_reference_algorithm() {
        // n=2, boundaries 9, 20, 30 => plain 0..9, encrypted 10..19, plain 20..29.
        // An even boundary is the last plain sector, an odd one is the first
        // sector after the encrypted run.
        let mut header = vec![0u8; SECTOR as usize];
        header[0..4].copy_from_slice(&2u32.to_be_bytes());
        header[12..16].copy_from_slice(&9u32.to_be_bytes());
        header[16..20].copy_from_slice(&20u32.to_be_bytes());
        header[20..24].copy_from_slice(&30u32.to_be_bytes());
        let count = be_u32(&header, 0) as usize;
        let entries = 2 * count - 1;
        let mut ranges = Vec::new();
        let mut lba = 0u64;
        let mut previous: Option<u32> = None;
        for index in 0..entries {
            let boundary = be_u32(&header, 12 + index * 4);
            if let Some(prev) = previous {
                assert!(boundary > prev);
            }
            previous = Some(boundary);
            let last = if index % 2 == 1 {
                boundary as u64 - 1
            } else {
                boundary as u64
            };
            if index % 2 == 1 && last >= lba {
                ranges.push(SectorRange {
                    start: lba,
                    end: last + 1,
                });
            }
            lba = last + 1;
        }
        assert_eq!(ranges, vec![SectorRange { start: 10, end: 20 }]);
    }

    #[test]
    fn decrypts_a_synthetic_image() {
        let dir = std::env::temp_dir().join(format!("godsend-ps3-dec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let encrypted = dir.join("disc.iso");
        // 40 sectors, sectors 10..20 encrypted.
        make_test_image(&encrypted, 40, 10, 10).unwrap();
        let encrypted_scan = scan(&encrypted).unwrap();
        assert!(encrypted_scan.encrypted);
        assert_eq!(
            encrypted_scan.encrypted_ranges,
            vec![SectorRange { start: 10, end: 20 }]
        );

        let plain = dir.join("decrypted.iso");
        let mgr = JobManager::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let enc = encrypted.clone();
        let out = plain.clone();
        mgr.spawn(
            "decrypt",
            "d",
            |_| {},
            move |ctx| {
                decrypt(&enc, &out, "11111111111111111111111111111111", ctx)?;
                tx.send(()).unwrap();
                Ok(())
            },
        );
        rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(!scan(&plain).unwrap().encrypted);
        // Plaintext pattern is (sector * 7 + i) % 251.
        let bytes = std::fs::read(&plain).unwrap();
        let start = (12 * SECTOR) as usize;
        assert_eq!(bytes[start], ((12 * 7) % 251) as u8);
        assert_eq!(bytes[start + 1], ((12 * 7 + 1) % 251) as u8);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
