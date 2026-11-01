use serde::Serialize;
use untwist::StageReport;

const OBFUSCATOR_IO: &str = "obfuscator.io";

pub const FAMILIES: [(&str, &str); 10] = [
    ("find-string-array", OBFUSCATOR_IO),
    ("find-decoders", OBFUSCATOR_IO),
    ("inline-decoder-wrappers", OBFUSCATOR_IO),
    ("inline-decoded-strings", OBFUSCATOR_IO),
    ("remove-decoder-machinery", OBFUSCATOR_IO),
    ("inline-object-props", OBFUSCATOR_IO),
    ("control-flow-object", OBFUSCATOR_IO),
    ("control-flow-switch", OBFUSCATOR_IO),
    ("self-defending", OBFUSCATOR_IO),
    ("debug-protection", OBFUSCATOR_IO),
];

#[derive(Serialize)]
pub struct FamilyRow {
    pub family: &'static str,
    pub pass: &'static str,
    pub changes: u32,
}

pub fn family_of(pass: &str) -> Option<&'static str> {
    FAMILIES
        .iter()
        .find(|(name, _)| *name == pass)
        .map(|(_, family)| *family)
}

pub fn families(stages: &[StageReport]) -> Vec<FamilyRow> {
    let mut rows: Vec<FamilyRow> = Vec::new();
    for (pass, changes) in stages.iter().flat_map(|stage| stage.per_pass.iter()) {
        let Some(family) = family_of(pass).filter(|_| *changes > 0) else {
            continue;
        };
        match rows.iter_mut().find(|row| row.pass == *pass) {
            Some(row) => row.changes += changes,
            None => rows.push(FamilyRow {
                family,
                pass,
                changes: *changes,
            }),
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::FAMILIES;

    #[test]
    fn every_family_key_is_a_pass_name() {
        let names: Vec<&'static str> = untwist::all_pass_ctors()
            .into_iter()
            .map(|ctor| ctor().name())
            .collect();
        let unknown: Vec<&str> = FAMILIES
            .iter()
            .map(|(pass, _)| *pass)
            .filter(|pass| !names.contains(pass))
            .collect();
        assert!(
            unknown.is_empty(),
            "unknown {unknown:?}; the passes are {names:?}"
        );
    }
}
