use std::hash::{DefaultHasher, Hash, Hasher};

use oxc_ast::ast::Program;
use untwist_core::print::pretty_codegen;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotCode {
    Text(String),
    Unchanged,
    OverCap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub input: usize,
    pub layer: u32,
    pub stage: &'static str,
    pub round: u32,
    pub code: SnapshotCode,
}

#[derive(Debug, Default)]
pub(super) struct Snapshots {
    room: Option<usize>,
    layer: u32,
    last: Option<u64>,
    pub(super) rows: Vec<Snapshot>,
}

fn fingerprint(code: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    hasher.finish()
}

impl Snapshots {
    pub(super) fn new(cap: Option<usize>) -> Self {
        Self {
            room: cap,
            ..Self::default()
        }
    }

    pub(super) fn enter_layer(&mut self, layer: u32) {
        self.layer = layer;
    }

    pub(super) fn mark(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn restore(&mut self, mark: usize) {
        self.rows.truncate(mark);
        self.last = None;
    }

    pub(super) fn take(&mut self, stage: &'static str, round: u32, program: &Program<'_>) {
        let Some(room) = self.room else {
            return;
        };
        let code = pretty_codegen(program);
        let print = fingerprint(&code);
        let kept = if self.last == Some(print) {
            SnapshotCode::Unchanged
        } else if code.len() <= room {
            self.room = Some(room - code.len());
            SnapshotCode::Text(code)
        } else {
            SnapshotCode::OverCap
        };
        self.last = Some(print);
        self.rows.push(Snapshot {
            input: 0,
            layer: self.layer,
            stage,
            round,
            code: kept,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;

    fn taken(snapshots: &mut Snapshots, stage: &'static str, source: &str) {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        snapshots.take(stage, 1, &parsed.program);
    }

    fn codes(snapshots: &Snapshots) -> Vec<SnapshotCode> {
        snapshots.rows.iter().map(|row| row.code.clone()).collect()
    }

    #[test]
    fn nothing_is_printed_without_a_cap() {
        let mut snapshots = Snapshots::new(None);
        taken(&mut snapshots, "prepare", "a;");
        assert!(snapshots.rows.is_empty());
    }

    #[test]
    fn a_step_that_prints_the_same_code_is_marked_unchanged() {
        let mut snapshots = Snapshots::new(Some(1024));
        taken(&mut snapshots, "prepare", "a;");
        taken(&mut snapshots, "deobfuscate", "a;");
        taken(&mut snapshots, "unminify", "b;");
        assert_eq!(
            codes(&snapshots),
            [
                SnapshotCode::Text("a;".to_string()),
                SnapshotCode::Unchanged,
                SnapshotCode::Text("b;".to_string()),
            ]
        );
        let stages: Vec<&str> = snapshots.rows.iter().map(|row| row.stage).collect();
        assert_eq!(stages, ["prepare", "deobfuscate", "unminify"]);
    }

    #[test]
    fn code_past_the_cap_is_not_kept_and_leaves_room_for_smaller_steps() {
        let mut snapshots = Snapshots::new(Some(4));
        taken(&mut snapshots, "prepare", "aaaaaaaa;");
        taken(&mut snapshots, "deobfuscate", "b;");
        assert_eq!(
            codes(&snapshots),
            [SnapshotCode::OverCap, SnapshotCode::Text("b;".to_string())]
        );
    }

    #[test]
    fn a_restored_mark_drops_the_steps_after_it_and_tags_the_layer() {
        let mut snapshots = Snapshots::new(Some(1024));
        taken(&mut snapshots, "prepare", "a;");
        let mark = snapshots.mark();
        snapshots.enter_layer(1);
        taken(&mut snapshots, "deobfuscate", "b;");
        assert_eq!(snapshots.rows[1].layer, 1);
        snapshots.restore(mark);
        snapshots.enter_layer(0);
        taken(&mut snapshots, "deobfuscate", "a;");
        assert_eq!(
            codes(&snapshots),
            [
                SnapshotCode::Text("a;".to_string()),
                SnapshotCode::Text("a;".to_string())
            ]
        );
    }
}
