const STRIDE: usize = 1 << 12;

pub(super) fn line_break(bytes: &[u8], at: usize) -> usize {
    match bytes.get(at) {
        Some(b'\n') => 1,
        Some(b'\r') => 1 + usize::from(bytes.get(at + 1) == Some(&b'\n')),
        Some(0xE2) if matches!(bytes.get(at + 1..at + 3), Some([0x80, 0xA8 | 0xA9])) => 3,
        _ => 0,
    }
}

pub(super) fn char_len(byte: u8) -> usize {
    match byte {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    }
}

pub(super) fn utf16_len(byte: u8) -> u32 {
    match byte {
        0x80..=0xBF => 0,
        0xF0..=0xFF => 2,
        _ => 1,
    }
}

fn utf16_units(bytes: &[u8]) -> u32 {
    bytes.iter().map(|byte| utf16_len(*byte)).sum()
}

pub(super) fn line_starts(bytes: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    let mut at = 0;
    while at < bytes.len() {
        match line_break(bytes, at) {
            0 => at += 1,
            broke => {
                at += broke;
                starts.push(at);
            }
        }
    }
    starts
}

pub struct Positions<'t> {
    text: &'t [u8],
    lines: Vec<usize>,
    units: Vec<u32>,
}

impl<'t> Positions<'t> {
    pub fn new(text: &'t str) -> Self {
        let text = text.as_bytes();
        let mut units = Vec::with_capacity(text.len() / STRIDE + 2);
        let mut total = 0u32;
        for chunk in text.chunks(STRIDE) {
            units.push(total);
            total = total.saturating_add(utf16_units(chunk));
        }
        units.push(total);
        Self {
            text,
            lines: line_starts(text),
            units,
        }
    }

    fn units_before(&self, offset: usize) -> u32 {
        let block = offset / STRIDE;
        let counted = self.units.get(block).copied().unwrap_or_default();
        let rest = self.text.get(block * STRIDE..offset).unwrap_or_default();
        counted.saturating_add(utf16_units(rest))
    }

    pub fn of(&self, offset: usize) -> (u32, u32) {
        let offset = offset.min(self.text.len());
        let line = self
            .lines
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let start = self.lines.get(line).copied().unwrap_or_default();
        (
            u32::try_from(line).unwrap_or(u32::MAX),
            self.units_before(offset) - self.units_before(start),
        )
    }
}
