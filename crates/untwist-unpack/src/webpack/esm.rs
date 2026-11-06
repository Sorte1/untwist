use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span};
use oxc_str::{Ident, Str};
use oxc_syntax::operator::AssignmentOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::{identifier_name, number_key};

use crate::matcher::{argument_expression, call_parts, const_member, expression_statement};
use crate::model::Module;
use crate::rename::rename_binding;

#[inline]
pub(crate) fn require_helper_call<'k, 'a>(
    expr: &'k Expression<'a>,
    helper: &str,
) -> Option<&'k oxc_allocator::Vec<'a, Argument<'a>>> {
    let (callee, arguments) = call_parts(expr)?;
    let object = const_member(callee, helper)?;
    if identifier_name(object) != Some("require") {
        return None;
    }
    Some(arguments)
}

pub(crate) fn is_define_esmodule(stmt: &Statement<'_>) -> bool {
    let Some(expr) = expression_statement(stmt) else {
        return false;
    };
    let Some(arguments) = require_helper_call(expr, "r") else {
        return false;
    };
    arguments.len() == 1
        && matches!(
            arguments.first().and_then(argument_expression),
            Some(Expression::Identifier(_))
        )
}

fn is_hmd(stmt: &Statement<'_>) -> bool {
    let Some(Expression::AssignmentExpression(assignment)) = expression_statement(stmt) else {
        return false;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return false;
    }
    let is_module_target = matches!(
        &assignment.left,
        AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == "module"
    );
    is_module_target && require_helper_call(&assignment.right, "hmd").is_some()
}

fn namespace_import_parts<'a>(stmt: &Statement<'a>) -> Option<(Ident<'a>, String)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(local) = &declarator.id else {
        return None;
    };
    let (callee, arguments) = call_parts(declarator.init.as_ref()?)?;
    if identifier_name(callee) != Some("require") || arguments.len() != 1 {
        return None;
    }
    let Expression::NumericLiteral(literal) = arguments.first().and_then(argument_expression)?
    else {
        return None;
    };
    Some((local.name, number_key(literal.value)))
}

fn namespace_import<'a>(
    span: Span,
    local: Ident<'a>,
    module_id: &str,
    builder: &AstBuilder<'a>,
) -> Statement<'a> {
    let specifier = ImportDeclarationSpecifier::new_import_namespace_specifier(
        span,
        BindingIdentifier::new(span, local, builder),
        builder,
    );
    let specifiers = AVec::from_iter_in([specifier], builder);
    let source = StringLiteral::new(span, Str::from_str_in(module_id, builder), None, builder);
    Statement::new_import_declaration(
        span,
        Some(specifiers),
        source,
        None,
        None,
        ImportOrExportKind::Value,
        builder,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportForm {
    Named,
    Default,
}

#[derive(Default)]
struct EsmPlan<'a> {
    renames: Vec<(String, String)>,
    drops: FxHashSet<usize>,
    inserts: FxHashMap<usize, Vec<Statement<'a>>>,
    folds: FxHashMap<usize, Vec<(String, Expression<'a>)>>,
    exports: FxHashMap<usize, ExportForm>,
}

struct DefineEntry<'a> {
    export_name: String,
    value: Expression<'a>,
}

pub(crate) fn zero_param_function_return<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::FunctionExpression(function) = expr else {
        return None;
    };
    if function.generator
        || function.r#async
        || !function.params.items.is_empty()
        || function.params.rest.is_some()
    {
        return None;
    }
    let body = function.body.as_deref()?;
    if body.statements.len() != 1 {
        return None;
    }
    let Statement::ReturnStatement(statement) = body.statements.first()? else {
        return None;
    };
    statement.argument.as_ref()
}

pub(crate) fn zero_param_arrow_body<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::ArrowFunctionExpression(arrow) = expr else {
        return None;
    };
    if arrow.r#async || !arrow.params.items.is_empty() || arrow.params.rest.is_some() {
        return None;
    }
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(_) => None,
        other => other.as_expression(),
    }
}

fn bulk_define_entries<'a>(
    alloc: &'a Allocator,
    object: &ObjectExpression<'a>,
) -> Option<Vec<DefineEntry<'a>>> {
    let mut entries = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.method {
            return None;
        }
        let PropertyKey::StaticIdentifier(key) = &property.key else {
            return None;
        };
        let value = zero_param_arrow_body(&property.value)?;
        entries.push(DefineEntry {
            export_name: key.name.as_str().to_string(),
            value: value.clone_in(alloc),
        });
    }
    Some(entries)
}

fn single_define_entry<'a>(
    alloc: &'a Allocator,
    name: &StringLiteral<'a>,
    value_expr: &Expression<'a>,
) -> Option<DefineEntry<'a>> {
    let export_name = name.value.as_str()?.to_string();
    let value = zero_param_function_return(value_expr)?;
    Some(DefineEntry {
        export_name,
        value: value.clone_in(alloc),
    })
}

fn define_entries<'a>(
    alloc: &'a Allocator,
    stmt: &Statement<'a>,
) -> Option<(String, Vec<DefineEntry<'a>>, bool)> {
    let arguments = require_helper_call(expression_statement(stmt)?, "d")?;
    let target = identifier_name(argument_expression(arguments.first()?)?)?.to_string();
    match arguments.len() {
        2 => {
            let Expression::ObjectExpression(object) = argument_expression(arguments.get(1)?)?
            else {
                return None;
            };
            let entries = bulk_define_entries(alloc, object)?;
            Some((target, entries, true))
        }
        3 => {
            let Expression::StringLiteral(name) = argument_expression(arguments.get(1)?)? else {
                return None;
            };
            let value_expr = argument_expression(arguments.get(2)?)?;
            let entry = single_define_entry(alloc, name, value_expr)?;
            Some((target, vec![entry], false))
        }
        _ => None,
    }
}

fn find_declaration(body: &[Statement<'_>], name: &str) -> Option<usize> {
    body.iter().position(|stmt| match stmt {
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.len() == 1
                && declaration.declarations.first().is_some_and(|declarator| {
                    matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                        if id.name.as_str() == name)
                })
        }
        Statement::ClassDeclaration(class) => {
            class.id.as_ref().is_some_and(|id| id.name.as_str() == name)
        }
        Statement::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .is_some_and(|id| id.name.as_str() == name),
        _ => false,
    })
}

fn find_empty_object_declaration(body: &[Statement<'_>], name: &str) -> Option<usize> {
    let index = find_declaration(body, name)?;
    let Statement::VariableDeclaration(declaration) = body.get(index)? else {
        return None;
    };
    let init = declaration.declarations.first()?.init.as_ref()?;
    let empty =
        matches!(init, Expression::ObjectExpression(object) if object.properties.is_empty());
    empty.then_some(index)
}

fn named_export_let<'a>(
    span: Span,
    name: &str,
    value: Expression<'a>,
    builder: &AstBuilder<'a>,
) -> Statement<'a> {
    let declarator = VariableDeclarator::new(
        span,
        BindingPattern::new_binding_identifier(span, Ident::from_str_in(name, builder), builder),
        None,
        Some(value),
        false,
        builder,
    );
    let declarations = AVec::from_iter_in([declarator], builder);
    let declaration = Declaration::new_variable_declaration(
        span,
        VariableDeclarationKind::Let,
        declarations,
        false,
        builder,
    );
    Statement::new_export_declaration(span, declaration, builder)
}

fn export_variable<'a>(
    plan: &mut EsmPlan<'a>,
    body: &[Statement<'a>],
    index: usize,
    entry: DefineEntry<'a>,
    builder: &AstBuilder<'a>,
) {
    let span = body.get(index).map_or(Span::default(), GetSpan::span);
    if let Some(name) = identifier_name(&entry.value) {
        let Some(declaration_index) = find_declaration(body, name) else {
            return;
        };
        if entry.export_name == "default" {
            plan.exports.insert(declaration_index, ExportForm::Default);
        } else {
            plan.renames
                .push((name.to_string(), entry.export_name.clone()));
            plan.exports.insert(declaration_index, ExportForm::Named);
        }
        return;
    }
    let statement = if entry.export_name == "default" {
        Statement::new_export_default_declaration(
            span,
            ExportDefaultDeclarationKind::from(entry.value),
            builder,
        )
    } else {
        named_export_let(span, &entry.export_name, entry.value, builder)
    };
    plan.inserts.entry(index).or_default().insert(0, statement);
}

fn plan_exports<'a>(alloc: &'a Allocator, program: &Program<'a>) -> EsmPlan<'a> {
    let builder = AstBuilder::new(alloc);
    let mut plan = EsmPlan::default();
    for index in 0..program.body.len() {
        let Some((target, entries, bulk)) = define_entries(alloc, &program.body[index]) else {
            continue;
        };
        plan.drops.insert(index);
        let fold_target = bulk
            .then(|| find_empty_object_declaration(&program.body, &target))
            .flatten();
        for entry in entries {
            match fold_target {
                Some(object_index) => plan
                    .folds
                    .entry(object_index)
                    .or_default()
                    .push((entry.export_name, entry.value)),
                None => export_variable(&mut plan, &program.body, index, entry, &builder),
            }
        }
    }
    plan
}

fn fold_into_object<'a>(
    stmt: &mut Statement<'a>,
    properties: Vec<(String, Expression<'a>)>,
    builder: &AstBuilder<'a>,
) -> usize {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return 0;
    };
    let Some(Expression::ObjectExpression(object)) = declaration
        .declarations
        .first_mut()
        .and_then(|declarator| declarator.init.as_mut())
    else {
        return 0;
    };
    let count = properties.len();
    for (name, value) in properties {
        let span = value.span();
        object
            .properties
            .push(ObjectPropertyKind::new_object_property(
                span,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(
                    span,
                    Ident::from_str_in(&name, builder),
                    builder,
                ),
                value,
                false,
                false,
                false,
                builder,
            ));
    }
    count
}

fn declaration_from_statement(stmt: Statement<'_>) -> Result<Declaration<'_>, Statement<'_>> {
    match stmt {
        Statement::VariableDeclaration(declaration) => {
            Ok(Declaration::VariableDeclaration(declaration))
        }
        Statement::ClassDeclaration(declaration) => Ok(Declaration::ClassDeclaration(declaration)),
        Statement::FunctionDeclaration(declaration) => {
            Ok(Declaration::FunctionDeclaration(declaration))
        }
        other => Err(other),
    }
}

fn default_export_from_variable<'a>(
    init: Expression<'a>,
    span: Span,
    builder: &AstBuilder<'a>,
) -> Statement<'a> {
    Statement::new_export_default_declaration(
        span,
        ExportDefaultDeclarationKind::from(init),
        builder,
    )
}

fn export_statement<'a>(
    stmt: Statement<'a>,
    form: ExportForm,
    builder: &AstBuilder<'a>,
) -> Statement<'a> {
    let span = stmt.span();
    if form == ExportForm::Named {
        return match declaration_from_statement(stmt) {
            Ok(declaration) => Statement::new_export_declaration(span, declaration, builder),
            Err(statement) => statement,
        };
    }
    match stmt {
        Statement::ClassDeclaration(class) => Statement::new_export_default_declaration(
            span,
            ExportDefaultDeclarationKind::ClassDeclaration(class),
            builder,
        ),
        Statement::FunctionDeclaration(function) => Statement::new_export_default_declaration(
            span,
            ExportDefaultDeclarationKind::FunctionDeclaration(function),
            builder,
        ),
        Statement::VariableDeclaration(mut declaration) => {
            match declaration
                .declarations
                .first_mut()
                .and_then(|declarator| declarator.init.take())
            {
                Some(init) => default_export_from_variable(init, span, builder),
                None => Statement::VariableDeclaration(declaration),
            }
        }
        other => other,
    }
}

fn apply_plan_statement<'a>(
    index: usize,
    stmt: Statement<'a>,
    plan: &mut EsmPlan<'a>,
    body: &mut AVec<'a, Statement<'a>>,
    is_esm: &mut bool,
    builder: &AstBuilder<'a>,
) -> usize {
    if plan.drops.contains(&index) {
        if let Some(inserted) = plan.inserts.remove(&index) {
            body.extend(inserted);
        }
        return 1;
    }
    if is_define_esmodule(&stmt) {
        *is_esm = true;
        return 1;
    }
    if *is_esm {
        if let Some((local, module_id)) = namespace_import_parts(&stmt) {
            let span = stmt.span();
            body.push(namespace_import(span, local, &module_id, builder));
            return 1;
        }
    }
    if is_hmd(&stmt) {
        return 1;
    }
    let mut stmt = stmt;
    let mut changes = 0usize;
    if let Some(properties) = plan.folds.remove(&index) {
        changes += fold_into_object(&mut stmt, properties, builder);
    }
    match plan.exports.remove(&index) {
        Some(form) => {
            body.push(export_statement(stmt, form, builder));
            changes += 1;
        }
        None => body.push(stmt),
    }
    changes
}

pub fn convert_esm<'a>(alloc: &'a Allocator, module: &mut Module<'a>) -> usize {
    let mut plan = plan_exports(alloc, module.program());
    let renames = std::mem::take(&mut plan.renames);
    let mut changes = 0usize;
    for (from, to) in &renames {
        if rename_binding(alloc, module.program_mut(), from, to) {
            changes += 1;
        }
    }
    let builder = AstBuilder::new(alloc);
    let program = module.program_mut();
    let mut is_esm = program.source_type.is_module();
    let old = std::mem::replace(&mut program.body, AVec::new_in(&builder));
    for (index, stmt) in old.into_iter().enumerate() {
        changes += apply_plan_statement(
            index,
            stmt,
            &mut plan,
            &mut program.body,
            &mut is_esm,
            &builder,
        );
    }
    if is_esm {
        program.source_type = program.source_type.with_module(true);
    }
    changes
}
