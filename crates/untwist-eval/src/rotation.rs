use oxc_ast::ast::Function;
use oxc_span::{GetSpan, Span};
use sha2::{Digest, Sha256};

use crate::decode::DecoderProgram;
use crate::source::{compact_expression, compact_function};
use crate::value::Value;

pub type RotationKey = [u8; 32];

const UNIT_CHUNK: usize = 512;

struct RotationEntry {
    key: RotationKey,
    rotation: usize,
}

#[derive(Default)]
pub struct RotationMemo {
    entries: Vec<RotationEntry>,
    searches: usize,
    hits: usize,
}

impl RotationMemo {
    pub fn searches(&self) -> usize {
        self.searches
    }

    pub fn hits(&self) -> usize {
        self.hits
    }

    pub fn stored(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn cached(&self, key: &RotationKey) -> Option<usize> {
        self.entries
            .iter()
            .find(|entry| &entry.key == key)
            .map(|entry| entry.rotation)
    }

    pub(crate) fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub(crate) fn record_search(&mut self) {
        self.searches += 1;
    }

    pub(crate) fn store(&mut self, key: RotationKey, rotation: usize) {
        if self.cached(&key).is_none() {
            self.entries.push(RotationEntry { key, rotation });
        }
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

fn hash_text(hasher: &mut Sha256, text: &str) {
    hasher.update((text.len() as u64).to_le_bytes());
    hasher.update(text.as_bytes());
}

fn hash_span(hasher: &mut Sha256, span: Span) {
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

fn hash_function(hasher: &mut Sha256, source_text: &str, name: &str, function: &Function<'_>) {
    hash_text(hasher, name);
    hash_span(hasher, function.span());
    hash_source_span(hasher, source_text, function.span());
    hash_text(hasher, &compact_function(function));
}

fn hash_decoders(hasher: &mut Sha256, program: &DecoderProgram<'_>) {
    hasher.update((program.decoders.len() as u64).to_le_bytes());
    for decoder in &program.decoders {
        hash_function(hasher, program.source_text, decoder.name, decoder.function);
    }
}

fn hash_rotator(hasher: &mut Sha256, program: &DecoderProgram<'_>) {
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
