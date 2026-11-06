use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, BindingPattern, BlockStatement, CatchClause, Class,
    Expression, ForInStatement, ForOfStatement, ForStatement, ForStatementInit, ForStatementLeft,
    FormalParameters, Function, FunctionBody, Statement, SwitchStatement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;

use crate::stack;

pub(crate) struct Shadows<'a> {
    targets: Vec<&'a str>,
    frames: Vec<Vec<&'a str>>,
    top_functions: Vec<Span>,
    nested_shadow_all: bool,
    pub(crate) unresolved: bool,
}

impl<'a> Shadows<'a> {
    pub(crate) fn new(targets: Vec<&'a str>, nested_shadow_all: bool) -> Self {
        Self {
            targets,
            frames: Vec::new(),
            top_functions: Vec::new(),
            nested_shadow_all,
            unresolved: false,
        }
    }

    pub(crate) fn reaches(&self, name: &str) -> bool {
        self.targets.contains(&name) && !self.frames.iter().any(|frame| frame.contains(&name))
    }

    pub(crate) fn note_top_functions(&mut self, statements: &[Statement<'a>]) {
        for statement in statements {
            if let Statement::FunctionDeclaration(function) = statement {
                self.top_functions.push(function.span);
            }
        }
    }

    fn keep(&self, names: &mut Vec<&'a str>, name: &'a str) {
        if self.targets.contains(&name) {
            names.push(name);
        }
    }

    fn pattern(&self, pattern: &BindingPattern<'a>, names: &mut Vec<&'a str>) {
        for id in pattern.get_binding_identifiers() {
            self.keep(names, id.name.as_str());
        }
    }

    fn declaration(&self, declaration: &VariableDeclaration<'a>, names: &mut Vec<&'a str>) {
        for declarator in &declaration.declarations {
            self.pattern(&declarator.id, names);
        }
    }

    fn lexical(&self, declaration: &VariableDeclaration<'a>) -> Vec<&'a str> {
        let mut names = Vec::new();
        if declaration.kind.is_lexical() {
            self.declaration(declaration, &mut names);
        }
        names
    }

    fn block(&self, statements: &[Statement<'a>], names: &mut Vec<&'a str>) {
        for statement in statements {
            if let Statement::VariableDeclaration(declaration) = statement {
                if declaration.kind.is_lexical() {
                    self.declaration(declaration, names);
                }
            }
        }
    }

    fn params(&self, params: &FormalParameters<'a>, names: &mut Vec<&'a str>) {
        for param in &params.items {
            self.pattern(&param.pattern, names);
        }
        if let Some(rest) = &params.rest {
            self.pattern(&rest.rest.argument, names);
        }
    }

    fn body(&mut self, statements: &[Statement<'a>]) -> Vec<&'a str> {
        self.note_top_functions(statements);
        let mut scan = VarScan {
            shadows: &*self,
            names: Vec::new(),
        };
        for statement in statements {
            if let Statement::FunctionDeclaration(function) = statement {
                if let Some(id) = &function.id {
                    scan.shadows.keep(&mut scan.names, id.name.as_str());
                }
            }
            scan.visit_statement(statement);
        }
        let mut names = scan.names;
        self.block(statements, &mut names);
        names
    }

    fn nested(&self, names: &mut Vec<&'a str>) {
        if self.nested_shadow_all {
            names.extend(self.targets.iter().copied());
        }
    }
}

struct VarScan<'s, 'a> {
    shadows: &'s Shadows<'a>,
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for VarScan<'_, 'a> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_expression(&mut self, _it: &Expression<'a>) {}

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            self.shadows.declaration(it, &mut self.names);
        }
    }
}

pub(crate) trait Scoped<'a>: VisitJs<'a> {
    fn shadows(&mut self) -> &mut Shadows<'a>;
}

fn scoped<'a, V: Scoped<'a>>(visitor: &mut V, names: Vec<&'a str>, walk: impl FnOnce(&mut V)) {
    visitor.shadows().frames.push(names);
    walk(visitor);
    visitor.shadows().frames.pop();
}

fn enter_function<'a, V: Scoped<'a>>(
    visitor: &mut V,
    mut outer: Vec<&'a str>,
    params: &FormalParameters<'a>,
    body: Option<&FunctionBody<'a>>,
) {
    let shadows = visitor.shadows();
    shadows.params(params, &mut outer);
    shadows.nested(&mut outer);
    scoped(visitor, outer, |visitor| {
        visitor.visit_formal_parameters(params);
        let Some(body) = body else {
            return;
        };
        let shadows = visitor.shadows();
        let mut inner = shadows.body(&body.statements);
        shadows.nested(&mut inner);
        scoped(visitor, inner, |visitor| visitor.visit_function_body(body));
    });
}

pub(crate) fn function<'a, V: Scoped<'a>>(visitor: &mut V, it: &Function<'a>) {
    let mut own = Vec::new();
    if let Some(id) = &it.id {
        let name = id.name.as_str();
        if it.is_declaration() {
            let shadows = visitor.shadows();
            let placed = shadows.top_functions.contains(&it.span);
            shadows.unresolved |= !placed && shadows.targets.contains(&name);
            visitor.visit_binding_identifier(id);
        } else {
            visitor.shadows().keep(&mut own, name);
        }
    }
    enter_function(visitor, own, &it.params, it.body.as_deref());
}

pub(crate) fn arrow<'a, V: Scoped<'a>>(visitor: &mut V, it: &ArrowFunctionExpression<'a>) {
    match &it.body {
        ArrowFunctionBody::FunctionBody(body) => {
            enter_function(visitor, Vec::new(), &it.params, Some(body));
        }
        _ => {
            let shadows = visitor.shadows();
            let mut outer = Vec::new();
            shadows.params(&it.params, &mut outer);
            shadows.nested(&mut outer);
            scoped(visitor, outer, |visitor| {
                walk_js::walk_arrow_function_expression(visitor, it);
            });
        }
    }
}

pub(crate) fn block<'a, V: Scoped<'a>>(visitor: &mut V, it: &BlockStatement<'a>) {
    let mut names = Vec::new();
    visitor.shadows().block(&it.body, &mut names);
    scoped(visitor, names, |visitor| {
        walk_js::walk_block_statement(visitor, it);
    });
}

pub(crate) fn for_statement<'a, V: Scoped<'a>>(visitor: &mut V, it: &ForStatement<'a>) {
    let names = match &it.init {
        Some(ForStatementInit::VariableDeclaration(declaration)) => {
            visitor.shadows().lexical(declaration)
        }
        _ => Vec::new(),
    };
    scoped(visitor, names, |visitor| {
        walk_js::walk_for_statement(visitor, it);
    });
}

fn head<'a, V: Scoped<'a>>(visitor: &mut V, left: &ForStatementLeft<'a>) -> Vec<&'a str> {
    match left {
        ForStatementLeft::VariableDeclaration(declaration) => {
            visitor.shadows().lexical(declaration)
        }
        _ => Vec::new(),
    }
}

pub(crate) fn for_in<'a, V: Scoped<'a>>(visitor: &mut V, it: &ForInStatement<'a>) {
    let names = head(visitor, &it.left);
    scoped(visitor, names, |visitor| {
        walk_js::walk_for_in_statement(visitor, it);
    });
}

pub(crate) fn for_of<'a, V: Scoped<'a>>(visitor: &mut V, it: &ForOfStatement<'a>) {
    let names = head(visitor, &it.left);
    scoped(visitor, names, |visitor| {
        walk_js::walk_for_of_statement(visitor, it);
    });
}

pub(crate) fn switch<'a, V: Scoped<'a>>(visitor: &mut V, it: &SwitchStatement<'a>) {
    visitor.visit_expression(&it.discriminant);
    let mut names = Vec::new();
    for case in &it.cases {
        visitor.shadows().block(&case.consequent, &mut names);
    }
    scoped(visitor, names, |visitor| {
        visitor.visit_switch_cases(&it.cases)
    });
}

pub(crate) fn catch<'a, V: Scoped<'a>>(visitor: &mut V, it: &CatchClause<'a>) {
    let mut names = Vec::new();
    if let Some(param) = &it.param {
        visitor.shadows().pattern(&param.pattern, &mut names);
    }
    scoped(visitor, names, |visitor| {
        walk_js::walk_catch_clause(visitor, it);
    });
}
