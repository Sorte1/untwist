use serde::Serialize;
use untwist_core::pass::{Note, NoteKind};

use super::{Output, Pipeline};
use crate::options::Progress;

pub const SVELTE_PASS: &str = "svelte";

pub const SVELTE_NOT_BUILT: &str = "this build of untwist has no Svelte restore";

pub const SOLID_PASS: &str = "solid";

pub const SOLID_NOT_BUILT: &str = "this build of untwist has no Solid restore";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreHolder {
    Input(usize),
    Module(usize),
}

pub type SvelteHolder = RestoreHolder;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RestoredFile {
    pub path: String,
    pub code: String,
    #[serde(skip)]
    pub holder: RestoreHolder,
}

pub type SvelteFile = RestoredFile;

struct Framework {
    pass: &'static str,
    extension: &'static str,
    not_built: &'static str,
    built: bool,
    #[allow(dead_code)]
    restore: fn(&str) -> untwist_restore::Restore,
}

fn note(notes: &mut Vec<Note>, pass: &'static str, message: String) {
    notes.push(Note {
        pass,
        kind: NoteKind::Progress,
        message,
        span: None,
    });
}

fn directory(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..=at])
}

fn free_path(taken: &[RestoredFile], dir: &str, name: &str, extension: &str) -> String {
    let mut path = format!("{dir}{name}.{extension}");
    let mut counter = 1;
    while taken.iter().any(|file| file.path == path) {
        counter += 1;
        path = format!("{dir}{name}-{counter}.{extension}");
    }
    path
}

fn restore(
    framework: &Framework,
    code: &str,
    dir: &str,
    holder: RestoreHolder,
    files: &mut Vec<RestoredFile>,
    notes: &mut Vec<Note>,
) {
    let restored = std::panic::catch_unwind(|| (framework.restore)(code)).unwrap_or_default();
    for component in restored.components {
        let path = free_path(files, dir, &component.name, framework.extension);
        note(
            notes,
            framework.pass,
            format!("restored the component {} as {path}", component.name),
        );
        files.push(RestoredFile {
            path,
            code: component.text,
            holder,
        });
    }
    for (name, reason) in restored.kept {
        note(
            notes,
            framework.pass,
            format!("{name} kept as JS: {reason}"),
        );
    }
}

#[cfg(feature = "svelte")]
fn svelte(code: &str) -> untwist_restore::Restore {
    untwist_svelte::restore(code)
}

#[cfg(not(feature = "svelte"))]
fn svelte(_: &str) -> untwist_restore::Restore {
    untwist_restore::Restore::default()
}

#[cfg(feature = "solid")]
fn solid(code: &str) -> untwist_restore::Restore {
    untwist_solid::restore(code)
}

#[cfg(not(feature = "solid"))]
fn solid(_: &str) -> untwist_restore::Restore {
    untwist_restore::Restore::default()
}

const SVELTE: Framework = Framework {
    pass: SVELTE_PASS,
    extension: "svelte",
    not_built: SVELTE_NOT_BUILT,
    built: cfg!(feature = "svelte"),
    restore: svelte,
};

const SOLID: Framework = Framework {
    pass: SOLID_PASS,
    extension: "jsx",
    not_built: SOLID_NOT_BUILT,
    built: cfg!(feature = "solid"),
    restore: solid,
};

impl Pipeline {
    pub(super) fn restore_stage(&self, output: &mut Output) {
        if self.options.svelte {
            output.svelte = self.framework_stage(&SVELTE, output);
        }
        if self.options.solid {
            output.solid = self.framework_stage(&SOLID, output);
        }
    }

    fn framework_stage(&self, framework: &Framework, output: &mut Output) -> Vec<RestoredFile> {
        if !framework.built {
            note(
                &mut output.notes,
                framework.pass,
                framework.not_built.to_string(),
            );
            return Vec::new();
        }
        self.options.progress(Progress::StageBegins(framework.pass));
        let mut files = Vec::new();
        let mut notes = Vec::new();
        match output.bundle.as_ref() {
            Some(bundle) => {
                for (index, module) in bundle.modules.iter().enumerate() {
                    restore(
                        framework,
                        &module.code,
                        directory(&module.path),
                        RestoreHolder::Module(index),
                        &mut files,
                        &mut notes,
                    );
                }
            }
            None => {
                for (index, code) in output.codes.iter().enumerate() {
                    restore(
                        framework,
                        code,
                        "",
                        RestoreHolder::Input(index),
                        &mut files,
                        &mut notes,
                    );
                }
            }
        }
        output.notes.extend(notes);
        files
    }
}
