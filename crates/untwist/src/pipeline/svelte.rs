use serde::Serialize;
use untwist_core::pass::{Note, NoteKind};

use super::{Output, Pipeline};
use crate::options::Progress;

pub const SVELTE_PASS: &str = "svelte";

pub const SVELTE_NOT_BUILT: &str = "this build of untwist has no Svelte restore";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvelteHolder {
    Input(usize),
    Module(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SvelteFile {
    pub path: String,
    pub code: String,
    #[serde(skip)]
    pub holder: SvelteHolder,
}

fn note(notes: &mut Vec<Note>, message: String) {
    notes.push(Note {
        pass: SVELTE_PASS,
        kind: NoteKind::Progress,
        message,
        span: None,
    });
}

fn directory(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..=at])
}

fn free_path(taken: &[SvelteFile], dir: &str, name: &str) -> String {
    let mut path = format!("{dir}{name}.svelte");
    let mut counter = 1;
    while taken.iter().any(|file| file.path == path) {
        counter += 1;
        path = format!("{dir}{name}-{counter}.svelte");
    }
    path
}

#[cfg(feature = "svelte")]
fn restore(
    code: &str,
    dir: &str,
    holder: SvelteHolder,
    files: &mut Vec<SvelteFile>,
    notes: &mut Vec<Note>,
) {
    let restored = std::panic::catch_unwind(|| untwist_svelte::restore(code)).unwrap_or_default();
    for component in restored.components {
        let path = free_path(files, dir, &component.name);
        note(
            notes,
            format!("restored the component {} as {path}", component.name),
        );
        files.push(SvelteFile {
            path,
            code: component.text,
            holder,
        });
    }
    for (name, reason) in restored.kept {
        note(notes, format!("{name} kept as JS: {reason}"));
    }
}

impl Pipeline {
    pub(super) fn svelte_stage(&self, output: &mut Output) {
        if !self.options.svelte {
            return;
        }
        if !cfg!(feature = "svelte") {
            note(&mut output.notes, SVELTE_NOT_BUILT.to_string());
            return;
        }
        self.options.progress(Progress::StageBegins(SVELTE_PASS));
        #[cfg(feature = "svelte")]
        {
            let mut files = Vec::new();
            let mut notes = Vec::new();
            match output.bundle.as_ref() {
                Some(bundle) => {
                    for (index, module) in bundle.modules.iter().enumerate() {
                        restore(
                            &module.code,
                            directory(&module.path),
                            SvelteHolder::Module(index),
                            &mut files,
                            &mut notes,
                        );
                    }
                }
                None => {
                    for (index, code) in output.codes.iter().enumerate() {
                        restore(code, "", SvelteHolder::Input(index), &mut files, &mut notes);
                    }
                }
            }
            output.notes.extend(notes);
            output.svelte = files;
        }
    }
}
