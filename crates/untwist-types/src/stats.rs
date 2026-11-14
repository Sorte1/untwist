use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Binding,
    Param,
    Return,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub eligible: usize,
    pub proven: usize,
    pub annotated: usize,
    pub redundant: usize,
    pub refused: usize,
}

impl Counts {
    pub fn absorb(&mut self, other: Counts) {
        self.eligible += other.eligible;
        self.proven += other.proven;
        self.annotated += other.annotated;
        self.redundant += other.redundant;
        self.refused += other.refused;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TypeStats {
    pub bindings: Counts,
    pub params: Counts,
    pub returns: Counts,
    pub files: usize,
    pub files_refused: usize,
    pub reasons: BTreeMap<String, usize>,
}

impl TypeStats {
    pub fn count(&mut self, position: Position) -> &mut Counts {
        match position {
            Position::Binding => &mut self.bindings,
            Position::Param => &mut self.params,
            Position::Return => &mut self.returns,
        }
    }

    pub fn refuse(&mut self, reason: String) {
        *self.reasons.entry(reason).or_default() += 1;
    }

    pub fn absorb(&mut self, other: TypeStats) {
        for (reason, count) in other.reasons {
            *self.reasons.entry(reason).or_default() += count;
        }
        self.bindings.absorb(other.bindings);
        self.params.absorb(other.params);
        self.returns.absorb(other.returns);
        self.files += other.files;
        self.files_refused += other.files_refused;
    }

    pub fn total(&self) -> Counts {
        let mut total = Counts::default();
        total.absorb(self.bindings);
        total.absorb(self.params);
        total.absorb(self.returns);
        total
    }
}
