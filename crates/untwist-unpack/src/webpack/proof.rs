use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use rustc_hash::{FxHashMap, FxHashSet};

use untwist_core::semantic::build_scoping;

use crate::matcher::{const_member, literal_id as id_text};
use crate::notes::runtime_not_proven;
use crate::webpack::guard::{dirname_suffix, guarded};
use crate::webpack::helpers::{declared_in, pattern_names};
use crate::webpack::locals::closed_value;
use crate::webpack::require_form::caching_cache;
use crate::webpack::runtime_module::uses_in;

const ENTRY_SLOT: &str = "s";
const HOST_MODULE: &str = "module";
const EXPORTS: &str = "exports";

pub struct Shape<'k, 'a> {
    pub params: Option<&'k FormalParameters<'a>>,
    pub statements: &'k [Statement<'a>],
    pub require: &'k str,
    pub container: &'k str,
    pub container_at: Option<usize>,
    pub skip: Option<Span>,
}

struct Proof<'k, 'a> {
    require: &'k str,
    skip: Option<Span>,
    declared: FxHashSet<&'k str>,
    values: Vec<&'k Expression<'a>>,
    units: Vec<&'k FunctionBody<'a>>,
    entries: FxHashSet<&'k str>,
    definitions: FxHashSet<u32>,
    defined: FxHashMap<&'k str, usize>,
}

type Proven = Result<(), String>;

fn refuse<T>(why: impl Into<String>) -> Result<T, String> {
    Err(why.into())
}

fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn literal_id(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::NumericLiteral(_) | Expression::StringLiteral(_)
    )
}

fn single<T>(items: &[T]) -> Option<&T> {
    match items {
        [only] => Some(only),
        _ => None,
    }
}

fn iife_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<Result<&'k FunctionBody<'a>, String>> {
    let expr = match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            &unary.argument
        }
        other => other,
    };
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let (body, generator, asynchronous, params) = match call.callee.without_parentheses() {
        Expression::FunctionExpression(function) => (
            function.body.as_deref()?,
            function.generator,
            function.r#async,
            &function.params,
        ),
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => (&**body, false, arrow.r#async, &arrow.params),
            _ => return None,
        },
        _ => return None,
    };
    if generator || asynchronous {
        return Some(refuse(
            "a wrapper in the prelude is a generator or async function",
        ));
    }
    if !call.arguments.is_empty() || !params.items.is_empty() || params.rest.is_some() {
        return Some(refuse("a wrapper in the prelude takes arguments"));
    }
    Some(Ok(body))
}

impl<'k, 'a> Proof<'k, 'a> {
    fn effect_free(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_) => true,
            Expression::TemplateLiteral(template) => template.expressions.is_empty(),
            Expression::Identifier(id) => {
                id.name == "undefined" || self.declared.contains(id.name.as_str())
            }
            Expression::UnaryExpression(unary) => {
                matches!(
                    unary.operator,
                    UnaryOperator::Void | UnaryOperator::LogicalNot | UnaryOperator::UnaryNegation
                ) && matches!(
                    unary.argument.without_parentheses(),
                    Expression::NumericLiteral(_) | Expression::BooleanLiteral(_)
                )
            }
            Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
                matches!(property, ObjectPropertyKind::ObjectProperty(property)
                    if !property.computed && property.kind == PropertyKind::Init
                        && self.effect_free(&property.value))
            }),
            Expression::ArrayExpression(array) => array
                .elements
                .iter()
                .all(|element| element.as_expression().is_some_and(|e| self.effect_free(e))),
            other => closed_value(other),
        }
    }

    fn define(&mut self, name: &'k str, assignment: &AssignmentExpression<'_>) -> Proven {
        let count = self.defined.entry(name).or_default();
        *count += 1;
        if *count > 1 {
            return refuse(format!("require.{name} is assigned more than once"));
        }
        self.definitions.insert(assignment.span.start);
        Ok(())
    }

    fn helper(&mut self, assignment: &'k AssignmentExpression<'a>) -> Option<Proven> {
        if assignment.operator != AssignmentOperator::Assign {
            return None;
        }
        let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
            return None;
        };
        if !is_name(&member.object, self.require) {
            return None;
        }
        let name = member.property.name.as_str();
        if !self.effect_free(&assignment.right) && dirname_suffix(assignment).is_none() {
            return Some(refuse(format!(
                "the value of require.{name} runs code when the prelude runs"
            )));
        }
        self.values.push(&assignment.right);
        Some(self.define(name, assignment))
    }

    fn local_init(&self, assignment: &AssignmentExpression<'_>) -> bool {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left else {
            return false;
        };
        assignment.operator == AssignmentOperator::Assign
            && id.name != self.require
            && self.declared.contains(id.name.as_str())
            && self.effect_free(&assignment.right)
    }

    fn entry_call(&mut self, expr: &'k Expression<'_>) -> Option<Proven> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if !is_name(&call.callee, self.require) || call.optional {
            return None;
        }
        let argument = single(&call.arguments)?
            .as_expression()?
            .without_parentheses();
        if literal_id(argument) {
            return Some(Ok(()));
        }
        let Expression::AssignmentExpression(slot) = argument else {
            return None;
        };
        let AssignmentTarget::StaticMemberExpression(member) = &slot.left else {
            return None;
        };
        let entry = member.property.name == ENTRY_SLOT
            && is_name(&member.object, self.require)
            && slot.operator == AssignmentOperator::Assign
            && literal_id(&slot.right);
        entry.then(|| self.define(ENTRY_SLOT, slot))
    }

    fn element(&mut self, expr: &'k Expression<'a>, top: bool) -> Proven {
        if top && self.skip == Some(expr.span()) {
            return Ok(());
        }
        if let Expression::AssignmentExpression(assignment) = expr.without_parentheses() {
            if let Some(proven) = self.helper(assignment) {
                return proven;
            }
            if self.local_init(assignment) {
                return Ok(());
            }
            if top && self.module_exports(assignment) {
                return Ok(());
            }
        }
        if top {
            if let Some(proven) = self.entry_call(expr) {
                return proven;
            }
            if let Some((loader, _)) = deferred_start(expr, self.require) {
                if self.defined.contains_key(loader) {
                    return Ok(());
                }
            }
        }
        if let Some(body) = iife_body(expr) {
            return self.unit(body?);
        }
        refuse("a prelude statement is neither a helper definition nor the entry call")
    }

    fn module_exports(&self, assignment: &AssignmentExpression<'_>) -> bool {
        let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
            return false;
        };
        assignment.operator == AssignmentOperator::Assign
            && member.property.name == EXPORTS
            && is_name(&member.object, HOST_MODULE)
            && !self.declared.contains(HOST_MODULE)
            && matches!(assignment.right.without_parentheses(), Expression::Identifier(id)
                if self.entries.contains(id.name.as_str()))
    }

    fn expression_statement(&mut self, expr: &'k Expression<'a>, top: bool) -> Proven {
        match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .try_for_each(|element| self.element(element, top)),
            other => self.element(other, top),
        }
    }

    fn declaration(&mut self, declaration: &'k VariableDeclaration<'a>, top: bool) -> Proven {
        if matches!(
            declaration.kind,
            VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing
        ) {
            return refuse("the prelude declares a disposable binding");
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return refuse("the prelude destructures a value");
            };
            let Some(init) = &declarator.init else {
                continue;
            };
            if self.effect_free(init) {
                continue;
            }
            match self.entry_call(init).filter(|_| top) {
                Some(proven) => {
                    proven?;
                    self.entries.insert(id.name.as_str());
                }
                None => {
                    return refuse(format!(
                        "the prelude initialises {} with code that runs",
                        id.name
                    ))
                }
            }
        }
        Ok(())
    }

    fn unit(&mut self, body: &'k FunctionBody<'a>) -> Proven {
        self.units.push(body);
        let mut local = Vec::new();
        declared_in(&body.statements, &mut local);
        for name in local {
            self.declared.insert(name);
        }
        for statement in &body.statements {
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    self.declaration(declaration, false)?
                }
                Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => {}
                Statement::ExpressionStatement(statement) => {
                    self.expression_statement(&statement.expression, false)?
                }
                _ => return refuse("a wrapper in the prelude holds more than helper definitions"),
            }
        }
        Ok(())
    }

    fn statement(&mut self, statement: &'k Statement<'a>, top: bool) -> Proven {
        match statement {
            Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => Ok(()),
            Statement::VariableDeclaration(declaration) => self.declaration(declaration, top),
            Statement::ExpressionStatement(statement) => {
                self.expression_statement(&statement.expression, top)
            }
            Statement::IfStatement(_) => {
                let Some((_, assignment)) = guarded(statement, self.require) else {
                    return refuse("the prelude holds a condition webpack does not write");
                };
                match self.helper(assignment) {
                    Some(proven) => proven,
                    None => refuse("the prelude holds a condition webpack does not write"),
                }
            }
            _ => refuse("the prelude holds a statement webpack does not write"),
        }
    }

    fn ending(&mut self, statement: &'k ReturnStatement<'a>) -> Proven {
        match &statement.argument {
            None => Ok(()),
            Some(argument) => self.expression_statement(argument, true),
        }
    }
}

fn started_by(expr: &Expression<'_>, require: &str) -> Option<String> {
    match expr.without_parentheses() {
        Expression::CallExpression(call) => {
            let target = const_member(&call.callee, "bind")?;
            let [this, id] = &call.arguments[..] else {
                return None;
            };
            let bound = is_name(target, require)
                && matches!(
                    this.as_expression()?.without_parentheses(),
                    Expression::NullLiteral(_)
                );
            bound.then(|| id_text(id.as_expression()?.without_parentheses()))?
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let body = match &arrow.body {
                ArrowFunctionBody::FunctionBody(_) => return None,
                body => body.as_expression()?,
            };
            if !arrow.params.items.is_empty() || arrow.r#async {
                return None;
            }
            let Expression::CallExpression(call) = body.without_parentheses() else {
                return None;
            };
            let [id] = &call.arguments[..] else {
                return None;
            };
            is_name(&call.callee, require)
                .then(|| id_text(id.as_expression()?.without_parentheses()))?
        }
        _ => None,
    }
}

fn deferred_start<'k>(expr: &'k Expression<'_>, require: &str) -> Option<(&'k str, String)> {
    let Expression::CallExpression(then) = expr.without_parentheses() else {
        return None;
    };
    let [start] = &then.arguments[..] else {
        return None;
    };
    let Expression::CallExpression(load) =
        const_member(&then.callee, "then")?.without_parentheses()
    else {
        return None;
    };
    let Expression::StaticMemberExpression(loader) = &load.callee else {
        return None;
    };
    let literals = load.arguments.iter().all(|argument| {
        argument
            .as_expression()
            .is_some_and(|value| id_text(value.without_parentheses()).is_some())
    });
    let fits = is_name(&loader.object, require) && literals;
    let id = fits.then(|| started_by(start.as_expression()?, require))??;
    Some((loader.property.name.as_str(), id))
}

pub fn deferred_entries(statements: &[Statement<'_>], require: &str) -> Vec<String> {
    let mut found = Vec::new();
    for statement in statements {
        let expr = match statement {
            Statement::ExpressionStatement(statement) => &statement.expression,
            Statement::ReturnStatement(ret) => match &ret.argument {
                Some(argument) => argument,
                None => continue,
            },
            _ => continue,
        };
        let elements = match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
            other => vec![other],
        };
        found.extend(
            elements
                .into_iter()
                .filter_map(|element| deferred_start(element, require).map(|(_, id)| id)),
        );
    }
    found
}

fn top_names<'k>(shape: &Shape<'k, '_>) -> Vec<&'k str> {
    let mut names = Vec::new();
    for item in shape.params.iter().flat_map(|params| &params.items) {
        pattern_names(&item.pattern, &mut names);
    }
    declared_in(shape.statements, &mut names);
    names
}

fn declares_cache(statements: &[Statement<'_>], cache: &str) -> bool {
    statements.iter().any(|statement| {
        let Statement::VariableDeclaration(declaration) = statement else {
            return false;
        };
        declaration.declarations.iter().any(|declarator| {
            matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == cache)
                && matches!(declarator.init.as_ref().map(Expression::without_parentheses),
                    Some(Expression::ObjectExpression(object)) if object.properties.is_empty())
        })
    })
}

fn require_function<'k, 'a>(shape: &Shape<'k, 'a>) -> Option<(usize, &'k Function<'a>)> {
    shape
        .statements
        .iter()
        .enumerate()
        .find_map(|(index, statement)| match statement {
            Statement::FunctionDeclaration(function)
                if function
                    .id
                    .as_ref()
                    .is_some_and(|id| id.name == shape.require) =>
            {
                Some((index, &**function))
            }
            _ => None,
        })
}

fn skipped(statement: &Statement<'_>, skip: Option<Span>) -> bool {
    matches!(statement, Statement::ExpressionStatement(expression)
        if skip == Some(expression.expression.span()))
}

fn ends_early(statements: &[Statement<'_>], at: usize) -> bool {
    statements[at + 1..].iter().any(|statement| {
        !matches!(
            statement,
            Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_)
        )
    })
}

fn classify<'k, 'a>(proof: &mut Proof<'k, 'a>, shape: &Shape<'k, 'a>, require_at: usize) -> Proven {
    for (index, statement) in shape.statements.iter().enumerate() {
        if Some(index) == shape.container_at
            || index == require_at
            || skipped(statement, shape.skip)
        {
            continue;
        }
        match statement {
            Statement::ReturnStatement(ending) if shape.params.is_some() => {
                if ends_early(shape.statements, index) {
                    return refuse("the prelude returns before its last statement");
                }
                proof.ending(ending)?;
            }
            other => proof.statement(other, true)?,
        }
    }
    Ok(())
}

fn uses_proven(proof: &Proof<'_, '_>, shape: &Shape<'_, '_>, require_at: usize) -> Proven {
    let statements = shape
        .statements
        .iter()
        .enumerate()
        .filter(|(index, statement)| {
            Some(*index) != shape.container_at
                && *index != require_at
                && !skipped(statement, shape.skip)
        })
        .map(|(_, statement)| match statement {
            Statement::IfStatement(branch) if guarded(statement, shape.require).is_some() => {
                &branch.consequent
            }
            other => other,
        });
    let uses = uses_in(shape.require, statements);
    if uses.bare {
        return refuse("the prelude passes the require function on or aliases it");
    }
    if uses.computed {
        return refuse("the prelude reaches the require function through a computed key");
    }
    match uses
        .written
        .iter()
        .find(|(_, at)| !at.is_some_and(|at| proof.definitions.contains(&at)))
    {
        Some((name, _)) => refuse(format!(
            "require.{name} is written somewhere other than its definition"
        )),
        None => Ok(()),
    }
}

struct Captures<'s> {
    scoping: &'s Scoping,
    outer: FxHashSet<ScopeId>,
    depth: u32,
    found: Option<String>,
}

impl Captures<'_> {
    fn note(&mut self, why: String) {
        self.found.get_or_insert(why);
    }
}

impl<'a> VisitJs<'a> for Captures<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.depth == 0 && it.name == "arguments" {
            self.note("a helper reads the prelude's arguments".to_string());
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let Some(symbol) = symbol else {
            return;
        };
        if self.outer.contains(&self.scoping.symbol_scope_id(symbol)) {
            self.note(format!("a helper reads {} from around the bundle", it.name));
        }
    }

    fn visit_this_expression(&mut self, _: &ThisExpression) {
        if self.depth == 0 {
            self.note("a helper reads the prelude's this".to_string());
        }
    }

    fn visit_new_target(&mut self, _: &NewTarget) {
        self.note("a helper reads new.target".to_string());
    }

    fn visit_import_meta(&mut self, _: &ImportMeta) {
        self.note("a helper reads import.meta".to_string());
    }

    fn visit_super(&mut self, _: &Super) {
        if self.depth == 0 {
            self.note("a helper reads super".to_string());
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

fn outer_scopes(scoping: &Scoping, function: &Function<'_>) -> Option<FxHashSet<ScopeId>> {
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let prelude = scoping.symbol_scope_id(symbol);
    Some(scoping.scope_ancestors(prelude).skip(1).collect())
}

fn closed(proof: &Proof<'_, '_>, scoping: &Scoping, function: &Function<'_>) -> Proven {
    let Some(outer) = outer_scopes(scoping, function) else {
        return refuse("the prelude's scope is not known");
    };
    let mut captures = Captures {
        scoping,
        outer,
        depth: 0,
        found: None,
    };
    for value in &proof.values {
        captures.visit_expression(value);
    }
    for unit in &proof.units {
        captures.visit_function_body(unit);
    }
    match captures.found {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

pub fn prove<'k, 'a>(shape: &Shape<'k, 'a>, scoping: &Scoping) -> Proven {
    let Some((require_at, function)) = require_function(shape) else {
        return refuse("the require function is not declared in the prelude");
    };
    let all = top_names(shape);
    if all.iter().filter(|name| **name == shape.require).count() != 1 {
        return refuse("the require function is declared more than once");
    }
    let names: FxHashSet<&str> = all.into_iter().collect();
    if names.contains("undefined") {
        return refuse("the prelude declares undefined");
    }
    let Some(cache) = caching_cache(function, shape.require, shape.container) else {
        return refuse("the require function is not webpack's caching form");
    };
    if !declares_cache(shape.statements, cache) || cache == shape.container {
        return refuse("the module cache is not an empty object of the prelude");
    }
    let mut proof = Proof {
        require: shape.require,
        skip: shape.skip,
        declared: names,
        values: Vec::new(),
        units: Vec::new(),
        entries: FxHashSet::default(),
        definitions: FxHashSet::default(),
        defined: FxHashMap::default(),
    };
    classify(&mut proof, shape, require_at)?;
    uses_proven(&proof, shape, require_at)?;
    closed(&proof, scoping, function)
}

pub struct Proving<'s> {
    scoping: Option<&'s Scoping>,
    built: Option<Scoping>,
    unproven: Option<&'s mut Option<String>>,
}

impl<'s> Proving<'s> {
    pub fn new(scoping: Option<&'s Scoping>, unproven: &'s mut Option<String>) -> Self {
        Self {
            scoping,
            built: None,
            unproven: Some(unproven),
        }
    }

    pub fn own(_: &Program<'_>) -> Self {
        Self {
            scoping: None,
            built: None,
            unproven: None,
        }
    }

    fn refused<T>(&mut self, why: String) -> Option<T> {
        if let Some(unproven) = self.unproven.as_deref_mut() {
            unproven.get_or_insert(why);
        }
        None
    }

    pub fn prove(&mut self, source: &Program<'_>, shape: &Shape<'_, '_>) -> Option<()> {
        let proven = match self.scoping {
            Some(scoping) => prove(shape, scoping),
            None => {
                let scoping = self
                    .built
                    .get_or_insert_with(|| build_scoping(source).scoping);
                prove(shape, scoping)
            }
        };
        match proven {
            Ok(()) => Some(()),
            Err(why) => self.refused(runtime_not_proven(why)),
        }
    }

    pub fn gate(&mut self, routed: Result<(), String>) -> Option<()> {
        match routed {
            Ok(()) => Some(()),
            Err(why) => self.refused(why),
        }
    }
}
