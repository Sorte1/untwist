use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::ast::{member, Ctx, Key};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Use {
    Standard,
    Typeof,
    Call,
}

#[derive(Clone)]
pub(super) struct Prop {
    pub(super) kind: Use,
    pub(super) target: SymbolId,
    pub(super) name: String,
    pub(super) settable: bool,
}

pub(super) struct Flat {
    pub(super) symbol: SymbolId,
    pub(super) span: Span,
    pub(super) object: SymbolId,
    pub(super) strict: bool,
}

pub(super) struct Wrapper {
    pub(super) span: Span,
    pub(super) scope_strict: bool,
    pub(super) flat: SymbolId,
    pub(super) object: SymbolId,
    pub(super) args: SymbolId,
    pub(super) props: FxHashMap<String, Prop>,
}

fn binding(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn is_arguments(cx: &Ctx<'_>, expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == "arguments")
        && cx.symbol(expr).is_none()
}

struct Hazards {
    depth: u32,
    arguments: usize,
    found: bool,
}

impl<'a> VisitJs<'a> for Hazards {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found |= self.depth == 0;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found |= self.depth == 0;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" && self.depth == 0 {
            self.arguments += 1;
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

fn hazardous(function: &Function<'_>, allowed_arguments: usize) -> bool {
    let mut hazards = Hazards {
        depth: 0,
        arguments: 0,
        found: false,
    };
    walk_js::walk_function(&mut hazards, function, ScopeFlags::Function);
    hazards.found || hazards.arguments != allowed_arguments
}

fn strict_head(cx: &Ctx<'_>, body: &FunctionBody<'_>) -> Option<SymbolId> {
    let [directive] = body.directives.as_slice() else {
        return None;
    };
    if directive.directive.as_str() != "use strict" {
        return None;
    }
    let Some(Statement::VariableDeclaration(declaration)) = body.statements.first() else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let (BindingPattern::ArrayPattern(pattern), Some(init)) = (&declarator.id, &declarator.init)
    else {
        return None;
    };
    let [Some(object), Some(BindingPattern::ArrayPattern(_))] = pattern.elements.as_slice() else {
        return None;
    };
    (declaration.kind == VariableDeclarationKind::Var
        && pattern.rest.is_none()
        && is_arguments(cx, init))
    .then_some(())?;
    binding(object)
}

pub(super) fn flat(cx: &Ctx<'_>, statement: &Statement<'_>) -> Option<Flat> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    if function.generator || function.r#async || function.params.rest.is_some() {
        return None;
    }
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let body = function.body.as_ref()?;
    let (object, strict) = match function.params.items.as_slice() {
        [] => (strict_head(cx, body)?, true),
        [object, params] => {
            if object.initializer.is_some()
                || params.initializer.is_some()
                || !matches!(params.pattern, BindingPattern::ArrayPattern(_))
                || !body.directives.is_empty()
            {
                return None;
            }
            (binding(&object.pattern)?, false)
        }
        _ => return None,
    };
    if hazardous(function, usize::from(strict))
        || !cx.scoping.symbol_redeclarations(object).is_empty()
    {
        return None;
    }
    Some(Flat {
        symbol,
        span: statement.span(),
        object,
        strict,
    })
}

fn returned<'k, 'a>(function: &'k Function<'a>) -> Option<&'k Expression<'a>> {
    let body = function.body.as_ref()?;
    match body.statements.as_slice() {
        [Statement::ReturnStatement(ret)] if body.directives.is_empty() => ret.argument.as_ref(),
        _ => None,
    }
}

fn no_params(function: &Function<'_>) -> bool {
    function.params.items.is_empty() && function.params.rest.is_none()
}

fn plain(function: &Function<'_>) -> bool {
    function.id.is_none() && !function.r#async && !function.generator
}

fn rest_only(function: &Function<'_>) -> Option<SymbolId> {
    let rest = function.params.rest.as_ref()?;
    function
        .params
        .items
        .is_empty()
        .then(|| binding(&rest.rest.argument))?
}

fn reference(cx: &Ctx<'_>, expr: &Expression<'_>) -> Option<(SymbolId, String)> {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return None;
    };
    Some((cx.symbol(expr)?, id.name.to_string()))
}

fn getter(cx: &Ctx<'_>, function: &Function<'_>) -> Option<(Use, SymbolId, String)> {
    if !plain(function) || !no_params(function) {
        return None;
    }
    let value = returned(function)?;
    if let Expression::UnaryExpression(unary) = value.without_parentheses() {
        if unary.operator != UnaryOperator::Typeof {
            return None;
        }
        let (target, name) = reference(cx, &unary.argument)?;
        return Some((Use::Typeof, target, name));
    }
    let (target, name) = reference(cx, value)?;
    Some((Use::Standard, target, name))
}

fn setter(cx: &Ctx<'_>, function: &Function<'_>) -> Option<SymbolId> {
    let [param] = function.params.items.as_slice() else {
        return None;
    };
    let body = function.body.as_ref()?;
    let [Statement::ExpressionStatement(statement)] = body.statements.as_slice() else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &statement.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    let target = cx
        .scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()?;
    let value = binding(&param.pattern)?;
    (plain(function)
        && function.params.rest.is_none()
        && param.initializer.is_none()
        && body.directives.is_empty()
        && assign.operator == AssignmentOperator::Assign
        && cx.is(&assign.right, value))
    .then_some(target)
}

fn caller(cx: &Ctx<'_>, function: &Function<'_>) -> Option<(SymbolId, String)> {
    let args = rest_only(function)?;
    let Expression::CallExpression(call) = returned(function)?.without_parentheses() else {
        return None;
    };
    let [Argument::SpreadElement(spread)] = call.arguments.as_slice() else {
        return None;
    };
    (plain(function) && !call.optional && cx.is(&spread.argument, args)).then_some(())?;
    reference(cx, &call.callee)
}

fn props(cx: &Ctx<'_>, object: &ObjectExpression<'_>) -> Option<FxHashMap<String, Prop>> {
    let mut props: FxHashMap<String, Prop> = FxHashMap::default();
    let mut setters: Vec<(String, SymbolId)> = Vec::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(p) = property else {
            return None;
        };
        let PropertyKey::StringLiteral(key) = &p.key else {
            return None;
        };
        let Expression::FunctionExpression(function) = &p.value else {
            return None;
        };
        let key = key.value.as_str()?.to_string();
        if p.computed || p.shorthand {
            return None;
        }
        let (kind, target, name) = match (p.kind, p.method) {
            (PropertyKind::Get, false) => getter(cx, function)?,
            (PropertyKind::Set, false) => {
                setters.push((key, setter(cx, function)?));
                continue;
            }
            (PropertyKind::Init, true) => {
                let (target, name) = caller(cx, function)?;
                (Use::Call, target, name)
            }
            _ => return None,
        };
        let prop = Prop {
            kind,
            target,
            name,
            settable: false,
        };
        if props.insert(key, prop).is_some() {
            return None;
        }
    }
    for (key, target) in setters {
        let prop = props.get_mut(&key)?;
        if prop.kind != Use::Standard || prop.target != target || prop.settable {
            return None;
        }
        prop.settable = true;
    }
    Some(props)
}

pub(super) fn wrapper(cx: &Ctx<'_>, function: &Function<'_>) -> Option<Wrapper> {
    if function.generator || function.r#async {
        return None;
    }
    let args = rest_only(function)?;
    let body = function.body.as_ref()?;
    let [Statement::VariableDeclaration(declaration), Statement::ReturnStatement(ret)] =
        body.statements.as_slice()
    else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let Some(Expression::ObjectExpression(object_init)) = &declarator.init else {
        return None;
    };
    let object = binding(&declarator.id)?;
    let Expression::CallExpression(call) = ret.argument.as_ref()?.without_parentheses() else {
        return None;
    };
    let [Argument::Identifier(passed_object), Argument::Identifier(passed_args)] =
        call.arguments.as_slice()
    else {
        return None;
    };
    let resolve =
        |id: &IdentifierReference<'_>| cx.scoping.get_reference(id.reference_id.get()?).symbol_id();
    if !body.directives.is_empty()
        || declaration.kind != VariableDeclarationKind::Var
        || call.optional
        || resolve(passed_object) != Some(object)
        || resolve(passed_args) != Some(args)
    {
        return None;
    }
    let scope = function.scope_id.get()?;
    Some(Wrapper {
        span: function.span,
        scope_strict: cx.scoping.scope_flags(scope).is_strict_mode(),
        flat: cx.symbol(&call.callee)?,
        object,
        args,
        props: props(cx, object_init)?,
    })
}

pub(super) fn object_member<'k>(
    cx: &Ctx<'_>,
    expr: &Expression<'_>,
    object: SymbolId,
    props: &'k FxHashMap<String, Prop>,
) -> Option<&'k Prop> {
    let (base, Key::Str(key)) = member(expr)? else {
        return None;
    };
    if !matches!(
        expr.without_parentheses(),
        Expression::ComputedMemberExpression(_)
    ) {
        return None;
    }
    cx.is(base, object).then(|| props.get(&key))?
}
