use oxc_ast::ast::{
    BindingPattern, BlockStatement, Expression, Function, FunctionBody, IdentifierReference,
    Program, Statement, StaticBlock, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::ContentEq;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::resolved;

const MAX_DEPTH: u32 = 32;

pub(super) struct Relation<'e, 'a> {
    or_side: [&'e Expression<'a>; 2],
    holds: bool,
}

pub(super) fn relation<'e, 'a>(expr: &'e Expression<'a>) -> Option<Relation<'e, 'a>> {
    let Expression::BinaryExpression(binary) = expr.without_parentheses() else {
        return None;
    };
    let left = unsigned_pair(&binary.left)?;
    let right = unsigned_pair(&binary.right)?;
    let (or_on_left, or_side, xor_side) = match (left, right) {
        ((BinaryOperator::BitwiseOR, or), (BinaryOperator::BitwiseXOR, xor)) => (true, or, xor),
        ((BinaryOperator::BitwiseXOR, xor), (BinaryOperator::BitwiseOR, or)) => (false, or, xor),
        _ => return None,
    };
    let same = (or_side[0].content_eq(xor_side[0]) && or_side[1].content_eq(xor_side[1]))
        || (or_side[0].content_eq(xor_side[1]) && or_side[1].content_eq(xor_side[0]));
    if !same {
        return None;
    }
    let holds = match (binary.operator, or_on_left) {
        (BinaryOperator::LessThan, true) | (BinaryOperator::GreaterThan, false) => false,
        (BinaryOperator::GreaterEqualThan, true) | (BinaryOperator::LessEqualThan, false) => true,
        _ => return None,
    };
    Some(Relation { or_side, holds })
}

fn unsigned_pair<'e, 'a>(
    expr: &'e Expression<'a>,
) -> Option<(BinaryOperator, [&'e Expression<'a>; 2])> {
    let Expression::BinaryExpression(shift) = expr.without_parentheses() else {
        return None;
    };
    let zero = matches!(
        shift.right.without_parentheses(),
        Expression::NumericLiteral(literal) if literal.value == 0.0
    );
    if shift.operator != BinaryOperator::ShiftRightZeroFill || !zero {
        return None;
    }
    let Expression::BinaryExpression(inner) = shift.left.without_parentheses() else {
        return None;
    };
    matches!(
        inner.operator,
        BinaryOperator::BitwiseOR | BinaryOperator::BitwiseXOR
    )
    .then_some((inner.operator, [&inner.left, &inner.right]))
}

impl Relation<'_, '_> {
    pub(super) fn decide(&self, plain: &PlainSymbols, scoping: &Scoping) -> Option<bool> {
        self.or_side
            .iter()
            .all(|side| pure(side, &plain.symbols, scoping, 0))
            .then_some(self.holds)
    }
}

fn pure(expr: &Expression<'_>, plain: &FxHashSet<SymbolId>, scoping: &Scoping, depth: u32) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let depth = depth + 1;
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::Identifier(id) => {
            resolved(id, scoping).is_some_and(|symbol| plain.contains(&symbol))
        }
        Expression::BinaryExpression(binary) => {
            !matches!(
                binary.operator,
                BinaryOperator::In | BinaryOperator::Instanceof
            ) && pure(&binary.left, plain, scoping, depth)
                && pure(&binary.right, plain, scoping, depth)
        }
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation
                    | UnaryOperator::UnaryPlus
                    | UnaryOperator::BitwiseNot
                    | UnaryOperator::LogicalNot
            ) && pure(&unary.argument, plain, scoping, depth)
        }
        _ => false,
    }
}

#[derive(Default)]
pub(super) struct PlainSymbols {
    pub(super) symbols: FxHashSet<SymbolId>,
    pub(super) flags: FxHashMap<SymbolId, bool>,
}

pub(super) fn plain_symbols(program: &Program<'_>, scoping: &Scoping, local: bool) -> PlainSymbols {
    let mut scan = Plain {
        scoping,
        local,
        found: FxHashSet::default(),
        consts: FxHashMap::default(),
        flags: FxHashMap::default(),
        hoisted: FxHashSet::default(),
    };
    scan.statements(&program.body);
    scan.visit_program(program);
    let Plain {
        mut found,
        consts,
        mut flags,
        ..
    } = scan;
    for (symbol, (_, checked)) in consts {
        if checked != usize::MAX && scoping.get_resolved_references(symbol).count() == checked {
            found.insert(symbol);
        } else {
            flags.remove(&symbol);
        }
    }
    PlainSymbols {
        symbols: found,
        flags,
    }
}

impl PlainSymbols {
    pub(super) fn clear(&mut self) {
        self.symbols.clear();
        self.flags.clear();
    }

    pub(super) fn flag(&self, expr: &Expression<'_>, scoping: &Scoping) -> Option<bool> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.flags.get(&resolved(id, scoping)?).copied(),
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                self.flag(&unary.argument, scoping).map(|value| !value)
            }
            _ => None,
        }
    }
}

struct Plain<'s> {
    scoping: &'s Scoping,
    local: bool,
    found: FxHashSet<SymbolId>,
    consts: FxHashMap<SymbolId, (u32, usize)>,
    flags: FxHashMap<SymbolId, bool>,
    hoisted: FxHashSet<ScopeId>,
}

impl Plain<'_> {
    fn statements(&mut self, statements: &[Statement<'_>]) {
        for statement in statements {
            let Statement::VariableDeclaration(declaration) = statement else {
                continue;
            };
            if declaration.kind != VariableDeclarationKind::Const {
                continue;
            }
            for declarator in &declaration.declarations {
                let (BindingPattern::BindingIdentifier(id), Some(init)) =
                    (&declarator.id, &declarator.init)
                else {
                    continue;
                };
                if let Some(symbol) = id
                    .symbol_id
                    .get()
                    .filter(|_| plain_typed(init, self.scoping, 0))
                {
                    self.consts
                        .entry(symbol)
                        .or_insert((declaration.span.end, 0));
                    if let Expression::BooleanLiteral(literal) = init.without_parentheses() {
                        self.flags.insert(symbol, literal.value);
                    }
                }
            }
        }
    }

    fn var_is_plain(&self, declaration: &VariableDeclaration<'_>) -> Vec<SymbolId> {
        let mut plain = Vec::new();
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(symbol) = id.symbol_id.get() else {
                continue;
            };
            let typed = declarator
                .init
                .as_ref()
                .is_none_or(|init| plain_typed(init, self.scoping, 0));
            let root = self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id();
            let fixed = self.scoping.symbol_redeclarations(symbol).is_empty()
                && !self
                    .scoping
                    .get_resolved_references(symbol)
                    .any(|reference| reference.is_write());
            if typed && fixed && (self.local || !root) {
                plain.push(symbol);
            }
        }
        plain
    }

    fn read_follows(&self, id: &IdentifierReference<'_>, symbol: SymbolId, end: u32) -> bool {
        let Some(reference) = id.reference_id.get() else {
            return false;
        };
        if id.span.start < end {
            return false;
        }
        let home = self.scoping.symbol_scope_id(symbol);
        let from: ScopeId = self.scoping.get_reference(reference).scope_id();
        for scope in self.scoping.scope_ancestors(from) {
            if scope == home {
                return true;
            }
            if self.hoisted.contains(&scope) {
                return false;
            }
        }
        false
    }
}

impl<'a> VisitJs<'a> for Plain<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() {
            if let Some(scope) = it.scope_id.get() {
                self.hoisted.insert(scope);
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.statements(&it.statements);
        walk_js::walk_function_body(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.statements(&it.body);
        walk_js::walk_block_statement(self, it);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.statements(&it.body);
        walk_js::walk_static_block(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            let plain = self.var_is_plain(it);
            self.found.extend(plain);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = resolved(it, self.scoping) else {
            return;
        };
        let Some(&(end, checked)) = self.consts.get(&symbol) else {
            return;
        };
        if checked == usize::MAX {
            return;
        }
        let next = if self.read_follows(it, symbol, end) {
            checked + 1
        } else {
            usize::MAX
        };
        self.consts.insert(symbol, (end, next));
    }
}

fn number_typed(expr: &Expression<'_>, scoping: &Scoping, depth: u32) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let depth = depth + 1;
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::UnaryPlus => true,
            UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot => {
                number_typed(&unary.argument, scoping, depth)
            }
            _ => false,
        },
        Expression::BinaryExpression(binary) => match binary.operator {
            BinaryOperator::ShiftRightZeroFill => true,
            BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Exponential => {
                number_typed(&binary.left, scoping, depth)
                    || number_typed(&binary.right, scoping, depth)
            }
            _ => false,
        },
        Expression::ConditionalExpression(conditional) => {
            (number_typed(&conditional.consequent, scoping, depth)
                || guards_number(&conditional.test, &conditional.consequent, scoping))
                && number_typed(&conditional.alternate, scoping, depth)
        }
        _ => false,
    }
}

fn guards_number(test: &Expression<'_>, consequent: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return false;
    };
    if binary.operator != BinaryOperator::StrictEquality {
        return false;
    }
    let (Expression::UnaryExpression(unary), Expression::StringLiteral(kind)) = (
        binary.left.without_parentheses(),
        binary.right.without_parentheses(),
    ) else {
        return false;
    };
    let (Expression::Identifier(checked), Expression::Identifier(chosen)) = (
        unary.argument.without_parentheses(),
        consequent.without_parentheses(),
    ) else {
        return false;
    };
    unary.operator == UnaryOperator::Typeof
        && kind.value == "number"
        && checked.name == chosen.name
        && resolved(checked, scoping).is_some()
}

fn plain_typed(expr: &Expression<'_>, scoping: &Scoping, depth: u32) -> bool {
    if number_typed(expr, scoping, depth) {
        return true;
    }
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => matches!(
            unary.operator,
            UnaryOperator::LogicalNot | UnaryOperator::Typeof | UnaryOperator::Void
        ),
        Expression::BinaryExpression(binary) => {
            binary.operator.is_equality()
                || binary.operator.is_compare()
                || matches!(
                    binary.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                )
        }
        Expression::ConditionalExpression(conditional) => {
            depth < MAX_DEPTH
                && plain_typed(&conditional.consequent, scoping, depth + 1)
                && plain_typed(&conditional.alternate, scoping, depth + 1)
        }
        _ => false,
    }
}
