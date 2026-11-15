use oxc_ast::ast::{
    Argument, AssignmentExpression, AssignmentTarget, AssignmentTargetPropertyIdentifier,
    AssignmentTargetPropertyProperty, BindingProperty, CallExpression, ComputedMemberExpression,
    Expression, IdentifierReference, ImportDeclaration, ImportExpression, NewExpression, Program,
    PropertyKey, StaticMemberExpression,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

use crate::ast_util::name_keys::counter_symbols;

const RUNNERS: [&str; 10] = [
    "eval",
    "Function",
    "execScript",
    "setTimeout",
    "setInterval",
    "constructor",
    "runInThisContext",
    "runInNewContext",
    "runInContext",
    "compileFunction",
];
const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];
const TEXT_MODULE_SCHEMES: [&str; 2] = ["data:", "blob:"];

const TEXT_HAZARDS: [&str; 18] = [
    "name",
    "\\",
    "[",
    "this",
    "globalThis",
    "window",
    "self",
    "global",
    "eval",
    "Function",
    "execScript",
    "setTimeout",
    "setInterval",
    "constructor",
    "runInThisContext",
    "runInNewContext",
    "runInContext",
    "compileFunction",
];

struct CodeText<'s> {
    scoping: Option<&'s Scoping>,
    found: bool,
    counters: Vec<SymbolId>,
}

pub fn may_run_code_text(program: &Program<'_>, _scoping: &Scoping) -> bool {
    let mut scan = CodeText {
        scoping: None,
        found: false,
        counters: Vec::new(),
    };
    scan.visit_program(program);
    scan.found
}

pub(crate) struct CodeTextScan {
    pub(crate) found: bool,
    pub(crate) counters: Vec<SymbolId>,
}

pub(crate) fn code_text_scan(program: &Program<'_>, scoping: &Scoping) -> CodeTextScan {
    let mut scan = CodeText {
        scoping: Some(scoping),
        found: false,
        counters: Vec::new(),
    };
    scan.visit_program(program);
    CodeTextScan {
        found: scan.found,
        counters: scan.counters,
    }
}

pub(crate) fn literal_text(expression: &Expression<'_>) -> Option<String> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str())
            .map(str::to_string),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            Some(literal_text(&binary.left)? + &literal_text(&binary.right)?)
        }
        _ => None,
    }
}

pub(crate) fn is_number(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => true,
        Expression::UnaryExpression(unary) => matches!(
            unary.operator,
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
        ),
        Expression::BinaryExpression(binary) => {
            (binary.operator.is_arithmetic() && binary.operator != BinaryOperator::Addition)
                || binary.operator.is_bitwise()
        }
        Expression::AssignmentExpression(assignment) => match assignment.operator {
            AssignmentOperator::Assign => is_number(&assignment.right),
            AssignmentOperator::Addition => false,
            operator => operator.is_arithmetic() || operator.is_bitwise(),
        },
        _ => false,
    }
}

fn key_may_run(key: &Expression<'_>) -> bool {
    if is_number(key) {
        return false;
    }
    literal_text(key).is_none_or(|name| RUNNERS.contains(&name.as_str()))
}

fn property_key_may_run(key: &PropertyKey<'_>, computed: bool) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => RUNNERS.contains(&id.name.as_str()),
        PropertyKey::PrivateIdentifier(_) => false,
        _ if !computed => key
            .as_expression()
            .and_then(literal_text)
            .is_some_and(|name| RUNNERS.contains(&name.as_str())),
        _ => key.as_expression().is_none_or(key_may_run),
    }
}

fn texts_read_no_name(arguments: &[Argument<'_>]) -> bool {
    arguments.iter().all(|argument| {
        argument
            .as_expression()
            .and_then(literal_text)
            .is_some_and(|text| !TEXT_HAZARDS.iter().any(|hazard| text.contains(hazard)))
    })
}

fn names_a_module_file(source: Option<&str>) -> bool {
    source.is_some_and(|text| {
        let text = text.trim_start().to_ascii_lowercase();
        !TEXT_MODULE_SCHEMES
            .iter()
            .any(|scheme| text.starts_with(scheme))
    })
}

fn names_a_declared_function(scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
    let symbol = id
        .reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id());
    symbol.is_some_and(|symbol| {
        let flags = scoping.symbol_flags(symbol);
        flags.is_function()
            && !flags.intersects(SymbolFlags::Variable)
            && !scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
    })
}

fn calls_back(first: Option<&Argument<'_>>, scoping: Option<&Scoping>) -> bool {
    match first
        .and_then(Argument::as_expression)
        .map(Expression::without_parentheses)
    {
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) => true,
        Some(Expression::Identifier(id)) => {
            scoping.is_some_and(|scoping| names_a_declared_function(scoping, id))
        }
        _ => false,
    }
}

const OPAQUE_KEY: &str = "constructor";

fn runner_name<'e>(callee: &'e Expression<'_>) -> Option<&'e str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        Expression::ComputedMemberExpression(member) if key_may_run(&member.expression) => {
            Some(OPAQUE_KEY)
        }
        _ => None,
    }
    .filter(|name| RUNNERS.contains(name))
}

impl CodeText<'_> {
    fn is_runner(id: &IdentifierReference<'_>) -> bool {
        RUNNERS.contains(&id.name.as_str())
    }

    fn harmless_call(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let Some(name) = runner_name(callee) else {
            return false;
        };
        if TIMERS.contains(&name) && calls_back(arguments.first(), self.scoping) {
            return true;
        }
        let texts = match TIMERS.contains(&name) {
            true => arguments.get(..1).unwrap_or_default(),
            false => arguments,
        };
        !arguments.is_empty() && texts_read_no_name(texts)
    }

    fn visit_callee_object<'a>(&mut self, callee: &Expression<'a>) {
        match callee.without_parentheses() {
            Expression::StaticMemberExpression(member) => self.visit_expression(&member.object),
            Expression::ComputedMemberExpression(member) => {
                self.visit_expression(&member.object);
                self.visit_expression(&member.expression);
            }
            _ => {}
        }
    }

    fn visit_run<'a>(&mut self, callee: &Expression<'a>, arguments: &[Argument<'a>]) -> bool {
        if !self.harmless_call(callee, arguments) {
            return false;
        }
        self.visit_callee_object(callee);
        for argument in arguments {
            self.visit_argument(argument);
        }
        true
    }
}

impl<'a> VisitJs<'a> for CodeText<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.found {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= Self::is_runner(it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if RUNNERS.contains(&it.property.name.as_str()) {
            self.found = true;
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let deferred = |scan: &mut Self| {
            scan.scoping
                .is_some_and(|scoping| counter_symbols(scoping, &it.expression, &mut scan.counters))
        };
        if key_may_run(&it.expression) && !deferred(self) {
            self.found = true;
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let written = match (&it.left, it.operator) {
            (AssignmentTarget::ComputedMemberExpression(member), AssignmentOperator::Assign) => {
                Some(member)
            }
            _ => None,
        };
        let Some(member) = written else {
            walk_js::walk_assignment_expression(self, it);
            return;
        };
        self.visit_expression(&member.object);
        self.visit_expression(&member.expression);
        self.visit_expression(&it.right);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.visit_run(&it.callee, &it.arguments) {
            walk_js::walk_call_expression(self, it);
        }
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if !self.visit_run(&it.callee, &it.arguments) {
            walk_js::walk_new_expression(self, it);
        }
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.found |= property_key_may_run(&it.key, it.computed);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.found |= property_key_may_run(&it.name, it.computed);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.found |= RUNNERS.contains(&it.binding.name.as_str());
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        let literal = match it.source.without_parentheses() {
            Expression::StringLiteral(literal) => literal.value.as_str(),
            _ => None,
        };
        if !names_a_module_file(literal) {
            self.found = true;
            return;
        }
        walk_js::walk_import_expression(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.found |= !names_a_module_file(it.source.value.as_str());
    }
}
