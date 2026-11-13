use std::path::Path;

use anyhow::{bail, Context, Result};

const GZIP_HEADER: [u8; 10] = [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 255];
const LEVEL: u8 = 9;

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 == 1 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
        }
        *slot = value;
    }
    let mut crc = !0u32;
    for &byte in data {
        crc = table[((crc ^ u32::from(byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    !crc
}

pub fn write(path: &Path, text: &str) -> Result<()> {
    let mut data = GZIP_HEADER.to_vec();
    data.extend(miniz_oxide::deflate::compress_to_vec(
        text.as_bytes(),
        LEVEL,
    ));
    data.extend(crc32(text.as_bytes()).to_le_bytes());
    data.extend((text.len() as u32).to_le_bytes());
    std::fs::write(path, data).with_context(|| format!("writing {}", path.display()))
}

pub fn read(path: &Path) -> Result<String> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if data.len() < 18 || data[..3] != GZIP_HEADER[..3] || data[3] != 0 {
        bail!("{} is not a plain gzip file", path.display());
    }
    let raw = miniz_oxide::inflate::decompress_to_vec(&data[10..data.len() - 8])
        .map_err(|error| anyhow::anyhow!("inflating {}: {error:?}", path.display()))?;
    Ok(String::from_utf8(raw)?)
}
