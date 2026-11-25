mod bits;
mod checksum;
mod eval;
mod inert;
mod value;

#[cfg(test)]
mod tests;

use oxc_ast::ast::{Expression, IdentifierReference, Program, Statement, VariableDeclarationKind};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::pass::Cx;
use untwist_core::semantic::builtins::{patched_by_another_input, scan, Reliance};
use untwist_core::semantic::{has_module_syntax, may_call_eval, other_inputs_given};

use crate::init_order::{read_before_declaration, read_during_initialiser};
pub use bits::Bits;
use eval::{abstract_value, concrete, low_determined, Leaves};
pub use value::{Kind, Val};

const ENUMERATED_BITS: u32 = 12;

static MATH_IMUL: Reliance = Reliance {
    roots: &["Math"],
    members: &[("Math", false, &["imul"])],
};

#[derive(Default)]
pub struct KnownBits {
    approved: FxHashMap<SymbolId, Val>,
    imul: bool,
    wanted: bool,
    barren: bool,
}

impl KnownBits {
    pub fn learn(
        &mut self,
        program: &Program<'_>,
        cx: &Cx<'_, '_>,
        has_with: impl FnOnce() -> bool,
    ) {
        self.approved.clear();
        self.imul = false;
        if !self.wanted || self.barren {
            return;
        }
        if has_with() || may_call_eval(cx.scoping()) {
            self.barren = true;
            return;
        }
        let scoping = cx.scoping();
        let computed = seeds(program, scoping, cx.opts.unsafe_transforms);
        if computed.is_empty() {
            self.barren = true;
            return;
        }
        let bound: Vec<(Span, SymbolId)> = computed
            .iter()
            .map(|(symbol, seed)| (seed.statement, *symbol))
            .collect();
        let early = read_before_declaration(program, scoping, &bound);
        let during: Vec<(Span, SymbolId, bool)> = computed
            .iter()
            .map(|(symbol, seed)| (seed.statement, *symbol, seed.inert))
            .collect();
        let during = read_during_initialiser(program, scoping, &during);
        self.approved = computed
            .into_iter()
            .filter(|(symbol, _)| !early.contains(symbol) && !during.contains(symbol))
            .map(|(symbol, seed)| (symbol, seed.value))
            .collect();
        self.imul = !self.approved.is_empty() && imul_holds(program, cx);
        cx.walk_whole_round();
    }

    pub fn decide(&mut self, expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<bool> {
        if self.approved.is_empty() {
            if !self.wanted && !self.barren && is_candidate(expr, cx.scoping()) {
                self.wanted = true;
                cx.opts.round_requested.set(true);
            }
            return None;
        }
        let leaves = Strict {
            approved: &self.approved,
            scoping: cx.scoping(),
            imul: self.imul,
        };
        decide_with(expr, &leaves)
    }
}

fn imul_holds(program: &Program<'_>, cx: &Cx<'_, '_>) -> bool {
    if patched_by_another_input(&MATH_IMUL) {
        return false;
    }
    let module = has_module_syntax(program);
    scan(program, cx.scoping(), module, false, &MATH_IMUL)
        .verdict
        .allows_safe(cx.opts.unsafe_transforms)
}

fn decide_with(expr: &Expression<'_>, leaves: &dyn Leaves) -> Option<bool> {
    match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            decide_with(&unary.argument, leaves).map(|value| !value)
        }
        Expression::BinaryExpression(binary) => {
            let negate = match binary.operator {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => false,
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => true,
                _ => return abstract_value(expr, leaves)?.truthy(),
            };
            equality(&binary.left, &binary.right, leaves).map(|equal| equal != negate)
        }
        other => abstract_value(other, leaves)?.truthy(),
    }
}

fn equality(left: &Expression<'_>, right: &Expression<'_>, leaves: &dyn Leaves) -> Option<bool> {
    let left_value = abstract_value(left, leaves)?;
    let right_value = abstract_value(right, leaves)?;
    if let Some(decided) = left_value.equals(right_value) {
        return Some(decided);
    }
    enumerate([(left, left_value), (right, right_value)], leaves)
}

fn enumerate(sides: [(&Expression<'_>, Val); 2], leaves: &dyn Leaves) -> Option<bool> {
    let width = sides
        .iter()
        .map(|(_, value)| 32 - value.bits.unknown().leading_zeros())
        .max()?;
    let window = bits::low_mask(width);
    let mut seen = Vec::new();
    if !sides
        .iter()
        .all(|(expr, _)| low_determined(expr, leaves, &mut seen))
    {
        return None;
    }
    let free: Vec<u32> = seen
        .iter()
        .map(|(_, known)| known.unknown() & window)
        .collect();
    let count: u32 = free.iter().map(|mask| mask.count_ones()).sum();
    if count > ENUMERATED_BITS {
        return None;
    }
    let mut outcome = None;
    for assignment in 0..(1u32 << count) {
        let env = |symbol: SymbolId| {
            let index = seen
                .iter()
                .position(|(candidate, _)| *candidate == symbol)?;
            let offset: u32 = free[..index].iter().map(|mask| mask.count_ones()).sum();
            Some(seen[index].1.ones | scatter(assignment >> offset, free[index]))
        };
        let [left, right] = sides.map(|(expr, value)| {
            let low = concrete(expr, leaves, &env)?;
            let full = (value.bits.ones & !window) | (low & window);
            value.bits.admits(full).then(|| Val {
                bits: Bits::exact(full),
                kind: value.kind,
            })
        });
        let decided = left?.equals(right?)?;
        if outcome.is_some_and(|previous| previous != decided) {
            return None;
        }
        outcome = Some(decided);
    }
    outcome
}

fn scatter(mut source: u32, mask: u32) -> u32 {
    let mut out = 0;
    let mut remaining = mask;
    while remaining != 0 {
        let bit = remaining.isolate_lowest_one();
        if source & 1 == 1 {
            out |= bit;
        }
        source >>= 1;
        remaining &= remaining - 1;
    }
    out
}

pub(super) fn resolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn is_global_math_imul(callee: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
        return false;
    };
    if member.optional || member.property.name != "imul" {
        return false;
    }
    let Expression::Identifier(object) = &member.object else {
        return false;
    };
    object.name == "Math"
        && object.reference_id.get().is_some()
        && resolved(object, scoping).is_none()
}

struct Strict<'k, 's> {
    approved: &'k FxHashMap<SymbolId, Val>,
    scoping: &'s Scoping,
    imul: bool,
}

impl Leaves for Strict<'_, '_> {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)> {
        let symbol = resolved(id, self.scoping)?;
        self.approved.get(&symbol).map(|value| (symbol, *value))
    }

    fn is_imul(&self, callee: &Expression<'_>) -> bool {
        self.imul && is_global_math_imul(callee, self.scoping)
    }

    fn lax(&self) -> bool {
        false
    }
}

struct Seed {
    statement: Span,
    value: Val,
    inert: bool,
}

struct Lax<'k, 's, 'r, 'p> {
    computed: &'k FxHashMap<SymbolId, Seed>,
    scoping: &'s Scoping,
    roots: &'r checksum::Roots<'p>,
    seed: Span,
}

impl Leaves for Lax<'_, '_, '_, '_> {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)> {
        let symbol = resolved(id, self.scoping)?;
        self.computed.get(&symbol).map(|seed| (symbol, seed.value))
    }

    fn is_imul(&self, _callee: &Expression<'_>) -> bool {
        false
    }

    fn lax(&self) -> bool {
        true
    }

    fn decided(&self, test: &Expression<'_>) -> Option<bool> {
        self.roots.decide(test, self.seed, self.scoping)
    }
}

fn seeds<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    unsafe_lane: bool,
) -> FxHashMap<SymbolId, Seed> {
    let roots = checksum::Roots::of(program, unsafe_lane);
    let alone = !other_inputs_given();
    let mut computed = FxHashMap::default();
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        if declaration.kind != VariableDeclarationKind::Const {
            continue;
        }
        for declarator in &declaration.declarations {
            let (Some(id), Some(init)) = (declarator.id.get_binding_identifier(), &declarator.init)
            else {
                continue;
            };
            let Some(symbol) = id.symbol_id.get() else {
                continue;
            };
            if !is_seed_shape(init) || written(symbol, scoping) {
                continue;
            }
            let lax = Lax {
                computed: &computed,
                scoping,
                roots: &roots,
                seed: declaration.span,
            };
            if let Some(value) = abstract_value(init, &lax).filter(|value| value.bits.known() != 0)
            {
                let inert = alone
                    && declaration.declarations.iter().all(|declarator| {
                        declarator
                            .init
                            .as_ref()
                            .is_some_and(|init| inert::runs_no_code(init, &lax))
                    });
                computed.insert(
                    symbol,
                    Seed {
                        statement: declaration.span,
                        value,
                        inert,
                    },
                );
            }
        }
    }
    computed
}

fn written(symbol: SymbolId, scoping: &Scoping) -> bool {
    !scoping.symbol_flags(symbol).is_const_variable()
        || scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
}

fn is_seed_shape(init: &Expression<'_>) -> bool {
    match init.without_parentheses() {
        Expression::BinaryExpression(binary) => matches!(
            binary.operator,
            BinaryOperator::BitwiseAnd
                | BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::ShiftRightZeroFill
        ),
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::BitwiseNot,
        _ => false,
    }
}

fn is_candidate(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            is_candidate(&unary.argument, scoping)
        }
        Expression::BinaryExpression(binary) => {
            let mut reads = Vec::new();
            let imul = const_reads(&binary.left, scoping, 0, &mut reads)
                | const_reads(&binary.right, scoping, 0, &mut reads);
            imul || reads
                .iter()
                .enumerate()
                .any(|(index, symbol)| reads[..index].contains(symbol))
        }
        _ => false,
    }
}

fn const_reads(
    expr: &Expression<'_>,
    scoping: &Scoping,
    depth: u32,
    reads: &mut Vec<SymbolId>,
) -> bool {
    if depth > 8 {
        return false;
    }
    match expr.without_parentheses() {
        Expression::Identifier(id) => {
            if let Some(symbol) = resolved(id, scoping)
                .filter(|symbol| depth > 0 && scoping.symbol_flags(*symbol).is_const_variable())
            {
                reads.push(symbol);
            }
            false
        }
        Expression::BinaryExpression(binary) if is_int_operator(binary.operator) => {
            const_reads(&binary.left, scoping, depth + 1, reads)
                | const_reads(&binary.right, scoping, depth + 1, reads)
        }
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::BitwiseNot | UnaryOperator::UnaryNegation
            ) =>
        {
            const_reads(&unary.argument, scoping, depth + 1, reads)
        }
        Expression::CallExpression(call) if is_global_math_imul(&call.callee, scoping) => {
            let before = reads.len();
            for argument in call
                .arguments
                .iter()
                .filter_map(|argument| argument.as_expression())
            {
                const_reads(argument, scoping, depth + 1, reads);
            }
            reads.len() > before
        }
        _ => false,
    }
}

fn is_int_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
            | BinaryOperator::Addition
            | BinaryOperator::Subtraction
    )
}
