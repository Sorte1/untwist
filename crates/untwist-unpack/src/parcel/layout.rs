use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};
use oxc_syntax::scope::ScopeId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::{member_parts, prop_name};

use super::callee::{self, is_global_name, Callee};
use super::{runtime, ENTRY_ID};
use crate::frame::{Frame, Shape};
use crate::matcher::{
    argument_expression, expression_statement, unreassigned_var, unwrap_statement_expression,
};
use crate::model::BundleKind;
use crate::runtime::runtime_id;

pub const NAME_PREFIX: &str = "parcelRequire";
pub const REGISTRY_PARAMS: usize = 4;
pub const HOISTED_PARAMS: usize = 2;
const MAX_RUNTIME_ARGUMENTS: usize = 8;
const REGISTER: &str = "register";
const UNUSED: usize = usize::MAX;
const PROTO_KEY: &str = "__proto__";

#[derive(Clone, Copy)]
pub struct DefFunction<'k, 'a> {
    pub params: &'k FormalParameters<'a>,
    pub body: &'k FunctionBody<'a>,
    pub scope: &'k Cell<Option<ScopeId>>,
    pub span: Span,
    pub arrow: bool,
}

pub struct Def<'k, 'a> {
    pub id: String,
    pub function: DefFunction<'k, 'a>,
    pub deps: Vec<(String, String)>,
    pub at: usize,
    pub element: Option<usize>,
}

impl<'k, 'a> Def<'k, 'a> {
    pub fn body(&self) -> Option<&'k FunctionBody<'a>> {
        Some(self.function.body)
    }
}

pub struct Registry<'k, 'a> {
    pub legacy: bool,
    pub defs: Vec<Def<'k, 'a>>,
    pub entries: Vec<String>,
    pub main: Option<String>,
    pub global: String,
    pub externals: bool,
}

pub struct Hoisted<'k, 'a> {
    pub defs: Vec<Def<'k, 'a>>,
    pub require: Option<&'k BindingIdentifier<'a>>,
    pub register: Option<&'k BindingIdentifier<'a>>,
    pub prelude: [usize; 3],
    pub definition: Option<usize>,
    pub first: usize,
    pub global: String,
    pub object: Option<String>,
    pub stores: Option<[String; 2]>,
}

impl Hoisted<'_, '_> {
    pub fn require_name(&self) -> &str {
        self.require.map_or("", |id| id.name.as_str())
    }
}

impl<'k, 'a> Hoisted<'k, 'a> {
    pub fn defined(&self) -> bool {
        self.definition.is_some()
    }

    pub fn defs_at(&self, index: usize) -> &[Def<'k, 'a>] {
        let start = self.defs.partition_point(|def| def.at < index);
        let end = self.defs.partition_point(|def| def.at <= index);
        &self.defs[start..end]
    }
}

pub enum Layout<'k, 'a> {
    Registry(Registry<'k, 'a>),
    Hoisted(Hoisted<'k, 'a>),
}

fn registry_arguments<'k, 'a>(frame: &Frame<'k, 'a>) -> Option<&'k [Argument<'a>]> {
    let wrapper = frame.wrapper.as_ref()?;
    if frame.top.len() != 1 || wrapper.shape != Shape::Iife {
        return None;
    }
    let arguments = wrapper.arguments;
    if !(REGISTRY_PARAMS..=MAX_RUNTIME_ARGUMENTS).contains(&arguments.len()) {
        return None;
    }
    let named = matches!(
        argument_expression(&arguments[3]),
        Some(Expression::StringLiteral(name)) if name.value.as_str().is_some_and(is_global_name)
    );
    let modules = matches!(
        argument_expression(&arguments[0]),
        Some(Expression::ObjectExpression(_))
    );
    (named && modules).then_some(arguments)
}

fn runtime_function<'k, 'a>(frame: &Frame<'k, 'a>) -> Option<&'k Function<'a>> {
    let wrapper = frame.wrapper.as_ref()?;
    let expression = unwrap_statement_expression(expression_statement(&frame.top[wrapper.index])?);
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    match call.callee.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

fn global_member(init: &Expression<'_>) -> Option<(String, Option<String>)> {
    if init
        .as_member_expression()
        .is_some_and(|member| member.optional())
    {
        return None;
    }
    let (object, name) = member_parts(init)?;
    let object = match object.without_parentheses() {
        Expression::Identifier(object) => Some(object.name.to_string()),
        other if callee::global_chain(other) => None,
        _ => return None,
    };
    is_global_name(&name).then(|| (name.into_owned(), object))
}

fn require_declarator<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(&'k BindingIdentifier<'a>, (String, Option<String>))> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    if !unreassigned_var(declaration.kind) {
        return None;
    }
    declaration.declarations.iter().find_map(|declarator| {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        Some((&**id, global_member(declarator.init.as_ref()?)?))
    })
}

fn global_register(statement: &Statement<'_>) -> bool {
    let Some(Expression::CallExpression(call)) = expression_statement(statement) else {
        return false;
    };
    callee::registers(&call.callee, Callee::Global).is_some()
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    registry_arguments(frame).is_some()
        || frame
            .top
            .iter()
            .any(|s| require_declarator(s).is_some() || global_register(s))
}

fn string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
        _ => None,
    }
}

fn plain_property<'k, 'a>(property: &'k ObjectPropertyKind<'a>) -> Option<&'k ObjectProperty<'a>> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    let plain = !property.computed
        && !property.method
        && !property.shorthand
        && property.kind == PropertyKind::Init;
    plain.then_some(&**property)
}

fn dependencies(object: &ObjectExpression<'_>) -> Option<Vec<(String, String)>> {
    let mut deps = object
        .properties
        .iter()
        .map(|property| {
            let property = plain_property(property)?;
            let key = prop_name(&property.key)?;
            if key == PROTO_KEY {
                return None;
            }
            Some((key.into_owned(), string(&property.value)?))
        })
        .collect::<Option<Vec<_>>>()?;
    deps.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let distinct = deps.windows(2).all(|pair| pair[0].0 != pair[1].0);
    distinct.then_some(deps)
}

fn simple_params(params: &FormalParameters<'_>, most: usize) -> bool {
    params.rest.is_none()
        && params.items.len() <= most
        && params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        })
}

pub fn simple_factory<'k, 'a>(
    expr: &'k Expression<'a>,
    most: usize,
) -> Option<DefFunction<'k, 'a>> {
    match expr {
        Expression::FunctionExpression(function) => {
            let plain = function.id.is_none() && !function.r#async && !function.generator;
            (plain && simple_params(&function.params, most)).then_some(())?;
            Some(DefFunction {
                params: &function.params,
                body: function.body.as_deref()?,
                scope: &function.scope_id,
                span: function.span,
                arrow: false,
            })
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            (!arrow.r#async && simple_params(&arrow.params, most)).then_some(())?;
            Some(DefFunction {
                params: &arrow.params,
                body,
                scope: &arrow.scope_id,
                span: arrow.span,
                arrow: true,
            })
        }
        _ => None,
    }
}

fn registry_def<'k, 'a>(property: &'k ObjectPropertyKind<'a>) -> Option<Def<'k, 'a>> {
    let property = plain_property(property)?;
    let Expression::ArrayExpression(record) = &property.value else {
        return None;
    };
    let [factory, deps] = record.elements.as_slice() else {
        return None;
    };
    let function = simple_factory(factory.as_expression()?, REGISTRY_PARAMS)?;
    let Expression::ObjectExpression(deps) = deps.as_expression()? else {
        return None;
    };
    Some(Def {
        id: prop_name(&property.key)?.into_owned(),
        function,
        deps: dependencies(deps)?,
        at: 0,
        element: None,
    })
}

fn inert(argument: &Argument<'_>) -> bool {
    matches!(
        argument_expression(argument),
        Some(
            Expression::StringLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NumericLiteral(_)
        )
    )
}

fn unique(defs: &[Def<'_, '_>], reserved: &[&str]) -> bool {
    let mut seen = FxHashSet::default();
    defs.iter()
        .all(|def| !reserved.contains(&def.id.as_str()) && seen.insert(def.id.as_str()))
}

fn registry<'k, 'a>(arguments: &'k [Argument<'a>], legacy: bool) -> Option<Registry<'k, 'a>> {
    if legacy && arguments.len() != REGISTRY_PARAMS {
        return None;
    }
    let global = string(argument_expression(&arguments[3])?)?;
    let Some(Expression::ObjectExpression(modules)) = argument_expression(&arguments[0]) else {
        return None;
    };
    let defs: Vec<Def<'k, 'a>> = modules
        .properties
        .iter()
        .map(registry_def)
        .collect::<Option<_>>()?;
    let Some(Expression::ArrayExpression(entries)) = argument_expression(&arguments[1]) else {
        return None;
    };
    let entries: Vec<String> = entries
        .elements
        .iter()
        .map(|element| string(element.as_expression()?))
        .collect::<Option<_>>()?;
    let main = match argument_expression(&arguments[2])? {
        Expression::NullLiteral(_) => None,
        other => Some(string(other)?),
    };
    let externals = arguments.get(4).is_none_or(|argument| {
        matches!(argument_expression(argument), Some(Expression::ObjectExpression(object)) if object.properties.is_empty())
    });
    let rest = arguments.iter().skip(5).all(inert);
    let runtime = runtime_id(BundleKind::Parcel);
    let most = if legacy {
        REGISTRY_PARAMS - 1
    } else {
        REGISTRY_PARAMS
    };
    let fitting = defs
        .iter()
        .all(|def| def.function.params.items.len() <= most);
    let sound = externals
        && rest
        && fitting
        && !defs.is_empty()
        && !entries.is_empty()
        && unique(&defs, &[runtime.as_str()]);
    sound.then_some(Registry {
        legacy,
        defs,
        entries,
        main,
        global,
        externals: arguments.len() > 4,
    })
}

fn is_identifier(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn null_test(expr: &Expression<'_>, name: &str) -> bool {
    let Expression::BinaryExpression(test) = expr.without_parentheses() else {
        return false;
    };
    let null =
        |side: &Expression<'_>| matches!(side.without_parentheses(), Expression::NullLiteral(_));
    test.operator == BinaryOperator::Equality
        && ((is_identifier(&test.left, name) && null(&test.right))
            || (null(&test.left) && is_identifier(&test.right, name)))
}

fn guarded(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr, Expression::LogicalExpression(logical)
        if logical.operator == LogicalOperator::And && null_test(&logical.left, name))
}

pub fn definition_head<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match expression_statement(statement)? {
        Expression::SequenceExpression(sequence) => sequence.expressions.first(),
        _ => None,
    }
}

fn is_definition(statement: &Statement<'_>, name: &str) -> bool {
    match statement {
        Statement::IfStatement(branch) => {
            branch.alternate.is_none() && null_test(&branch.test, name)
        }
        Statement::ExpressionStatement(statement) => match &statement.expression {
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .first()
                .is_some_and(|head| guarded(head, name)),
            other => guarded(other, name),
        },
        _ => false,
    }
}

fn register_declarator<'k, 'a>(
    statement: &'k Statement<'a>,
    require: &str,
) -> Option<&'k BindingIdentifier<'a>> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Some(Expression::StaticMemberExpression(member)) = &declarator.init else {
        return None;
    };
    let fits = unreassigned_var(declaration.kind)
        && !member.optional
        && member.property.name == REGISTER
        && is_identifier(&member.object, require);
    fits.then_some(&**id)
}

type Registered<'k, 'a> = (String, DefFunction<'k, 'a>, Option<String>);

fn register_call<'k, 'a>(expr: &'k Expression<'a>, how: Callee<'_>) -> Option<Registered<'k, 'a>> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    if call.optional {
        return None;
    }
    let key = callee::registers(&call.callee, how)?;
    let [id, factory] = call.arguments.as_slice() else {
        return None;
    };
    let id = string(argument_expression(id)?)?;
    Some((
        id,
        simple_factory(argument_expression(factory)?, HOISTED_PARAMS)?,
        key,
    ))
}

fn inert_init(init: Option<&Expression<'_>>) -> bool {
    let Some(init) = init else {
        return true;
    };
    match init {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::Identifier(_) => true,
        Expression::ObjectExpression(object) => object.properties.is_empty(),
        Expression::ConditionalExpression(_) => callee::global_chain(init),
        Expression::CallExpression(call) => {
            !call.optional
                && is_identifier(&call.callee, "require")
                && matches!(call.arguments.as_slice(), [Argument::StringLiteral(_)])
        }
        _ => false,
    }
}

fn inert_statement(statement: &Statement<'_>, prelude: &[usize; 3], index: usize) -> bool {
    if prelude.contains(&index) {
        return true;
    }
    match statement {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .all(|declarator| inert_init(declarator.init.as_ref())),
        _ => false,
    }
}

struct Registers<'k, 'a, 'n> {
    how: Callee<'n>,
    defs: Vec<Def<'k, 'a>>,
    effects: bool,
    first: Option<usize>,
    key: Option<String>,
    keys_agree: bool,
}

impl<'k, 'a> Registers<'k, 'a, '_> {
    fn element(
        &mut self,
        expr: &'k Expression<'a>,
        at: usize,
        element: Option<usize>,
    ) -> Option<()> {
        let Some((id, function, key)) = register_call(expr, self.how) else {
            self.effects = true;
            return Some(());
        };
        if self.effects {
            return None;
        }
        if let Some(key) = key {
            self.keys_agree &= self.key.get_or_insert_with(|| key.clone()) == &key;
        }
        self.first.get_or_insert(at);
        self.defs.push(Def {
            id,
            function,
            deps: Vec::new(),
            at,
            element,
        });
        Some(())
    }

    fn statement(
        &mut self,
        statement: &'k Statement<'a>,
        at: usize,
        skip: Option<usize>,
    ) -> Option<()> {
        let Some(skip) = skip else {
            return Some(());
        };
        match expression_statement(statement) {
            Some(Expression::SequenceExpression(sequence)) => {
                for (index, expr) in sequence.expressions.iter().enumerate().skip(skip) {
                    self.element(expr, at, Some(index))?;
                }
                Some(())
            }
            Some(expr) => self.element(expr, at, None),
            None => {
                self.effects = true;
                Some(())
            }
        }
    }
}

fn at_most_one<'k, 'a, T>(
    top: &'k [Statement<'a>],
    found: impl Fn(&'k Statement<'a>) -> Option<T>,
) -> Option<Option<(usize, T)>> {
    let mut hits = top
        .iter()
        .enumerate()
        .filter_map(|(index, s)| found(s).map(|t| (index, t)));
    let first = hits.next();
    hits.next().is_none().then_some(first)
}

fn definition(top: &[Statement<'_>], name: &str) -> Option<Option<usize>> {
    let mut found = top
        .iter()
        .enumerate()
        .filter(|(_, statement)| is_definition(statement, name))
        .map(|(index, _)| index);
    match (found.next(), found.next()) {
        (first, None) => Some(first),
        _ => None,
    }
}

fn collect<'k, 'a>(
    top: &'k [Statement<'a>],
    how: Callee<'_>,
    prelude: &[usize; 3],
    merged: Option<usize>,
) -> Option<(Vec<Def<'k, 'a>>, usize, Option<String>)> {
    let mut registers = Registers {
        how,
        defs: Vec::new(),
        effects: false,
        first: None,
        key: None,
        keys_agree: true,
    };
    for (index, statement) in top.iter().enumerate() {
        let skip = if merged == Some(index) {
            Some(1)
        } else if inert_statement(statement, prelude, index) {
            None
        } else {
            Some(0)
        };
        registers.statement(statement, index, skip)?;
    }
    let first = registers.first?;
    let runtime = runtime_id(BundleKind::Parcel);
    let sound = registers.keys_agree && unique(&registers.defs, &[ENTRY_ID, runtime.as_str()]);
    sound.then_some((registers.defs, first, registers.key))
}

fn global_bound(top: &[Statement<'_>]) -> bool {
    top.iter().any(|statement| match statement {
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .any(|id| callee::GLOBAL_NAMES.contains(&id.name.as_str())),
        Statement::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .is_some_and(|id| callee::GLOBAL_NAMES.contains(&id.name.as_str())),
        Statement::ClassDeclaration(class) => class
            .id
            .as_ref()
            .is_some_and(|id| callee::GLOBAL_NAMES.contains(&id.name.as_str())),
        _ => false,
    })
}

fn detached<'k, 'a>(top: &'k [Statement<'a>]) -> Option<Hoisted<'k, 'a>> {
    if global_bound(top) {
        return None;
    }
    let prelude = [UNUSED; 3];
    let (defs, first, key) = collect(top, Callee::Global, &prelude, None)?;
    Some(Hoisted {
        defs,
        require: None,
        register: None,
        prelude,
        definition: None,
        first,
        global: key?,
        object: None,
        stores: None,
    })
}

fn hoisted<'k, 'a>(top: &'k [Statement<'a>], source_text: &str) -> Option<Hoisted<'k, 'a>> {
    let Some(found) = at_most_one(top, require_declarator)? else {
        return detached(top);
    };
    let (at_require, (require, (global, object))) = found;
    let name = require.name.as_str();
    let at_definition = definition(top, name)?;
    let merged = at_definition.filter(|&at| definition_head(&top[at]).is_some());
    let stores = match at_definition {
        Some(at) => Some(runtime::defined(
            &top[at],
            definition_head(&top[at]),
            [name, object.as_deref()?],
            &global,
            source_text,
        )?),
        None => None,
    };
    let between = at_definition.unwrap_or(at_require);
    if at_require > between || at_definition == Some(at_require) {
        return None;
    }
    let alias = at_most_one(top, |s| register_declarator(s, name))?;
    let (prelude, how, after) = match alias {
        Some((at_register, register)) => {
            if between >= at_register || merged.is_some() {
                return None;
            }
            (
                [at_require, between, at_register],
                Callee::Alias(register.name.as_str()),
                at_register + 1,
            )
        }
        None => (
            [at_require, between, between],
            Callee::Member(name),
            between,
        ),
    };
    let (defs, first, _) = collect(top, how, &prelude, merged)?;
    (first >= after).then_some(Hoisted {
        defs,
        require: Some(require),
        register: alias.map(|(_, register)| register),
        prelude,
        definition: at_definition,
        first,
        global,
        object,
        stores,
    })
}

impl Layout<'_, '_> {
    pub fn is_chunk(&self) -> bool {
        matches!(self, Layout::Hoisted(hoisted) if !hoisted.defined())
    }
}

pub fn layout<'k, 'a>(frame: &Frame<'k, 'a>, program: &'k Program<'a>) -> Option<Layout<'k, 'a>> {
    if !program.directives.is_empty() || frame.top.iter().any(Statement::is_module_declaration) {
        return None;
    }
    if let Some(arguments) = registry_arguments(frame) {
        let wrapper = frame.wrapper.as_ref()?;
        if !wrapper.function.body.directives.is_empty() {
            return None;
        }
        let function = runtime_function(frame)?;
        if arguments.len() > function.params.items.len() {
            return None;
        }
        let generation = runtime::proven(function, program.source_text)?;
        let legacy = generation == runtime::Generation::Legacy;
        return registry(arguments, legacy).map(Layout::Registry);
    }
    hoisted(frame.top, program.source_text).map(Layout::Hoisted)
}
