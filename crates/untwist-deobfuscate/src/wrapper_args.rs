use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::ast_util::inline::{param_names, returned_expression};

use crate::call_checks::is_under_with;
use crate::primitive_names::{is_stable, PrimitiveNames};

const UNDEFINED: &str = "undefined";
const FORWARDED_CALLS: usize = 1;

#[derive(Clone, Copy, Default)]
struct ParamUse {
    count: usize,
    bare: usize,
    crooked: bool,
}

impl ParamUse {
    fn is_single(self) -> bool {
        self.count == 1 && !self.crooked
    }

    fn is_bare(self) -> bool {
        self.bare == self.count && !self.crooked
    }
}

pub struct BodyUses {
    params: Vec<ParamUse>,
    order: Vec<usize>,
    calls: usize,
    blocked: bool,
}

impl BodyUses {
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    fn reads_in_order(&self, indices: &[usize]) -> bool {
        let read = self.order.iter().filter(|index| indices.contains(index));
        read.eq(indices.iter())
    }
}

pub fn keeps_its_operand(operator: UnaryOperator) -> bool {
    matches!(
        operator,
        UnaryOperator::UnaryNegation
            | UnaryOperator::UnaryPlus
            | UnaryOperator::LogicalNot
            | UnaryOperator::BitwiseNot
            | UnaryOperator::Void
    )
}

pub fn is_arithmetic(operator: BinaryOperator) -> bool {
    !matches!(operator, BinaryOperator::In | BinaryOperator::Instanceof)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    Straight,
    Branching,
    Foreign,
}

struct UseScan<'n> {
    names: &'n [&'n str],
    uses: Vec<ParamUse>,
    order: Vec<usize>,
    calls: usize,
    straight: bool,
    blocked: bool,
}

impl UseScan<'_> {
    fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|known| *known == name)
    }

    fn record(&mut self, index: usize) {
        let straight = self.straight;
        if let Some(entry) = self.uses.get_mut(index) {
            entry.count += 1;
            entry.crooked |= !straight;
            self.order.push(index);
        }
    }

    fn note_bare_arguments(&mut self, call: &CallExpression<'_>) {
        for argument in &call.arguments {
            let Some(Expression::Identifier(id)) = argument.as_expression() else {
                continue;
            };
            let index = self.index_of(id.name.as_str());
            if let Some(entry) = index.and_then(|index| self.uses.get_mut(index)) {
                entry.bare += 1;
            }
        }
    }

    fn calls_a_fixed_name(&self, call: &CallExpression<'_>) -> bool {
        matches!(
            &call.callee,
            Expression::Identifier(callee) if self.index_of(callee.name.as_str()).is_none()
        )
    }

    fn flow_of(&self, expression: &Expression<'_>) -> Flow {
        match expression {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::ParenthesizedExpression(_)
            | Expression::SequenceExpression(_) => Flow::Straight,
            Expression::UnaryExpression(unary) if keeps_its_operand(unary.operator) => {
                Flow::Straight
            }
            Expression::BinaryExpression(binary) if is_arithmetic(binary.operator) => {
                Flow::Straight
            }
            Expression::CallExpression(call) if !call.optional && self.calls_a_fixed_name(call) => {
                Flow::Straight
            }
            Expression::ConditionalExpression(_) | Expression::LogicalExpression(_) => {
                Flow::Branching
            }
            _ => Flow::Foreign,
        }
    }
}

impl<'a> VisitJs<'a> for UseScan<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::Identifier(id) = it {
            if let Some(index) = self.index_of(id.name.as_str()) {
                self.record(index);
            }
            return;
        }
        if let Expression::CallExpression(call) = it {
            self.calls += 1;
            self.note_bare_arguments(call);
        }
        let flow = self.flow_of(it);
        let outer = self.straight;
        self.blocked |= flow == Flow::Foreign;
        self.straight = outer && flow == Flow::Straight;
        walk_js::walk_expression(self, it);
        self.straight = outer;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.blocked |= self.index_of(it.name.as_str()).is_some();
    }

    fn visit_spread_element(&mut self, _it: &SpreadElement<'a>) {
        self.blocked = true;
    }
}

pub fn body_uses(function: &Function<'_>) -> Option<BodyUses> {
    let names = param_names(function)?;
    let returned = returned_expression(function)?;
    let mut scan = UseScan {
        names: &names,
        uses: vec![ParamUse::default(); names.len()],
        order: Vec::new(),
        calls: 0,
        straight: true,
        blocked: false,
    };
    scan.visit_expression(returned);
    Some(BodyUses {
        params: scan.uses,
        order: scan.order,
        calls: scan.calls,
        blocked: scan.blocked,
    })
}

fn is_pure_value(expr: &Expression<'_>, leaf: &dyn Fn(&IdentifierReference<'_>) -> bool) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::Identifier(id) => leaf(id),
        Expression::UnaryExpression(unary) => {
            keeps_its_operand(unary.operator) && is_pure_value(&unary.argument, leaf)
        }
        Expression::BinaryExpression(binary) => {
            is_arithmetic(binary.operator)
                && is_pure_value(&binary.left, leaf)
                && is_pure_value(&binary.right, leaf)
        }
        _ => false,
    }
}

enum Binding {
    Unknown,
    Free,
    Bound(SymbolId),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pure,
    Settled,
    Stable,
    Name,
    Valued,
    Compound,
}

fn operated(kind: Kind) -> Kind {
    match kind {
        Kind::Pure | Kind::Settled => kind,
        _ => Kind::Valued,
    }
}

fn joined(left: Kind, right: Kind) -> Kind {
    match (operated(left), operated(right)) {
        (Kind::Pure, Kind::Pure) => Kind::Pure,
        (Kind::Pure | Kind::Settled, Kind::Pure | Kind::Settled) => Kind::Settled,
        _ => Kind::Valued,
    }
}

fn yields_a_primitive(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::UnaryExpression(unary) => keeps_its_operand(unary.operator),
        Expression::BinaryExpression(binary) => is_arithmetic(binary.operator),
        _ => false,
    }
}

pub struct SiteView<'s> {
    scoping: &'s Scoping,
    primitives: &'s PrimitiveNames,
}

impl<'s> SiteView<'s> {
    pub fn new(scoping: &'s Scoping, primitives: &'s PrimitiveNames) -> Self {
        SiteView {
            scoping,
            primitives,
        }
    }

    fn holds_a_primitive(&self, id: &IdentifierReference<'_>) -> bool {
        let Binding::Bound(symbol) = self.binding(id) else {
            return false;
        };
        let Some(reference) = id.reference_id.get() else {
            return false;
        };
        let scope = self.scoping.get_reference(reference).scope_id();
        self.primitives
            .holds(self.scoping, symbol, scope, id.span.start)
    }

    fn binding(&self, id: &IdentifierReference<'_>) -> Binding {
        let Some(reference) = id.reference_id.get() else {
            return Binding::Unknown;
        };
        let reference = self.scoping.get_reference(reference);
        if is_under_with(self.scoping, reference.scope_id()) {
            return Binding::Unknown;
        }
        match reference.symbol_id() {
            Some(symbol) => Binding::Bound(symbol),
            None => Binding::Free,
        }
    }

    pub fn is_plain_undefined(&self, id: &IdentifierReference<'_>) -> bool {
        id.name.as_str() == UNDEFINED && matches!(self.binding(id), Binding::Free)
    }

    pub fn is_plain_local(&self, id: &IdentifierReference<'_>) -> bool {
        let Binding::Bound(symbol) = self.binding(id) else {
            return false;
        };
        let flags = self.scoping.symbol_flags(symbol);
        let scope = self.scoping.symbol_scope_id(symbol);
        let is_variable = flags
            .intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::BlockScopedVariable);
        let is_fixed = flags.intersects(
            SymbolFlags::ConstVariable
                | SymbolFlags::Function
                | SymbolFlags::Class
                | SymbolFlags::Import
                | SymbolFlags::CatchVariable,
        );
        let is_global = flags.contains(SymbolFlags::FunctionScopedVariable)
            && scope == self.scoping.root_scope_id();
        is_variable && !is_fixed && !is_global
    }

    fn is_hoisted(&self, symbol: SymbolId) -> bool {
        let flags = self.scoping.symbol_flags(symbol);
        flags.intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function)
            && !flags.intersects(
                SymbolFlags::BlockScopedVariable
                    | SymbolFlags::Class
                    | SymbolFlags::Import
                    | SymbolFlags::CatchVariable,
            )
    }

    fn is_settled_read(&self, id: &IdentifierReference<'_>) -> bool {
        matches!(self.binding(id), Binding::Bound(_))
            && self.primitives.written_just_before(id.span.start)
    }

    fn kind(&self, expression: &Expression<'_>, in_parameters: bool) -> Kind {
        let fixed = |id: &IdentifierReference<'_>| {
            self.is_plain_undefined(id) || self.holds_a_primitive(id)
        };
        if is_pure_value(expression, &fixed) {
            return Kind::Pure;
        }
        if is_pure_value(expression, &|id| fixed(id) || self.is_settled_read(id)) {
            return Kind::Settled;
        }
        if yields_a_primitive(expression) {
            return Kind::Valued;
        }
        let Expression::Identifier(id) = expression.without_parentheses() else {
            return Kind::Compound;
        };
        let Binding::Bound(symbol) = self.binding(id) else {
            return Kind::Compound;
        };
        if in_parameters || !self.is_hoisted(symbol) {
            return Kind::Compound;
        }
        if is_stable(self.scoping, symbol) {
            return Kind::Stable;
        }
        Kind::Name
    }

    pub fn kinds_of(
        &self,
        arguments: &AVec<'_, Argument<'_>>,
        in_parameters: bool,
    ) -> Option<Vec<Kind>> {
        arguments
            .iter()
            .map(|argument| {
                argument
                    .as_expression()
                    .map(|expression| self.kind(expression, in_parameters))
            })
            .collect()
    }

    pub fn kinds_through(
        &self,
        bound: &[Kind],
        names: &[&str],
        arguments: &AVec<'_, Argument<'_>>,
    ) -> Option<Vec<Kind>> {
        arguments
            .iter()
            .map(|argument| {
                argument
                    .as_expression()
                    .map(|expression| self.composed(expression, bound, names))
            })
            .collect()
    }

    fn composed(&self, expression: &Expression<'_>, bound: &[Kind], names: &[&str]) -> Kind {
        match expression.without_parentheses() {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => Kind::Pure,
            Expression::Identifier(id) => match names.iter().position(|name| *name == id.name) {
                Some(index) => bound.get(index).copied().unwrap_or(Kind::Pure),
                None if self.is_plain_undefined(id) || self.holds_a_primitive(id) => Kind::Pure,
                None => Kind::Compound,
            },
            Expression::UnaryExpression(unary) if keeps_its_operand(unary.operator) => {
                operated(self.composed(&unary.argument, bound, names))
            }
            Expression::BinaryExpression(binary) if is_arithmetic(binary.operator) => joined(
                self.composed(&binary.left, bound, names),
                self.composed(&binary.right, bound, names),
            ),
            _ => Kind::Compound,
        }
    }

    pub fn arguments_fit(
        &self,
        uses: &BodyUses,
        arguments: &AVec<'_, Argument<'_>>,
        in_parameters: bool,
    ) -> bool {
        self.kinds_of(arguments, in_parameters)
            .is_some_and(|kinds| kinds_fit(uses, &kinds))
    }
}

pub fn kinds_fit(uses: &BodyUses, kinds: &[Kind]) -> bool {
    if uses.blocked {
        return false;
    }
    let mut sensitive = Vec::new();
    let mut settled = false;
    for (index, kind) in kinds.iter().enumerate() {
        let used = uses.params.get(index).copied().unwrap_or_default();
        match kind {
            Kind::Pure => {}
            Kind::Settled => settled = true,
            Kind::Name if used.count == 0 => {}
            Kind::Stable if used.is_bare() => {}
            _ if used.is_single() => sensitive.push(index),
            _ => return false,
        }
    }
    if settled && !sensitive.is_empty() {
        return false;
    }
    let Some((_, earlier)) = sensitive.split_last() else {
        return true;
    };
    earlier.is_empty()
        || (uses.calls <= FORWARDED_CALLS
            && uses.reads_in_order(&sensitive)
            && earlier
                .iter()
                .all(|index| kinds.get(*index) == Some(&Kind::Valued)))
}
