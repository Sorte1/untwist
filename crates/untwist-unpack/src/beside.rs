use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use untwist_core::semantic::build_scoping;

use crate::model::{Bundle, Module};
use crate::scan::is_direct_eval;
use crate::strict::written_as_strict;

const WRAPPER_NAMES: [&str; 5] = ["module", "exports", "require", "__filename", "__dirname"];

pub struct Beside<'a> {
    pub before: Vec<Statement<'a>>,
    pub after: Vec<Statement<'a>>,
    free: Vec<String>,
    strict: bool,
}

pub enum Around<'a> {
    Kept(Beside<'a>),
    Refused(String),
}

struct Loose<'s> {
    scoping: &'s Scoping,
    functions: u32,
    arrows: u32,
    statements: u32,
    free: Vec<String>,
    why: Option<String>,
}

impl Loose<'_> {
    fn refuse(&mut self, why: impl Into<String>) {
        self.why.get_or_insert_with(|| why.into());
    }

    fn in_function(&mut self, arrow: bool, walk: impl FnOnce(&mut Self)) {
        let statements = std::mem::replace(&mut self.statements, 0);
        let count = match arrow {
            true => &mut self.arrows,
            false => &mut self.functions,
        };
        *count += 1;
        walk(self);
        match arrow {
            true => self.arrows -= 1,
            false => self.functions -= 1,
        }
        self.statements = statements;
    }

    fn at_top(&self) -> bool {
        self.functions + self.arrows == 0
    }

    fn visit_key<'a>(&mut self, key: &PropertyKey<'a>, computed: bool) {
        if computed {
            self.visit_property_key(key);
        }
    }
}

impl<'a> VisitJs<'a> for Loose<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.why.is_some() {
            return;
        }
        self.statements += 1;
        walk_js::walk_statement(self, it);
        self.statements -= 1;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.at_top() && self.statements > 1 && it.is_declaration() {
            self.refuse("declares a function in a block");
        }
        self.in_function(false, |this| walk_js::walk_function(this, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.in_function(true, |this| {
            walk_js::walk_arrow_function_expression(this, it)
        });
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.visit_decorators(&it.decorators);
        if let Some(heritage) = &it.heritage {
            self.visit_class_heritage(heritage);
        }
        for element in &it.body.body {
            self.visit_class_element(element);
        }
    }

    fn visit_class_element(&mut self, it: &ClassElement<'a>) {
        match it {
            ClassElement::MethodDefinition(method) => {
                self.visit_decorators(&method.decorators);
                self.visit_key(&method.key, method.computed);
                self.visit_function(&method.value, ScopeFlags::Function);
            }
            ClassElement::PropertyDefinition(property) => {
                self.visit_decorators(&property.decorators);
                self.visit_key(&property.key, property.computed);
                if let Some(value) = &property.value {
                    self.in_function(false, |this| this.visit_expression(value));
                }
            }
            ClassElement::AccessorProperty(property) => {
                self.visit_decorators(&property.decorators);
                self.visit_key(&property.key, property.computed);
                if let Some(value) = &property.value {
                    self.in_function(false, |this| this.visit_expression(value));
                }
            }
            ClassElement::StaticBlock(block) => {
                self.in_function(false, |this| walk_js::walk_static_block(this, block));
            }
            ClassElement::TSIndexSignature(_) => {}
        }
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if self.at_top() && it.kind == VariableDeclarationKind::Var {
            self.refuse("declares a var");
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        if self.functions == 0 {
            self.refuse("reads this");
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let name = it.name.as_str();
        if let Some(symbol) = symbol {
            if self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id() {
                self.refuse(format!(
                    "reads {name}, which the written files do not declare"
                ));
            }
            return;
        }
        if WRAPPER_NAMES.contains(&name) || (name == "arguments" && self.functions == 0) {
            self.refuse(format!("reads {name}"));
        }
        if !self.free.iter().any(|known| known == name) {
            self.free.push(name.to_string());
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(&it.callee) {
            self.refuse("calls eval directly");
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.refuse("holds a with statement");
        walk_js::walk_with_statement(self, it);
    }
}

fn declares(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
    ) || statement.is_module_declaration()
}

fn line(text: &str, span: Span) -> usize {
    let end = (span.start as usize).min(text.len());
    let before = text.get(..end).unwrap_or_default();
    before.bytes().filter(|byte| *byte == b'\n').count() + 1
}

fn refused(program: &Program<'_>, statement: &Statement<'_>, why: &str) -> String {
    let line = line(program.source_text, statement.span());
    crate::notes::code_beside(line, why)
}

fn inert(argument: &Argument<'_>) -> bool {
    matches!(
        argument,
        Argument::ThisExpression(_)
            | Argument::StringLiteral(_)
            | Argument::NumericLiteral(_)
            | Argument::BooleanLiteral(_)
            | Argument::NullLiteral(_)
    )
}

fn spanned(statements: &[Statement<'_>]) -> Option<Span> {
    let (first, last) = statements.first().zip(statements.last())?;
    Some(Span::new(first.span().start, last.span().end))
}

fn body_holds_only(statements: &[Statement<'_>], site: Span) -> bool {
    if spanned(statements) == Some(site) {
        return true;
    }
    let mut kept = statements
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)));
    match (kept.next(), kept.next()) {
        (Some(statement), None) => statement_holds_only(statement, site, true),
        _ => false,
    }
}

fn bare(params: &FormalParameters<'_>) -> bool {
    params.items.is_empty() && params.rest.is_none()
}

fn called_body_holds_only(call: &CallExpression<'_>, site: Span) -> bool {
    if call.optional || !call.arguments.iter().all(inert) {
        return false;
    }
    let callee = match call.callee.without_parentheses() {
        Expression::StaticMemberExpression(member) if !member.optional => {
            let called = match member.property.name.as_str() {
                "call" => true,
                "apply" => call.arguments.len() <= 1,
                _ => false,
            };
            match called {
                true => member.object.without_parentheses(),
                false => return false,
            }
        }
        Expression::StaticMemberExpression(_) => return false,
        callee => callee,
    };
    match callee {
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            bare(&function.params)
                && function
                    .body
                    .as_ref()
                    .is_some_and(|body| body_holds_only(&body.statements, site))
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async && bare(&arrow.params) => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => body_holds_only(&body.statements, site),
                body => body
                    .as_expression()
                    .is_some_and(|expression| holds_only(expression, site)),
            }
        }
        _ => false,
    }
}

fn holds_only(expression: &Expression<'_>, site: Span) -> bool {
    if expression.span() == site {
        return true;
    }
    let expression = expression.without_parentheses();
    if expression.span() == site {
        return true;
    }
    match expression {
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void
            ) && holds_only(&unary.argument, site)
        }
        Expression::CallExpression(call) => called_body_holds_only(call, site),
        _ => false,
    }
}

fn statement_holds_only(statement: &Statement<'_>, site: Span, in_function: bool) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => holds_only(&statement.expression, site),
        Statement::ReturnStatement(statement) if in_function => statement
            .argument
            .as_ref()
            .is_some_and(|argument| holds_only(argument, site)),
        _ => false,
    }
}

fn umd_factory_holds_only(statement: &Statement<'_>, site: Span) -> bool {
    crate::umd::wrapper::node_factory(statement).is_some_and(|factory| {
        factory.params.items.is_empty()
            && factory.params.rest.is_none()
            && body_holds_only(&factory.body.statements, site)
    })
}

pub fn around<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    (site, wrapped): (Span, bool),
    scoping: &Scoping,
) -> Option<Around<'a>> {
    let body = &program.body;
    let at = body
        .iter()
        .position(|statement| statement.span().contains_inclusive(site))?;
    let whole = wrapped
        || statement_holds_only(&body[at], site, false)
        || umd_factory_holds_only(&body[at], site);
    if !whole {
        let line = line(program.source_text, body[at].span());
        return Some(Around::Refused(crate::notes::bundle_statement_shared(line)));
    }
    let kept = |statement: &&Statement<'a>| !matches!(statement, Statement::EmptyStatement(_));
    let before: Vec<&Statement<'a>> = body[..at].iter().filter(kept).collect();
    let after: Vec<&Statement<'a>> = body[at + 1..].iter().filter(kept).collect();
    if before.is_empty() && after.is_empty() {
        return None;
    }
    let mut loose = Loose {
        scoping,
        functions: 0,
        arrows: 0,
        statements: 0,
        free: Vec::new(),
        why: None,
    };
    for statement in before.iter().chain(&after) {
        if declares(statement) {
            return Some(Around::Refused(refused(
                program,
                statement,
                "declares a binding",
            )));
        }
        loose.visit_statement(statement);
        if let Some(why) = loose.why.take() {
            return Some(Around::Refused(refused(program, statement, &why)));
        }
    }
    let cloned = |list: Vec<&Statement<'a>>| -> Vec<Statement<'a>> {
        list.into_iter()
            .map(|statement| statement.clone_in(alloc))
            .collect()
    };
    Some(Around::Kept(Beside {
        before: cloned(before),
        after: cloned(after),
        free: loose.free,
        strict: program.source_type.is_module() || program.has_use_strict_directive(),
    }))
}

struct TopReturn {
    found: bool,
}

impl<'a> VisitJs<'a> for TopReturn {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}

    fn visit_return_statement(&mut self, _it: &ReturnStatement<'a>) {
        self.found = true;
    }
}

fn returns_early(program: &Program<'_>) -> bool {
    let mut top = TopReturn { found: false };
    top.visit_program(program);
    top.found
}

fn entry_takes(beside: &Beside<'_>, module: &Module<'_>) -> Result<(), String> {
    let program = module.program();
    if program.body.iter().any(Statement::is_module_declaration) {
        return Err("the entry module is written as an ES module, whose imports run first".into());
    }
    if written_as_strict(module) != beside.strict {
        return Err("the entry module and that code differ in strictness".into());
    }
    if returns_early(program) {
        return Err("the entry module can return before the code after the bundle runs".into());
    }
    let scoping = build_scoping(program).scoping;
    let root = scoping.root_scope_id();
    let declared = scoping.get_bindings(root).keys().find(|name| {
        beside
            .free
            .iter()
            .any(|free| free.as_str() == name.as_str())
    });
    match declared {
        Some(name) => Err(format!(
            "the entry module declares {name}, which that code reads as a global"
        )),
        None => Ok(()),
    }
}

pub fn keep_around<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Result<(), String> {
    let beside = match bundle.around.take() {
        None => return Ok(()),
        Some(Around::Refused(why)) => return Err(why),
        Some(Around::Kept(beside)) => beside,
    };
    let refuse = |why: &str| -> Result<(), String> { Err(crate::notes::code_beside_entry(why)) };
    if !bundle.later_entries.is_empty() || !bundle.deferred.is_empty() {
        return refuse("the bundle starts more than one entry");
    }
    let Some(entry) = bundle.entry_id.clone() else {
        return refuse("the bundle has no entry module");
    };
    let Some(module) = bundle.modules.get_mut(&entry) else {
        return refuse("the bundle has no entry module");
    };
    if let Err(why) = entry_takes(&beside, module) {
        return refuse(&why);
    }
    let program = module.program_mut();
    let mut body = oxc_allocator::Vec::with_capacity_in(
        beside.before.len() + program.body.len() + beside.after.len(),
        &alloc,
    );
    body.extend(beside.before);
    body.extend(program.body.drain(..));
    body.extend(beside.after);
    program.body = body;
    Ok(())
}
