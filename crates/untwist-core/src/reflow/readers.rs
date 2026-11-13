use oxc_ast::ast::Program;

use crate::guard_shape::{reads_source, source_readers, SourceReaders};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PendingReaders {
    #[default]
    Absent,
    Unresolved,
    Present,
}

impl PendingReaders {
    pub fn of(program: &Program<'_>) -> Self {
        if reads_source(program) {
            PendingReaders::Present
        } else {
            PendingReaders::Absent
        }
    }

    pub fn unguarded(&self, printed: &str) -> bool {
        match self {
            PendingReaders::Absent => false,
            PendingReaders::Unresolved => true,
            PendingReaders::Present => source_readers(printed) == SourceReaders::Unguarded,
        }
    }
}
