const CHUNK: usize = 32;

fn candidate(byte: u8, tabs: bool) -> bool {
    let control = (byte < 0x20) & (byte != b'\n') & (tabs | (byte != b'\t'));
    control | (byte == 0x7f) | (byte == 0xc2) | (byte == 0xe2)
}

fn next_candidate(bytes: &[u8], from: usize, tabs: bool) -> Option<usize> {
    let mut at = from;
    while let Some(chunk) = bytes.get(at..at + CHUNK) {
        if chunk
            .iter()
            .fold(false, |found, &byte| found | candidate(byte, tabs))
        {
            break;
        }
        at += CHUNK;
    }
    let rest = bytes.get(at..)?;
    rest.iter()
        .position(|&byte| candidate(byte, tabs))
        .map(|offset| at + offset)
}

pub(super) struct Hits<'t> {
    text: &'t str,
    at: usize,
    tabs: bool,
    wanted: fn(char) -> bool,
}

impl<'t> Hits<'t> {
    pub(super) fn new(text: &'t str, tabs: bool, wanted: fn(char) -> bool) -> Self {
        Self {
            text,
            at: 0,
            tabs,
            wanted,
        }
    }
}

impl Iterator for Hits<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        loop {
            let at = next_candidate(self.text.as_bytes(), self.at, self.tabs)?;
            let ch = self.text.get(at..)?.chars().next()?;
            self.at = at + ch.len_utf8();
            if (self.wanted)(ch) {
                return Some((at, ch));
            }
        }
    }
}
