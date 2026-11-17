use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use rustc_hash::FxHashSet;

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

#[derive(Default)]
struct Named<'k> {
    names: FxHashSet<&'k str>,
}

impl<'k, 'a: 'k> Visit<'a> for Named<'k> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.names.insert(it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
    }
}

pub(crate) fn without_inert<'k, 'a>(
    statements: &'k [Statement<'a>],
    reserved: &[&str],
) -> Option<Vec<&'k Statement<'a>>> {
    let mut kept = Vec::with_capacity(statements.len());
    let mut inert = Vec::new();
    for statement in statements {
        match uninitialised_names(statement) {
            Some(names) => inert.extend(names),
            None => kept.push(statement),
        }
    }
    if inert.is_empty() {
        return Some(kept);
    }
    let mut named = Named::default();
    for statement in &kept {
        named.visit_statement(statement);
    }
    inert
        .iter()
        .all(|name| !reserved.contains(name) && !named.names.contains(name))
        .then_some(kept)
}
