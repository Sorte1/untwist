use crate::ast_util::code_text::code_text_scan;
use crate::ast_util::name_keys::{
    call_reads_name, is_never_a_function, key_may_name, key_text, member_key,
    never_reaches_a_function, property_key_may_name, reader, NAME,
};
use crate::ast_util::name_keys::{
    counter_symbols, head_bindings, numeric_assignment, numeric_declarator, CounterBook,
};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

#[derive(Default)]
pub struct FunctionNames {
    pub givers: Vec<SymbolId>,
    pub name_read: bool,
}

fn walked<'s>(program: &Program<'_>, scoping: &'s Scoping, first_read: bool) -> Walker<'s> {
    let mut walker = Walker {
        scoping,
        out: FunctionNames::default(),
        first_read,
        counters: Vec::new(),
        book: CounterBook::default(),
    };
    walker.visit_program(program);
    walker.settle_counters();
    walker
}

pub fn function_names(program: &Program<'_>, scoping: &Scoping) -> FunctionNames {
    walked(program, scoping, false).out
}

pub fn function_names_or_code_text(program: &Program<'_>, scoping: &Scoping) -> FunctionNames {
    let mut walker = walked(program, scoping, false);
    walker.settle_code_text(program);
    walker.out
}

pub fn reads_function_names(program: &Program<'_>, scoping: &Scoping) -> bool {
    walked(program, scoping, true).out.name_read
}

pub fn reads_names_or_code_text(program: &Program<'_>, scoping: &Scoping) -> bool {
    let mut walker = walked(program, scoping, true);
    walker.settle_code_text(program);
    walker.out.name_read
}

struct Walker<'s> {
    scoping: &'s Scoping,
    out: FunctionNames,
    first_read: bool,
    counters: Vec<SymbolId>,
    book: CounterBook,
}

fn is_anonymous_function(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => function.id.is_none(),
        Expression::ArrowFunctionExpression(_) => true,
        Expression::ClassExpression(class) => class.id.is_none(),
        _ => false,
    }
}

fn names_on_assignment(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalNullish
    )
}

impl Walker<'_> {
    fn give(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.out.givers.push(symbol);
        }
    }

    fn give_pattern(&mut self, pattern: &BindingPattern<'_>, value: Option<&Expression<'_>>) {
        if let (BindingPattern::BindingIdentifier(id), Some(value)) = (pattern, value) {
            if is_anonymous_function(value) {
                self.give(id);
            }
        }
    }

    fn give_reference(&mut self, id: &IdentifierReference<'_>, value: &Expression<'_>) {
        if !is_anonymous_function(value) {
            return;
        }
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol {
            self.out.givers.push(symbol);
        }
    }

    fn defers_to_counters(&mut self, key: &Expression<'_>) -> bool {
        counter_symbols(self.scoping, key, &mut self.counters)
    }

    fn settle_counters(&mut self) {
        if !self.out.name_read && !self.book.all_counters(self.scoping, &self.counters) {
            self.out.name_read = true;
        }
    }

    fn settle_code_text(&mut self, program: &Program<'_>) {
        if self.out.name_read {
            return;
        }
        let scan = code_text_scan(program, self.scoping);
        if scan.found || !self.book.all_counters(self.scoping, &scan.counters) {
            self.out.name_read = true;
        }
    }

    fn write_counter(&mut self, id: &IdentifierReference<'_>) {
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol {
            self.book.write_numeric(symbol);
        }
    }

    fn note_block_function(&mut self, it: &Function<'_>, id: &BindingIdentifier<'_>) {
        let in_block = id.symbol_id.get().is_some_and(|symbol| {
            !self
                .scoping
                .scope_flags(self.scoping.symbol_scope_id(symbol))
                .is_var()
        });
        if it.is_declaration() && in_block {
            self.book.block_function(&id.name);
        }
    }

    fn bind_head(&mut self, left: &ForStatementLeft<'_>) {
        for symbol in head_bindings(left) {
            self.book.bind(symbol);
        }
    }

    fn read_on(&mut self, object: &Expression<'_>) {
        if !never_reaches_a_function(object) {
            self.out.name_read = true;
        }
    }

    fn visit_member_parts<'a>(&mut self, member: &MemberExpression<'a>) {
        match member {
            MemberExpression::StaticMemberExpression(it) => self.visit_expression(&it.object),
            MemberExpression::ComputedMemberExpression(it) => {
                self.visit_expression(&it.object);
                self.visit_expression(&it.expression);
            }
            MemberExpression::PrivateFieldExpression(it) => self.visit_expression(&it.object),
        }
    }

    fn visit_reader_call<'a>(&mut self, it: &CallExpression<'a>) -> bool {
        let Some(member) = it.callee.without_parentheses().as_member_expression() else {
            return false;
        };
        let Some(found) = member_key(member).and_then(|key| reader(&key)) else {
            return false;
        };
        if call_reads_name(found, &it.arguments) {
            self.out.name_read = true;
        }
        self.visit_member_parts(member);
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
        true
    }
}

impl<'a> VisitJs<'a> for Walker<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !(self.first_read && self.out.name_read) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !(self.first_read && self.out.name_read) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.out.name_read = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.give(id);
            self.note_block_function(it, id);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.give(id);
        }
        walk_js::walk_class(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.book.bind(symbol);
        }
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.bind_head(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.bind_head(&it.left);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &it.argument {
            self.write_counter(id);
        }
        walk_js::walk_update_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(symbol) = numeric_declarator(it) {
            self.book.bind_numeric(symbol);
        }
        self.give_pattern(&it.id, it.init.as_ref());
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        self.give_pattern(&it.left, Some(&it.right));
        walk_js::walk_assignment_pattern(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        self.give_pattern(&it.pattern, it.initializer.as_deref());
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if numeric_assignment(it.operator, &it.right) {
                self.write_counter(id);
            }
        }
        if names_on_assignment(it.operator) {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                self.give_reference(id, &it.right);
            }
        }
        if it.operator == AssignmentOperator::Assign {
            if let Some(member) = it.left.as_member_expression() {
                self.visit_member_parts(member);
                self.visit_expression(&it.right);
                return;
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            self.give_reference(id, &it.init);
        }
        walk_js::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if it.binding.name == NAME || reader(&it.binding.name).is_some() {
            self.out.name_read = true;
        }
        if let Some(init) = &it.init {
            self.give_reference(&it.binding, init);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        if property_key_may_name(&it.name, it.computed) {
            self.out.name_read = true;
        }
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if property_key_may_name(&it.key, it.computed) {
            self.out.name_read = true;
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == NAME || reader(&it.property.name).is_some() {
            self.read_on(&it.object);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let reads = key_text(&it.expression).is_some_and(|key| reader(&key).is_some());
        let may_name = || key_may_name(&it.expression);
        if reads
            || (may_name()
                && !never_reaches_a_function(&it.object)
                && !self.defers_to_counters(&it.expression))
        {
            self.read_on(&it.object);
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.visit_reader_call(it) {
            walk_js::walk_call_expression(self, it);
        }
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        let spreads = it.properties.iter().any(|property| match property {
            ObjectPropertyKind::SpreadProperty(spread) => !is_never_a_function(&spread.argument),
            ObjectPropertyKind::ObjectProperty(_) => false,
        });
        if spreads {
            self.out.name_read = true;
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        if it.rest.is_some() {
            self.out.name_read = true;
        }
        walk_js::walk_object_pattern(self, it);
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        if it.rest.is_some() {
            self.out.name_read = true;
        }
        walk_js::walk_object_assignment_target(self, it);
    }
}
