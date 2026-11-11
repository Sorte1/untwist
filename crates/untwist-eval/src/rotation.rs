use oxc_ast::ast::Function;
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;
use sha2::{Digest, Sha256};

use crate::decode::{DecoderProgram, KeptUnits};
use crate::source::{compact_expression, compact_function};
use crate::value::Value;

pub type RotationKey = [u8; 32];

const UNIT_CHUNK: usize = 512;

#[derive(Default)]
pub struct RotationMemo {
    entries: FxHashMap<RotationKey, usize>,
    searches: usize,
    hits: usize,
    pub(crate) units: KeptUnits,
}

impl RotationMemo {
    pub fn searches(&self) -> usize {
        self.searches
    }

    pub fn hits(&self) -> usize {
        self.hits
    }

    pub fn unit_hits(&self) -> usize {
        self.units.hits()
    }

    pub fn stored(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn cached(&self, key: &RotationKey) -> Option<usize> {
        self.entries.get(key).copied()
    }

    pub(crate) fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub(crate) fn record_search(&mut self) {
        self.searches += 1;
    }

    pub(crate) fn store(&mut self, key: RotationKey, rotation: usize) {
        self.entries.entry(key).or_insert(rotation);
    }
}

fn hash_units(hasher: &mut Sha256, units: &[u16]) {
    let mut buffer = [0u8; UNIT_CHUNK];
    let mut filled = 0usize;
    for unit in units {
        for byte in unit.to_le_bytes() {
            if let Some(slot) = buffer.get_mut(filled) {
                *slot = byte;
                filled += 1;
            }
        }
        if filled == buffer.len() {
            hasher.update(buffer);
            filled = 0;
        }
    }
    if let Some(tail) = buffer.get(..filled) {
        hasher.update(tail);
    }
}

pub(crate) fn hash_text(hasher: &mut Sha256, text: &str) {
    hasher.update((text.len() as u64).to_le_bytes());
    hasher.update(text.as_bytes());
}

pub(crate) fn hash_span(hasher: &mut Sha256, span: Span) {
    hasher.update(span.start.to_le_bytes());
    hasher.update(span.end.to_le_bytes());
}

fn hash_element(hasher: &mut Sha256, value: &Value<'_>) -> bool {
    match value {
        Value::Undefined => hasher.update([0u8]),
        Value::Null => hasher.update([1u8]),
        Value::Bool(flag) => hasher.update([2u8, u8::from(*flag)]),
        Value::Number(number) => {
            hasher.update([3u8]);
            hasher.update(number.to_bits().to_le_bytes());
        }
        Value::Str(text) => {
            hasher.update([4u8]);
            hasher.update((text.len() as u64).to_le_bytes());
            hash_units(hasher, text.units());
        }
        Value::Object(_) => return false,
    }
    true
}

fn hash_source_span(hasher: &mut Sha256, source_text: &str, span: Span) {
    let Some(text) = source_text.get(span.start as usize..span.end as usize) else {
        hasher.update([0u8]);
        return;
    };
    hasher.update([1u8]);
    hash_text(hasher, text);
}

pub(crate) fn hash_function(
    hasher: &mut Sha256,
    source_text: &str,
    name: &str,
    function: &Function<'_>,
) {
    hash_text(hasher, name);
    hash_span(hasher, function.span());
    hash_source_span(hasher, source_text, function.span());
    hash_text(hasher, &compact_function(function));
}

pub(crate) fn hash_decoders(hasher: &mut Sha256, program: &DecoderProgram<'_>) {
    hasher.update((program.decoders.len() as u64).to_le_bytes());
    for decoder in &program.decoders {
        hash_function(hasher, program.source_text, decoder.name, decoder.function);
    }
}

pub(crate) fn hash_rotator(hasher: &mut Sha256, program: &DecoderProgram<'_>) {
    let Some(rotator) = program.rotator else {
        hasher.update([0u8]);
        return;
    };
    hasher.update([1u8]);
    hash_span(hasher, rotator.span());
    hash_source_span(hasher, program.source_text, rotator.span());
    hash_text(hasher, &compact_expression(rotator));
}

pub(crate) fn machinery_key(
    program: &DecoderProgram<'_>,
    elements: &[Value<'_>],
) -> Option<RotationKey> {
    let mut hasher = Sha256::new();
    hasher.update((program.source_text.len() as u64).to_le_bytes());
    hash_function(
        &mut hasher,
        program.source_text,
        program.string_array.name,
        program.string_array.function,
    );
    hasher.update((elements.len() as u64).to_le_bytes());
    for element in elements {
        if !hash_element(&mut hasher, element) {
            return None;
        }
    }
    hash_decoders(&mut hasher, program);
    hash_rotator(&mut hasher, program);
    Some(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::{RotationKey, RotationMemo};
    use std::time::{Duration, Instant};

    fn key(index: usize) -> RotationKey {
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&index.to_le_bytes());
        key
    }

    fn fill_and_read(count: usize) -> Duration {
        (0..3)
            .map(|_| {
                let started = Instant::now();
                let mut memo = RotationMemo::default();
                for index in 0..count {
                    memo.store(key(index), index);
                }
                for index in 0..count {
                    assert_eq!(memo.cached(&key(index)), Some(index));
                }
                started.elapsed()
            })
            .min()
            .unwrap_or_default()
    }

    #[test]
    fn many_kept_rotations_are_found_without_a_scan() {
        let small = fill_and_read(2_000).max(Duration::from_micros(200));
        let large = fill_and_read(8_000);
        let ratio = large.as_secs_f64() / small.as_secs_f64();
        assert!(
            ratio < 8.0,
            "8000 rotations took {large:?}, 2000 took {small:?}"
        );
    }

    #[test]
    fn the_first_stored_rotation_for_a_key_wins() {
        let mut memo = RotationMemo::default();
        memo.store(key(1), 3);
        memo.store(key(1), 5);
        assert_eq!(memo.cached(&key(1)), Some(3));
        assert_eq!(memo.stored(), 1);
    }
}
