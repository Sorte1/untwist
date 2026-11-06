use std::collections::BTreeSet;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;

use super::shapes::*;
use super::*;

pub(super) struct Scan<'s> {
    pub(super) source: &'s str,
    pub(super) closure: &'s BTreeSet<String>,
    pub(super) found: Found,
    pub(super) folded_depth: usize,
}

impl Scan<'_> {
    fn text(&self, start: u32, end: u32) -> &str {
        self.source.get(start as usize..end as usize).unwrap_or("")
    }

    fn string_value(&mut self, value: &str) {
        if DEBUG_STRINGS.contains(&value) || DEBUG_PATTERNS.iter().any(|p| value.contains(p)) {
            self.found.bump("debug_protection");
        }
        if value.contains(SELF_DEFENDING) {
            self.found.bump("self_defending");
        }
        if value.contains(GLOBAL_GETTER) {
            self.found.bump("global_getters");
        }
        if value == REDIRECT {
            self.found.bump("domain_lock");
        }
        if is_order_string(value) {
            self.found.bump("dispatchers");
        }
    }

    fn statement_list(&mut self, statements: &[Statement]) {
        for pair in statements.windows(2) {
            let Statement::VariableDeclaration(decl) = &pair[0] else {
                continue;
            };
            let Some(last) = decl.declarations.last() else {
                continue;
            };
            let empty = matches!(
                last.init.as_ref().map(|e| e.without_parentheses()),
                Some(Expression::ObjectExpression(o)) if o.properties.is_empty()
            );
            let BindingPattern::BindingIdentifier(id) = &last.id else {
                continue;
            };
            if empty && assigns_member_of(&pair[1], id.name.as_str()) {
                self.found.bump("object_key_assignments");
            }
        }
    }
}

fn assigns_member_of(statement: &Statement, name: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = statement else {
        return false;
    };
    let first = match stmt.expression.without_parentheses() {
        Expression::SequenceExpression(seq) => seq.expressions.first(),
        other => Some(other),
    };
    let Some(Expression::AssignmentExpression(assign)) = first.map(|e| e.without_parentheses())
    else {
        return false;
    };
    let object = match &assign.left {
        AssignmentTarget::ComputedMemberExpression(m) => &m.object,
        AssignmentTarget::StaticMemberExpression(m) => &m.object,
        _ => return false,
    };
    ident_name(object) == Some(name)
}

impl<'a> Visit<'a> for Scan<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if ident_name(&call.callee).is_some_and(|n| self.closure.contains(n)) {
            self.found.bump("decoder_calls");
        }
        if is_rotation(call) {
            self.found.bump("rotators");
        }
        if is_hidden_string_replace(call) {
            self.found.bump("domain_lock");
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        let mut checks = ForInChecks { found: false };
        checks.visit_statement(&stmt.body);
        if checks.found {
            self.found.bump("domain_lock");
        }
        walk::walk_for_in_statement(self, stmt);
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        if is_call_controller(func) {
            self.found.bump("call_controllers");
        }
        walk::walk_function(self, func, flags);
    }

    fn visit_array_expression(&mut self, array: &ArrayExpression<'a>) {
        let strings: Vec<&str> = array
            .elements
            .iter()
            .filter_map(|e| match e {
                ArrayExpressionElement::StringLiteral(s) => s.value.as_str(),
                _ => None,
            })
            .collect();
        if strings.len() >= MIN_LOOSE_ARRAY_STRINGS && strings.len() == array.elements.len() {
            self.found.bump("string_arrays");
        }
        if CONSOLE_METHODS.iter().all(|m| strings.contains(m)) {
            self.found.bump("console_override");
        }
        walk::walk_array_expression(self, array);
    }

    fn visit_switch_statement(&mut self, switch: &SwitchStatement<'a>) {
        if is_dispatcher(switch) {
            self.found.bump("dispatchers");
        }
        walk::walk_switch_statement(self, switch);
    }

    fn visit_object_expression(&mut self, object: &ObjectExpression<'a>) {
        if is_storage_object(object) {
            self.found.bump("proxy_objects");
        }
        walk::walk_object_expression(self, object);
    }

    fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
        if let (false, PropertyKey::StringLiteral(key)) = (prop.computed, &prop.key) {
            if key.value.as_str().is_some_and(is_identifier_name) {
                self.found.bump("quoted_keys");
            }
        }
        walk::walk_object_property(self, prop);
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        if let PropertyKey::StringLiteral(key) = &method.key {
            if key.value.as_str().is_some_and(is_identifier_name) {
                self.found.bump("quoted_keys");
            }
        }
        walk::walk_method_definition(self, method);
    }

    fn visit_property_definition(&mut self, prop: &PropertyDefinition<'a>) {
        if let PropertyKey::StringLiteral(key) = &prop.key {
            if key.value.as_str().is_some_and(is_identifier_name) {
                self.found.bump("quoted_keys");
            }
        }
        walk::walk_property_definition(self, prop);
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(key) = &member.expression {
            if key.value.as_str().is_some_and(is_identifier_name) {
                self.found.bump("computed_members");
            }
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression<'a>) {
        let folded = if is_number_expression(binary) {
            Some("number_expressions")
        } else if is_split_string(binary) {
            Some("split_strings")
        } else {
            None
        };
        if let Some(class) = folded {
            if self.folded_depth == 0 {
                self.found.bump(class);
            }
            self.folded_depth += 1;
            walk::walk_binary_expression(self, binary);
            self.folded_depth -= 1;
            return;
        }
        let equality = matches!(
            binary.operator,
            BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::Equality
                | BinaryOperator::Inequality
        );
        let both_strings = matches!(
            binary.left.without_parentheses(),
            Expression::StringLiteral(_)
        ) && matches!(
            binary.right.without_parentheses(),
            Expression::StringLiteral(_)
        );
        if equality && both_strings {
            self.found.bump("opaque_predicates");
        }
        walk::walk_binary_expression(self, binary);
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression<'a>) {
        let empty_array = matches!(
            unary.argument.without_parentheses(),
            Expression::ArrayExpression(a) if a.elements.is_empty()
        );
        if unary.operator == UnaryOperator::LogicalNot && empty_array {
            self.found.bump("bool_arrays");
        }
        walk::walk_unary_expression(self, unary);
    }

    fn visit_numeric_literal(&mut self, lit: &NumericLiteral<'a>) {
        let raw = self.text(lit.span.start, lit.span.end);
        if raw.starts_with("0x") || raw.starts_with("0X") {
            self.found.bump("hex_numbers");
        }
    }

    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        let escapes = printable_escapes(self.text(lit.span.start, lit.span.end));
        if escapes > 0 {
            self.found.bump("escapes");
        }
        if let Some(value) = lit.value.as_str() {
            self.string_value(value);
        }
    }

    fn visit_debugger_statement(&mut self, _: &DebuggerStatement) {
        self.found.bump("debug_protection");
    }

    fn visit_program(&mut self, program: &Program<'a>) {
        self.statement_list(&program.body);
        walk::walk_program(self, program);
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        self.statement_list(&body.statements);
        walk::walk_function_body(self, body);
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.statement_list(&block.body);
        walk::walk_block_statement(self, block);
    }
}
