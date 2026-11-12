use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::is_constant_binding;

use crate::call_checks::is_under_with;
use crate::wrapper_args::{is_arithmetic, keeps_its_operand};

const FUNCTION_BOUNDARY: ScopeFlags = ScopeFlags::Top
    .union(ScopeFlags::Function)
    .union(ScopeFlags::Arrow)
    .union(ScopeFlags::ClassStaticBlock);

fn is_primitive_literal(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::UnaryNegation
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

struct Collect<'s> {
    scoping: &'s Scoping,
    found: FxHashMap<SymbolId, u32>,
    early: FxHashMap<SymbolId, u32>,
}

fn declares_only_literals(declaration: &VariableDeclaration<'_>) -> bool {
    matches!(
        declaration.kind,
        VariableDeclarationKind::Var
            | VariableDeclarationKind::Let
            | VariableDeclarationKind::Const
    ) && declaration.declarations.iter().all(|declarator| {
        matches!(declarator.id, BindingPattern::BindingIdentifier(_))
            && declarator.init.as_ref().is_none_or(is_primitive_literal)
    })
}

fn bound_symbols<'d>(
    declaration: &'d VariableDeclaration<'_>,
) -> impl Iterator<Item = SymbolId> + 'd {
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
}

impl Collect<'_> {
    fn note_inert_prefix(&mut self, body: &FunctionBody<'_>) {
        for statement in &body.statements {
            match statement {
                Statement::FunctionDeclaration(_) => {}
                Statement::VariableDeclaration(declaration)
                    if declares_only_literals(declaration) =>
                {
                    for symbol in bound_symbols(declaration) {
                        self.early.insert(symbol, body.span.start);
                    }
                }
                _ => return,
            }
        }
    }
}

struct Sequences<'s> {
    scoping: &'s Scoping,
    known: &'s PrimitiveNames,
    settled: FxHashSet<u32>,
}

impl Sequences<'_> {
    fn holds_a_primitive(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
            Expression::UnaryExpression(unary) => {
                keeps_its_operand(unary.operator) && self.holds_a_primitive(&unary.argument)
            }
            Expression::BinaryExpression(binary) => {
                is_arithmetic(binary.operator)
                    && self.holds_a_primitive(&binary.left)
                    && self.holds_a_primitive(&binary.right)
            }
            Expression::Identifier(id) => self.identifier_holds_a_primitive(id),
            _ => false,
        }
    }

    fn identifier_holds_a_primitive(&self, id: &IdentifierReference<'_>) -> bool {
        let Some(reference) = id.reference_id.get() else {
            return false;
        };
        let reference = self.scoping.get_reference(reference);
        let scope = reference.scope_id();
        reference.symbol_id().is_some_and(|symbol| {
            !is_under_with(self.scoping, scope)
                && self.known.holds(self.scoping, symbol, scope, id.span.start)
        })
    }

    fn literal_write(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::AssignmentExpression(assignment) = expression else {
            return None;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return None;
        };
        if assignment.operator != AssignmentOperator::Assign
            || !self.holds_a_primitive(&assignment.right)
        {
            return None;
        }
        let reference = target.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn settle_reads(&mut self, expression: &Expression<'_>, written: &[SymbolId]) {
        match expression.without_parentheses() {
            Expression::Identifier(id) => {
                let symbol = id
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
                if symbol.is_some_and(|symbol| written.contains(&symbol)) {
                    self.settled.insert(id.span.start);
                }
            }
            Expression::UnaryExpression(unary) => self.settle_reads(&unary.argument, written),
            Expression::BinaryExpression(binary) => {
                self.settle_reads(&binary.left, written);
                self.settle_reads(&binary.right, written);
            }
            _ => {}
        }
    }

    fn settle_sequence(&mut self, sequence: &SequenceExpression<'_>) {
        let mut written = Vec::new();
        for expression in &sequence.expressions {
            if let Some(symbol) = self.literal_write(expression) {
                written.push(symbol);
                continue;
            }
            if let Some(call) = first_evaluated_call(expression) {
                self.settle_call(call, &written);
            }
            return;
        }
    }

    fn settle_call(&mut self, call: &CallExpression<'_>, written: &[SymbolId]) {
        for argument in &call.arguments {
            if let Some(argument) = argument.as_expression() {
                self.settle_reads(argument, written);
            }
        }
    }

    fn settle_statements(&mut self, statements: &[Statement<'_>]) {
        let mut written = Vec::new();
        for statement in statements {
            if let Statement::ExpressionStatement(write) = statement {
                if let Some(symbol) = self.literal_write(&write.expression) {
                    written.push(symbol);
                    continue;
                }
            }
            if let Some(call) = leading_call(statement).filter(|_| !written.is_empty()) {
                self.settle_call(call, &written);
            }
            written.clear();
        }
    }
}

fn first_evaluated_call<'s>(expression: &'s Expression<'s>) -> Option<&'s CallExpression<'s>> {
    match expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        Expression::BinaryExpression(binary) => first_evaluated_call(&binary.left),
        _ => None,
    }
}

fn leading_call<'s>(statement: &'s Statement<'_>) -> Option<&'s CallExpression<'s>> {
    let expression = match statement {
        Statement::ReturnStatement(statement) => statement.argument.as_ref()?,
        Statement::ExpressionStatement(statement) => &statement.expression,
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.first()?.init.as_ref()?
        }
        _ => return None,
    };
    match expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Sequences<'_> {
    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        self.settle_sequence(it);
        walk_js::walk_sequence_expression(self, it);
    }

    fn visit_statements(&mut self, it: &oxc_allocator::Vec<'a, Statement<'a>>) {
        self.settle_statements(it);
        walk_js::walk_statements(self, it);
    }
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.note_inert_prefix(it);
        walk_js::walk_function_body(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let symbol = id.symbol_id.get();
            if let Some(symbol) = symbol.filter(|symbol| {
                is_primitive_literal(init) && is_constant_binding(self.scoping, *symbol)
            }) {
                self.found.insert(symbol, it.span.end);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }
}

#[derive(Default)]
pub struct PrimitiveNames {
    declared_until: FxHashMap<SymbolId, u32>,
    early: FxHashMap<SymbolId, u32>,
    settled: FxHashSet<u32>,
}

impl PrimitiveNames {
    pub fn collect(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut collect = Collect {
            scoping,
            found: FxHashMap::default(),
            early: FxHashMap::default(),
        };
        collect.visit_program(program);
        let known = PrimitiveNames {
            declared_until: collect.found,
            early: collect.early,
            settled: FxHashSet::default(),
        };
        let mut sequences = Sequences {
            scoping,
            known: &known,
            settled: FxHashSet::default(),
        };
        sequences.visit_program(program);
        let settled = sequences.settled;
        PrimitiveNames { settled, ..known }
    }

    pub fn written_just_before(&self, read: u32) -> bool {
        self.settled.contains(&read)
    }

    pub fn holds(&self, scoping: &Scoping, symbol: SymbolId, scope: ScopeId, at: u32) -> bool {
        let Some(end) = self.declared_until.get(&symbol) else {
            return false;
        };
        if sees_a_direct_eval(scoping, scope) {
            return false;
        }
        if scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::FunctionScopedVariable)
        {
            return true;
        }
        if self.early.get(&symbol).is_some_and(|body| *body <= at) {
            return true;
        }
        *end <= at
            && runs_in_the_declaring_function(scoping, scoping.symbol_scope_id(symbol), scope)
    }
}

const ARGUMENTS: &str = "arguments";
const EVAL: &str = "eval";

fn reads_arguments(scoping: &Scoping, function: ScopeId) -> bool {
    let unresolved = scoping.root_unresolved_references();
    unresolved.get(ARGUMENTS).is_some_and(|references| {
        references.iter().any(|reference| {
            let scope = scoping.get_reference(*reference).scope_id();
            scoping.scope_ancestors(scope).find(|ancestor| {
                let flags = scoping.scope_flags(*ancestor);
                flags.intersects(ScopeFlags::Top | ScopeFlags::Function)
                    && !flags.contains(ScopeFlags::Arrow)
            }) == Some(function)
        })
    })
}

pub fn is_stable(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
        && is_constant_binding(scoping, symbol)
        && !scoping.root_unresolved_references().contains_key(EVAL)
        && !reads_arguments(scoping, scoping.symbol_scope_id(symbol))
}

fn sees_a_direct_eval(scoping: &Scoping, scope: ScopeId) -> bool {
    scoping.scope_ancestors(scope).any(|ancestor| {
        scoping
            .scope_flags(ancestor)
            .contains(ScopeFlags::DirectEval)
    })
}

fn runs_in_the_declaring_function(
    scoping: &Scoping,
    declared: ScopeId,
    reference: ScopeId,
) -> bool {
    if !scoping.scope_flags(declared).intersects(FUNCTION_BOUNDARY) {
        return false;
    }
    scoping
        .scope_ancestors(reference)
        .find(|ancestor| scoping.scope_flags(*ancestor).intersects(FUNCTION_BOUNDARY))
        == Some(declared)
}
