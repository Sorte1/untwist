use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};
use oxc_syntax::scope::ScopeFlags;

use super::controller::handed_out_targets;
use crate::ast_util::prop::member_parts;
use crate::parse::{has_error, parse_program};

const LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];
const READER_PARAMS: usize = 2;
const CHAIN_CALLS: usize = 3;
const ESCAPED_BAIT: &str = "window";
const PREFILTERS: [&str; 2] = ["this, function () {", "this, () => {"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReaders {
    None,
    Guarded,
    Unguarded,
}

pub fn source_readers(code: &str) -> SourceReaders {
    if !PREFILTERS.iter().any(|needle| code.contains(needle)) {
        return SourceReaders::None;
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, code);
    if has_error(&parsed.diagnostics) {
        return SourceReaders::None;
    }
    let mut on_one_line = |target: &Function<'_>| {
        let span = target.span();
        code.get(span.start as usize..span.end as usize)
            .is_some_and(|printed| !printed.contains(LINE_TERMINATORS))
    };
    judge_readers(&parsed.program, &mut on_one_line)
}

pub fn judge_readers(
    program: &Program<'_>,
    on_one_line: &mut dyn FnMut(&Function<'_>) -> bool,
) -> SourceReaders {
    let mut readers = Readers {
        on_one_line: &mut *on_one_line,
        controllers: Vec::new(),
        self_readers: false,
        self_broken: false,
    };
    readers.visit_program(program);
    let (controllers, self_readers, self_broken) = (
        readers.controllers,
        readers.self_readers,
        readers.self_broken,
    );
    if controllers.is_empty() {
        return match (self_readers, self_broken) {
            (_, true) => SourceReaders::Unguarded,
            (true, false) => SourceReaders::Guarded,
            (false, false) => SourceReaders::None,
        };
    }
    let mut targets = Targets {
        on_one_line,
        names: &controllers,
        found: vec![false; controllers.len()],
        broken: false,
    };
    targets.visit_program(program);
    if self_broken || targets.broken || targets.found.contains(&false) {
        SourceReaders::Unguarded
    } else {
        SourceReaders::Guarded
    }
}

const SELF_READ: &str = "charCodeAt";

struct SelfText<'n> {
    name: &'n str,
    texts: Vec<String>,
    read: bool,
    escaped: bool,
}

fn is_function_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

fn adds(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::BinaryExpression(sum)
        if sum.operator == BinaryOperator::Addition && names(&sum.right, name))
}

impl<'a> VisitJs<'a> for SelfText<'_> {
    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if adds(init, self.name) {
                self.texts.push(id.name.to_string());
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator == BinaryOperator::Addition {
            self.escaped |= is_function_literal(&it.left) || is_function_literal(&it.right);
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

fn reads_its_own_text(function: &Function<'_>, name: &str) -> bool {
    let Some(body) = &function.body else {
        return false;
    };
    let mut text = SelfText {
        name,
        texts: Vec::new(),
        read: false,
        escaped: false,
    };
    text.visit_function_body(body);
    text.read && !text.escaped
}

struct Readers<'j> {
    on_one_line: &'j mut dyn FnMut(&Function<'_>) -> bool,
    controllers: Vec<String>,
    self_readers: bool,
    self_broken: bool,
}

fn callback_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k FunctionBody<'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) if function.params.items.is_empty() => {
            function.body.as_deref()
        }
        Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(body),
                _ => None,
            }
        }
        _ => None,
    }
}

fn controller_and_callback<'k, 'a>(
    call: &'k CallExpression<'a>,
) -> Option<(&'k Expression<'a>, &'k Expression<'a>)> {
    match call.arguments.as_slice() {
        [Argument::ThisExpression(_), callback] => {
            Some((call.callee.without_parentheses(), callback.as_expression()?))
        }
        [controller, Argument::ThisExpression(_), callback] => Some((
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
    let Some(body) =
        controller_and_callback(call).and_then(|(_, callback)| callback_body(callback))
    else {
        return false;
    };
    let mut check = EscapeCheck { found: false };
    check.visit_function_body(body);
    check.found
}

fn reader_controller<'k>(init: &'k Expression<'_>, own: &str) -> Option<&'k str> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let (controller, callback) = controller_and_callback(call)?;
    let body = callback_body(callback)?;
    let mut chain = SelfChain {
        name: own,
        found: false,
    };
    chain.visit_function_body(body);
    if !chain.found {
        return None;
    }
    match controller {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => Some(""),
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
            self.self_readers = true;
            self.self_broken = true;
        }
        walk_js::walk_call_expression(self, it);
    }
}

impl Readers<'_> {
    fn note(&mut self, init: &Expression<'_>, own: &str) {
        if let Some(controller) = reader_controller(init, own) {
            self.controllers.push(controller.to_string());
        }

        if let Expression::FunctionExpression(function) = init.without_parentheses() {
            if function.id.is_none() {
                self.self_read(function, own);
            }
        }
    }

    fn self_read(&mut self, function: &Function<'_>, name: &str) {
        if reads_its_own_text(function, name) {
            self.self_readers = true;
            self.self_broken |= !(self.on_one_line)(function);
        }
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

struct Targets<'n, 'j> {
    on_one_line: &'j mut dyn FnMut(&Function<'_>) -> bool,
    names: &'n [String],
    found: Vec<bool>,
    broken: bool,
}

impl Targets<'_, '_> {
    fn settle(&mut self, own: &str, init: &Expression<'_>) {
        let wanted: Vec<usize> = self
            .names
            .iter()
            .enumerate()
            .filter(|(_, name)| name.as_str() == own)
            .map(|(index, _)| index)
            .collect();
        if wanted.is_empty() {
            return;
        }
        let mut handed = HandedOut {
            on_one_line: &mut *self.on_one_line,
            seen: false,
            broken: false,
        };
        handed.visit_expression(init);
        if !handed.seen {
            return;
        }
        self.broken |= handed.broken;
        for index in wanted {
            self.found[index] = true;
        }
    }
}

impl<'a> VisitJs<'a> for Targets<'_, '_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.settle(id.name.as_str(), init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                self.settle(id.name.as_str(), &it.right);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some((controller, _)) = controller_and_callback(it) {
            if !matches!(controller, Expression::Identifier(_)) {
                self.settle("", controller);
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}

struct HandedOut<'j> {
    on_one_line: &'j mut dyn FnMut(&Function<'_>) -> bool,
    seen: bool,
    broken: bool,
}

impl<'a> VisitJs<'a> for HandedOut<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.params.items.len() == READER_PARAMS {
            let targets = handed_out_targets(it);
            if !targets.is_empty() {
                self.seen = true;
                for target in targets {
                    self.broken |= !(self.on_one_line)(target);
                }
                return;
            }
        }
        walk_js::walk_function(self, it, flags);
    }
}
