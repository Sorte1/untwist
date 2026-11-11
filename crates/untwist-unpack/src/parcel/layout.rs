use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::{member_parts, prop_name};

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
const PROTO_KEY: &str = "__proto__";

pub struct Def<'k, 'a> {
    pub id: String,
    pub function: &'k Function<'a>,
    pub deps: Vec<(String, String)>,
    pub at: usize,
    pub element: Option<usize>,
}

impl<'k, 'a> Def<'k, 'a> {
    pub fn body(&self) -> Option<&'k FunctionBody<'a>> {
        self.function.body.as_deref()
    }
}

pub struct Registry<'k, 'a> {
    pub defs: Vec<Def<'k, 'a>>,
    pub entries: Vec<String>,
    pub main: Option<String>,
    pub global: String,
    pub externals: bool,
}

pub struct Hoisted<'k, 'a> {
    pub defs: Vec<Def<'k, 'a>>,
    pub require: &'k BindingIdentifier<'a>,
    pub register: &'k BindingIdentifier<'a>,
    pub prelude: [usize; 3],
    pub definition: Option<usize>,
    pub first: usize,
    pub global: String,
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

fn is_global_name(text: &str) -> bool {
    text.len() > NAME_PREFIX.len() && text.starts_with(NAME_PREFIX)
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

fn global_member(init: &Expression<'_>) -> Option<String> {
    let (object, name) = member_parts(init)?;
    (matches!(object, Expression::Identifier(_)) && is_global_name(&name))
        .then(|| name.into_owned())
}

fn require_declarator<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(&'k BindingIdentifier<'a>, String)> {
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

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    registry_arguments(frame).is_some() || frame.top.iter().any(|s| require_declarator(s).is_some())
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

pub fn simple_factory<'k, 'a>(expr: &'k Expression<'a>, most: usize) -> Option<&'k Function<'a>> {
    let Expression::FunctionExpression(function) = expr else {
        return None;
    };
    let params = &function.params;
    let simple = params.rest.is_none()
        && params.items.len() <= most
        && params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        });
    let plain = function.id.is_none() && !function.r#async && !function.generator;
    (simple && plain && function.body.is_some()).then_some(&**function)
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

fn registry<'k, 'a>(arguments: &'k [Argument<'a>]) -> Option<Registry<'k, 'a>> {
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
    let sound = externals
        && rest
        && !defs.is_empty()
        && !entries.is_empty()
        && unique(&defs, &[runtime.as_str()]);
    sound.then_some(Registry {
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

fn is_definition(statement: &Statement<'_>, name: &str) -> bool {
    match statement {
        Statement::IfStatement(branch) => {
            branch.alternate.is_none() && null_test(&branch.test, name)
        }
        Statement::ExpressionStatement(statement) => matches!(
            &statement.expression,
            Expression::LogicalExpression(logical)
                if logical.operator == LogicalOperator::And && null_test(&logical.left, name)
        ),
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

fn register_call<'k, 'a>(
    expr: &'k Expression<'a>,
    register: &str,
) -> Option<(String, &'k Function<'a>)> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    if call.optional || !is_identifier(&call.callee, register) {
        return None;
    }
    let [id, factory] = call.arguments.as_slice() else {
        return None;
    };
    let id = string(argument_expression(id)?)?;
    Some((
        id,
        simple_factory(argument_expression(factory)?, HOISTED_PARAMS)?,
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

struct Registers<'k, 'a> {
    name: &'a str,
    defs: Vec<Def<'k, 'a>>,
    effects: bool,
    first: Option<usize>,
}

impl<'k, 'a> Registers<'k, 'a> {
    fn element(
        &mut self,
        expr: &'k Expression<'a>,
        at: usize,
        element: Option<usize>,
    ) -> Option<()> {
        let Some((id, function)) = register_call(expr, self.name) else {
            self.effects = true;
            return Some(());
        };
        if self.effects {
            return None;
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

    fn statement(&mut self, statement: &'k Statement<'a>, at: usize, inert: bool) -> Option<()> {
        if inert {
            return Some(());
        }
        match expression_statement(statement) {
            Some(Expression::SequenceExpression(sequence)) => {
                for (index, expr) in sequence.expressions.iter().enumerate() {
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

fn position<'k, 'a, T>(
    top: &'k [Statement<'a>],
    found: impl Fn(&'k Statement<'a>) -> Option<T>,
) -> Option<(usize, T)> {
    let mut hits = top
        .iter()
        .enumerate()
        .filter_map(|(index, s)| found(s).map(|t| (index, t)));
    let first = hits.next()?;
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

fn hoisted<'k, 'a>(top: &'k [Statement<'a>], source_text: &str) -> Option<Hoisted<'k, 'a>> {
    let (at_require, (require, global)) = position(top, require_declarator)?;
    let name = require.name.as_str();
    let at_definition = definition(top, name)?;
    if at_definition.is_some_and(|at| !runtime::defined(top, at, &global, source_text)) {
        return None;
    }
    let (at_register, register) = position(top, |s| register_declarator(s, name))?;
    let between = at_definition.unwrap_or(at_require);
    if !(at_require <= between && between < at_register) || at_definition == Some(at_require) {
        return None;
    }
    let prelude = [at_require, between, at_register];
    let mut registers = Registers {
        name: register.name.as_str(),
        defs: Vec::new(),
        effects: false,
        first: None,
    };
    for (index, statement) in top.iter().enumerate() {
        registers.statement(
            statement,
            index,
            inert_statement(statement, &prelude, index),
        )?;
    }
    let first = registers.first?;
    let defs = registers.defs;
    let runtime = runtime_id(BundleKind::Parcel);
    (first > at_register && unique(&defs, &[ENTRY_ID, runtime.as_str()])).then_some(Hoisted {
        defs,
        require,
        register,
        prelude,
        definition: at_definition,
        first,
        global,
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
        if arguments.len() > function.params.items.len()
            || !runtime::proven(function, program.source_text)
        {
            return None;
        }
        return registry(arguments).map(Layout::Registry);
    }
    hoisted(frame.top, program.source_text).map(Layout::Hoisted)
}
