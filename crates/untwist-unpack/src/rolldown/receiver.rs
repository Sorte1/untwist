use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;

use crate::scan::{Frame, Policy, Scan};

const ARGUMENTS: &str = "arguments";

struct Receiver<'s> {
    scoping: &'s Scoping,
    free: bool,
}

impl<'a> Policy<'a> for Receiver<'_> {
    fn done(&self) -> bool {
        !self.free
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        let unbound = it
            .reference_id
            .get()
            .is_none_or(|id| self.scoping.get_reference(id).symbol_id().is_none());
        if it.name == ARGUMENTS && unbound && !frame.receiver() {
            self.free = false;
        }
    }

    fn this(&mut self, frame: Frame) {
        self.free &= frame.receiver();
    }

    fn new_target(&mut self, frame: Frame) {
        self.free &= frame.receiver();
    }

    fn with(&mut self) {
        self.free = false;
    }
}

pub fn receiver_free<'k, 'a: 'k>(
    scoping: &Scoping,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> bool {
    let mut policy = Receiver {
        scoping,
        free: true,
    };
    let mut scan = Scan::new(&mut policy);
    for statement in statements {
        scan.visit_statement(statement);
    }
    policy.free
}
