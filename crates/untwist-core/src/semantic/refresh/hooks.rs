use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::reference::ReferenceFlags;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolFlags;

use super::{Context, Walk};

fn declaration_context(kind: VariableDeclarationKind) -> Context {
    match kind {
        VariableDeclarationKind::Var => Context::Var,
        VariableDeclarationKind::Let => Context::Lexical(SymbolFlags::BlockScopedVariable),
        VariableDeclarationKind::Const
        | VariableDeclarationKind::Using
        | VariableDeclarationKind::AwaitUsing => {
            Context::Lexical(SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable)
        }
    }
}

fn function_context(it: &Function<'_>) -> Context {
    let declaration = it.is_declaration();
    let mut includes = SymbolFlags::Function;
    if !declaration {
        includes |= SymbolFlags::FunctionExpression;
    }
    if it.r#async || it.generator {
        includes |= SymbolFlags::AsyncOrGeneratorFunction;
    }
    Context::FunctionName {
        declaration,
        includes,
    }
}

fn method_flags(kind: MethodDefinitionKind) -> ScopeFlags {
    match kind {
        MethodDefinitionKind::Get => ScopeFlags::Function | ScopeFlags::GetAccessor,
        MethodDefinitionKind::Set => ScopeFlags::Function | ScopeFlags::SetAccessor,
        MethodDefinitionKind::Constructor => ScopeFlags::Function | ScopeFlags::Constructor,
        MethodDefinitionKind::Method => ScopeFlags::Function,
    }
}

fn accessor_flags(kind: PropertyKind) -> ScopeFlags {
    match kind {
        PropertyKind::Get => ScopeFlags::GetAccessor,
        PropertyKind::Set => ScopeFlags::SetAccessor,
        PropertyKind::Init => ScopeFlags::empty(),
    }
}

impl<'a> Walk<'_, 'a> {
    fn within(&mut self, context: Option<Context>, visit: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.context, context);
        visit(self);
        self.context = outer;
    }
}

impl<'a> VisitJs<'a> for Walk<'_, 'a> {
    fn enter_node(&mut self, _kind: AstKind<'a>) {
        self.nodes += 1;
    }

    fn enter_scope(&mut self, flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.enter(flags, scope_id.get());
    }

    fn leave_scope(&mut self) {
        self.leave();
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.failed {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.failed {
            return;
        }
        if self.context.is_none() {
            walk_js::walk_expression(self, it);
        } else {
            self.within(None, |walk| walk_js::walk_expression(walk, it));
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.nodes += 1;
        self.bind(it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.nodes += 1;
        self.refer(it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.declare {
            return self.fail();
        }
        let context = declaration_context(it.kind);
        self.within(Some(context), |walk| {
            walk_js::walk_variable_declaration(walk, it);
        });
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.failed || it.declare {
            return self.fail();
        }
        let context = function_context(it);
        self.within(Some(context), |walk| {
            walk_js::walk_function(walk, it, flags)
        });
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.within(None, |walk| walk_js::walk_function_body(walk, it));
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        self.enter_params(it);
        walk_js::walk_formal_parameters(self, it);
        self.leave_params();
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        self.within(Some(Context::Param), |walk| {
            walk_js::walk_formal_parameter(walk, it);
        });
    }

    fn visit_formal_parameter_rest(&mut self, it: &FormalParameterRest<'a>) {
        self.within(Some(Context::Param), |walk| {
            walk_js::walk_formal_parameter_rest(walk, it);
        });
    }

    fn visit_catch_parameter(&mut self, it: &CatchParameter<'a>) {
        let includes = if it.pattern.is_binding_identifier() {
            SymbolFlags::FunctionScopedVariable | SymbolFlags::CatchVariable
        } else {
            SymbolFlags::BlockScopedVariable | SymbolFlags::CatchVariable
        };
        self.enter_catch_param(it);
        self.within(Some(Context::CatchParam(includes)), |walk| {
            walk_js::walk_catch_parameter(walk, it);
        });
        self.leave_params();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if it.declare || !it.decorators.is_empty() {
            return self.fail();
        }
        let context = Context::ClassName {
            declaration: it.is_declaration(),
            scope: it.scope_id.get(),
        };
        self.within(Some(context), |walk| walk_js::walk_class(walk, it));
    }

    fn visit_class_body(&mut self, it: &ClassBody<'a>) {
        self.within(None, |walk| walk_js::walk_class_body(walk, it));
    }

    fn visit_class_heritage(&mut self, it: &ClassHeritage<'a>) {
        walk_js::walk_class_heritage(self, it);
        self.flags = ReferenceFlags::empty();
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        self.nodes += 1;
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.flags = ReferenceFlags::empty();
        self.visit_function(&it.value, method_flags(it.kind));
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.nodes += 1;
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.flags = ReferenceFlags::empty();
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        self.nodes += 1;
        self.visit_property_key(&it.key);
        match &it.value {
            Expression::FunctionExpression(function) if !self.failed => {
                let flags = ScopeFlags::Function | accessor_flags(it.kind);
                self.within(None, |walk| walk.visit_function(function, flags));
            }
            value => self.visit_expression(value),
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !it.optional && it.callee.is_specific_id("eval") {
            self.direct_eval();
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if !it.operator.is_assign() {
            self.flags = ReferenceFlags::read_write();
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.nodes += 1;
        let outer = self.flags;
        self.flags -= ReferenceFlags::MemberWriteTarget;
        self.visit_expression(&it.test);
        self.flags = outer;
        self.visit_expression(&it.consequent);
        self.visit_expression(&it.alternate);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.flags = ReferenceFlags::read_write();
        walk_js::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete && it.argument.is_member_expression() {
            self.flags = ReferenceFlags::Write;
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if self.flags.is_write() {
            self.flags = ReferenceFlags::Read | ReferenceFlags::MemberWriteTarget;
        } else {
            self.flags -= ReferenceFlags::Write;
        }
        walk_js::walk_member_expression(self, it);
        self.flags = ReferenceFlags::empty();
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.nodes += 1;
        self.visit_expression(&it.object);
        self.flags -= ReferenceFlags::MemberWriteTarget;
        self.visit_expression(&it.expression);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if !self.flags.is_write() {
            self.flags = ReferenceFlags::Write;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.nodes += 1;
        self.flags = ReferenceFlags::Write;
        self.visit_identifier_reference(&it.binding);
        if let Some(init) = &it.init {
            self.visit_expression(init);
        }
    }
}
