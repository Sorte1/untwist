use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_str::Ident;
use rustc_hash::FxHashMap;
use untwist_core::semantic::build_scoping;

use crate::matcher::literal_id;
use crate::model::Bundle;
use crate::runtime::synthetic_statements;
use crate::scan::is_free;
use crate::webpack::canon::canonical_statement;
use crate::webpack::gate::module_require;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buildin {
    Module,
    Harmony,
}

const NO_REQUIRE: &str = "$";

const MODULE_FORMS: &[&str] = &[
    r#"(function($0){if(!$0.webpackPolyfill){$0.deprecate=function(){};$0.paths=[];if(!$0.children){$0.children=[]}Object.defineProperty($0,"loaded",{enumerable:true,get:function(){return $0.l}});Object.defineProperty($0,"id",{enumerable:true,get:function(){return $0.i}});$0.webpackPolyfill=1}return $0});"#,
    r#"(function($0){if(!$0.webpackPolyfill){$0.deprecate=function(){};$0.paths=[];$0.children||=[];Object.defineProperty($0,"loaded",{enumerable:true,get:function(){return $0.l}});Object.defineProperty($0,"id",{enumerable:true,get:function(){return $0.i}});$0.webpackPolyfill=1}return $0});"#,
];

const HARMONY_FORMS: &[&str] = &[
    r#"(function($0){if(!$0.webpackPolyfill){var $1=Object.create($0);if(!$1.children){$1.children=[]}Object.defineProperty($1,"loaded",{enumerable:true,get:function(){return $1.l}});Object.defineProperty($1,"id",{enumerable:true,get:function(){return $1.i}});Object.defineProperty($1,"exports",{enumerable:true});$1.webpackPolyfill=1}return $1});"#,
    r#"(function($0){if(!$0.webpackPolyfill){var $1=Object.create($0);$1.children||=[];Object.defineProperty($1,"loaded",{enumerable:true,get:function(){return $1.l}});Object.defineProperty($1,"id",{enumerable:true,get:function(){return $1.i}});Object.defineProperty($1,"exports",{enumerable:true});$1.webpackPolyfill=1}return $1});"#,
];

fn polyfill_function<'k, 'a>(
    statements: &'k [Statement<'a>],
    module: &str,
) -> Option<&'k Function<'a>> {
    let [Statement::ExpressionStatement(statement)] = statements else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = statement.expression.without_parentheses()
    else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(target) = &assignment.left else {
        return None;
    };
    let installs = assignment.operator == AssignmentOperator::Assign
        && target.property.name == "exports"
        && matches!(&target.object, Expression::Identifier(id) if id.name == module);
    let Expression::FunctionExpression(function) = assignment.right.without_parentheses() else {
        return None;
    };
    installs.then_some(function)
}

pub fn buildin_text(statements: &[Statement<'_>], module: &str) -> Option<String> {
    let function = polyfill_function(statements, module)?;
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let expression = Expression::FunctionExpression(oxc_allocator::Box::new_in(
        function.clone_in(&alloc),
        &builder,
    ));
    let statement = Statement::new_expression_statement(SPAN, expression, &builder);
    canonical_statement(&statement, NO_REQUIRE)
}

pub fn buildin_kind(statements: &[Statement<'_>], module: &str) -> Option<Buildin> {
    let text = buildin_text(statements, module)?;
    if MODULE_FORMS.contains(&text.as_str()) {
        return Some(Buildin::Module);
    }
    HARMONY_FORMS
        .contains(&text.as_str())
        .then_some(Buildin::Harmony)
}

pub fn polyfill_ids<'k, 'a: 'k>(
    modules: impl IntoIterator<Item = (&'k str, &'k [Statement<'a>], &'k str)>,
) -> FxHashMap<String, Buildin> {
    modules
        .into_iter()
        .filter_map(|(id, statements, module)| {
            buildin_kind(statements, module).map(|kind| (id.to_string(), kind))
        })
        .collect()
}

pub fn polyfill_id<'k>(
    callee: &'k Expression<'_>,
    require: &str,
    ids: &'k FxHashMap<String, Buildin>,
) -> Option<Buildin> {
    let Expression::CallExpression(inner) = callee.without_parentheses() else {
        return None;
    };
    let required = !inner.optional
        && matches!(inner.callee.without_parentheses(), Expression::Identifier(id) if id.name == require);
    let [argument] = inner.arguments.as_slice() else {
        return None;
    };
    let id = literal_id(argument.as_expression()?)?;
    required.then(|| ids.get(&id).copied()).flatten()
}

pub const MODULE_NAME: &str = "module";

const RECORD: &str = "({ get l() { return module.loaded; }, get exports() { return module.exports; }, set exports(value) { module.exports = value; } });";

pub fn webpack_record<'a>(alloc: &'a Allocator) -> Option<Expression<'a>> {
    let mut statements = synthetic_statements(alloc, RECORD.to_string())?;
    let Some(Statement::ExpressionStatement(statement)) = statements.pop() else {
        return None;
    };
    match statement.unbox().expression {
        Expression::ParenthesizedExpression(inner) => Some(inner.unbox().expression),
        expression => Some(expression),
    }
}

pub fn bundle_buildins(bundle: &Bundle<'_>) -> FxHashMap<String, Buildin> {
    polyfill_ids(bundle.modules.iter().filter_map(|(id, module)| {
        let program = module.program();
        program
            .directives
            .is_empty()
            .then(|| (id.as_str(), program.body.as_slice(), MODULE_NAME))
    }))
}

struct DropCalls<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: Scoping,
    require: &'r str,
    ids: &'r FxHashMap<String, Buildin>,
    record: Option<Option<Expression<'a>>>,
    dropped: usize,
}

impl DropCalls<'_, '_> {
    fn dropped_call(&self, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr else {
            return false;
        };
        let Expression::CallExpression(inner) = call.callee.without_parentheses() else {
            return false;
        };
        let free_require = matches!(inner.callee.without_parentheses(), Expression::Identifier(id) if is_free(Some(&self.scoping), id));
        let module_arg = matches!(call.arguments.as_slice(), [Argument::Identifier(id)]
            if id.name == MODULE_NAME && is_free(Some(&self.scoping), id));
        free_require && module_arg && polyfill_id(&call.callee, self.require, self.ids).is_some()
    }
}

impl<'a> VisitMut<'a> for DropCalls<'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.dropped_call(it) && self.record.is_some() {
            let record = self.record.as_mut().and_then(Option::take);
            if let (Expression::CallExpression(call), Some(record)) = (it, record) {
                call.arguments[0] = Argument::from(record);
                self.dropped += 1;
            }
            return;
        }
        if self.dropped_call(it) {
            *it = Expression::new_identifier(
                SPAN,
                Ident::from_str_in(MODULE_NAME, &self.builder),
                &self.builder,
            );
            self.dropped += 1;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

pub fn drop_polyfill_calls<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> usize {
    let ids = bundle_buildins(bundle);
    if ids.is_empty() {
        return 0;
    }
    let requires: Vec<Option<String>> = bundle
        .modules
        .iter()
        .map(|(id, module)| module_require(bundle, id, module).map(str::to_string))
        .collect();
    let mut dropped = 0;
    for (module, require) in bundle.modules.values_mut().zip(requires) {
        let Some(require) = require else {
            continue;
        };
        let record = (!module.node_module).then(|| webpack_record(alloc));
        let program = module.program_mut();
        let mut drop = DropCalls {
            builder: AstBuilder::new(alloc),
            scoping: build_scoping(program).scoping,
            require: &require,
            ids: &ids,
            record,
            dropped: 0,
        };
        drop.visit_program(program);
        dropped += drop.dropped;
    }
    dropped
}
