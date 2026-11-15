use oxc_ast::ast::{Program, Statement, VariableDeclarationKind};

const WRAPPER_NAMES: [&str; 5] = ["exports", "require", "module", "__filename", "__dirname"];

fn wrapper_name(name: &str) -> bool {
    WRAPPER_NAMES.contains(&name)
}

pub fn lexical_wrapper_name(program: &Program<'_>) -> Option<String> {
    program.body.iter().find_map(|statement| match statement {
        Statement::VariableDeclaration(declaration)
            if declaration.kind != VariableDeclarationKind::Var =>
        {
            declaration.declarations.iter().find_map(|declarator| {
                declarator
                    .id
                    .get_binding_identifiers()
                    .into_iter()
                    .map(|id| id.name.as_str())
                    .find(|name| wrapper_name(name))
                    .map(str::to_string)
            })
        }
        Statement::ClassDeclaration(class) => class
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .filter(|name| wrapper_name(name))
            .map(str::to_string),
        _ => None,
    })
}
