use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::{patched_by_another_input, scan, Reliance};
use untwist_core::semantic::{has_module_syntax, is_commonjs, scope_hazard};

pub const PASS: &str = "unread-decoys";

const OBJECT: &str = "Object";
const FREEZE: &str = "freeze";
const PROTO: &str = "__proto__";

static OBJECT_FREEZE: Reliance = Reliance {
    roots: &[OBJECT],
    members: &[(OBJECT, false, &[FREEZE])],
};

#[derive(Default)]
struct References {
    symbols: FxHashSet<SymbolId>,
    count: FxHashMap<SymbolId, usize>,
}

struct Reads<'s> {
    scoping: &'s Scoping,
    found: References,
}

impl<'a> VisitJs<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol {
            self.found.symbols.insert(symbol);
            *self.found.count.entry(symbol).or_default() += 1;
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn references_in(statement: &Statement<'_>, scoping: &Scoping) -> References {
    let mut reads = Reads {
        scoping,
        found: References::default(),
    };
    reads.visit_statement(statement);
    reads.found
}

enum Decoy {
    Declares(Vec<SymbolId>),
    Reads(SymbolId),
}

struct Literals<'s> {
    scoping: &'s Scoping,
    freezes: bool,
}

impl Literals<'_> {
    fn is_inert(&mut self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(template) => template.expressions.is_empty(),
            Expression::UnaryExpression(unary) => {
                matches!(
                    unary.operator,
                    UnaryOperator::UnaryNegation | UnaryOperator::LogicalNot
                ) && matches!(
                    unary.argument.without_parentheses(),
                    Expression::NumericLiteral(_) | Expression::BooleanLiteral(_)
                )
            }
            Expression::ArrayExpression(array) => {
                array.elements.iter().all(|element| match element {
                    ArrayExpressionElement::Elision(_) => true,
                    ArrayExpressionElement::SpreadElement(_) => false,
                    other => other
                        .as_expression()
                        .is_some_and(|value| self.is_inert(value)),
                })
            }
            Expression::ObjectExpression(object) => object
                .properties
                .iter()
                .all(|property| self.is_inert_property(property)),
            Expression::CallExpression(call) => self.is_frozen_literal(call),
            _ => false,
        }
    }

    fn is_inert_property(&mut self, property: &ObjectPropertyKind<'_>) -> bool {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        if property.kind != PropertyKind::Init || property.method {
            return false;
        }
        let plain_key = match &property.key {
            PropertyKey::StaticIdentifier(_) => !property.computed,
            PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_) => true,
            _ => false,
        };
        let named_proto = property
            .key
            .static_name()
            .is_some_and(|name| name.as_str() == Some(PROTO));
        plain_key && !named_proto && self.is_inert(&property.value)
    }

    fn is_frozen_literal(&mut self, call: &CallExpression<'_>) -> bool {
        let Expression::StaticMemberExpression(callee) = call.callee.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(root) = callee.object.without_parentheses() else {
            return false;
        };
        let unbound = root
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).symbol_id().is_none());
        let [Argument::ArrayExpression(_) | Argument::ObjectExpression(_)] =
            call.arguments.as_slice()
        else {
            return false;
        };
        let inert = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|value| self.is_inert(value));
        if root.name != OBJECT || callee.property.name != FREEZE || !unbound || call.optional {
            return false;
        }
        self.freezes |= inert;
        inert
    }
}

fn declared_symbols(declaration: &VariableDeclaration<'_>) -> Option<Vec<SymbolId>> {
    declaration
        .declarations
        .iter()
        .map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn decoy_of(statement: &Statement<'_>, literals: &mut Literals<'_>) -> Option<Decoy> {
    match statement {
        Statement::FunctionDeclaration(function) => {
            let symbol = function.id.as_ref()?.symbol_id.get()?;
            Some(Decoy::Declares(vec![symbol]))
        }
        Statement::VariableDeclaration(declaration) => {
            if declaration.kind.is_using() || declaration.declare {
                return None;
            }
            let inert = declaration.declarations.iter().all(|declarator| {
                declarator
                    .init
                    .as_ref()
                    .is_none_or(|init| literals.is_inert(init))
            });
            inert
                .then(|| declared_symbols(declaration))
                .flatten()
                .map(Decoy::Declares)
        }
        Statement::ExpressionStatement(statement) => {
            let read = match statement.expression.without_parentheses() {
                Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void => {
                    &unary.argument
                }
                other => other,
            };
            let Expression::Identifier(id) = read.without_parentheses() else {
                return None;
            };
            let symbol = id
                .reference_id
                .get()
                .and_then(|reference| literals.scoping.get_reference(reference).symbol_id())?;
            let function = literals.scoping.symbol_flags(symbol).is_function();
            function.then_some(Decoy::Reads(symbol))
        }
        _ => None,
    }
}

struct Plan {
    decoys: Vec<Option<Decoy>>,
    freezes: Vec<bool>,
    references: Vec<References>,
    declared_at: FxHashMap<SymbolId, usize>,
}

impl Plan {
    fn of(program: &Program<'_>, scoping: &Scoping) -> Option<Self> {
        let mut decoys = Vec::with_capacity(program.body.len());
        let mut freezes = Vec::with_capacity(program.body.len());
        for statement in &program.body {
            let mut literals = Literals {
                scoping,
                freezes: false,
            };
            decoys.push(decoy_of(statement, &mut literals));
            freezes.push(literals.freezes);
        }
        let references: Vec<References> = program
            .body
            .iter()
            .map(|statement| references_in(statement, scoping))
            .collect();
        let mut declared_at = FxHashMap::default();
        for (index, decoy) in decoys.iter().enumerate() {
            let Some(Decoy::Declares(symbols)) = decoy else {
                continue;
            };
            for symbol in symbols {
                if declared_at.insert(*symbol, index).is_some() {
                    return None;
                }
            }
        }
        Some(Self {
            decoys,
            freezes,
            references,
            declared_at,
        })
    }

    fn fully_seen(&self, symbol: SymbolId, scoping: &Scoping) -> bool {
        let seen: usize = self
            .references
            .iter()
            .map(|found| found.count.get(&symbol).copied().unwrap_or(0))
            .sum();
        scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
            && scoping.symbol_redeclarations(symbol).is_empty()
            && scoping.get_resolved_reference_ids(symbol).len() == seen
    }

    fn candidate(&self, index: usize, scoping: &Scoping, frozen: bool) -> bool {
        if self.freezes[index] && !frozen {
            return false;
        }
        match &self.decoys[index] {
            Some(Decoy::Declares(symbols)) => symbols
                .iter()
                .all(|symbol| self.fully_seen(*symbol, scoping)),
            Some(Decoy::Reads(symbol)) => self.declared_at.contains_key(symbol),
            None => false,
        }
    }

    fn removable(&self, scoping: &Scoping, frozen: bool) -> Vec<usize> {
        let mut kept: FxHashSet<usize> = (0..self.decoys.len())
            .filter(|index| self.candidate(*index, scoping, frozen))
            .collect();
        loop {
            let before = kept.len();
            let snapshot = kept.clone();
            kept.retain(|index| self.still_unread(*index, &snapshot));
            if kept.len() == before {
                break;
            }
        }
        let mut indices: Vec<usize> = kept.into_iter().collect();
        indices.sort_unstable();
        indices
    }

    fn still_unread(&self, index: usize, removable: &FxHashSet<usize>) -> bool {
        match &self.decoys[index] {
            Some(Decoy::Declares(symbols)) => symbols.iter().all(|symbol| {
                self.references
                    .iter()
                    .enumerate()
                    .all(|(at, found)| removable.contains(&at) || !found.symbols.contains(symbol))
            }),
            Some(Decoy::Reads(symbol)) => self
                .declared_at
                .get(symbol)
                .is_some_and(|at| removable.contains(at)),
            None => false,
        }
    }
}

fn freeze_holds(program: &Program<'_>, scoping: &Scoping, unsafe_lane: bool) -> bool {
    !patched_by_another_input(&OBJECT_FREEZE)
        && scan(
            program,
            scoping,
            has_module_syntax(program),
            false,
            &OBJECT_FREEZE,
        )
        .verdict
        .allows_safe(unsafe_lane)
}

fn planned_removals(program: &Program<'_>, cx: &Cx<'_, '_>) -> Vec<Span> {
    let scoping = cx.scoping();
    let Some(plan) = Plan::of(program, scoping) else {
        return Vec::new();
    };
    let frozen = plan.freezes.iter().any(|freezes| *freezes)
        && freeze_holds(program, scoping, cx.opts.unsafe_transforms);
    plan.removable(scoping, frozen)
        .into_iter()
        .map(|index| program.body[index].span())
        .collect()
}

#[derive(Default)]
pub struct UnreadDecoys;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UnreadDecoys)
}

impl<'a> Pass<'a> for UnreadDecoys {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if !is_commonjs(program) || scope_hazard(cx.scoping()).is_some() {
            return;
        }
        let removals = planned_removals(program, cx);
        if removals.is_empty() {
            return;
        }
        let mut edits = StmtEdits::new();
        for span in removals {
            edits.remove(span);
        }
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
    }
}
