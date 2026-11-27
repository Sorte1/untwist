use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

pub(super) struct Entry {
    pub(super) key: String,
}

pub(super) struct Machine {
    pub(super) dispatcher: SymbolId,
    pub(super) payload: SymbolId,
    pub(super) cache: SymbolId,
    pub(super) setter: SymbolId,
    pub(super) span: Span,
    pub(super) keys: [String; 4],
    pub(super) lengths: FxHashMap<String, f64>,
    pub(super) entries: Vec<Entry>,
    pub(super) payload_refs: Vec<Span>,
    pub(super) fns_span: Span,
}

pub(super) const CLEAR: usize = 0;
pub(super) const NON_CALL: usize = 1;
pub(super) const AS_OBJECT: usize = 2;
pub(super) const PROPERTY: usize = 3;

pub(super) use super::super::ast::{member, property_key, string, Ctx, Key};

fn indexed(cx: &Ctx<'_>, expr: &Expression<'_>, object: SymbolId, index: SymbolId) -> bool {
    matches!(member(expr), Some((o, Key::Expr(k))) if cx.is(o, object) && cx.is(k, index))
}

fn body_of<'k, 'a>(stmt: &'k Statement<'a>) -> &'k [Statement<'a>] {
    match stmt {
        Statement::BlockStatement(block) => &block.body,
        other => std::slice::from_ref(other),
    }
}

fn expression_of<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match stmt {
        Statement::ExpressionStatement(s) => Some(&s.expression),
        _ => None,
    }
}

fn single_expression<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match body_of(stmt) {
        [only] => expression_of(only),
        _ => None,
    }
}

fn returned<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match body_of(stmt) {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

fn assignment<'k, 'a>(
    cx: &Ctx<'_>,
    expr: &'k Expression<'a>,
) -> Option<(SymbolId, Span, &'k Expression<'a>)> {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let (symbol, span) = cx.target_symbol(&assign.left)?;
    Some((symbol, span, &assign.right))
}

fn compared(cx: &Ctx<'_>, expr: &Expression<'_>, symbol: SymbolId) -> Option<String> {
    let Expression::BinaryExpression(binary) = expr.without_parentheses() else {
        return None;
    };
    if binary.operator != BinaryOperator::StrictEquality {
        return None;
    }
    if cx.is(&binary.left, symbol) {
        return string(&binary.right);
    }
    if cx.is(&binary.right, symbol) {
        return string(&binary.left);
    }
    None
}

type Declared<'k, 'a> = Vec<(SymbolId, Option<&'k Expression<'a>>)>;

fn leading_vars<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<(Declared<'k, 'a>, usize)> {
    let mut declared = Vec::new();
    let mut used = 0;
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            break;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            declared.push((id.symbol_id.get()?, declarator.init.as_ref()));
        }
        used += 1;
    }
    Some((declared, used))
}

struct Locals {
    name: SymbolId,
    flag: SymbolId,
    returns: SymbolId,
    lengths: SymbolId,
    output: SymbolId,
    fns: SymbolId,
}

fn plain_param(item: &FormalParameter<'_>) -> Option<SymbolId> {
    match (&item.pattern, &item.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
        _ => None,
    }
}

fn lengths_of(object: &ObjectExpression<'_>) -> Option<FxHashMap<String, f64>> {
    object
        .properties
        .iter()
        .map(|property| {
            let ObjectPropertyKind::ObjectProperty(p) = property else {
                return None;
            };
            if p.kind != PropertyKind::Init || p.method || p.shorthand {
                return None;
            }
            let Expression::NumericLiteral(n) = &p.value else {
                return None;
            };
            Some((property_key(&p.key, p.computed)?, n.value))
        })
        .collect()
}

struct Found {
    payload: Option<SymbolId>,
    cache: Option<SymbolId>,
    setter: Option<SymbolId>,
    payload_refs: Vec<Span>,
    keys: [Option<String>; 4],
}

impl Found {
    fn payload(&mut self, symbol: SymbolId, span: Span) -> Option<()> {
        if *self.payload.get_or_insert(symbol) != symbol {
            return None;
        }
        self.payload_refs.push(span);
        Some(())
    }

    fn same(slot: &mut Option<SymbolId>, symbol: SymbolId) -> Option<()> {
        (*slot.get_or_insert(symbol) == symbol).then_some(())
    }
}

fn clear_branch(cx: &Ctx<'_>, stmt: &Statement<'_>, l: &Locals, found: &mut Found) -> Option<()> {
    let Statement::IfStatement(branch) = stmt else {
        return None;
    };
    if branch.alternate.is_some() {
        return None;
    }
    found.keys[CLEAR] = Some(compared(cx, &branch.test, l.flag)?);
    let (payload, span, value) = assignment(cx, single_expression(&branch.consequent)?)?;
    let Expression::ArrayExpression(array) = value.without_parentheses() else {
        return None;
    };
    if !array.elements.is_empty() {
        return None;
    }
    found.payload(payload, span)
}

fn wrapper(cx: &Ctx<'_>, expr: &Expression<'_>, l: &Locals, found: &mut Found) -> Option<()> {
    let Expression::FunctionExpression(function) = expr.without_parentheses() else {
        return None;
    };
    if function.id.is_some() || function.r#async || function.generator {
        return None;
    }
    let rest = function.params.rest.as_ref()?;
    let BindingPattern::BindingIdentifier(args) = &rest.rest.argument else {
        return None;
    };
    let args = args.symbol_id.get()?;
    let body = function.body.as_ref()?;
    if !function.params.items.is_empty() || !body.directives.is_empty() {
        return None;
    }
    let [first, Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return None;
    };
    let (payload, span, value) = assignment(cx, expression_of(first)?)?;
    if !cx.is(value, args) {
        return None;
    }
    found.payload(payload, span)?;
    let Expression::CallExpression(call) = ret.argument.as_ref()?.without_parentheses() else {
        return None;
    };
    let (callee, Key::Str(apply)) = member(&call.callee)? else {
        return None;
    };
    let this_only = matches!(call.arguments.as_slice(), [Argument::ThisExpression(_)]);
    (apply == "apply" && this_only && !call.optional && indexed(cx, callee, l.fns, l.name))
        .then_some(())
}

fn create_function(
    cx: &Ctx<'_>,
    stmt: &Statement<'_>,
    l: &Locals,
    found: &mut Found,
) -> Option<SymbolId> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    let body = function.body.as_ref()?;
    if function.r#async
        || function.generator
        || !function.params.items.is_empty()
        || function.params.rest.is_some()
        || !body.directives.is_empty()
    {
        return None;
    }
    let (declared, used) = leading_vars(&body.statements)?;
    let [(fn_symbol, Some(fn_value)), (length, Some(length_value))] = declared.as_slice() else {
        return None;
    };
    wrapper(cx, fn_value, l, found)?;
    if !indexed(cx, length_value, l.lengths, l.name) {
        return None;
    }
    let [Statement::IfStatement(check), Statement::ReturnStatement(ret)] = &body.statements[used..]
    else {
        return None;
    };
    if check.alternate.is_some() || !cx.is(&check.test, *length) {
        return None;
    }
    let Expression::CallExpression(set) =
        single_expression(&check.consequent)?.without_parentheses()
    else {
        return None;
    };
    let [Argument::Identifier(a), Argument::Identifier(b)] = set.arguments.as_slice() else {
        return None;
    };
    let resolve =
        |id: &IdentifierReference<'_>| cx.scoping.get_reference(id.reference_id.get()?).symbol_id();
    if resolve(a) != Some(*fn_symbol) || resolve(b) != Some(*length) || set.optional {
        return None;
    }
    Found::same(&mut found.setter, cx.symbol(&set.callee)?)?;
    if !cx.is(ret.argument.as_ref()?, *fn_symbol) {
        return None;
    }
    function.id.as_ref()?.symbol_id.get()
}

fn cached(
    cx: &Ctx<'_>,
    expr: &Expression<'_>,
    l: &Locals,
    create: SymbolId,
    found: &mut Found,
) -> Option<()> {
    let (output, _, value) = assignment(cx, expr)?;
    let Expression::LogicalExpression(logical) = value.without_parentheses() else {
        return None;
    };
    if output != l.output || logical.operator != LogicalOperator::Or {
        return None;
    }
    let (cache, Key::Expr(index)) = member(&logical.left)? else {
        return None;
    };
    Found::same(&mut found.cache, cx.symbol(cache)?)?;
    if !cx.is(index, l.name) {
        return None;
    }
    let Expression::AssignmentExpression(store) = logical.right.without_parentheses() else {
        return None;
    };
    let SimpleAssignmentTarget::ComputedMemberExpression(slot) =
        store.left.as_simple_assignment_target()?
    else {
        return None;
    };
    Found::same(&mut found.cache, cx.symbol(&slot.object)?)?;
    let Expression::CallExpression(call) = store.right.without_parentheses() else {
        return None;
    };
    (store.operator == AssignmentOperator::Assign
        && cx.is(&slot.expression, l.name)
        && call.arguments.is_empty()
        && !call.optional
        && cx.is(&call.callee, create))
    .then_some(())
}

fn non_call_branch(
    cx: &Ctx<'_>,
    stmt: &Statement<'_>,
    l: &Locals,
    found: &mut Found,
) -> Option<()> {
    let Statement::IfStatement(branch) = stmt else {
        return None;
    };
    found.keys[NON_CALL] = Some(compared(cx, &branch.test, l.flag)?);
    let [create, store] = body_of(&branch.consequent) else {
        return None;
    };
    let create = create_function(cx, create, l, found)?;
    cached(cx, expression_of(store)?, l, create, found)?;
    let (output, _, value) = assignment(cx, single_expression(branch.alternate.as_ref()?)?)?;
    let Expression::CallExpression(call) = value.without_parentheses() else {
        return None;
    };
    (output == l.output
        && call.arguments.is_empty()
        && !call.optional
        && indexed(cx, &call.callee, l.fns, l.name))
    .then_some(())
}

fn wrapped_output(cx: &Ctx<'_>, expr: &Expression<'_>, l: &Locals) -> Option<String> {
    let Expression::ObjectExpression(object) = expr.without_parentheses() else {
        return None;
    };
    let [ObjectPropertyKind::ObjectProperty(p)] = object.properties.as_slice() else {
        return None;
    };
    if p.kind != PropertyKind::Init || p.method || p.shorthand || !cx.is(&p.value, l.output) {
        return None;
    }
    property_key(&p.key, p.computed)
}

fn tail(cx: &Ctx<'_>, statements: &[Statement<'_>], l: &Locals, found: &mut Found) -> Option<()> {
    let (test, wrapped, plain) = match statements {
        [Statement::IfStatement(branch)] => (
            &branch.test,
            returned(&branch.consequent)?,
            returned(branch.alternate.as_ref()?)?,
        ),
        [Statement::ReturnStatement(ret)] => {
            let Expression::ConditionalExpression(choice) =
                ret.argument.as_ref()?.without_parentheses()
            else {
                return None;
            };
            (&choice.test, &choice.consequent, &choice.alternate)
        }
        _ => return None,
    };
    found.keys[AS_OBJECT] = Some(compared(cx, test, l.returns)?);
    found.keys[PROPERTY] = Some(wrapped_output(cx, wrapped, l)?);
    cx.is(plain, l.output).then_some(())
}

fn entry(
    cx: &Ctx<'_>,
    property: &ObjectPropertyKind<'_>,
    found: &mut Found,
    scope: ScopeId,
) -> Option<Entry> {
    let ObjectPropertyKind::ObjectProperty(p) = property else {
        return None;
    };
    if p.kind != PropertyKind::Init || p.method || p.shorthand {
        return None;
    }
    let key = property_key(&p.key, p.computed)?;
    let Expression::FunctionExpression(function) = &p.value else {
        return None;
    };
    let body = function.body.as_ref()?;
    if function.id.is_some()
        || function.r#async
        || function.generator
        || !function.params.items.is_empty()
        || function.params.rest.is_some()
        || !body.directives.is_empty()
        || super::hazards::hazardous(function, cx.scoping, scope)
    {
        return None;
    }
    let mut rest = body.statements.as_slice();
    if let Some(Statement::VariableDeclaration(declaration)) = rest.first() {
        if let (Some(declarator), VariableDeclarationKind::Var) =
            (declaration.declarations.first(), declaration.kind)
        {
            if let (BindingPattern::ArrayPattern(pattern), Some(init)) =
                (&declarator.id, &declarator.init)
            {
                let Expression::Identifier(id) = init.without_parentheses() else {
                    return None;
                };
                let payload = cx.symbol(init)?;
                found.payload(payload, id.span)?;
                if pattern.elements.iter().any(Option::is_none) {
                    return None;
                }
                rest = &rest[1..];
            }
        }
    }
    if let Some(Statement::ExpressionStatement(first)) = rest.first() {
        if matches!(first.expression, Expression::StringLiteral(_)) {
            return None;
        }
    }
    Some(Entry { key })
}

fn locals<'k, 'a>(
    function: &'k Function<'a>,
) -> Option<(
    Locals,
    FxHashMap<String, f64>,
    &'k ObjectExpression<'a>,
    usize,
)> {
    let params = &function.params;
    let [name, flag, returns, lengths] = params.items.as_slice() else {
        return None;
    };
    if params.rest.is_some() {
        return None;
    }
    let (BindingPattern::BindingIdentifier(lengths_id), Some(defaults)) =
        (&lengths.pattern, &lengths.initializer)
    else {
        return None;
    };
    let Expression::ObjectExpression(defaults) = defaults.without_parentheses() else {
        return None;
    };
    let body = function.body.as_ref()?;
    if !body.directives.is_empty() {
        return None;
    }
    let (declared, used) = leading_vars(&body.statements)?;
    let [(output, None), (fns, Some(table))] = declared.as_slice() else {
        return None;
    };
    let Expression::ObjectExpression(table) = table.without_parentheses() else {
        return None;
    };
    let l = Locals {
        name: plain_param(name)?,
        flag: plain_param(flag)?,
        returns: plain_param(returns)?,
        lengths: lengths_id.symbol_id.get()?,
        output: *output,
        fns: *fns,
    };
    let symbols = [l.name, l.flag, l.returns, l.lengths, l.output, l.fns];
    if (0..symbols.len()).any(|i| symbols[..i].contains(&symbols[i])) {
        return None;
    }
    Some((l, lengths_of(defaults)?, table, used))
}

pub(super) fn dispatcher(cx: &Ctx<'_>, stmt: &Statement<'_>) -> Option<Machine> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    if function.r#async || function.generator {
        return None;
    }
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let (l, lengths, table, used) = locals(function)?;
    let scope = function.scope_id.get()?;
    let statements = &function.body.as_ref()?.statements[used..];
    let [clear, non_call, rest @ ..] = statements else {
        return None;
    };
    let mut found = Found {
        payload: None,
        cache: None,
        setter: None,
        payload_refs: Vec::new(),
        keys: [None, None, None, None],
    };
    clear_branch(cx, clear, &l, &mut found)?;
    non_call_branch(cx, non_call, &l, &mut found)?;
    tail(cx, rest, &l, &mut found)?;
    let entries: Vec<Entry> = table
        .properties
        .iter()
        .map(|property| entry(cx, property, &mut found, scope))
        .collect::<Option<_>>()?;
    let [Some(k0), Some(k1), Some(k2), Some(k3)] = found.keys else {
        return None;
    };
    let keys = [k0, k1, k2, k3];
    let distinct = (0..4).all(|i| (0..i).all(|j| keys[i] != keys[j]));
    let unique = (0..entries.len()).all(|i| (0..i).all(|j| entries[i].key != entries[j].key));
    if !distinct || !unique {
        return None;
    }
    Some(Machine {
        dispatcher: symbol,
        payload: found.payload?,
        cache: found.cache?,
        setter: found.setter?,
        span: stmt.span(),
        keys,
        lengths,
        entries,
        payload_refs: found.payload_refs,
        fns_span: table.span,
    })
}
