use std::rc::Rc;

use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::bail::Bail;
use crate::builtins::WIDE_SCOPE_BINDINGS;
use crate::compile::expr::binary_value;
use crate::compile::lexical::{pending, progressive};
use crate::compile::scope::{checksum_layout, PreludeHoist, Provable, Resolved, ScopeChain};
use crate::compile::{checked, depth, eligible, Compiler, Lazy, LazyExpr, Reach};
use crate::decode::Checksum;
use crate::env::{assign, declare, has_binding, read_binding, Assigned};
use crate::frame::{Frame, FrameLayout, FrameRef};
use crate::interp::Interp;
use crate::names::Names;
use crate::ops::{binary_op, unary_op};
use crate::stack;
use crate::value::{js_string_from_literal, Eval, FnCode, JsString, Value};

pub(crate) struct CompiledChecksum<'a> {
    layout: Rc<FrameLayout>,
    array_slot: u32,
    expected_slot: u32,
    array_argument: Term<'a>,
    expected: Term<'a>,
    hoists: Box<[PreludeHoist<'a>]>,
    prelude: Box<[Lazy<'a>]>,
    terms: Box<[Term<'a>]>,
}

pub(crate) enum Term<'a> {
    Direct(TermSource<'a>),
    Compiled(LazyExpr<'a>),
}

pub(crate) struct TermSource<'a> {
    node: &'a Expression<'a>,
    strings: Box<[JsString]>,
    resolved: Box<[Resolved]>,
}

impl<'a> TermSource<'a> {
    pub(crate) fn new(
        node: &'a Expression<'a>,
        names: &mut Names<'a>,
        chain: Option<&Rc<ScopeChain>>,
    ) -> Self {
        let mut found = Prebuilt::sized(node, chain.is_some());
        prebuild(node, (names, chain), &mut found);
        TermSource {
            node,
            strings: found.strings.into_boxed_slice(),
            resolved: found.resolved.into_boxed_slice(),
        }
    }

    pub(crate) fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
    ) -> Eval<'a, Value<'a>> {
        let mut prebuilt = Cursor {
            strings: self.strings.iter(),
            resolved: self.resolved.iter(),
        };
        term_node(interp, frame, self.node, &mut prebuilt)
    }
}

struct Prebuilt {
    strings: Vec<JsString>,
    resolved: Vec<Resolved>,
}

impl Prebuilt {
    fn sized(node: &Expression<'_>, resolving: bool) -> Self {
        let (strings, reads) = leaf_counts(node);
        Prebuilt {
            strings: Vec::with_capacity(strings),
            resolved: Vec::with_capacity(if resolving { reads } else { 0 }),
        }
    }
}

fn leaf_counts(expr: &Expression<'_>) -> (usize, usize) {
    stack::maybe_grow(|| match expr {
        Expression::StringLiteral(_) => (1, 0),
        Expression::Identifier(_) => (0, 1),
        Expression::ParenthesizedExpression(node) => leaf_counts(&node.expression),
        Expression::UnaryExpression(node) => leaf_counts(&node.argument),
        Expression::BinaryExpression(node) => {
            sum(leaf_counts(&node.left), leaf_counts(&node.right))
        }
        Expression::CallExpression(call) => call
            .arguments
            .iter()
            .filter_map(|argument| argument.as_expression())
            .map(leaf_counts)
            .fold(leaf_counts(&call.callee), sum),
        _ => (0, 0),
    })
}

fn sum(left: (usize, usize), right: (usize, usize)) -> (usize, usize) {
    (left.0 + right.0, left.1 + right.1)
}

type Resolving<'n, 'c, 'a> = (&'n mut Names<'a>, Option<&'c Rc<ScopeChain>>);

fn prebuild<'a>(expr: &'a Expression<'a>, cx: Resolving<'_, '_, 'a>, out: &mut Prebuilt) {
    let (names, chain) = cx;
    stack::maybe_grow(|| match expr {
        Expression::StringLiteral(literal) => out.strings.push(js_string_from_literal(literal)),
        Expression::Identifier(id) => {
            if let Some(chain) = chain {
                out.resolved
                    .push(chain.resolve(names.intern(id.name.as_str())));
            }
        }
        Expression::ParenthesizedExpression(node) => {
            prebuild(&node.expression, (names, chain), out)
        }
        Expression::UnaryExpression(node) => prebuild(&node.argument, (names, chain), out),
        Expression::BinaryExpression(node) => {
            prebuild(&node.left, (&mut *names, chain), out);
            prebuild(&node.right, (names, chain), out);
        }
        Expression::CallExpression(call) => {
            prebuild(&call.callee, (&mut *names, chain), out);
            for argument in call.arguments.iter().filter_map(|a| a.as_expression()) {
                prebuild(argument, (&mut *names, chain), out);
            }
        }
        _ => {}
    })
}

pub(crate) struct Cursor<'s> {
    strings: std::slice::Iter<'s, JsString>,
    resolved: std::slice::Iter<'s, Resolved>,
}

type Strings<'s> = Cursor<'s>;

impl<'a> Term<'a> {
    pub(crate) fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
    ) -> Eval<'a, Value<'a>> {
        match self {
            Term::Direct(source) => source.eval(interp, frame),
            Term::Compiled(code) => code.eval(interp, frame, Reach::CHECKED),
        }
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn is_direct(&self) -> bool {
        matches!(self, Term::Direct(_))
    }
}

fn checksum_term<'a>(
    term: &'a Expression<'a>,
    (names, resolve): (&mut Names<'a>, Option<&Rc<ScopeChain>>),
    chain: impl FnOnce() -> Rc<ScopeChain>,
) -> Term<'a> {
    match is_direct_term(term) {
        true => Term::Direct(TermSource::new(term, names, resolve)),
        false => Term::Compiled(compile_in(term, &chain())),
    }
}

pub(crate) fn compile_in<'a>(expr: &'a Expression<'a>, chain: &Rc<ScopeChain>) -> LazyExpr<'a> {
    LazyExpr::new(expr, chain.clone())
}

pub(crate) fn synthetic_frame<'a>(
    interp: &Interp<'a>,
    layout: &Rc<FrameLayout>,
    parent: &FrameRef<'a>,
) -> FrameRef<'a> {
    interp.realm.new_fixed_env(layout, parent)
}

pub(crate) fn compile_checksum<'a>(
    checksum: &Checksum<'a>,
    terms: &[&'a Expression<'a>],
    cx: &mut Compiler<'_, 'a>,
) -> Option<CompiledChecksum<'a>> {
    let prelude = eligible::prelude(checksum)?;
    let params = (checksum.array_param, checksum.expected_param);
    let shape = checksum_layout(cx.names, params, prelude)?;
    let chain = |provable| ScopeChain::new(shape.layout.clone(), provable, None);
    let all = chain(Provable::All);
    let head = match shape.lexicals.is_empty() {
        true => all.clone(),
        false => chain(pending(&shape.layout, &shape.lexicals)?),
    };
    for hoist in shape.hoists.iter() {
        for (_, function) in hoist.functions.iter() {
            cx.functions.register(FnCode::Function(function), &head);
        }
    }
    let array_slot = shape.array;
    Some(CompiledChecksum {
        array_argument: checksum_term(checksum.array_argument, (cx.names, None), || {
            chain(Provable::Slots(Box::new([])))
        }),
        expected: checksum_term(checksum.expected, (cx.names, None), || {
            chain(Provable::Slots(Box::new([array_slot])))
        }),
        prelude: progressive(prelude, &head, &shape.lexicals),
        terms: terms
            .iter()
            .map(|term| checksum_term(term, (cx.names, Some(&all)), || all.clone()))
            .collect(),
        layout: shape.layout,
        array_slot: shape.array,
        expected_slot: shape.expected,
        hoists: shape.hoists,
    })
}

impl<'a> CompiledChecksum<'a> {
    pub(crate) fn layout(&self) -> &Rc<FrameLayout> {
        &self.layout
    }

    pub(crate) fn open(
        &self,
        interp: &mut Interp<'a>,
        scope: FrameRef<'a>,
    ) -> Eval<'a, (FrameRef<'a>, Value<'a>)> {
        let array_value = self.array_argument.eval(interp, &scope)?;
        Frame::declare_slot(&scope, self.array_slot, array_value);
        let expected = self.expected.eval(interp, &scope)?;
        Frame::declare_slot(&scope, self.expected_slot, expected.clone());
        self.declare_prelude(interp, &scope);
        let reach = depth::frame_reach(interp);
        for statement in self.prelude.iter() {
            statement.run(interp, &scope, reach)?;
        }
        Ok((scope, expected))
    }

    fn declare_prelude(&self, interp: &mut Interp<'a>, scope: &FrameRef<'a>) {
        for hoist in self.hoists.iter() {
            for slot in hoist.vars.iter() {
                Frame::declare_slot_if_absent(scope, *slot, Value::Undefined);
            }
            for (slot, function) in hoist.functions.iter() {
                let closure = interp.make_closure(FnCode::Function(function), scope);
                Frame::declare_slot(scope, *slot, closure);
            }
            if hoist.count > WIDE_SCOPE_BINDINGS / 2 {
                interp.realm.charge_wide(scope);
            }
        }
    }

    pub(crate) fn term(&self, index: usize) -> Option<&Term<'a>> {
        self.terms.get(index)
    }
}

pub(crate) fn is_direct(expr: &Expression<'_>) -> bool {
    stack::maybe_grow(|| match expr {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::ParenthesizedExpression(node) => is_direct(&node.expression),
        Expression::UnaryExpression(node) => {
            node.operator != UnaryOperator::Delete && is_direct(&node.argument)
        }
        Expression::BinaryExpression(node) => is_direct(&node.left) && is_direct(&node.right),
        _ => false,
    })
}

#[inline]
fn leaf_step<'a>(interp: &mut Interp<'a>) -> Eval<'a, ()> {
    interp.budget.enter_nesting()?;
    let spent = interp.budget.spend(1);
    interp.budget.leave_nesting();
    Ok(spent?)
}

#[inline]
pub(crate) fn eval_leaf<'a>(
    interp: &mut Interp<'a>,
    expr: &'a Expression<'a>,
) -> Option<Eval<'a, Value<'a>>> {
    match expr {
        Expression::NumericLiteral(literal) => {
            Some(leaf_step(interp).map(|()| Value::Number(literal.value)))
        }
        Expression::StringLiteral(literal) => {
            Some(leaf_step(interp).map(|()| Value::Str(js_string_from_literal(literal))))
        }
        Expression::BooleanLiteral(literal) => {
            Some(leaf_step(interp).map(|()| Value::Bool(literal.value)))
        }
        Expression::NullLiteral(_) => Some(leaf_step(interp).map(|()| Value::Null)),
        _ => None,
    }
}

pub(crate) fn eval_direct_tree<'a>(
    interp: &mut Interp<'a>,
    expr: &'a Expression<'a>,
) -> Eval<'a, Value<'a>> {
    direct(interp, expr)
}

fn direct<'a>(interp: &mut Interp<'a>, expr: &'a Expression<'a>) -> Eval<'a, Value<'a>> {
    checked(interp, |interp| {
        interp.budget.spend(1)?;
        direct_value(interp, expr)
    })
}

pub(crate) fn direct_value<'a>(
    interp: &mut Interp<'a>,
    expr: &'a Expression<'a>,
) -> Eval<'a, Value<'a>> {
    match expr {
        Expression::NumericLiteral(literal) => Ok(Value::Number(literal.value)),
        Expression::StringLiteral(literal) => Ok(Value::Str(js_string_from_literal(literal))),
        Expression::BooleanLiteral(literal) => Ok(Value::Bool(literal.value)),
        Expression::NullLiteral(_) => Ok(Value::Null),
        Expression::ParenthesizedExpression(node) => direct(interp, &node.expression),
        Expression::UnaryExpression(node) if node.operator != UnaryOperator::Delete => {
            let argument = direct(interp, &node.argument)?;
            unary_op(node.operator, &argument, interp)
        }
        Expression::BinaryExpression(node) => {
            let left = direct(interp, &node.left)?;
            let right = direct(interp, &node.right)?;
            binary_value(interp, node.operator, &left, &right)
        }
        _ => Err(Bail::Unsupported(&"a call argument the direct path does not take").into()),
    }
}

pub(crate) fn is_direct_term(expr: &Expression<'_>) -> bool {
    stack::maybe_grow(|| match expr {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::Identifier(_) => true,
        Expression::ParenthesizedExpression(node) => is_direct_term(&node.expression),
        Expression::UnaryExpression(node) => {
            let probes = node.operator == UnaryOperator::Typeof
                && matches!(node.argument, Expression::Identifier(_));
            node.operator != UnaryOperator::Delete && !probes && is_direct_term(&node.argument)
        }
        Expression::BinaryExpression(node) => {
            is_direct_term(&node.left) && is_direct_term(&node.right)
        }
        Expression::CallExpression(call) => {
            !call.optional
                && matches!(call.callee, Expression::Identifier(_))
                && call
                    .arguments
                    .iter()
                    .all(|argument| argument.as_expression().is_some_and(is_direct_term))
        }
        _ => false,
    })
}

pub(crate) fn direct_term<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    expr: &'a Expression<'a>,
) -> Eval<'a, Value<'a>> {
    let mut empty = Cursor {
        strings: [].iter(),
        resolved: [].iter(),
    };
    term_node(interp, frame, expr, &mut empty)
}

fn term_node<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    expr: &'a Expression<'a>,
    strings: &mut Strings<'_>,
) -> Eval<'a, Value<'a>> {
    if !is_leaf(expr) {
        return checked(interp, |interp| {
            interp.budget.spend(1)?;
            term_value(interp, frame, expr, strings)
        });
    }
    interp.budget.enter_nesting()?;
    let value = match interp.budget.spend(1) {
        Ok(()) => leaf_value(interp, frame, expr, strings),
        Err(bail) => Err(bail.into()),
    };
    interp.budget.leave_nesting();
    value
}

fn is_leaf(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::Identifier(_)
    )
}

#[inline(always)]
fn leaf_value<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    expr: &'a Expression<'a>,
    strings: &mut Strings<'_>,
) -> Eval<'a, Value<'a>> {
    match expr {
        Expression::NumericLiteral(literal) => Ok(Value::Number(literal.value)),
        Expression::StringLiteral(literal) => Ok(Value::Str(
            strings
                .strings
                .next()
                .cloned()
                .unwrap_or_else(|| js_string_from_literal(literal)),
        )),
        Expression::BooleanLiteral(literal) => Ok(Value::Bool(literal.value)),
        Expression::NullLiteral(_) => Ok(Value::Null),
        Expression::Identifier(id) => {
            if let Some(resolved) = strings.resolved.next() {
                return resolved.read(interp, frame, id.name.as_str());
            }
            let name = interp.names.intern(id.name.as_str());
            read_binding(frame, name, || Bail::unresolved_global(id.name.as_str()))
        }
        other => direct_value(interp, other),
    }
}

#[inline(always)]
fn term_value<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    expr: &'a Expression<'a>,
    strings: &mut Strings<'_>,
) -> Eval<'a, Value<'a>> {
    match expr {
        Expression::ParenthesizedExpression(node) => {
            term_node(interp, frame, &node.expression, strings)
        }
        Expression::UnaryExpression(node) if node.operator != UnaryOperator::Delete => {
            let argument = term_node(interp, frame, &node.argument, strings)?;
            unary_op(node.operator, &argument, interp)
        }
        Expression::BinaryExpression(node) => {
            let left = term_node(interp, frame, &node.left, strings)?;
            let right = term_node(interp, frame, &node.right, strings)?;
            match node.operator {
                BinaryOperator::In | BinaryOperator::Instanceof => {
                    binary_value(interp, node.operator, &left, &right)
                }
                operator => binary_op(operator, &left, &right, interp),
            }
        }
        Expression::CallExpression(call) => term_call(interp, frame, call, strings),
        other => direct_value(interp, other),
    }
}

fn term_call<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    call: &'a CallExpression<'a>,
    strings: &mut Strings<'_>,
) -> Eval<'a, Value<'a>> {
    let function = term_node(interp, frame, &call.callee, strings)?;
    let mut values = Vec::with_capacity(call.arguments.len());
    for argument in &call.arguments {
        match argument.as_expression() {
            Some(expr) => values.push(term_node(interp, frame, expr, strings)?),
            None => return Err(Bail::Unsupported(&"spread argument").into()),
        }
    }
    interp.call(&function, Value::Undefined, &values)
}

fn direct_declaration(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.kind == VariableDeclarationKind::Var
        && declaration.declarations.iter().all(|declarator| {
            matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                && declarator.init.as_ref().is_none_or(is_direct_term)
        })
}

pub(crate) fn direct_checksum(checksum: &Checksum<'_>, terms: &[&Expression<'_>]) -> bool {
    is_direct_term(checksum.array_argument)
        && is_direct_term(checksum.expected)
        && checksum
            .prelude
            .iter()
            .all(|statement| direct_declaration(statement))
        && terms.iter().all(|term| is_direct_term(term))
}

pub(crate) fn direct_statement<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    statement: &'a Statement<'a>,
) -> Eval<'a, ()> {
    checked(interp, |interp| {
        interp.budget.spend(1)?;
        match statement {
            Statement::VariableDeclaration(declaration) => {
                direct_declarators(interp, frame, declaration)
            }
            _ => Err(Bail::Unsupported(&"statement kind").into()),
        }
    })
}

fn direct_declarators<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    declaration: &'a VariableDeclaration<'a>,
) -> Eval<'a, ()> {
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return Err(Bail::Unsupported(&"destructuring declaration").into());
        };
        let value = match &declarator.init {
            Some(init) => direct_term(interp, frame, init)?,
            None => Value::Undefined,
        };
        let name = interp.names.intern(id.name.as_str());
        if declarator.init.is_some() {
            if assign(frame, name, value.clone())? == Assigned::Unbound {
                declare(frame, name, value);
            }
        } else if !has_binding(frame, name)? {
            declare(frame, name, Value::Undefined);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;

    use super::{direct_term, is_direct_term};
    use crate::budget::Budget;
    use crate::interp::Interp;
    use crate::value::{Eval, Value};

    const PARSE_STACK: usize = 1 << 30;

    fn outcome(result: Eval<'_, Value<'_>>, steps: u64) -> String {
        match result {
            Ok(value) => format!("{value:?} at {steps}"),
            Err(abrupt) => match abrupt.kind() {
                crate::bail::AbruptKind::Bail(bail) => format!("{bail} at {steps}"),
                crate::bail::AbruptKind::Throw(_) => format!("a throw at {steps}"),
            },
        }
    }

    fn both(source: String) -> (String, String) {
        let alloc = Allocator::default();
        let parsed = crate::stack::grow(PARSE_STACK, || {
            untwist_core::parse::parse_program(&alloc, &source)
        });
        let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.last() else {
            panic!("an expression statement");
        };
        let term = &statement.expression;
        assert!(is_direct_term(term));
        let mut compiled = Interp::new(&source, Budget::default());
        let tree = compiled.eval_root_expression(term);
        let tree = outcome(tree, compiled.budget.steps_used());
        let mut direct = Interp::new(&source, Budget::default());
        let root = direct.global_env();
        let run = direct_term(&mut direct, &root, term);
        (tree, outcome(run, direct.budget.steps_used()))
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "release-only: terms 100,000 deep on a 2 MiB thread"
    )]
    fn a_deep_direct_term_runs_and_bails_where_the_compiled_tree_does_on_a_2_mib_thread() {
        let row = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(|| {
                for n in [99_990usize, 100_001] {
                    let negated = format!(
                        "{}parseInt('7'){}",
                        "-(-(".repeat(n / 2),
                        "))".repeat(n / 2)
                    );
                    let (tree, direct) = both(format!("{negated};"));
                    assert_eq!(tree, direct, "negated {n}");
                    let chain = format!("parseInt('7'){}", " + 1".repeat(n));
                    let (tree, direct) = both(format!("{chain};"));
                    assert_eq!(tree, direct, "chain {n}");
                }
            })
            .expect("a thread");
        row.join().expect("the row's thread finished");
    }
}
