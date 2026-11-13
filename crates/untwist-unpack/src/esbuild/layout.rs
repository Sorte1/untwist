use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::prop_name;

use crate::esbuild::shape::{
    commonjs_call, export_helper, helper_shadows, is_object_get_own_property_names,
    is_object_member_chain, is_require_shim, lazy_helper, CallForm,
};
use crate::frame::{Frame, Shape};
use crate::matcher::{anonymous_factory, unreassigned_var, Factory};
use crate::model::BundleKind;
use crate::rolldown;

const MIN_LAZY_FACTORIES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Top,
    Iife,
    Function,
    Bun,
}

pub struct Scope<'k, 'a> {
    pub host: Host,
    pub list: &'k [Statement<'a>],
    pub directives: &'k [Directive<'a>],
    pub function: Option<Factory<'k, 'a>>,
    pub wrapper: Option<&'k Statement<'a>>,
}

pub struct FactoryDecl<'k, 'a> {
    pub statement: usize,
    pub declarator: usize,
    pub name: String,
    pub binding: Option<&'k BindingIdentifier<'a>>,
    pub inline: Option<Span>,
    pub key: Option<String>,
    pub function: Factory<'k, 'a>,
    pub scope: &'k Cell<Option<ScopeId>>,
}

pub struct Layout<'k, 'a> {
    pub kind: BundleKind,
    pub factories: Vec<FactoryDecl<'k, 'a>>,
    pub inline: Vec<FactoryDecl<'k, 'a>>,
    pub lazy_factories: usize,
    pub exports_helper: bool,
    pub prefix: Vec<usize>,
    pub shim: Option<&'k BindingIdentifier<'a>>,
    pub created_shim: bool,
    pub banner: Option<usize>,
    pub trusted: Vec<&'k BindingIdentifier<'a>>,
    pub unsound: bool,
    pub early: bool,
    pub first_effect: (usize, usize),
}

impl<'k, 'a> Layout<'k, 'a> {
    pub fn accepted(&self) -> bool {
        let all = self.factories.len() + self.lazy_factories;
        !self.factories.is_empty()
            || !self.inline.is_empty()
            || all >= MIN_LAZY_FACTORIES
            || (self.exports_helper && all >= 1)
    }

    pub fn all(&self) -> impl Iterator<Item = &FactoryDecl<'k, 'a>> {
        self.factories.iter().chain(&self.inline)
    }

    pub fn is_factory_statement(&self, index: usize) -> bool {
        self.factories
            .binary_search_by_key(&index, |factory| factory.statement)
            .is_ok()
    }

    pub fn factories_at(&self, index: usize) -> &[FactoryDecl<'_, '_>] {
        let start = self
            .factories
            .partition_point(|factory| factory.statement < index);
        let rest = &self.factories[start..];
        &rest[..rest.partition_point(|factory| factory.statement == index)]
    }
}

fn only_empty_besides(top: &[Statement<'_>], index: usize) -> bool {
    top.iter()
        .enumerate()
        .all(|(at, statement)| at == index || matches!(statement, Statement::EmptyStatement(_)))
}

fn unwrappable_iife(statement: &Statement<'_>) -> bool {
    let Statement::ExpressionStatement(statement) = statement else {
        return false;
    };
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return false;
    };
    if call.optional || !call.arguments.is_empty() {
        return false;
    }
    matches!(call.callee.without_parentheses(), Expression::ArrowFunctionExpression(arrow)
        if !arrow.r#async && arrow.params.items.is_empty() && arrow.params.rest.is_none()
            && arrow.body.is_function_body())
}

fn bare_function(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::ExpressionStatement(statement)
        if matches!(statement.expression.without_parentheses(), Expression::FunctionExpression(_)))
}

pub fn wrapped<'k, 'a>(frame: &Frame<'k, 'a>) -> Option<Scope<'k, 'a>> {
    let wrapper = frame.wrapper.as_ref()?;
    let statement = frame.top.get(wrapper.index)?;
    let host = match wrapper.shape {
        Shape::Iife if unwrappable_iife(statement) => Host::Iife,
        Shape::Iife if rolldown::shape::function_iife(statement) => Host::Function,
        Shape::Bun if bare_function(statement) => Host::Bun,
        _ => return None,
    };
    if !only_empty_besides(frame.top, wrapper.index) {
        return None;
    }
    Some(Scope {
        host,
        list: wrapper.body(),
        directives: &wrapper.function.body.directives,
        function: Some(wrapper.function),
        wrapper: Some(statement),
    })
}

pub fn scopes<'k, 'a>(
    frame: &Frame<'k, 'a>,
    directives: &'k [Directive<'a>],
) -> impl Iterator<Item = Scope<'k, 'a>> {
    let top = Scope {
        host: Host::Top,
        list: frame.top,
        directives,
        function: None,
        wrapper: None,
    };
    wrapped(frame).into_iter().chain(std::iter::once(top))
}

struct Lazies<'a> {
    rolldown: bool,
    commonjs: FxHashMap<&'a str, CallForm>,
    esm: FxHashSet<&'a str>,
    getters: FxHashSet<&'a str>,
    exports_helper: bool,
    shadowed: bool,
}

impl Lazies<'_> {
    fn trusts(&self, name: &str) -> bool {
        self.commonjs.contains_key(name) || self.esm.contains(name) || self.getters.contains(name)
    }
}

fn declarators<'k, 'a>(
    list: &'k [Statement<'a>],
) -> impl Iterator<
    Item = (
        usize,
        usize,
        &'k BindingIdentifier<'a>,
        Option<&'k Expression<'a>>,
    ),
> {
    list.iter().enumerate().flat_map(|(index, statement)| {
        let declarations: &[VariableDeclarator<'a>] = match statement {
            Statement::VariableDeclaration(declaration) if unreassigned_var(declaration.kind) => {
                &declaration.declarations
            }
            _ => &[],
        };
        declarations
            .iter()
            .enumerate()
            .filter_map(move |(at, declarator)| match &declarator.id {
                BindingPattern::BindingIdentifier(id) => {
                    Some((index, at, &**id, declarator.init.as_ref()))
                }
                _ => None,
            })
    })
}

fn names_getters<'a>(list: &[Statement<'a>]) -> FxHashSet<&'a str> {
    declarators(list)
        .filter(|(.., init)| init.is_some_and(is_object_get_own_property_names))
        .map(|(_, _, id, _)| id.name.as_str())
        .collect()
}

fn lazies<'a>(list: &[Statement<'a>]) -> Lazies<'a> {
    let mut found = Lazies {
        rolldown: false,
        commonjs: FxHashMap::default(),
        esm: FxHashSet::default(),
        getters: FxHashSet::default(),
        exports_helper: false,
        shadowed: false,
    };
    let getters = names_getters(list);
    for (_, _, id, init) in declarators(list) {
        let Some(init) = init else { continue };
        if export_helper(init) {
            found.exports_helper = true;
            continue;
        }
        let Some((params, inner)) = lazy_helper(init) else {
            continue;
        };
        let proven = commonjs_call(&params, &inner).filter(|(form, getter)| match form {
            CallForm::Direct => true,
            CallForm::FirstKey => getter.is_some_and(|getter| getters.contains(getter)),
        });
        let rolldown = proven.is_none() && rolldown::shape::commonjs_min(&params, &inner);
        found.rolldown |= rolldown;
        let proven = proven.or(rolldown.then_some((CallForm::Direct, None)));
        match proven {
            Some((form, getter)) => {
                found.shadowed |= helper_shadows(init, &params, getter);
                found.commonjs.insert(id.name.as_str(), form);
                found
                    .getters
                    .extend(getter.filter(|_| form == CallForm::FirstKey));
            }
            None => {
                found.esm.insert(id.name.as_str());
            }
        }
    }
    found
}

fn callee_of<'k, 'a>(init: &'k Expression<'a>) -> Option<(&'a str, &'k Argument<'a>)> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    match (&call.arguments[..], call.optional) {
        ([argument], false) => Some((callee.name.as_str(), argument)),
        _ => None,
    }
}

type Scoped<'k, 'a> = (Factory<'k, 'a>, &'k Cell<Option<ScopeId>>);

fn block_factory<'k, 'a>(expr: &'k Expression<'a>) -> Option<Scoped<'k, 'a>> {
    let scope = match expr {
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            &function.scope_id
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => &arrow.scope_id,
        _ => return None,
    };
    anonymous_factory(expr).map(|factory| (factory, scope))
}

fn keyed_factory<'k, 'a>(argument: &'k Argument<'a>) -> Option<(String, Scoped<'k, 'a>)> {
    let Argument::ObjectExpression(object) = argument else {
        return None;
    };
    let [ObjectPropertyKind::ObjectProperty(property)] = &object.properties[..] else {
        return None;
    };
    if property.computed || property.kind != PropertyKind::Init {
        return None;
    }
    let key = prop_name(&property.key)?.into_owned();
    Some((key, block_factory(&property.value)?))
}

fn factory_of<'k, 'a>(
    form: CallForm,
    argument: &'k Argument<'a>,
) -> Option<(Option<String>, Scoped<'k, 'a>)> {
    match form {
        CallForm::FirstKey => keyed_factory(argument).map(|(key, factory)| (Some(key), factory)),
        CallForm::Direct => block_factory(argument.as_expression()?).map(|factory| (None, factory)),
    }
}

fn fits_factory_params(factory: &Factory<'_, '_>) -> bool {
    crate::esbuild::shape::simple_params(factory.params).is_some_and(|params| params.len() <= 2)
}

struct Found<'k, 'a> {
    factories: Vec<FactoryDecl<'k, 'a>>,
    lazy: usize,
    proto_key: bool,
}

fn factories<'k, 'a>(list: &'k [Statement<'a>], lazies: &Lazies<'a>) -> Option<Found<'k, 'a>> {
    let mut found = Vec::new();
    let mut lazy = 0;
    let mut proto_key = false;
    for (statement, declarator, binding, init) in declarators(list) {
        let Some((callee, argument)) = init.and_then(callee_of) else {
            continue;
        };
        if lazies.esm.contains(&callee) {
            lazy += 1;
            continue;
        }
        let Some(&form) = lazies.commonjs.get(callee) else {
            continue;
        };
        let (key, (function, scope)) = factory_of(form, argument)?;
        if !fits_factory_params(&function) {
            return None;
        }
        proto_key |= key.as_deref() == Some("__proto__");
        found.push(FactoryDecl {
            statement,
            declarator,
            name: binding.name.to_string(),
            binding: Some(binding),
            inline: None,
            key,
            function,
            scope,
        });
    }
    Some(Found {
        factories: found,
        lazy,
        proto_key,
    })
}

pub fn require_creators<'a>(list: &[Statement<'a>]) -> FxHashSet<&'a str> {
    list.iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import) => Some(import),
            _ => None,
        })
        .filter(|import| matches!(import.source.value.as_str(), Some("module" | "node:module")))
        .flat_map(|import| import.specifiers.iter().flatten())
        .filter_map(|specifier| match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(named)
                if named.imported.name() == "createRequire" =>
            {
                Some(named.local.name.as_str())
            }
            _ => None,
        })
        .collect()
}

fn creates_require(list: &[Statement<'_>], callee: &str) -> bool {
    list.iter().any(|statement| {
        let Statement::ImportDeclaration(import) = statement else {
            return false;
        };
        let from_module = matches!(import.source.value.as_str(), Some("module" | "node:module"));
        from_module
            && import.specifiers.iter().flatten().any(|specifier| {
                matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(named)
                    if named.local.name == callee
                        && named.imported.name() == "createRequire")
            })
    })
}

fn banner_of(list: &[Statement<'_>]) -> Option<usize> {
    list.iter().position(|statement| {
        banner_callee(statement).is_some_and(|callee| creates_require(list, callee))
    })
}

pub fn is_banner(statement: &Statement<'_>) -> bool {
    banner_callee(statement).is_some()
}

fn banner_callee<'a>(statement: &Statement<'a>) -> Option<&'a str> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = &declaration.declarations[..] else {
        return None;
    };
    let named_require = matches!(&declarator.id,
        BindingPattern::BindingIdentifier(id) if id.name == "require");
    if declaration.kind != VariableDeclarationKind::Const || !named_require {
        return None;
    }
    let Expression::CallExpression(call) = declarator.init.as_ref()?.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    let at_url = matches!(&call.arguments[..], [Argument::StaticMemberExpression(url)]
        if url.property.name == "url"
            && matches!(&url.object, Expression::ImportMeta(_)));
    (at_url && !call.optional).then_some(callee.name.as_str())
}

fn effect_free(init: Option<&Expression<'_>>, lazy_callees: &Callees<'_>) -> bool {
    let Some(init) = init else {
        return true;
    };
    match init.without_parentheses() {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => true,
        other if is_object_member_chain(other) || is_require_shim(other) => true,
        other if rolldown::shape::require_shim(&lazy_callees.creators, other) => true,
        other => callee_of(other).is_some_and(|(callee, argument)| {
            lazy_callees.lazy.contains(&callee) && is_function_literal(argument)
        }),
    }
}

fn is_function(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    )
}

fn is_function_literal(argument: &Argument<'_>) -> bool {
    let Some(expression) = argument.as_expression() else {
        return false;
    };
    match expression.without_parentheses() {
        Expression::ObjectExpression(object) => matches!(&object.properties[..],
            [ObjectPropertyKind::ObjectProperty(property)]
                if !property.computed
                    && property.kind == PropertyKind::Init
                    && is_function(&property.value)),
        other => is_function(other),
    }
}

fn prefix_statement(statement: &Statement<'_>, lazy_callees: &Callees<'_>) -> Option<bool> {
    match statement {
        Statement::FunctionDeclaration(_) => Some(true),
        other if is_banner(other) => Some(false),
        Statement::VariableDeclaration(declaration) if unreassigned_var(declaration.kind) => {
            let pure = declaration.declarations.iter().all(|declarator| {
                matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                    && effect_free(declarator.init.as_ref(), lazy_callees)
            });
            pure.then_some(true)
        }
        Statement::ImportDeclaration(_) | Statement::EmptyStatement(_) => Some(false),
        _ => None,
    }
}

pub struct Callees<'a> {
    lazy: FxHashSet<&'a str>,
    creators: FxHashSet<&'a str>,
}

fn lazy_callees<'a>(list: &[Statement<'a>], lazies: &Lazies<'a>) -> Callees<'a> {
    let lazy = lazies
        .commonjs
        .keys()
        .copied()
        .chain(lazies.esm.iter().copied())
        .collect();
    Callees {
        lazy,
        creators: require_creators(list),
    }
}

fn defined_before_any_effect(
    list: &[Statement<'_>],
    lazy_callees: &Callees<'_>,
    factories: &[FactoryDecl<'_, '_>],
) -> bool {
    let Some(last) = factories.last() else {
        return true;
    };
    let Some(Statement::VariableDeclaration(declaration)) = list.get(last.statement) else {
        return false;
    };
    let quiet_before = list[..last.statement]
        .iter()
        .all(|statement| quiet_statement(statement, lazy_callees));
    quiet_before
        && declaration.declarations[..last.declarator]
            .iter()
            .all(|declarator| quiet_init(declarator.init.as_ref(), lazy_callees))
}

fn inert_value(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(inert_value)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if !property.computed
                    && property.kind == PropertyKind::Init
                    && inert_value(&property.value))
        }),
        _ => false,
    }
}

fn quiet_init(init: Option<&Expression<'_>>, lazy_callees: &Callees<'_>) -> bool {
    init.is_some_and(inert_value) || effect_free(init, lazy_callees)
}

fn first_effect(list: &[Statement<'_>], lazy_callees: &Callees<'_>) -> (usize, usize) {
    let Some(index) = list
        .iter()
        .position(|statement| !quiet_statement(statement, lazy_callees))
    else {
        return (list.len(), 0);
    };
    let declarator = match &list[index] {
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .position(|declarator| {
                !matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                    || !quiet_init(declarator.init.as_ref(), lazy_callees)
            })
            .unwrap_or(0),
        _ => 0,
    };
    (index, declarator)
}

fn quiet_statement(statement: &Statement<'_>, lazy_callees: &Callees<'_>) -> bool {
    if prefix_statement(statement, lazy_callees).is_some() {
        return true;
    }
    matches!(statement, Statement::VariableDeclaration(declaration)
    if declaration.declarations.iter().all(|declarator| {
            matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                && quiet_init(declarator.init.as_ref(), lazy_callees)
        }))
}

fn prefix(
    list: &[Statement<'_>],
    lazy_callees: &Callees<'_>,
    layout: &Layout<'_, '_>,
) -> Vec<usize> {
    let mut found = Vec::new();
    for (index, statement) in list.iter().enumerate() {
        match prefix_statement(statement, lazy_callees) {
            Some(true) if !layout.is_factory_statement(index) => found.push(index),
            Some(_) => {}
            None => break,
        }
    }
    found
}

fn shim<'k, 'a>(
    list: &'k [Statement<'a>],
    prefix: &[usize],
    creators: &FxHashSet<&str>,
) -> Option<(&'k BindingIdentifier<'a>, bool)> {
    declarators(list)
        .filter(|(statement, ..)| prefix.binary_search(statement).is_ok())
        .find_map(|(_, _, id, init)| {
            let init = init?;
            let created = rolldown::shape::require_shim(creators, init);
            (created || is_require_shim(init)).then_some((id, created))
        })
}

fn trusted<'k, 'a>(
    list: &'k [Statement<'a>],
    lazies: &Lazies<'_>,
) -> Vec<&'k BindingIdentifier<'a>> {
    declarators(list)
        .map(|(_, _, id, _)| id)
        .filter(|id| lazies.trusts(id.name.as_str()))
        .collect()
}

pub const INLINE_PREFIX: &str = "__untwist_inline_require_";

fn inline_factories<'k, 'a>(
    sites: Vec<rolldown::inline::InlineSite<'k, 'a>>,
) -> Vec<FactoryDecl<'k, 'a>> {
    sites
        .into_iter()
        .enumerate()
        .map(|(index, site)| FactoryDecl {
            statement: site.statement,
            declarator: site.declarator,
            name: format!("{INLINE_PREFIX}{index}"),
            binding: None,
            inline: Some(site.span),
            key: site
                .hint
                .map(|hint| hint.strip_prefix("import_").unwrap_or(hint).to_string()),
            function: site.function,
            scope: site.scope,
        })
        .collect()
}

pub fn layout<'k, 'a>(list: &'k [Statement<'a>]) -> Option<Layout<'k, 'a>> {
    let lazies = lazies(list);
    let direct: FxHashSet<&str> = lazies
        .commonjs
        .iter()
        .filter(|(_, form)| **form == CallForm::Direct)
        .map(|(name, _)| *name)
        .collect();
    let sites = rolldown::inline::inline_sites(list, &direct);
    if lazies.commonjs.is_empty() && lazies.esm.is_empty() && sites.is_empty() {
        return None;
    }
    let found = factories(list, &lazies)?;
    let lazy_callees = lazy_callees(list, &lazies);
    let early = !defined_before_any_effect(list, &lazy_callees, &found.factories);
    let kind = match lazies.rolldown || !sites.is_empty() {
        true => BundleKind::Rolldown,
        false => BundleKind::Esbuild,
    };
    let mut layout = Layout {
        kind,
        unsound: lazies.shadowed || found.proto_key,
        early,
        first_effect: first_effect(list, &lazy_callees),
        factories: found.factories,
        inline: inline_factories(sites),
        lazy_factories: found.lazy,
        exports_helper: lazies.exports_helper,
        prefix: Vec::new(),
        shim: None,
        created_shim: false,
        banner: banner_of(list),
        trusted: trusted(list, &lazies),
    };
    if !layout.accepted() {
        return None;
    }
    layout.prefix = prefix(list, &lazy_callees, &layout);
    let found = shim(list, &layout.prefix, &lazy_callees.creators);
    layout.shim = found.map(|(id, _)| id);
    layout.created_shim = found.is_some_and(|(_, created)| created);
    Some(layout)
}

pub fn statement_span(list: &[Statement<'_>], index: usize) -> Span {
    list.get(index).map_or(oxc_span::SPAN, GetSpan::span)
}
