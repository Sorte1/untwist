use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

const MAX_HANDS: usize = 8;

#[derive(Clone, Copy)]
pub enum Callee {
    Local(SymbolId, isize),
    Unknown,
    Constructs,
}

pub fn callee_of(callee: &Expression<'_>, scoping: &Scoping) -> Callee {
    let (id, shift): (&IdentifierReference<'_>, isize) = match callee.without_parentheses() {
        Expression::Identifier(id) => (id, 0),
        Expression::CallExpression(bound) => match bound_callee(bound) {
            Some(id) => (id, 1 - bound.arguments.len().max(1) as isize),
            None => return Callee::Unknown,
        },
        Expression::StaticMemberExpression(member) => match member.property.name.as_str() {
            "construct" => return Callee::Constructs,
            "call" | "bind" => match member.object.without_parentheses() {
                Expression::Identifier(id) => (id, 1),
                _ => return Callee::Unknown,
            },
            _ => return Callee::Unknown,
        },
        _ => return Callee::Unknown,
    };
    let symbol = id
        .reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id());
    match symbol {
        Some(symbol) => Callee::Local(symbol, shift),
        None => Callee::Unknown,
    }
}

fn bound_callee<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    match member.object.without_parentheses() {
        Expression::Identifier(id) if member.property.name == "bind" => Some(id),
        _ => None,
    }
}

pub fn parameter_at(position: usize, shift: isize) -> Option<usize> {
    usize::try_from(position as isize - shift).ok()
}

struct Params {
    items: Vec<Option<SymbolId>>,
    rest: bool,
    reads_arguments: bool,
}

#[derive(Default)]
pub struct CalleeParams {
    functions: FxHashMap<SymbolId, Params>,
    harmless: Vec<u64>,
    handed: FxHashMap<ReferenceId, (SymbolId, usize)>,
}

impl CalleeParams {
    pub fn of(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut scan = Scan {
            scoping,
            frames: Vec::new(),
            pending: None,
            found: CalleeParams {
                functions: FxHashMap::default(),
                harmless: vec![0; scoping.references_len().div_ceil(64)],
                handed: FxHashMap::default(),
            },
        };
        scan.visit_program(program);
        scan.found
    }

    pub fn may_construct(&self, scoping: &Scoping, callee: SymbolId, position: usize) -> bool {
        self.reaches_new(scoping, callee, position, 0)
    }

    fn reaches_new(
        &self,
        scoping: &Scoping,
        callee: SymbolId,
        position: usize,
        hands: usize,
    ) -> bool {
        let Some(params) = self.functions.get(&callee) else {
            return false;
        };
        if params.reads_arguments || hands > MAX_HANDS {
            return true;
        }
        match params.items.get(position) {
            Some(Some(symbol)) => !self.only_called(scoping, *symbol, hands),
            Some(None) => true,
            None => params.rest,
        }
    }

    fn only_called(&self, scoping: &Scoping, symbol: SymbolId, hands: usize) -> bool {
        scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| match self.handed.get(reference) {
                Some((callee, position)) => {
                    !self.reaches_new(scoping, *callee, *position, hands + 1)
                }
                None => self.is_harmless(*reference),
            })
    }

    fn is_harmless(&self, reference: ReferenceId) -> bool {
        let index = reference.index();
        self.harmless
            .get(index / 64)
            .is_some_and(|word| word & (1u64 << (index % 64)) != 0)
    }

    fn mark(&mut self, id: &IdentifierReference<'_>) {
        let Some(reference) = id.reference_id.get() else {
            return;
        };
        let index = reference.index();
        if let Some(word) = self.harmless.get_mut(index / 64) {
            *word |= 1u64 << (index % 64);
        }
    }

    fn note_arguments(&mut self, callee: Callee, arguments: &[Argument<'_>]) {
        if matches!(callee, Callee::Constructs) {
            return;
        }
        for (position, argument) in arguments.iter().enumerate() {
            let Argument::Identifier(id) = argument else {
                continue;
            };
            match callee {
                Callee::Local(symbol, shift) => {
                    let parameter = parameter_at(position, shift);
                    if let (Some(reference), Some(parameter)) = (id.reference_id.get(), parameter) {
                        self.handed.insert(reference, (symbol, parameter));
                    }
                }
                Callee::Unknown => self.mark(id),
                Callee::Constructs => {}
            }
        }
    }

    fn note(
        &mut self,
        id: &BindingIdentifier<'_>,
        params: &FormalParameters<'_>,
    ) -> Option<SymbolId> {
        let symbol = id.symbol_id.get()?;
        let items = params
            .items
            .iter()
            .map(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            })
            .collect();
        self.functions.insert(
            symbol,
            Params {
                items,
                rest: params.rest.is_some(),
                reads_arguments: false,
            },
        );
        Some(symbol)
    }

    fn note_arguments_read(&mut self, frames: &[(Option<SymbolId>, bool)]) {
        for (symbol, arrow) in frames.iter().rev() {
            if let Some(params) = symbol.and_then(|symbol| self.functions.get_mut(&symbol)) {
                params.reads_arguments = true;
            }
            if !arrow {
                return;
            }
        }
    }
}

struct Scan<'s> {
    scoping: &'s Scoping,
    found: CalleeParams,
    frames: Vec<(Option<SymbolId>, bool)>,
    pending: Option<SymbolId>,
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        match it.callee.without_parentheses() {
            Expression::Identifier(id) => self.found.mark(id),
            Expression::StaticMemberExpression(member)
                if matches!(member.property.name.as_str(), "call" | "apply") =>
            {
                if let Expression::Identifier(id) = member.object.without_parentheses() {
                    self.found.mark(id);
                }
            }
            _ => {}
        }
        let callee = callee_of(&it.callee, self.scoping);
        self.found.note_arguments(callee, &it.arguments);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if matches!(it.property.name.as_str(), "length" | "name") {
            if let Expression::Identifier(id) = it.object.without_parentheses() {
                self.found.mark(id);
            }
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Typeof {
            if let Expression::Identifier(id) = it.argument.without_parentheses() {
                self.found.mark(id);
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let own = match (it.is_declaration(), &it.id) {
            (true, Some(id)) => self.found.note(id, &it.params),
            _ => self.pending.take(),
        };
        self.frames.push((own, false));
        walk_js::walk_function(self, it, flags);
        self.frames.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let own = self.pending.take();
        self.frames.push((own, true));
        walk_js::walk_arrow_function_expression(self, it);
        self.frames.pop();
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" {
            self.found.note_arguments_read(&self.frames);
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            match it.init.as_ref().map(Expression::without_parentheses) {
                Some(Expression::FunctionExpression(function)) => {
                    self.pending = self.found.note(id, &function.params);
                }
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    self.pending = self.found.note(id, &arrow.params);
                }
                _ => {}
            }
        }
        walk_js::walk_variable_declarator(self, it);
        self.pending = None;
    }
}
