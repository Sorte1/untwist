use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_syntax::scope::ScopeId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::frame::{Frame, Shape};

pub const ENTRY_ID: &str = "index";
const STRICT: &str = "use strict";

pub enum Bind<'k, 'a> {
    Import(&'a str, &'k IdentifierReference<'a>),
    Namespace(&'k IdentifierReference<'a>),
    ReExport(&'a str, &'a str),
}

pub struct Declare<'k, 'a> {
    pub params: Vec<&'k BindingIdentifier<'a>>,
    pub directives: &'k [Directive<'a>],
    pub vars: Vec<&'k BindingIdentifier<'a>>,
    pub setters: Vec<Vec<Bind<'k, 'a>>>,
    pub execute: &'k FunctionBody<'a>,
    pub execute_scope: &'k Cell<Option<ScopeId>>,
}

impl<'a> Declare<'_, 'a> {
    pub fn export_name(&self) -> Option<&'a str> {
        self.params.first().map(|id| id.name.as_str())
    }

    pub fn context_name(&self) -> Option<&'a str> {
        self.params.get(1).map(|id| id.name.as_str())
    }
}

pub struct Register<'k, 'a> {
    pub name: Option<&'a str>,
    pub deps: Vec<&'a str>,
    pub declare: Declare<'k, 'a>,
}

pub struct Layout<'k, 'a> {
    pub registers: Vec<Register<'k, 'a>>,
    pub live: Vec<usize>,
    pub entry: usize,
    pub strict: &'k [Directive<'a>],
}

impl<'k, 'a> Layout<'k, 'a> {
    pub fn id(&self, index: usize) -> &'a str {
        self.registers[index].name.unwrap_or(ENTRY_ID)
    }

    pub fn names(&self) -> FxHashSet<&'a str> {
        self.registers.iter().filter_map(|r| r.name).collect()
    }

    pub fn strict_directives(&self, register: &Register<'k, 'a>) -> &'k [Directive<'a>] {
        match is_strict(register.declare.directives) {
            true => register.declare.directives,
            false => self.strict,
        }
    }
}

fn is_strict(directives: &[Directive<'_>]) -> bool {
    directives.iter().any(|d| d.directive.as_str() == STRICT)
}

fn register_call<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k CallExpression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = &statement.expression else {
        return None;
    };
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return None;
    };
    let Expression::Identifier(object) = &callee.object else {
        return None;
    };
    let named = object.name == "System" && callee.property.name == "register";
    (named && !call.optional && !callee.optional).then_some(call)
}

fn is_function(argument: &Argument<'_>) -> bool {
    matches!(
        argument.as_expression(),
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_))
    )
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    frame.lists().any(|list| {
        list.iter().any(|statement| {
            register_call(statement)
                .is_some_and(|call| call.arguments.last().is_some_and(is_function))
        })
    })
}

fn plain_string<'a>(expression: &Expression<'a>) -> Option<&'a str> {
    match expression {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn simple_params<'k, 'a>(
    params: &'k FormalParameters<'a>,
) -> Option<Vec<&'k BindingIdentifier<'a>>> {
    if params.rest.is_some() {
        return None;
    }
    let mut ids: Vec<&BindingIdentifier<'a>> = Vec::with_capacity(params.items.len());
    for item in &params.items {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            return None;
        };
        if item.initializer.is_some() || ids.iter().any(|seen| seen.name == id.name) {
            return None;
        }
        ids.push(id);
    }
    Some(ids)
}

struct Callable<'k, 'a> {
    params: Vec<&'k BindingIdentifier<'a>>,
    body: &'k FunctionBody<'a>,
    scope: &'k Cell<Option<ScopeId>>,
}

fn callable<'k, 'a>(expression: &'k Expression<'a>) -> Option<Callable<'k, 'a>> {
    match expression {
        Expression::FunctionExpression(function)
            if function.id.is_none() && !function.r#async && !function.generator =>
        {
            Some(Callable {
                params: simple_params(&function.params)?,
                body: function.body.as_deref()?,
                scope: &function.scope_id,
            })
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            Some(Callable {
                params: simple_params(&arrow.params)?,
                body,
                scope: &arrow.scope_id,
            })
        }
        _ => None,
    }
}

fn member_name<'a>(expression: &Expression<'a>, param: &str) -> Option<&'a str> {
    let (object, name) = match expression {
        Expression::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            (&member.object, plain_string(&member.expression)?)
        }
        _ => return None,
    };
    let Expression::Identifier(object) = object else {
        return None;
    };
    (object.name == param).then_some(name)
}

pub fn property_name<'k, 'a>(
    property: &'k ObjectPropertyKind<'a>,
) -> Option<(&'a str, &'k Expression<'a>)> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if property.method {
        return None;
    }
    property_key(property)
}

fn re_exports<'k, 'a>(call: &'k CallExpression<'a>, param: &str) -> Option<Vec<Bind<'k, 'a>>> {
    match call.arguments.as_slice() {
        [name, value] => {
            let exported = plain_string(name.as_expression()?)?;
            let name = member_name(value.as_expression()?, param)?;
            Some(vec![Bind::ReExport(name, exported)])
        }
        [Argument::ObjectExpression(object)] => object
            .properties
            .iter()
            .map(|property| {
                let (exported, value) = property_name(property)?;
                Some(Bind::ReExport(member_name(value, param)?, exported))
            })
            .collect(),
        _ => None,
    }
}

fn setter_statement<'k, 'a>(
    statement: &'k Statement<'a>,
    param: &str,
    export: Option<&str>,
) -> Option<Vec<Bind<'k, 'a>>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    match &statement.expression {
        Expression::AssignmentExpression(assign) if assign.operator.is_assign() => {
            let AssignmentTarget::AssignmentTargetIdentifier(local) = &assign.left else {
                return None;
            };
            if local.name == param {
                return None;
            }
            match &assign.right {
                Expression::Identifier(id) if id.name == param => {
                    Some(vec![Bind::Namespace(local)])
                }
                right => Some(vec![Bind::Import(member_name(right, param)?, local)]),
            }
        }
        Expression::CallExpression(call) if !call.optional => {
            let Expression::Identifier(callee) = &call.callee else {
                return None;
            };
            (Some(callee.name.as_str()) == export && callee.name != param)
                .then(|| re_exports(call, param))
                .flatten()
        }
        _ => None,
    }
}

fn setter<'k, 'a>(
    element: &'k ArrayExpressionElement<'a>,
    export: Option<&str>,
) -> Option<Vec<Bind<'k, 'a>>> {
    match element {
        ArrayExpressionElement::Elision(_) | ArrayExpressionElement::NullLiteral(_) => {
            return Some(Vec::new())
        }
        ArrayExpressionElement::SpreadElement(_) => return None,
        _ => {}
    }
    let function = callable(element.as_expression()?)?;
    if !function.body.directives.is_empty() || function.params.len() > 1 {
        return None;
    }
    let Some(param) = function.params.first() else {
        return function.body.statements.is_empty().then(Vec::new);
    };
    let mut binds = Vec::new();
    for statement in &function.body.statements {
        binds.extend(setter_statement(statement, param.name.as_str(), export)?);
    }
    Some(binds)
}

fn execute_body<'k, 'a>(
    value: &'k Expression<'a>,
) -> Option<(&'k FunctionBody<'a>, &'k Cell<Option<ScopeId>>)> {
    let function = callable(value)?;
    function
        .params
        .is_empty()
        .then_some((function.body, function.scope))
}

struct Returned<'k, 'a> {
    setters: Option<&'k ArrayExpression<'a>>,
    execute: Option<(&'k FunctionBody<'a>, &'k Cell<Option<ScopeId>>)>,
}

fn returned<'k, 'a>(statement: &'k Statement<'a>) -> Option<Returned<'k, 'a>> {
    let Statement::ReturnStatement(statement) = statement else {
        return None;
    };
    let Some(Expression::ObjectExpression(object)) = &statement.argument else {
        return None;
    };
    let mut found = Returned {
        setters: None,
        execute: None,
    };
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        let (name, value) = property_key(property)?;
        if property.shorthand || (property.method && name != "execute") {
            return None;
        }
        match (name, value) {
            ("setters", Expression::ArrayExpression(array)) if found.setters.is_none() => {
                found.setters = Some(array)
            }
            ("execute", value) if found.execute.is_none() => {
                found.execute = Some(execute_body(value)?)
            }
            _ => return None,
        }
    }
    Some(found)
}

fn property_key<'k, 'a>(property: &'k ObjectProperty<'a>) -> Option<(&'a str, &'k Expression<'a>)> {
    if property.computed || property.kind != PropertyKind::Init {
        return None;
    }
    let name = match &property.key {
        PropertyKey::StaticIdentifier(id) => id.name.as_str(),
        PropertyKey::StringLiteral(literal) => literal.value.as_str()?,
        _ => return None,
    };
    Some((name, &property.value))
}

fn declared_vars<'k, 'a>(
    statements: &'k [Statement<'a>],
) -> Option<Vec<&'k BindingIdentifier<'a>>> {
    let mut vars = Vec::new();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            return None;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            if declarator.init.is_some() {
                return None;
            }
            vars.push(id.as_ref());
        }
    }
    Some(vars)
}

fn declare<'k, 'a>(expression: &'k Expression<'a>) -> Option<Declare<'k, 'a>> {
    let function = callable(expression)?;
    if function.params.len() > 2 {
        return None;
    }
    let (last, prefix) = function.body.statements.split_last()?;
    let vars = declared_vars(prefix)?;
    let found = returned(last)?;
    let (execute, execute_scope) = found.execute?;
    let export = function.params.first().map(|id| id.name.as_str());
    let setters = match found.setters {
        Some(array) => array
            .elements
            .iter()
            .map(|element| setter(element, export))
            .collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    Some(Declare {
        params: function.params,
        directives: &function.body.directives,
        vars,
        setters,
        execute,
        execute_scope,
    })
}

fn register<'k, 'a>(call: &'k CallExpression<'a>) -> Option<Register<'k, 'a>> {
    let arguments: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .map(|argument| {
            (!argument.is_spread())
                .then(|| argument.as_expression())
                .flatten()
        })
        .collect::<Option<_>>()?;
    let (name, deps, declare_fn) = match arguments.as_slice() {
        [deps, declare_fn] => (None, *deps, *declare_fn),
        [name, deps, declare_fn] => (Some(plain_string(name)?), *deps, *declare_fn),
        _ => return None,
    };
    let Expression::ArrayExpression(deps) = deps else {
        return None;
    };
    let deps: Vec<&str> = deps
        .elements
        .iter()
        .map(|element| element.as_expression().and_then(plain_string))
        .collect::<Option<_>>()?;
    let declare = declare(declare_fn)?;
    (declare.setters.len() <= deps.len()).then_some(Register {
        name,
        deps,
        declare,
    })
}

fn register_list<'k, 'a>(frame: &Frame<'k, 'a>) -> Option<(&'k [Statement<'a>], bool)> {
    let top: Vec<&Statement<'a>> = frame
        .top
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)))
        .collect();
    if top
        .iter()
        .all(|statement| register_call(statement).is_some())
    {
        return Some((frame.top, false));
    }
    let wrapper = frame.wrapper.as_ref()?;
    let bare = matches!(wrapper.shape, Shape::Iife | Shape::Call)
        && wrapper.function.params.items.is_empty()
        && wrapper.function.params.rest.is_none()
        && wrapper
            .arguments
            .iter()
            .all(|argument| matches!(argument, Argument::ThisExpression(_)));
    (bare && top.len() == 1)
        .then_some((wrapper.body(), is_strict(&wrapper.function.body.directives)))
}

pub fn is_plain(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.starts_with('/') && !name.contains(':')
}

fn is_external(name: &str) -> bool {
    is_plain(name) || name.strip_prefix("node:").is_some_and(is_plain)
}

fn deps_closed(registers: &[Register<'_, '_>], names: &FxHashSet<&str>) -> bool {
    registers.iter().all(|register| {
        let mut seen = FxHashSet::default();
        let unique = register.deps.iter().all(|dep| seen.insert(*dep));
        let external_ok = register
            .deps
            .iter()
            .zip(
                register
                    .declare
                    .setters
                    .iter()
                    .map(Some)
                    .chain(std::iter::repeat(None)),
            )
            .filter(|(dep, _)| !names.contains(*dep))
            .all(|(dep, setter)| {
                is_external(dep)
                    && setter.is_none_or(|binds| {
                        binds
                            .iter()
                            .all(|bind| matches!(bind, Bind::Import("default", _)))
                    })
            });
        unique && external_ok
    })
}

fn acyclic(registers: &[Register<'_, '_>], live: &[usize]) -> bool {
    let index: FxHashMap<&str, usize> = live
        .iter()
        .filter_map(|&at| registers[at].name.map(|name| (name, at)))
        .collect();
    let mut state = vec![0u8; registers.len()];
    for &root in live {
        if state[root] != 0 {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        state[root] = 1;
        while let Some((node, next)) = stack.pop() {
            let deps = &registers[node].deps;
            let Some(dep) = deps.get(next) else {
                state[node] = 2;
                continue;
            };
            stack.push((node, next + 1));
            let Some(&target) = index.get(dep) else {
                continue;
            };
            match state[target] {
                0 => {
                    state[target] = 1;
                    stack.push((target, 0));
                }
                1 => return false,
                _ => {}
            }
        }
    }
    true
}

pub fn layout<'k, 'a>(frame: &Frame<'k, 'a>, program: &'k Program<'a>) -> Option<Layout<'k, 'a>> {
    let (list, wrapper_strict) = register_list(frame)?;
    let registers: Vec<Register<'k, 'a>> = list
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)))
        .map(|statement| register(register_call(statement)?))
        .collect::<Option<_>>()?;
    let names = registers.iter().filter_map(|r| r.name);
    let mut seen = FxHashSet::default();
    if !names
        .clone()
        .all(|name| is_plain(name) && seen.insert(name))
    {
        return None;
    }
    let anonymous: Vec<usize> = (0..registers.len())
        .filter(|&at| registers[at].name.is_none())
        .collect();
    let entry = match registers.iter().position(|r| r.name.is_some()) {
        Some(first) => first,
        None => *anonymous.first()?,
    };
    if anonymous.len() > 1 || !deps_closed(&registers, &seen) {
        return None;
    }
    let live: Vec<usize> = (0..registers.len())
        .filter(|&at| registers[at].name.is_some() || at == entry)
        .collect();
    if !acyclic(&registers, &live) {
        return None;
    }
    let strict = match (is_strict(&program.directives), wrapper_strict) {
        (true, _) => &program.directives[..],
        (false, true) => frame
            .wrapper
            .as_ref()
            .map_or(&[][..], |w| &w.function.body.directives[..]),
        (false, false) => &[][..],
    };
    let all_strict = live
        .iter()
        .all(|&at| !strict.is_empty() || is_strict(registers[at].declare.directives));
    all_strict.then_some(Layout {
        registers,
        live,
        entry,
        strict,
    })
}
