use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use oxc_traverse::Ancestor;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::readonly::{is_readonly_but_keyed, scan_symbol_uses};
use untwist_core::pass::Cx;
use untwist_core::semantic::is_constant_binding;

use super::identify::{resolved, Table};
use super::shapes::Kind;
use super::values::{into_inner, is_inert};

pub const STEP: &str = "object/define-chain/safe";
pub const RUN: &str = "object/define-chain-run/safe";

const INHERITED: [&str; 12] = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

#[derive(Default)]
pub struct Keys {
    primitives: FxHashSet<SymbolId>,
    temps: FxHashSet<SymbolId>,
    candidates: FxHashMap<Span, SymbolId>,
    proven: FxHashMap<Span, bool>,
    asking: Vec<Span>,
    armed: bool,
}

enum Verdict {
    Primitive,
    Unknown,
    Ask(Span),
}

impl Keys {
    pub fn begin_round(&mut self) {
        self.primitives.clear();
        self.temps.clear();
        self.candidates.clear();
        self.asking.clear();
    }

    pub fn arm(&mut self) {
        self.armed = true;
    }

    pub fn armed(&self) -> bool {
        self.armed
    }

    pub fn waiting(&self) -> bool {
        !self.asking.is_empty()
    }

    pub fn record(&mut self, declaration: &VariableDeclaration<'_>, scoping: &Scoping) {
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(symbol) = id.symbol_id.get() else {
                continue;
            };
            match &declarator.init {
                None if declaration.kind == VariableDeclarationKind::Var => {
                    self.temps.insert(symbol);
                }
                Some(init) if is_primitive_literal(init) => {
                    self.primitives.insert(symbol);
                }
                Some(Expression::ObjectExpression(object))
                    if is_constant_object(object) && is_constant_binding(scoping, symbol) =>
                {
                    self.candidates.insert(id.span, symbol);
                }
                _ => {}
            }
        }
    }

    pub fn prove(&mut self, program: &Program<'_>, scoping: &Scoping) -> bool {
        let unproven = |span: &Span| !self.proven.contains_key(span);
        let asked = self
            .asking
            .iter()
            .any(|span| unproven(span) && self.candidates.contains_key(span));
        if !asked {
            return false;
        }
        let pending: Vec<(Span, SymbolId)> = self
            .candidates
            .iter()
            .filter(|(span, _)| unproven(span))
            .map(|(span, symbol)| (*span, *symbol))
            .collect();
        let symbols: FxHashSet<SymbolId> = pending.iter().map(|(_, symbol)| *symbol).collect();
        let uses = scan_symbol_uses(program, scoping, &symbols);
        for (span, symbol) in pending {
            let holds = uses.of(symbol).is_some_and(is_readonly_but_keyed);
            self.proven.insert(span, holds);
        }
        self.asking
            .iter()
            .any(|span| self.proven.get(span) == Some(&true))
    }

    fn verdict(&self, key: &Expression<'_>, scoping: &Scoping, module: bool) -> Verdict {
        let key = key.without_parentheses();
        if is_primitive_literal(key) {
            return Verdict::Primitive;
        }
        match key {
            Expression::Identifier(_) => match resolved(key, scoping) {
                Some(symbol)
                    if self.primitives.contains(&symbol)
                        && is_constant_binding(scoping, symbol)
                        && !is_global_var(scoping, symbol, module) =>
                {
                    Verdict::Primitive
                }
                _ => Verdict::Unknown,
            },
            Expression::StaticMemberExpression(member) if !member.optional => self.member_verdict(
                &member.object,
                member.property.name.as_str(),
                scoping,
                module,
            ),
            _ => Verdict::Unknown,
        }
    }

    fn member_verdict(
        &self,
        object: &Expression<'_>,
        name: &str,
        scoping: &Scoping,
        module: bool,
    ) -> Verdict {
        if INHERITED.contains(&name) {
            return Verdict::Unknown;
        }
        let Some(symbol) = resolved(object, scoping) else {
            return Verdict::Unknown;
        };
        if !is_constant_binding(scoping, symbol) || is_global_var(scoping, symbol, module) {
            return Verdict::Unknown;
        }
        let span = scoping.symbol_span(symbol);
        match self.proven.get(&span) {
            None => Verdict::Ask(span),
            Some(true) => Verdict::Primitive,
            Some(false) => Verdict::Unknown,
        }
    }
}

fn is_proto_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name == "__proto__",
        PropertyKey::StringLiteral(literal) => literal.value == "__proto__",
        _ => false,
    }
}

fn is_constant_object(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|member| match member {
        ObjectPropertyKind::ObjectProperty(property) => {
            property.kind == PropertyKind::Init
                && !property.computed
                && !property.method
                && !property.shorthand
                && !is_proto_key(&property.key)
                && is_primitive_literal(&property.value)
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn is_global_var(scoping: &Scoping, symbol: SymbolId, module: bool) -> bool {
    !module
        && scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
        && scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::FunctionScopedVariable)
}

fn is_primitive_literal(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::Void
            ) && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

fn is_plain(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|member| match member {
        ObjectPropertyKind::ObjectProperty(property) => {
            property.kind == PropertyKind::Init
                && !property.method
                && (property.computed
                    || property.shorthand
                    || !property.key.is_specific_static_name("__proto__"))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn helper_parts<'k, 'a>(
    expression: &'k Expression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<[&'k Expression<'a>; 3]> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    if call.optional || call.arguments.len() != 3 || !is_definer(&call.callee, table, scoping) {
        return None;
    }
    let mut parts = call.arguments.iter().map(Argument::as_expression);
    match (parts.next()?, parts.next()?, parts.next()?) {
        (Some(base), Some(key), Some(value)) => Some([base, key, value]),
        _ => None,
    }
}

fn is_definer(callee: &Expression<'_>, table: &Table, scoping: &Scoping) -> bool {
    matches!(callee, Expression::Identifier(_))
        && matches!(
            table.kind_of_callee(callee, scoping),
            Some(Kind::DefineProperty | Kind::DefinePropertyPlain)
        )
}

pub fn may_be_site(node: &Expression<'_>, table: &Table, scoping: &Scoping) -> bool {
    match node {
        Expression::CallExpression(call) => {
            call.arguments.len() == 3 && is_definer(&call.callee, table, scoping)
        }
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .iter()
            .any(|expression| helper_parts(expression, table, scoping).is_some()),
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum Root {
    Fresh,
    Assigned(SymbolId),
    Read(SymbolId),
}

fn temp_of(target: &AssignmentTarget<'_>, scoping: &Scoping, keys: &Keys) -> Option<SymbolId> {
    let AssignmentTarget::AssignmentTargetIdentifier(id) = target else {
        return None;
    };
    let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
    (keys.temps.contains(&symbol) && scoping.symbol_redeclarations(symbol).is_empty())
        .then_some(symbol)
}

fn root_of(expression: &Expression<'_>, scoping: &Scoping, keys: &Keys) -> Option<Root> {
    match expression.without_parentheses() {
        Expression::ObjectExpression(object) => is_plain(object).then_some(Root::Fresh),
        Expression::AssignmentExpression(assignment)
            if assignment.operator == AssignmentOperator::Assign =>
        {
            let Expression::ObjectExpression(object) = &assignment.right else {
                return None;
            };
            let temp = temp_of(&assignment.left, scoping, keys)?;
            is_plain(object).then_some(Root::Assigned(temp))
        }
        Expression::Identifier(_) => {
            let symbol = resolved(expression, scoping)?;
            keys.temps.contains(&symbol).then_some(Root::Read(symbol))
        }
        _ => None,
    }
}

pub enum Outcome {
    Fired(&'static str),
    Ready,
    Kept,
}

struct Check<'s> {
    scoping: &'s Scoping,
    module: bool,
    inert: bool,
    asks: Vec<Span>,
}

impl Check<'_> {
    fn step(&mut self, key: &Expression<'_>, value: &Expression<'_>, keys: &Keys) -> bool {
        match keys.verdict(key, self.scoping, self.module) {
            Verdict::Primitive => {}
            Verdict::Unknown => return false,
            Verdict::Ask(span) => self.asks.push(span),
        }
        self.inert &= is_inert(value, self.scoping);
        true
    }

    fn settled(self, keys: &mut Keys) -> bool {
        if self.asks.is_empty() {
            return true;
        }
        for span in self.asks {
            if !keys.asking.contains(&span) {
                keys.asking.push(span);
            }
        }
        false
    }

    fn chain(
        &mut self,
        mut expression: &Expression<'_>,
        table: &Table,
        keys: &Keys,
    ) -> Option<(Root, usize)> {
        let mut steps = 0;
        while let Some([base, key, value]) = helper_parts(expression, table, self.scoping) {
            if !self.step(key, value, keys) {
                return None;
            }
            steps += 1;
            expression = base;
        }
        Some((root_of(expression, self.scoping, keys)?, steps))
    }

    fn drops(&self, temp: SymbolId, reads: usize) -> bool {
        self.scoping.get_resolved_reference_ids(temp).len() == 1 + reads
            && !is_global_var(self.scoping, temp, self.module)
    }
}

fn property<'a>(
    key: Expression<'a>,
    value: Expression<'a>,
    cx: &Cx<'a, '_>,
) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
        SPAN,
        PropertyKind::Init,
        PropertyKey::from(key),
        value,
        false,
        false,
        true,
        cx,
    )
}

pub(super) fn unnamed<'a>(value: Expression<'a>, cx: &Cx<'a, '_>) -> Expression<'a> {
    if !value.is_anonymous_function_definition() {
        return value;
    }
    let zero = Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, cx);
    Expression::new_sequence_expression(SPAN, AVec::from_iter_in([zero, value], cx), cx)
}

fn object_slot<'k, 'a>(root: &'k mut Expression<'a>) -> Option<&'k mut ObjectExpression<'a>> {
    match root {
        Expression::ObjectExpression(object) => Some(object),
        Expression::AssignmentExpression(assignment) => match &mut assignment.right {
            Expression::ObjectExpression(object) => Some(object),
            _ => None,
        },
        _ => None,
    }
}

fn unchain<'a>(
    mut expression: Expression<'a>,
    steps: usize,
    out: &mut Vec<(Expression<'a>, Expression<'a>)>,
) -> Option<Expression<'a>> {
    let from = out.len();
    for _ in 0..steps {
        let Expression::CallExpression(call) = into_inner(expression) else {
            return None;
        };
        let mut parts = call
            .unbox()
            .arguments
            .into_iter()
            .filter_map(|argument| Expression::try_from(argument).ok());
        let (base, key, value) = (parts.next()?, parts.next()?, parts.next()?);
        out.push((into_inner(key), value));
        expression = base;
    }
    out[from..].reverse();
    Some(into_inner(expression))
}

fn merged<'a>(
    mut root: Expression<'a>,
    steps: Vec<(Expression<'a>, Expression<'a>)>,
    drop: bool,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let object = object_slot(&mut root)?;
    object.properties.reserve(steps.len());
    for (key, value) in steps {
        let slot = literal_name(&key)
            .and_then(|name| collapse_slot(&object.properties, &name, &value, cx.scoping()));
        match slot.and_then(|at| object.properties.get_mut(at)) {
            Some(ObjectPropertyKind::ObjectProperty(earlier)) => earlier.value = unnamed(value, cx),
            _ => object
                .properties
                .push(property(key, unnamed(value, cx), cx)),
        }
    }
    match root {
        Expression::AssignmentExpression(assignment) if drop => Some(assignment.unbox().right),
        other => Some(other),
    }
}

const MAX_SAFE_INDEX: f64 = 9_007_199_254_740_991.0;

fn literal_name(key: &Expression<'_>) -> Option<String> {
    match key.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::NumericLiteral(literal)
            if literal.value.fract() == 0.0 && (0.0..=MAX_SAFE_INDEX).contains(&literal.value) =>
        {
            Some(format!("{}", literal.value as u64))
        }
        _ => None,
    }
}

fn property_name(property: &ObjectProperty<'_>) -> Option<String> {
    match &property.key {
        PropertyKey::StaticIdentifier(id) if !property.computed => Some(id.name.to_string()),
        key => key.as_expression().and_then(literal_name),
    }
}

fn collapse_slot(
    properties: &[ObjectPropertyKind<'_>],
    name: &str,
    value: &Expression<'_>,
    scoping: &Scoping,
) -> Option<usize> {
    if name == "__proto__" {
        return None;
    }
    let mut first = None;
    for (at, member) in properties.iter().enumerate() {
        let ObjectPropertyKind::ObjectProperty(property) = member else {
            return None;
        };
        let known = property_name(property)?;
        if known != name {
            continue;
        }
        if property.kind != PropertyKind::Init || property.method || property.shorthand {
            return None;
        }
        if first.replace(at).is_some() {
            return None;
        }
    }
    let first = first?;
    let ObjectPropertyKind::ObjectProperty(earlier) = properties.get(first)? else {
        return None;
    };
    let later_inert = properties[first + 1..].iter().all(|member| {
        matches!(member, ObjectPropertyKind::ObjectProperty(property) if is_inert(&property.value, scoping))
    });
    (is_inert(&earlier.value, scoping) && (later_inert || is_inert(value, scoping)))
        .then_some(first)
}

struct Plan {
    drop: bool,
    steps: Vec<usize>,
}

fn plan(
    slots: &[&Expression<'_>],
    table: &Table,
    keys: &Keys,
    check: &mut Check<'_>,
) -> Option<Plan> {
    let (first, rest) = slots.split_first()?;
    if rest.is_empty() {
        return None;
    }
    let (Root::Assigned(temp), first_steps) = check.chain(first, table, keys)? else {
        return None;
    };
    let mut steps = Vec::with_capacity(slots.len());
    steps.push(first_steps);
    for (at, slot) in rest.iter().enumerate() {
        let (root, count) = check.chain(slot, table, keys)?;
        let reads_temp = matches!(root, Root::Read(symbol) if symbol == temp);
        if !reads_temp || (count == 0 && at + 1 < rest.len()) {
            return None;
        }
        steps.push(count);
    }
    if steps.iter().all(|count| *count == 0) {
        return None;
    }
    let drop = check.drops(temp, rest.len());
    (drop || check.inert).then_some(Plan { drop, steps })
}

fn build<'a>(slots: Vec<Expression<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> Option<Expression<'a>> {
    let mut steps = Vec::with_capacity(plan.steps.iter().sum());
    let mut parts = slots.into_iter().zip(&plan.steps);
    let (first, count) = parts.next()?;
    let root = unchain(first, *count, &mut steps)?;
    for (slot, count) in parts {
        unchain(slot, *count, &mut steps)?;
    }
    merged(root, steps, plan.drop, cx)
}

fn starts_run(expression: &Expression<'_>, table: &Table, scoping: &Scoping, keys: &Keys) -> bool {
    matches!(
        root_of(chain_root(expression, table, scoping), scoping, keys),
        Some(Root::Assigned(_))
    )
}

pub fn in_slot(parent: Ancestor<'_, '_>) -> bool {
    matches!(
        parent,
        Ancestor::ExpressionStatementExpression(_)
            | Ancestor::ReturnStatementArgument(_)
            | Ancestor::VariableDeclaratorInit(_)
    )
}

fn is_run_slot(expression: &Expression<'_>, table: &Table, keys: &Keys, scoping: &Scoping) -> bool {
    helper_parts(expression, table, scoping).is_some()
        && matches!(
            root_of(chain_root(expression, table, scoping), scoping, keys),
            Some(Root::Read(_) | Root::Assigned(_))
        )
}

pub fn list_owner(cx: &Cx<'_, '_>) -> Option<(usize, bool)> {
    let total = cx.tcx.ancestors_depth();
    for (at, ancestor) in cx.tcx.ancestors().enumerate() {
        match ancestor {
            Ancestor::ProgramBody(_) => return Some((total - at, true)),
            Ancestor::FunctionBodyStatements(_)
            | Ancestor::BlockStatementBody(_)
            | Ancestor::SwitchCaseConsequent(_)
            | Ancestor::StaticBlockBody(_)
            | Ancestor::TSModuleBlockBody(_) => return Some((total - at, false)),
            _ => {}
        }
    }
    None
}

pub struct Site<'k> {
    pub table: &'k Table,
    pub module: bool,
    pub live: bool,
}

impl Site<'_> {
    pub fn rewrite<'a>(
        &self,
        node: &mut Expression<'a>,
        keys: &mut Keys,
        cx: &mut Cx<'a, '_>,
    ) -> Outcome {
        let outcome = match node {
            Expression::CallExpression(_) => self.step(node, keys, cx),
            Expression::SequenceExpression(_) => self.sequence(node, keys, cx),
            _ => None,
        };
        outcome.unwrap_or(Outcome::Kept)
    }

    pub fn runs_from(&self, node: &Expression<'_>, keys: &Keys, scoping: &Scoping) -> bool {
        is_run_slot(node, self.table, keys, scoping)
    }

    fn check<'s>(&self, scoping: &'s Scoping) -> Check<'s> {
        Check {
            scoping,
            module: self.module,
            inert: true,
            asks: Vec::new(),
        }
    }

    fn step<'a>(
        &self,
        node: &mut Expression<'a>,
        keys: &mut Keys,
        cx: &mut Cx<'a, '_>,
    ) -> Option<Outcome> {
        let cx = &*cx;
        let mut check = self.check(cx.scoping());
        let [base, key, value] = helper_parts(node, self.table, cx.scoping())?;
        if !check.step(key, value, keys) {
            return None;
        }
        let drop = match root_of(base, cx.scoping(), keys)? {
            Root::Fresh => false,
            Root::Assigned(temp) => {
                let drop = check.drops(temp, 0);
                if !drop && !check.inert {
                    return None;
                }
                drop
            }
            Root::Read(_) if self.live => return None,
            Root::Read(_) => false,
        };
        if !check.settled(keys) {
            return None;
        }
        if !self.live {
            return Some(Outcome::Ready);
        }
        let taken = std::mem::replace(node, Expression::new_null_literal(SPAN, cx));
        let mut steps = Vec::with_capacity(1);
        let root = unchain(taken, 1, &mut steps)?;
        *node = merged(root, steps, drop, cx)?;
        Some(Outcome::Fired(STEP))
    }

    fn sequence<'a>(
        &self,
        node: &mut Expression<'a>,
        keys: &mut Keys,
        cx: &mut Cx<'a, '_>,
    ) -> Option<Outcome> {
        let cx = &*cx;
        let Expression::SequenceExpression(sequence) = node else {
            return None;
        };
        let scoping = cx.scoping();
        let at = sequence
            .expressions
            .iter()
            .position(|expression| starts_run(expression, self.table, scoping, keys))?;
        let slots: Vec<&Expression<'_>> = sequence.expressions[at..].iter().collect();
        let mut check = self.check(scoping);
        let plan = plan(&slots, self.table, keys, &mut check)?;
        if !check.settled(keys) {
            return None;
        }
        if !self.live {
            return Some(Outcome::Ready);
        }
        let taken: Vec<Expression<'a>> = sequence.expressions.drain(at..).collect();
        let built = build(taken, &plan, cx)?;
        if at > 0 {
            sequence.expressions.push(built);
        } else {
            *node = built;
        }
        Some(Outcome::Fired(RUN))
    }

    pub fn statements<'a>(
        &self,
        statements: &mut AVec<'a, Statement<'a>>,
        keys: &mut Keys,
        cx: &Cx<'a, '_>,
    ) -> usize {
        let mut fired = 0;
        let mut at = 0;
        while at < statements.len() {
            if self.statement_run(statements, at, keys, cx).is_some() {
                fired += 1;
            }
            at += 1;
        }
        fired
    }

    fn statement_run<'a>(
        &self,
        statements: &mut AVec<'a, Statement<'a>>,
        at: usize,
        keys: &mut Keys,
        cx: &Cx<'a, '_>,
    ) -> Option<()> {
        let scoping = cx.scoping();
        let Statement::ExpressionStatement(first) = statements.get(at)? else {
            return None;
        };
        if !starts_run(&first.expression, self.table, scoping, keys) {
            return None;
        }
        let temp = assigned_root(&first.expression, self.table, scoping)?;
        let mut slots = vec![&first.expression];
        for statement in statements.iter().skip(at + 1) {
            let Some((slot, open)) = slot_of(statement) else {
                break;
            };
            if read_root(slot, self.table, scoping) != Some(temp) {
                break;
            }
            slots.push(slot);
            if !open {
                break;
            }
        }
        let mut check = self.check(scoping);
        let plan = plan(&slots, self.table, keys, &mut check)?;
        if !check.settled(keys) {
            return None;
        }
        let end = at + slots.len() - 1;
        let mut taken = Vec::with_capacity(slots.len());
        for statement in statements.iter_mut().take(end + 1).skip(at) {
            let slot = slot_mut(statement)?;
            taken.push(std::mem::replace(
                slot,
                Expression::new_null_literal(SPAN, cx),
            ));
        }
        let built = build(taken, &plan, cx)?;
        *slot_mut(statements.get_mut(end)?)? = built;
        statements.drain(at..end);
        Some(())
    }
}

fn chain_root<'k, 'a>(
    mut expression: &'k Expression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> &'k Expression<'a> {
    while let Some([base, _, _]) = helper_parts(expression, table, scoping) {
        expression = base;
    }
    expression.without_parentheses()
}

fn assigned_root(
    expression: &Expression<'_>,
    table: &Table,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let Expression::AssignmentExpression(assignment) = chain_root(expression, table, scoping)
    else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn read_root(expression: &Expression<'_>, table: &Table, scoping: &Scoping) -> Option<SymbolId> {
    resolved(chain_root(expression, table, scoping), scoping)
}

fn slot_of<'k, 'a>(statement: &'k Statement<'a>) -> Option<(&'k Expression<'a>, bool)> {
    match statement {
        Statement::ExpressionStatement(statement) => Some((&statement.expression, true)),
        Statement::ReturnStatement(statement) => Some((statement.argument.as_ref()?, false)),
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            Some((declarator.init.as_ref()?, false))
        }
        _ => None,
    }
}

fn slot_mut<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(statement) => Some(&mut statement.expression),
        Statement::ReturnStatement(statement) => statement.argument.as_mut(),
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_mut_slice() else {
                return None;
            };
            declarator.init.as_mut()
        }
        _ => None,
    }
}
