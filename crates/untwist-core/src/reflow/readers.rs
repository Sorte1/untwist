use oxc_ast::ast::{Function, Program};

use super::guards::{guard_text, GuardRecord};
use crate::guard_shape::{judge_readers, SourceReaders};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PendingReaders {
    #[default]
    Absent,
    Unresolved,
    Pending(Vec<String>),
}

impl PendingReaders {
    pub fn of(program: &Program<'_>) -> Self {
        let mut pending = Vec::new();
        let mut note = |target: &Function<'_>| {
            pending.extend(guard_text(target));
            true
        };
        match judge_readers(program, &mut note) {
            SourceReaders::None => PendingReaders::Absent,
            SourceReaders::Unguarded => PendingReaders::Unresolved,
            SourceReaders::Guarded => PendingReaders::Pending(pending),
        }
    }

    pub fn unguarded(&self, guards: &GuardRecord) -> bool {
        match self {
            PendingReaders::Absent => false,
            PendingReaders::Unresolved => true,
            PendingReaders::Pending(texts) => texts.iter().any(|text| !guards.compacted(text)),
        }
    }
}
