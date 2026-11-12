use serde::Serialize;
use untwist_core::pass::{Note, NoteKind};
use untwist_deobfuscate::layers::notes::LAYER_PEELED;
use untwist_deobfuscate::notes::{
    DECODERS_FOUND, DECODER_ENCODING, ROTATOR_FOUND, STRING_ARRAY_FOUND,
};

use crate::pipeline::Output;
use crate::report::StageReport;

pub const JAVASCRIPT_OBFUSCATOR: &str = "javascript-obfuscator";
pub const PACKER: &str = "packer";

const LAYER_KEPT: &str = "a layer was kept";
const LAYER_PASSES: [&str; 2] = ["layers", "inline-layers"];

const STRING_ARRAY_PASSES: [&str; 5] = [
    "find-string-array",
    "find-decoders",
    "inline-decoder-wrappers",
    "inline-decoded-strings",
    "remove-decoder-machinery",
];

const PASS_TECHNIQUES: [(&str, &str); 7] = [
    ("inline-decoder-wrappers", "decoder-wrappers"),
    ("inline-object-props", "proxy-objects"),
    ("control-flow-object", "control-flow-storage"),
    ("control-flow-switch", "control-flow-flattening"),
    ("dead-code", "dead-code-injection"),
    ("self-defending", "self-defending"),
    ("debug-protection", "debug-protection"),
];

const OBFUSCATOR_ONLY: [&str; 2] = ["self-defending", "debug-protection"];

const LAYER_FORMS: [(&str, &str); 4] = [
    ("W-packer", "packer-layer"),
    ("W-eval", "eval-layer"),
    ("W-Function", "function-layer"),
    ("I-", "inline-layer"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Undone,
    Partly,
    Kept,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Undone => "undone",
            State::Partly => "partly undone",
            State::Kept => "kept",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Technique {
    pub name: &'static str,
    pub state: State,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Obfuscation {
    pub families: Vec<String>,
    pub techniques: Vec<Technique>,
}

impl Obfuscation {
    pub fn is_empty(&self) -> bool {
        self.families.is_empty() && self.techniques.is_empty()
    }

    pub fn has(&self, name: &str) -> bool {
        self.techniques
            .iter()
            .any(|technique| technique.name == name)
    }

    pub fn summary(&self) -> String {
        let techniques: Vec<String> = self.techniques.iter().map(technique_text).collect();
        if self.families.is_empty() {
            return techniques.join(", ");
        }
        if techniques.is_empty() {
            return self.families.join(", ");
        }
        format!("{}: {}", self.families.join(", "), techniques.join(", "))
    }
}

fn technique_text(technique: &Technique) -> String {
    match &technique.detail {
        Some(detail) => format!(
            "{} ({detail}, {})",
            technique.name,
            technique.state.as_str()
        ),
        None => format!("{} ({})", technique.name, technique.state.as_str()),
    }
}

struct Evidence<'o> {
    stages: &'o [StageReport],
    notes: &'o [Note],
}

impl Evidence<'_> {
    fn changes(&self, pass: &str) -> u32 {
        self.stages
            .iter()
            .flat_map(|stage| stage.per_pass.iter().chain(&stage.fixpoint_per_pass))
            .filter(|(name, _)| *name == pass)
            .map(|(_, count)| *count)
            .sum()
    }

    fn changes_in(&self, stage_name: &str, pass: &str) -> u32 {
        self.stages
            .iter()
            .filter(|stage| stage.stage == stage_name)
            .flat_map(|stage| stage.per_pass.iter().chain(&stage.fixpoint_per_pass))
            .filter(|(name, _)| *name == pass)
            .map(|(_, count)| *count)
            .sum()
    }

    fn refused(&self, pass: &str) -> bool {
        self.notes
            .iter()
            .any(|note| note.pass == pass && note.kind == NoteKind::Refusal)
    }

    fn progress<'n>(&'n self, prefix: &'n str) -> impl Iterator<Item = &'n str> + 'n {
        self.notes
            .iter()
            .filter(|note| note.kind == NoteKind::Progress)
            .filter_map(move |note| note.message.strip_prefix(prefix))
            .map(str::trim)
    }

    fn first_progress(&self, prefix: &str) -> Option<String> {
        self.progress(prefix).next().map(str::to_string)
    }
}

pub fn detect(output: &Output) -> Obfuscation {
    let evidence = Evidence {
        stages: &output.stages,
        notes: &output.notes,
    };
    let mut techniques = string_array_techniques(&evidence);
    techniques.extend(pass_techniques(&evidence));
    techniques.extend(split_strings(&evidence));
    techniques.extend(layer_techniques(&evidence));
    let families = families(&techniques, output);
    Obfuscation {
        families,
        techniques,
    }
}

fn string_array_state(evidence: &Evidence<'_>) -> State {
    let removed = evidence.changes("remove-decoder-machinery") > 0;
    let decoded = evidence.changes("inline-decoded-strings") > 0;
    let refused = STRING_ARRAY_PASSES
        .iter()
        .any(|pass| evidence.refused(pass))
        || evidence.changes("unstamp-remaining-machinery") > 0;
    match (removed || decoded, refused) {
        (false, _) => State::Kept,
        (true, true) => State::Partly,
        (true, false) if removed => State::Undone,
        (true, false) => State::Partly,
    }
}

fn string_array_techniques(evidence: &Evidence<'_>) -> Vec<Technique> {
    let Some(entries) = evidence.first_progress(STRING_ARRAY_FOUND) else {
        return Vec::new();
    };
    let state = string_array_state(evidence);
    let mut techniques = vec![Technique {
        name: "string-array",
        state,
        detail: Some(entries),
    }];
    if evidence.progress(ROTATOR_FOUND).next().is_some() {
        techniques.push(Technique {
            name: "string-array-rotate",
            state,
            detail: None,
        });
    }
    if let Some(count) = evidence.first_progress(DECODERS_FOUND) {
        let encoding = evidence.first_progress(DECODER_ENCODING);
        let detail = match encoding {
            Some(encoding) => format!("{count}, {encoding}"),
            None => count,
        };
        techniques.push(Technique {
            name: "string-array-decoders",
            state,
            detail: Some(detail),
        });
    }
    techniques
}

fn pass_techniques<'e>(evidence: &'e Evidence<'_>) -> impl Iterator<Item = Technique> + 'e {
    PASS_TECHNIQUES
        .iter()
        .filter_map(|(pass, name)| pass_technique(evidence, pass, name))
}

fn pass_technique(evidence: &Evidence<'_>, pass: &str, name: &'static str) -> Option<Technique> {
    let changes = evidence.changes(pass);
    let refused = evidence.refused(pass);
    let state = match (changes > 0, refused) {
        (false, false) => return None,
        (false, true) => State::Kept,
        (true, true) => State::Partly,
        (true, false) => State::Undone,
    };
    Some(Technique {
        name,
        state,
        detail: (changes > 0).then(|| changes.to_string()),
    })
}

fn split_strings(evidence: &Evidence<'_>) -> Option<Technique> {
    let changes = evidence.changes_in("deobfuscate", "merge-strings");
    (changes > 0).then(|| Technique {
        name: "split-strings",
        state: State::Undone,
        detail: Some(changes.to_string()),
    })
}

fn layer_form(message: &str) -> Option<&'static str> {
    LAYER_FORMS
        .iter()
        .find(|(form, _)| message.contains(form))
        .map(|(_, name)| *name)
}

fn layer_techniques(evidence: &Evidence<'_>) -> Vec<Technique> {
    let mut found: Vec<(&'static str, u32, bool)> = Vec::new();
    for note in evidence.notes {
        if !LAYER_PASSES.contains(&note.pass) {
            continue;
        }
        let peeled = note.message.starts_with(LAYER_PEELED);
        if !peeled && !note.message.starts_with(LAYER_KEPT) {
            continue;
        }
        let Some(name) = layer_form(&note.message) else {
            continue;
        };
        match found.iter_mut().find(|(known, _, _)| *known == name) {
            Some(entry) => {
                entry.1 += u32::from(peeled);
                entry.2 |= !peeled;
            }
            None => found.push((name, u32::from(peeled), !peeled)),
        }
    }
    found
        .into_iter()
        .map(|(name, peeled, kept)| Technique {
            name,
            state: match (peeled > 0, kept) {
                (true, false) => State::Undone,
                (true, true) => State::Partly,
                (false, _) => State::Kept,
            },
            detail: (peeled > 0).then(|| peeled.to_string()),
        })
        .collect()
}

fn families(techniques: &[Technique], output: &Output) -> Vec<String> {
    let mut families = Vec::new();
    let obfuscator = techniques.iter().any(|technique| {
        matches!(
            technique.name,
            "string-array-rotate" | "string-array-decoders"
        ) || OBFUSCATOR_ONLY.contains(&technique.name)
    });
    if obfuscator {
        families.push(JAVASCRIPT_OBFUSCATOR.to_string());
    }
    if techniques
        .iter()
        .any(|technique| technique.name == "packer-layer")
    {
        families.push(PACKER.to_string());
    }
    if let Some(bundle) = &output.bundle {
        families.push(bundle.kind.clone());
    }
    families
}
