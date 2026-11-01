use untwist_eval::decode::Decoded;
use untwist_eval::value::JsString;

fn is_identifier_unit(unit: u16) -> bool {
    u8::try_from(unit)
        .is_ok_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$')
}

fn bounded_at(units: &[u16], name: &[u16], at: usize) -> bool {
    let before = at.checked_sub(1).and_then(|index| units.get(index));
    let after = units.get(at + name.len());
    !before.is_some_and(|unit| is_identifier_unit(*unit))
        && !after.is_some_and(|unit| is_identifier_unit(*unit))
}

fn token_at(units: &[u16], name: &[u16]) -> bool {
    let Some(&first) = name.first() else {
        return false;
    };
    let mut from = 0;
    while let Some(offset) = units
        .get(from..)
        .and_then(|rest| rest.iter().position(|unit| *unit == first))
    {
        let at = from + offset;
        if units[at..].starts_with(name) && bounded_at(units, name, at) {
            return true;
        }
        from = at + 1;
    }
    false
}

const INLINE_UNITS: usize = 48;

pub enum EncodedName {
    Inline([u16; INLINE_UNITS], usize),
    Heap(Vec<u16>),
}

impl EncodedName {
    fn of(name: &str) -> Self {
        let mut units = [0u16; INLINE_UNITS];
        let mut len = 0;
        for unit in name.encode_utf16() {
            let Some(slot) = units.get_mut(len) else {
                return Self::Heap(name.encode_utf16().collect());
            };
            *slot = unit;
            len += 1;
        }
        Self::Inline(units, len)
    }

    fn units(&self) -> &[u16] {
        match self {
            Self::Inline(units, len) => &units[..*len],
            Self::Heap(units) => units,
        }
    }
}

pub fn encode_names(names: &[&str]) -> Vec<EncodedName> {
    names
        .iter()
        .filter(|name| !name.is_empty())
        .map(|name| EncodedName::of(name))
        .collect()
}

fn spells_a_name(text: &JsString, encoded: &[EncodedName]) -> bool {
    let units = text.units();
    encoded.iter().any(|name| token_at(units, name.units()))
}

const RECENT_MISS_SLOTS: usize = 8;

struct RecentMisses {
    identities: [usize; RECENT_MISS_SLOTS],
    len: usize,
    next: usize,
}

impl RecentMisses {
    fn new() -> Self {
        Self {
            identities: [0; RECENT_MISS_SLOTS],
            len: 0,
            next: 0,
        }
    }

    fn contains(&self, identity: usize) -> bool {
        self.identities[..self.len].contains(&identity)
    }

    fn insert(&mut self, identity: usize) {
        self.identities[self.next] = identity;
        self.next = (self.next + 1) % RECENT_MISS_SLOTS;
        self.len = (self.len + 1).min(RECENT_MISS_SLOTS);
    }
}

pub fn names_machinery<'v>(
    values: impl IntoIterator<Item = &'v Decoded>,
    names: &[EncodedName],
) -> bool {
    if names.is_empty() {
        return false;
    }
    let mut recent_misses = RecentMisses::new();
    for value in values {
        let Decoded::Str(text) = value else {
            continue;
        };
        let identity = text.identity();
        if recent_misses.contains(identity) {
            continue;
        }
        if spells_a_name(text, names) {
            return true;
        }
        recent_misses.insert(identity);
    }
    false
}
