use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::SemanticBuilder;
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;

use crate::ast_util::prop::member_parts;
use crate::parse::{has_error, parse_program};

mod bound;

use bound::{handed_out_guarded, settle_bound};

const LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];
const ESCAPE_BREAK: char = '\n';
const CHAIN_CALLS: usize = 3;
const ESCAPED_BAIT: &str = "window";
const SELF_READ: &str = "charCodeAt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReaders {
    None,
    Guarded,
    Unguarded,
}

pub fn reads_source(program: &Program<'_>) -> bool {
    let mut readers = Readers::new(None);
    readers.visit_program(program);
    readers.any()
}

pub fn source_readers(code: &str) -> SourceReaders {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, code);
    if has_error(&parsed.diagnostics) {
        return SourceReaders::Unguarded;
    }
    let program = &parsed.program;
    let mut readers = Readers::new(Some(code));
    readers.visit_program(program);
    if !readers.any() {
        return SourceReaders::None;
    }
    if readers.broken {
        return SourceReaders::Unguarded;
    }
    if readers.bound.is_empty() {
        return SourceReaders::Guarded;
    }
    let semantic = SemanticBuilder::new().build(program).semantic;
    let mut readers = Readers::new(Some(code));
    readers.visit_program(program);
    if settle_bound(program, semantic.scoping(), &readers.bound, code) {
        SourceReaders::Guarded
    } else {
        SourceReaders::Unguarded
    }
}

#[derive(Clone, Copy)]
struct Printed<'t> {
    text: Option<&'t str>,
}

impl Printed<'_> {
    fn spelled(&self, span: Span) -> Option<&str> {
        self.text?.get(span.start as usize..span.end as usize)
    }

    fn on_one_line(&self, span: Span) -> bool {
        match self.text {
            None => true,
            Some(_) => self
                .spelled(span)
                .is_some_and(|printed| !printed.contains(LINE_TERMINATORS)),
        }
    }

    fn breaks(&self, span: Span) -> bool {
        self.spelled(span)
            .is_some_and(|printed| printed.contains(ESCAPE_BREAK))
    }
}

struct SelfText<'n, 't> {
    name: &'n str,
    printed: Printed<'t>,
    texts: Vec<String>,
    read: bool,
    escaped: bool,
}

fn function_literal_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

fn adds(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::BinaryExpression(sum)
        if sum.operator == BinaryOperator::Addition && names(&sum.right, name))
}

impl SelfText<'_, '_> {
    fn note_text(&mut self, holder: &str, value: &Expression<'_>) {
        if adds(value, self.name) {
            self.texts.push(holder.to_string());
        }
    }

    fn escapes(&self, operand: &Expression<'_>) -> bool {
        function_literal_span(operand).is_some_and(|span| self.printed.breaks(span))
    }
}

impl<'a> VisitJs<'a> for SelfText<'_, '_> {
    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.note_text(id.name.as_str(), init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                self.note_text(id.name.as_str(), &it.right);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator == BinaryOperator::Addition {
            self.escaped |= self.escapes(&it.left) || self.escapes(&it.right);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some((object, key)) = member_parts(&it.callee) {
            let reads = key == SELF_READ
                && matches!(object.without_parentheses(),
                    Expression::Identifier(id) if self.texts.iter().any(|text| text == id.name.as_str()));
            self.read |= reads;
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn reads_its_own_text(function: &Function<'_>, name: &str, printed: Printed<'_>) -> bool {
    let Some(body) = &function.body else {
        return false;
    };
    let mut text = SelfText {
        name,
        printed,
        texts: Vec::new(),
        read: false,
        escaped: false,
    };
    text.visit_function_body(body);
    text.read && !text.escaped
}

enum Callback<'k, 'a> {
    Body(&'k FunctionBody<'a>),
    Expression(&'k Expression<'a>),
}

impl<'a> Callback<'_, 'a> {
    fn visit_with<V: VisitJs<'a>>(&self, visitor: &mut V) {
        match self {
            Callback::Body(body) => visitor.visit_function_body(body),
            Callback::Expression(expr) => visitor.visit_expression(expr),
        }
    }
}

fn callback_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<Callback<'k, 'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) if function.params.items.is_empty() => {
            function.body.as_deref().map(Callback::Body)
        }
        Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(Callback::Body(body)),
                expression => expression.as_expression().map(Callback::Expression),
            }
        }
        _ => None,
    }
}

fn controller_and_callback<'k, 'a>(
    call: &'k CallExpression<'a>,
) -> Option<(&'k Expression<'a>, &'k Expression<'a>)> {
    match call.arguments.as_slice() {
        [Argument::ThisExpression(_), callback, ..] => {
            Some((call.callee.without_parentheses(), callback.as_expression()?))
        }
        [controller, Argument::ThisExpression(_), callback, ..] => Some((
            controller.as_expression()?.without_parentheses(),
            callback.as_expression()?,
        )),
        _ => None,
    }
}

struct EscapeCheck {
    found: bool,
}

impl<'a> VisitJs<'a> for EscapeCheck {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let bait = it.params.items.is_empty()
            && it.body.as_ref().is_some_and(|body| {
                matches!(body.statements.as_slice(), [Statement::ReturnStatement(ret)]
                    if matches!(&ret.argument, Some(Expression::StringLiteral(text)) if text.value == ESCAPED_BAIT))
            });
        self.found |= bait;
        walk_js::walk_function(self, it, flags);
    }
}

fn checks_an_escaped_spelling(call: &CallExpression<'_>) -> bool {
    let Some(callback) =
        controller_and_callback(call).and_then(|(_, callback)| callback_body(callback))
    else {
        return false;
    };
    let mut check = EscapeCheck { found: false };
    callback.visit_with(&mut check);
    check.found
}

fn reader_controller<'k, 'a>(init: &'k Expression<'a>, own: &str) -> Option<&'k Expression<'a>> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let (controller, callback) = controller_and_callback(call)?;
    let callback = callback_body(callback)?;
    let mut chain = SelfChain {
        name: own,
        found: false,
    };
    callback.visit_with(&mut chain);
    chain.found.then_some(controller)
}

struct Readers<'t> {
    printed: Printed<'t>,
    bound: Vec<Option<ReferenceId>>,
    found: bool,
    broken: bool,
}

impl<'t> Readers<'t> {
    fn new(text: Option<&'t str>) -> Self {
        Readers {
            printed: Printed { text },
            bound: Vec::new(),
            found: false,
            broken: false,
        }
    }

    fn any(&self) -> bool {
        self.found
    }

    fn note(&mut self, init: &Expression<'_>, own: &str) {
        if let Some(controller) = reader_controller(init, own) {
            self.found = true;
            match controller {
                Expression::Identifier(id) => self.bound.push(id.reference_id.get()),
                inline => self.broken |= !handed_out_guarded(inline, self.printed),
            }
        }

        if let Expression::FunctionExpression(function) = init.without_parentheses() {
            if function.id.is_none() {
                self.self_read(function, own);
            }
        }
    }

    fn self_read(&mut self, function: &Function<'_>, name: &str) {
        if reads_its_own_text(function, name, self.printed) {
            self.found = true;
            self.broken |= !self.printed.on_one_line(function.span);
        }
    }
}

impl<'a> VisitJs<'a> for Readers<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.note(init, id.name.as_str());
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                self.note(&it.right, id.name.as_str());
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.self_read(it, id.name.as_str());
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if checks_an_escaped_spelling(it) {
            self.found = true;
            self.broken = true;
        }
        walk_js::walk_call_expression(self, it);
    }
}

struct SelfChain<'n> {
    name: &'n str,
    found: bool,
}

fn names(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn passes(call: &CallExpression<'_>, name: &str) -> bool {
    call.arguments
        .iter()
        .any(|argument| argument.as_expression().is_some_and(|arg| names(arg, name)))
}

fn reads_itself(call: &CallExpression<'_>, name: &str) -> bool {
    let mut passed = passes(call, name);
    let mut calls = 1;
    let mut current = &call.callee;
    loop {
        match current.without_parentheses() {
            Expression::CallExpression(inner) => {
                passed |= passes(inner, name);
                calls += 1;
                current = &inner.callee;
            }
            Expression::Identifier(_) => {
                return passed && calls >= CHAIN_CALLS;
            }
            other => match other.as_member_expression() {
                Some(member) => current = member.object(),
                None => return false,
            },
        }
    }
}

impl<'a> VisitJs<'a> for SelfChain<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if reads_itself(it, self.name) {
            self.found = true;
            return;
        }
        walk_js::walk_call_expression(self, it);
    }
}
