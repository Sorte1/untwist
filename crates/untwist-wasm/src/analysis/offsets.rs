pub struct Utf16Map {
    wide: Vec<Wide>,
}

struct Wide {
    byte_end: u32,
    extra: u32,
}

impl Utf16Map {
    pub fn new(text: &str) -> Self {
        let mut wide = Vec::new();
        if !text.is_ascii() {
            let mut extra = 0u32;
            for (byte, ch) in text.char_indices().filter(|(_, ch)| !ch.is_ascii()) {
                let bytes = u32::try_from(ch.len_utf8()).unwrap_or(4);
                let units = u32::try_from(ch.len_utf16()).unwrap_or(2);
                extra += bytes - units;
                wide.push(Wide {
                    byte_end: u32::try_from(byte)
                        .unwrap_or(u32::MAX)
                        .saturating_add(bytes),
                    extra,
                });
            }
        }
        Self { wide }
    }

    pub fn at(&self, byte: u32) -> u32 {
        let before = self.wide.partition_point(|wide| wide.byte_end <= byte);
        match before.checked_sub(1).and_then(|index| self.wide.get(index)) {
            Some(last) => byte - last.extra,
            None => byte,
        }
    }

    pub fn span(&self, start: u32, end: u32) -> [u32; 2] {
        [self.at(start), self.at(end)]
    }
}

#[cfg(test)]
mod tests {
    use super::Utf16Map;

    #[test]
    fn byte_offsets_after_wide_characters_shift_by_the_saved_units() {
        let text = "aé😀b";
        let map = Utf16Map::new(text);
        assert_eq!(map.at(0), 0);
        assert_eq!(map.at(1), 1);
        assert_eq!(map.at(3), 2);
        assert_eq!(map.at(7), 4);
        assert_eq!(map.at(8), 5);
        assert_eq!(text.encode_utf16().count(), 5);
    }

    #[test]
    fn an_ascii_text_maps_every_offset_to_itself() {
        let map = Utf16Map::new("let a = 1;");
        assert_eq!(map.span(4, 5), [4, 5]);
    }
}
