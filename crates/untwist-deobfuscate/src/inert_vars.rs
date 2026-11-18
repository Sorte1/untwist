use oxc_ast::ast::*;

const SHADOWED_GLOBALS: [&str; 1] = ["undefined"];

fn uninitialised_names<'k>(stmt: &'k Statement<'_>) -> Option<Vec<&'k str>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if !matches!(
        declaration.kind,
        VariableDeclarationKind::Var | VariableDeclarationKind::Let
    ) {
        return None;
    }
    declaration
        .declarations
        .iter()
        .map(|declarator| match (&declarator.id, &declarator.init) {
            (BindingPattern::BindingIdentifier(id), None) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

pub(crate) fn is_uninitialised(stmt: &Statement<'_>) -> bool {
    uninitialised_names(stmt).is_some()
}

pub(crate) fn without_inert<'k, 'a>(
    statements: &'k [Statement<'a>],
    shape_names: &[&str],
) -> Option<Vec<&'k Statement<'a>>> {
    let mut kept = Vec::with_capacity(statements.len());
    for statement in statements {
        match uninitialised_names(statement) {
            Some(names) => {
                let named = names
                    .iter()
                    .any(|name| shape_names.contains(name) || SHADOWED_GLOBALS.contains(name));
                if named {
                    return None;
                }
            }
            None => kept.push(statement),
        }
    }
    Some(kept)
}
