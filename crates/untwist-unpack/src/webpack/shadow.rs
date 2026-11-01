use oxc_ast::ast::*;

const REQUIRE: &str = "require";

pub fn binds(pattern: &BindingPattern<'_>) -> bool {
    binds_name(pattern, REQUIRE)
}

pub fn binds_name(pattern: &BindingPattern<'_>, name: &str) -> bool {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.name == name,
        pattern => pattern
            .get_binding_identifiers()
            .iter()
            .any(|id| id.name == name),
    }
}

pub fn params_bind(params: &FormalParameters<'_>) -> bool {
    params_bind_name(params, REQUIRE)
}

pub fn params_bind_name(params: &FormalParameters<'_>, name: &str) -> bool {
    params
        .items
        .iter()
        .any(|item| binds_name(&item.pattern, name))
        || params
            .rest
            .as_ref()
            .is_some_and(|rest| binds_name(&rest.rest.argument, name))
}

fn declaration_binds(declaration: &VariableDeclaration<'_>, name: &str) -> bool {
    declaration
        .declarations
        .iter()
        .any(|d| binds_name(&d.id, name))
}

fn named(id: Option<&BindingIdentifier<'_>>, name: &str) -> bool {
    id.is_some_and(|id| id.name == name)
}

pub fn lexical(statements: &[Statement<'_>]) -> bool {
    lexical_name(statements, REQUIRE)
}

pub fn lexical_name(statements: &[Statement<'_>], name: &str) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::VariableDeclaration(declaration) => {
            declaration.kind != VariableDeclarationKind::Var && declaration_binds(declaration, name)
        }
        Statement::ClassDeclaration(class) => named(class.id.as_ref(), name),
        Statement::FunctionDeclaration(function) => named(function.id.as_ref(), name),
        _ => false,
    })
}

pub fn hoisted(statements: &[Statement<'_>], sloppy: bool) -> bool {
    hoisted_name(statements, sloppy, REQUIRE)
}

pub fn hoisted_name(statements: &[Statement<'_>], sloppy: bool, name: &str) -> bool {
    statements.iter().any(|s| hoists(s, (sloppy, name), false))
}

fn for_head_var(declaration: &VariableDeclaration<'_>, name: &str) -> bool {
    declaration.kind == VariableDeclarationKind::Var && declaration_binds(declaration, name)
}

fn hoists(statement: &Statement<'_>, (sloppy, name): (bool, &str), nested: bool) -> bool {
    let inner = |s: &Statement<'_>| hoists(s, (sloppy, name), true);
    let all = |list: &[Statement<'_>]| list.iter().any(|s| hoists(s, (sloppy, name), true));
    let for_head_var = |declaration: &VariableDeclaration<'_>| for_head_var(declaration, name);
    match statement {
        Statement::VariableDeclaration(declaration) => for_head_var(declaration),
        Statement::FunctionDeclaration(function) => {
            nested && sloppy && named(function.id.as_ref(), name)
        }
        Statement::BlockStatement(block) => all(&block.body),
        Statement::IfStatement(branch) => {
            inner(&branch.consequent) || branch.alternate.as_ref().is_some_and(inner)
        }
        Statement::ForStatement(each) => {
            let head = matches!(&each.init, Some(ForStatementInit::VariableDeclaration(d)) if for_head_var(d));
            head || inner(&each.body)
        }
        Statement::ForInStatement(each) => {
            let head =
                matches!(&each.left, ForStatementLeft::VariableDeclaration(d) if for_head_var(d));
            head || inner(&each.body)
        }
        Statement::ForOfStatement(each) => {
            let head =
                matches!(&each.left, ForStatementLeft::VariableDeclaration(d) if for_head_var(d));
            head || inner(&each.body)
        }
        Statement::WhileStatement(each) => inner(&each.body),
        Statement::DoWhileStatement(each) => inner(&each.body),
        Statement::LabeledStatement(labeled) => inner(&labeled.body),
        Statement::TryStatement(attempt) => {
            all(&attempt.block.body)
                || attempt.handler.as_ref().is_some_and(|h| all(&h.body.body))
                || attempt.finalizer.as_ref().is_some_and(|f| all(&f.body))
        }
        Statement::SwitchStatement(switch) => switch.cases.iter().any(|case| all(&case.consequent)),
        _ => false,
    }
}

pub fn for_head_lexical(left: Option<&VariableDeclaration<'_>>) -> bool {
    for_head_lexical_name(left, REQUIRE)
}

pub fn for_head_lexical_name(left: Option<&VariableDeclaration<'_>>, name: &str) -> bool {
    left.is_some_and(|d| d.kind != VariableDeclarationKind::Var && declaration_binds(d, name))
}

pub fn strict(directives: &[Directive<'_>]) -> bool {
    directives.iter().any(|d| d.directive == "use strict")
}
