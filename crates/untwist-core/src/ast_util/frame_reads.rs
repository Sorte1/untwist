use oxc_ast::ast::{
    Argument, AssignmentTargetPropertyIdentifier, AssignmentTargetPropertyProperty,
    BindingProperty, CallExpression, ComputedMemberExpression, Expression, IdentifierReference,
    ImportExpression, MemberExpression, Program, PropertyKey, Statement, StaticMemberExpression,
    StringLiteral, TemplateLiteral,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;

const FRAME_PROPERTIES: [&str; 2] = ["arguments", "caller"];
const ARGUMENTS_PROPERTY: [&str; 1] = ["arguments"];

const KEYED_READERS: [(&str, usize); 3] = [
    ("get", 1),
    ("getOwnPropertyDescriptor", 1),
    ("__lookupGetter__", 0),
];
const WHOLE_READERS: [&str; 1] = ["getOwnPropertyDescriptors"];

const CODE_RUNNERS: [&str; 3] = ["eval", "Function", "execScript"];
const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];
const CODE_REACHING_MEMBERS: [&str; 5] = [
    "constructor",
    "runInThisContext",
    "runInNewContext",
    "runInContext",
    "compileFunction",
];
const TEXT_MODULE_SCHEMES: [&str; 2] = ["data:", "blob:"];

fn is_never_a_function(object: &Expression<'_>) -> bool {
    match object.without_parentheses() {
        Expression::Identifier(id) => id.name == "arguments",
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::TemplateLiteral(_)
        | Expression::ArrayExpression(_)
        | Expression::ObjectExpression(_) => true,
        _ => false,
    }
}

fn literal_key_name<'k>(key: &'k Expression<'_>) -> Option<&'k str> {
    match key.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

fn static_key_name<'k>(key: &'k PropertyKey<'_>) -> Option<&'k str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn member_name<'m>(member: &'m MemberExpression<'_>) -> Option<&'m str> {
    match member {
        MemberExpression::StaticMemberExpression(it) => Some(it.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(it) => literal_key_name(&it.expression),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn is_reader(name: &str) -> bool {
    WHOLE_READERS.contains(&name) || KEYED_READERS.iter().any(|(reader, _)| *reader == name)
}

fn reaches_code(name: &str) -> bool {
    CODE_RUNNERS.contains(&name) || TIMERS.contains(&name) || CODE_REACHING_MEMBERS.contains(&name)
}

struct FrameReads<'s> {
    names: &'static [&'static str],
    code_text: bool,
    opaque_keys: bool,
    spelled_text: bool,
    scoping: &'s Scoping,
    found: bool,
}

impl FrameReads<'_> {
    fn is_frame_property(&self, name: &str) -> bool {
        self.names.contains(&name)
    }

    fn names_another_key(&self, key: &Expression<'_>) -> bool {
        match key.without_parentheses() {
            Expression::NumericLiteral(_) | Expression::BigIntLiteral(_) => true,
            other => match literal_key_name(other) {
                Some(name) => !self.is_frame_property(name),
                None => !self.opaque_keys,
            },
        }
    }

    fn is_suspect_name(&self, name: &str) -> bool {
        !self.spelled_text && (is_reader(name) || (self.code_text && reaches_code(name)))
    }

    fn is_suspect_key(&self, computed: bool, key: &PropertyKey<'_>) -> bool {
        let name = match key.as_expression().filter(|_| computed) {
            Some(key) if !self.names_another_key(key) => return true,
            Some(key) => literal_key_name(key),
            None => static_key_name(key),
        };
        name.is_some_and(|name| self.is_frame_property(name) || self.is_suspect_name(name))
    }

    fn resolves(&self, it: &IdentifierReference<'_>) -> Option<oxc_syntax::symbol::SymbolId> {
        it.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn global_name<'n>(&self, callee: &'n Expression<'_>) -> Option<&'n str> {
        match callee {
            Expression::Identifier(id) if self.resolves(id).is_none() => Some(id.name.as_str()),
            _ => None,
        }
    }

    fn calls_back(&self, first: Option<&Argument<'_>>) -> bool {
        let Some(expression) = first.and_then(Argument::as_expression) else {
            return false;
        };
        match expression.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) => self
                .resolves(id)
                .is_some_and(|symbol| self.scoping.symbol_flags(symbol).is_function()),
            Expression::CallExpression(call) => call
                .callee
                .without_parentheses()
                .as_member_expression()
                .and_then(member_name)
                .is_some_and(|name| name == "bind"),
            _ => false,
        }
    }

    fn reader_reads_a_frame(&self, name: &str, arguments: &[Argument<'_>]) -> bool {
        let Some(&(_, index)) = KEYED_READERS.iter().find(|(reader, _)| *reader == name) else {
            return true;
        };
        let spread = arguments
            .iter()
            .take(index + 1)
            .any(|argument| matches!(argument, Argument::SpreadElement(_)));
        spread
            || arguments
                .get(index)
                .and_then(Argument::as_expression)
                .is_some_and(|key| !self.names_another_key(key))
    }

    fn call_is_harmless(&self, name: &str, arguments: &[Argument<'_>]) -> Option<bool> {
        if self.spelled_text && WHOLE_READERS.contains(&name) {
            return Some(true);
        }
        if is_reader(name) {
            return Some(!self.reader_reads_a_frame(name, arguments));
        }
        if self.code_text && TIMERS.contains(&name) {
            return Some(self.calls_back(arguments.first()));
        }
        None
    }

    fn visit_callee_object<'a>(&mut self, callee: &Expression<'a>) {
        match callee.as_member_expression() {
            Some(MemberExpression::ComputedMemberExpression(it)) => {
                self.visit_expression(&it.object);
                self.visit_expression(&it.expression);
            }
            Some(member) => self.visit_expression(member.object()),
            None => {}
        }
    }
}

impl<'a> VisitJs<'a> for FrameReads<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.found {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.found {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let name = it.property.name.as_str();
        let frame = self.is_frame_property(name) && !is_never_a_function(&it.object);
        if frame || self.is_suspect_name(name) {
            self.found = true;
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let frame = !self.names_another_key(&it.expression) && !is_never_a_function(&it.object);
        let suspect =
            literal_key_name(&it.expression).is_some_and(|name| self.is_suspect_name(name));
        if frame || suspect {
            self.found = true;
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let callee = it.callee.without_parentheses();
        let name = callee
            .as_member_expression()
            .and_then(member_name)
            .or_else(|| self.global_name(callee));
        match name.and_then(|name| self.call_is_harmless(name, &it.arguments)) {
            Some(false) => self.found = true,
            Some(true) => {
                self.visit_callee_object(callee);
                for argument in &it.arguments {
                    self.visit_argument(argument);
                }
            }
            None => walk_js::walk_call_expression(self, it),
        }
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        let names_a_module = match it.source.without_parentheses() {
            Expression::StringLiteral(literal) => literal.value.as_str().is_some_and(|text| {
                !TEXT_MODULE_SCHEMES
                    .iter()
                    .any(|scheme| text.starts_with(scheme))
            }),
            _ => false,
        };
        if self.code_text && !names_a_module {
            self.found = true;
            return;
        }
        walk_js::walk_import_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        let runner = CODE_RUNNERS.contains(&name) || TIMERS.contains(&name);
        self.found |= self.code_text && runner && self.resolves(it).is_none();
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.found |= self.is_suspect_key(it.computed, &it.key);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.found |= self.is_suspect_key(it.computed, &it.name);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let name = it.binding.name.as_str();
        self.found |= self.is_frame_property(name) || self.is_suspect_name(name);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        let spells = it.quasis.iter().any(|quasi| {
            let raw = quasi.value.raw.as_str();
            self.names.iter().any(|name| raw.contains(name))
        });
        if spells {
            self.found = true;
            return;
        }
        walk_js::walk_template_literal(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found |= it.value.as_str().is_some_and(|text| {
            self.is_frame_property(text)
                || (self.spelled_text && self.names.iter().any(|name| text.contains(name)))
        });
    }
}

fn scan(
    program: &Program<'_>,
    scoping: &Scoping,
    names: &'static [&'static str],
    (code_text, opaque_keys): (bool, bool),
) -> bool {
    let mut reads = FrameReads {
        names,
        code_text,
        opaque_keys,
        spelled_text: !code_text && !opaque_keys,
        scoping,
        found: false,
    };
    reads.visit_program(program);
    reads.found
}

pub fn reads_function_frame(program: &Program<'_>, scoping: &Scoping) -> bool {
    scan(program, scoping, &FRAME_PROPERTIES, (true, true))
}

pub fn spells_function_frame(program: &Program<'_>, scoping: &Scoping) -> bool {
    scan(program, scoping, &FRAME_PROPERTIES, (false, false))
}

pub fn reads_arguments_property(program: &Program<'_>, scoping: &Scoping) -> bool {
    scan(program, scoping, &ARGUMENTS_PROPERTY, (false, true))
}
