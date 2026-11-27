use oxc_allocator::TakeIn;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};
use untwist_core::semantic::builtins::{intact, patched_by_another_input, Reliance};
use untwist_core::semantic::has_module_syntax;

static RELIED_ON: Reliance = Reliance {
    roots: &["Math"],
    members: &[],
};

const MATH_CONSTANTS: [&str; 8] = [
    "E", "LN10", "LN2", "LOG10E", "LOG2E", "PI", "SQRT1_2", "SQRT2",
];
const NUMERIC_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];
const REALM_NAMES: [&str; 14] = [
    "globalThis",
    "window",
    "self",
    "global",
    "top",
    "parent",
    "frames",
    "eval",
    "Function",
    "Reflect",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "execScript",
];

#[derive(Default)]
pub struct Exponentiation {
    unpatched: Option<bool>,
    aliases: FxHashSet<SymbolId>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Exponentiation::default())
}

fn unresolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> bool {
    id.reference_id
        .get()
        .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

fn global(expression: &Expression<'_>, name: &str, scoping: &Scoping) -> bool {
    matches!(expression.without_parentheses(), Expression::Identifier(id)
        if id.name.as_str() == name && unresolved(id, scoping))
}

fn math_member<'k>(expression: &'k Expression<'_>, scoping: &Scoping) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StaticMemberExpression(member)
            if !member.optional && global(&member.object, "Math", scoping) =>
        {
            Some(member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(member)
            if !member.optional && global(&member.object, "Math", scoping) =>
        {
            match &member.expression {
                Expression::StringLiteral(name) => name.value.as_str(),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_math_pow(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    math_member(expression, scoping) == Some("pow")
}

fn symbol_of(expression: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(id) = expression.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn always_numeric(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterEqualThan
            | BinaryOperator::In
            | BinaryOperator::Instanceof
            | BinaryOperator::ShiftRightZeroFill
    )
}

struct Classify<'s> {
    scoping: &'s Scoping,
    aliases: &'s FxHashSet<SymbolId>,
}

impl Classify<'_> {
    fn math_call(&self, call: &CallExpression<'_>) -> bool {
        !call.optional
            && (math_member(&call.callee, self.scoping).is_some()
                || symbol_of(&call.callee, self.scoping)
                    .is_some_and(|symbol| self.aliases.contains(&symbol)))
    }

    fn numeric(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => {
                NUMERIC_GLOBALS.contains(&id.name.as_str()) && unresolved(id, self.scoping)
            }
            Expression::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot => {
                    self.numeric(&unary.argument)
                }
                _ => true,
            },
            Expression::BinaryExpression(binary) => {
                always_numeric(binary.operator)
                    || self.numeric(&binary.left)
                    || self.numeric(&binary.right)
            }
            Expression::LogicalExpression(logical) => {
                self.numeric(&logical.left) && self.numeric(&logical.right)
            }
            Expression::ConditionalExpression(conditional) => {
                self.numeric(&conditional.consequent) && self.numeric(&conditional.alternate)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .is_some_and(|last| self.numeric(last)),
            Expression::AssignmentExpression(assign) => {
                let plain = assign.operator == AssignmentOperator::Assign
                    || assign.operator.is_arithmetic()
                    || assign.operator.is_bitwise();
                plain && self.numeric(&assign.right)
            }
            Expression::CallExpression(call) => self.math_call(call),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                math_member(expression, self.scoping)
                    .is_some_and(|name| MATH_CONSTANTS.contains(&name))
            }
            _ => false,
        }
    }
}

fn quiet_init(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::Identifier(id) => unresolved(id, scoping),
        Expression::StaticMemberExpression(member) => {
            !member.optional && quiet_init(&member.object, scoping)
        }
        _ => false,
    }
}

fn pow_declarator(declarator: &VariableDeclarator<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let init = declarator.init.as_ref()?;
    is_math_pow(init, scoping)
        .then(|| id.symbol_id.get())
        .flatten()
}

fn imports(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportFromDeclaration(_)
    )
}

struct Aliases<'s> {
    scoping: &'s Scoping,
    module: bool,
    found: FxHashSet<SymbolId>,
    calls: FxHashMap<SymbolId, usize>,
}

impl Aliases<'_> {
    fn leading(&mut self, statements: &[Statement<'_>]) {
        for statement in statements {
            match statement {
                Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => {}
                Statement::VariableDeclaration(declaration) if !declaration.declare => {
                    for declarator in &declaration.declarations {
                        if let Some(symbol) = pow_declarator(declarator, self.scoping) {
                            self.found.insert(symbol);
                        } else if !matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                            || !declarator
                                .init
                                .as_ref()
                                .is_none_or(|init| quiet_init(init, self.scoping))
                        {
                            return;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn closed(&self, symbol: SymbolId) -> bool {
        let scoping = self.scoping;
        let references = scoping.get_resolved_reference_ids(symbol);
        scoping.symbol_redeclarations(symbol).is_empty()
            && references
                .iter()
                .all(|reference| !scoping.get_reference(*reference).is_write())
            && self.calls.get(&symbol).copied().unwrap_or(0) == references.len()
    }
}

impl<'a> Visit<'a> for Aliases<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        if self.module && !it.body.iter().any(imports) {
            self.leading(&it.body);
        }
        walk::walk_program(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(body) = it.body.as_ref() {
            self.leading(&body.statements);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if let ArrowFunctionBody::FunctionBody(body) = &it.body {
            self.leading(&body.statements);
        }
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let pair = it.arguments.len() == 2 && it.arguments.iter().all(|a| !a.is_spread());
        if let Some(symbol) = symbol_of(&it.callee, self.scoping).filter(|_| pair && !it.optional) {
            *self.calls.entry(symbol).or_default() += 1;
        }
        walk::walk_call_expression(self, it);
    }
}

fn reaches_the_realm(scoping: &Scoping) -> bool {
    scoping
        .root_unresolved_references()
        .keys()
        .any(|name| REALM_NAMES.contains(&name.as_str()))
}

fn may_alias(text: &str) -> bool {
    text.match_indices("Math.pow").any(|(at, found)| {
        let rest = text[at + found.len()..].trim_start();
        !rest.starts_with('(')
    })
}

fn find_aliases(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    if !may_alias(program.source_text) {
        return FxHashSet::default();
    }
    let mut aliases = Aliases {
        scoping,
        module: has_module_syntax(program),
        found: FxHashSet::default(),
        calls: FxHashMap::default(),
    };
    aliases.visit_program(program);
    let found = std::mem::take(&mut aliases.found);
    found
        .into_iter()
        .filter(|symbol| aliases.closed(*symbol))
        .collect()
}

fn same_target(target: &AssignmentTarget<'_>, read: &Expression<'_>, scoping: &Scoping) -> bool {
    match (target, read.without_parentheses()) {
        (AssignmentTarget::AssignmentTargetIdentifier(left), Expression::Identifier(right)) => {
            left.name == right.name
                && match (left.reference_id.get(), right.reference_id.get()) {
                    (Some(a), Some(b)) => {
                        scoping.get_reference(a).symbol_id() == scoping.get_reference(b).symbol_id()
                    }
                    _ => false,
                }
        }
        (
            AssignmentTarget::StaticMemberExpression(left),
            Expression::StaticMemberExpression(right),
        ) => {
            !left.optional
                && !right.optional
                && left.property.name == right.property.name
                && matches!(
                    (left.object.without_parentheses(), right.object.without_parentheses()),
                    (Expression::Identifier(a), Expression::Identifier(b))
                        if a.name == b.name
                            && match (a.reference_id.get(), b.reference_id.get()) {
                                (Some(x), Some(y)) => scoping.get_reference(x).symbol_id()
                                    == scoping.get_reference(y).symbol_id(),
                                _ => false,
                            }
                )
        }
        _ => false,
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::CallExpression,
    ExpressionKind::AssignmentExpression,
]);

impl Exponentiation {
    fn holds(&self, cx: &Cx<'_, '_>) -> bool {
        cx.opts.unsafe_transforms || self.unpatched == Some(true)
    }

    fn rewrite_call<'a>(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Expression::CallExpression(call) = &*node else {
            return false;
        };
        let scoping = cx.scoping();
        let callee = is_math_pow(&call.callee, scoping)
            || symbol_of(&call.callee, scoping).is_some_and(|s| self.aliases.contains(&s));
        let classify = Classify {
            scoping,
            aliases: &self.aliases,
        };
        let operands = match call.arguments.as_slice() {
            [base, exponent] => base.as_expression().zip(exponent.as_expression()),
            _ => None,
        };
        let Some((base, exponent)) = operands else {
            return false;
        };
        let numeric = classify.numeric(base) || classify.numeric(exponent);
        if call.optional || !callee || !numeric || !self.holds(cx) {
            return false;
        }
        let Expression::CallExpression(call) = node else {
            return false;
        };
        let span = call.span;
        let mut arguments = call.arguments.take_in(&*cx).into_iter();
        let (Some(base), Some(exponent)) = (arguments.next(), arguments.next()) else {
            return false;
        };
        let (Ok(base), Ok(exponent)) = (Expression::try_from(base), Expression::try_from(exponent))
        else {
            return false;
        };
        *node = Expression::new_binary_expression(
            span,
            base,
            BinaryOperator::Exponential,
            exponent,
            &*cx,
        );
        true
    }

    fn compound<'a>(node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Expression::AssignmentExpression(assign) = node else {
            return false;
        };
        if assign.operator != AssignmentOperator::Assign {
            return false;
        }
        let Expression::BinaryExpression(binary) = &assign.right else {
            return false;
        };
        if binary.operator != BinaryOperator::Exponential
            || !same_target(&assign.left, &binary.left, cx.scoping())
        {
            return false;
        }
        let Expression::BinaryExpression(binary) = assign.right.take_in(&*cx) else {
            return false;
        };
        assign.right = binary.unbox().right;
        assign.operator = AssignmentOperator::Exponential;
        true
    }
}

impl<'a> Pass<'a> for Exponentiation {
    fn name(&self) -> &'static str {
        "exponentiation"
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION).on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let asks = !cx.opts.unsafe_transforms && program.source_text.contains("pow");
        if self.unpatched.is_none() && asks {
            let scoping = cx.scoping();
            let module = has_module_syntax(program);
            let proven = !reaches_the_realm(scoping)
                && !patched_by_another_input(&RELIED_ON)
                && intact(program, scoping, module, &RELIED_ON);
            self.unpatched = Some(proven);
        }
        self.aliases = find_aliases(program, cx.scoping());
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let changed = match node {
            Expression::CallExpression(_) => self.rewrite_call(node, cx),
            Expression::AssignmentExpression(_) => Self::compound(node, cx),
            _ => false,
        };
        if changed {
            cx.changed();
        }
    }
}
