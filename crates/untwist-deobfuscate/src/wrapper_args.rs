use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::ast_util::inline::{param_names, returned_expression};

use crate::call_checks::is_under_with;
use crate::primitive_names::PrimitiveNames;

const UNDEFINED: &str = "undefined";
const SENSITIVE_ARGUMENT_CAP: usize = 1;

#[derive(Clone, Copy, Default)]
struct ParamUse {
    count: usize,
    crooked: bool,
}

impl ParamUse {
    fn is_single(self) -> bool {
        self.count == 1 && !self.crooked
    }
}

pub struct BodyUses {
    params: Vec<ParamUse>,
    blocked: bool,
}

impl BodyUses {
    pub fn is_blocked(&self) -> bool {
        self.blocked
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
        straight: true,
        blocked: false,
    };
    scan.visit_expression(returned);
    Some(BodyUses {
        params: scan.uses,
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

#[derive(Clone, Copy)]
enum Kind {
    Pure,
    Name,
    Compound,
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

    fn kind(&self, expression: &Expression<'_>, in_parameters: bool) -> Kind {
        if is_pure_value(expression, &|id| {
            self.is_plain_undefined(id) || self.holds_a_primitive(id)
        }) {
            return Kind::Pure;
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
        Kind::Name
    }

    pub fn arguments_fit(
        &self,
        uses: &BodyUses,
        arguments: &AVec<'_, Argument<'_>>,
        in_parameters: bool,
    ) -> bool {
        if uses.blocked {
            return false;
        }
        let mut sensitive = 0;
        for (index, argument) in arguments.iter().enumerate() {
            let Some(expression) = argument.as_expression() else {
                return false;
            };
            let used = uses.params.get(index).copied().unwrap_or_default();
            match self.kind(expression, in_parameters) {
                Kind::Pure => {}
                Kind::Name if used.count == 0 => {}
                Kind::Name | Kind::Compound if used.is_single() => sensitive += 1,
                Kind::Name | Kind::Compound => return false,
            }
        }
        sensitive <= SENSITIVE_ARGUMENT_CAP
    }
}
