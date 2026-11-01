use oxc_allocator::{Allocator, Box as ABox, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::pass::Cx;

const ANONYMOUS: &str = "anonymous";
const ARGUMENTS: &str = "arguments";
const AWAIT: &str = "await";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellRefusal {
    RepeatedParameter,
    ParameterDeclared,
    ReadsReceiver,
    Returns,
    UsesAwait,
    ReferencesAnonymous,
}

impl ShellRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RepeatedParameter => "a parameter name is repeated",
            Self::ParameterDeclared => {
                "a parameter name is declared again by a function, class, let or const in its body"
            }
            Self::ReadsReceiver => {
                "its own level reads this, arguments, new.target, super or import.meta"
            }
            Self::Returns => "its own level returns",
            Self::UsesAwait => "its own level uses the identifier await",
            Self::ReferencesAnonymous => "its body references its own name, anonymous",
        }
    }
}

#[derive(Debug)]
pub enum ShellStrip {
    Absent,
    Kept { span: Span, reason: ShellRefusal },
    Stripped { hoisted: Vec<String> },
}

fn directive_text<'a>(statement: &Statement<'a>) -> Option<&'a str> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::StringLiteral(literal) = &statement.expression else {
        return None;
    };
    if literal.lone_surrogates {
        return None;
    }
    let raw = literal.raw.as_ref()?.as_str();
    let quote = raw.chars().next()?;
    if (quote != '"' && quote != '\'') || raw.len() < 2 || !raw.ends_with(quote) {
        return None;
    }
    let inner = &raw[1..raw.len() - 1];
    (!inner.contains('\\') && !inner.contains(quote)).then_some(inner)
}

fn take_directive<'a>(
    statement: &mut Statement<'a>,
    builder: &AstBuilder<'a>,
) -> Option<Directive<'a>> {
    let text = directive_text(statement)?;
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let span = statement.span;
    let placeholder = Expression::new_null_literal(span, builder);
    match std::mem::replace(&mut statement.expression, placeholder) {
        Expression::StringLiteral(literal) => {
            Some(Directive::new(span, literal.unbox(), text, builder))
        }
        restored => {
            statement.expression = restored;
            None
        }
    }
}

fn promote<'a>(
    directives: &mut AVec<'a, Directive<'a>>,
    statements: &mut AVec<'a, Statement<'a>>,
    builder: &AstBuilder<'a>,
) {
    let mut promoted = 0usize;
    for statement in statements.iter_mut() {
        let Some(directive) = take_directive(statement, builder) else {
            break;
        };
        directives.push(directive);
        promoted += 1;
    }
    if promoted == 0 {
        return;
    }
    let taken: Vec<Statement<'a>> = statements.drain(..).collect();
    statements.extend(taken.into_iter().skip(promoted));
}

struct Heads {
    removed: FxHashSet<Span>,
    found: FxHashSet<Span>,
}

impl Heads {
    fn record(&mut self, container: Span, statements: &[Statement<'_>]) {
        if statements
            .first()
            .is_some_and(|statement| self.removed.contains(&statement.span()))
        {
            self.found.insert(container);
        }
    }
}

impl<'a> VisitJs<'a> for Heads {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.record(it.span, &it.body);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.record(it.span, &it.statements);
        walk_js::walk_function_body(self, it);
    }
}

struct Restore<'a, 'b> {
    heads: &'b FxHashSet<Span>,
    builder: &'b AstBuilder<'a>,
}

impl<'a> VisitMut<'a> for Restore<'a, '_> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        if self.heads.contains(&it.span) {
            promote(&mut it.directives, &mut it.body, self.builder);
        }
        walk_mut::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        if self.heads.contains(&it.span) {
            promote(&mut it.directives, &mut it.statements, self.builder);
        }
        walk_mut::walk_function_body(self, it);
    }
}

fn emptied_heads(program: &Program<'_>, spans: &[Span]) -> FxHashSet<Span> {
    let mut heads = Heads {
        removed: spans.iter().copied().collect(),
        found: FxHashSet::default(),
    };
    heads.visit_program(program);
    heads.found
}

pub fn remove_protection_statements<'a>(
    program: &mut Program<'a>,
    spans: &[Span],
    cx: &mut Cx<'a, '_>,
) -> Result<(usize, usize), Span> {
    let mut removals = StmtEdits::new();
    for span in spans {
        if !removals.remove(*span) {
            return Err(*span);
        }
    }
    let wanted = removals.len();
    if cx.opts.extended {
        let applied = removals.apply(program, cx.allocator(), cx.scoping_mut());
        return Ok((wanted, applied));
    }
    let heads = emptied_heads(program, spans);
    let applied = removals.apply(program, cx.allocator(), cx.scoping_mut());
    let mut restore = Restore {
        heads: &heads,
        builder: cx.ast(),
    };
    restore.visit_program(program);
    Ok((wanted, applied))
}

fn shell_function<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Function<'a>> {
    match statement {
        Statement::FunctionDeclaration(function) => Some(function),
        Statement::ExpressionStatement(statement) => {
            match statement.expression.without_parentheses() {
                Expression::FunctionExpression(function) => Some(function),
                _ => None,
            }
        }
        _ => None,
    }
}

fn shell_parts<'k, 'a>(
    program: &'k Program<'a>,
) -> Option<(&'k Function<'a>, &'k FunctionBody<'a>)> {
    let [statement] = program.body.as_slice() else {
        return None;
    };
    let function = shell_function(statement)?;
    if function.id.as_ref()?.name != ANONYMOUS || function.params.rest.is_some() {
        return None;
    }
    let plain = function.params.items.iter().all(|param| {
        param.initializer.is_none() && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
    });
    if !plain {
        return None;
    }
    Some((function, function.body.as_deref()?))
}

fn parameter_names<'k>(function: &'k Function<'_>) -> Vec<&'k str> {
    function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

fn has_repeated_name(names: &[&str]) -> bool {
    let mut seen = FxHashSet::default();
    names.iter().any(|name| !seen.insert(*name))
}

#[derive(Default)]
struct OwnLevel<'p> {
    parameters: &'p [&'p str],
    returns: bool,
    reads_receiver: bool,
    uses_await: bool,
    declares_parameter: bool,
}

impl<'a> VisitJs<'a> for OwnLevel<'_> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if it.r#type != FunctionType::FunctionDeclaration {
            return;
        }
        if it
            .id
            .as_ref()
            .is_some_and(|id| self.parameters.contains(&id.name.as_str()))
        {
            self.declares_parameter = true;
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let returns_before = self.returns;
        walk_js::walk_arrow_function_expression(self, it);
        self.returns = returns_before;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.returns = true;
        walk_js::walk_return_statement(self, it);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.reads_receiver = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.reads_receiver = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.reads_receiver = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.reads_receiver = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == ARGUMENTS {
            self.reads_receiver = true;
        }
        if it.name == AWAIT {
            self.uses_await = true;
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if it.name == AWAIT {
            self.uses_await = true;
        }
    }
}

fn scan_own_level<'p>(body: &FunctionBody<'_>, parameters: &'p [&'p str]) -> OwnLevel<'p> {
    let mut scan = OwnLevel {
        parameters,
        ..OwnLevel::default()
    };
    for statement in &body.statements {
        scan.visit_statement(statement);
    }
    scan
}

#[derive(Default)]
struct NamesShell {
    found: bool,
}

impl<'a> VisitJs<'a> for NamesShell {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == ANONYMOUS {
            self.found = true;
        }
    }
}

fn references_anonymous(body: &FunctionBody<'_>) -> bool {
    let mut scan = NamesShell::default();
    for statement in &body.statements {
        scan.visit_statement(statement);
        if scan.found {
            return true;
        }
    }
    false
}

fn declares_lexically(statement: &Statement<'_>, name: &str) -> bool {
    match statement {
        Statement::ClassDeclaration(class) => class.id.as_ref().is_some_and(|id| id.name == name),
        Statement::VariableDeclaration(declaration) => {
            declaration.kind != VariableDeclarationKind::Var
                && declaration.declarations.iter().any(|declarator| {
                    declarator
                        .id
                        .get_binding_identifiers()
                        .iter()
                        .any(|id| id.name == name)
                })
        }
        _ => false,
    }
}

fn parameter_declared_again(
    body: &FunctionBody<'_>,
    parameters: &[&str],
    scan: &OwnLevel<'_>,
) -> bool {
    scan.declares_parameter
        || parameters.iter().any(|name| {
            body.statements
                .iter()
                .any(|statement| declares_lexically(statement, name))
        })
}

fn shell_refusal(function: &Function<'_>, body: &FunctionBody<'_>) -> Option<ShellRefusal> {
    let parameters = parameter_names(function);
    if has_repeated_name(&parameters) {
        return Some(ShellRefusal::RepeatedParameter);
    }
    let scan = scan_own_level(body, &parameters);
    if parameter_declared_again(body, &parameters, &scan) {
        return Some(ShellRefusal::ParameterDeclared);
    }
    if scan.reads_receiver {
        return Some(ShellRefusal::ReadsReceiver);
    }
    if scan.returns {
        return Some(ShellRefusal::Returns);
    }
    if scan.uses_await || parameters.contains(&AWAIT) {
        return Some(ShellRefusal::UsesAwait);
    }
    if references_anonymous(body) {
        return Some(ShellRefusal::ReferencesAnonymous);
    }
    None
}

fn restore_statement<'a>(
    program: &mut Program<'a>,
    span: Span,
    expression: Expression<'a>,
    alloc: &'a Allocator,
) {
    let builder = AstBuilder::new(alloc);
    let restored = ExpressionStatement::new(span, expression, &builder);
    program
        .body
        .push(Statement::ExpressionStatement(ABox::new_in(
            restored, &alloc,
        )));
}

fn hoisted_parameters<'a>(
    params: FormalParameters<'a>,
    alloc: &'a Allocator,
) -> (Statement<'a>, Vec<String>) {
    let builder = AstBuilder::new(alloc);
    let mut names = Vec::new();
    let mut declarators = AVec::new_in(&builder);
    for param in params.items {
        if let BindingPattern::BindingIdentifier(id) = &param.pattern {
            names.push(id.name.to_string());
        }
        declarators.push(VariableDeclarator::new(
            param.span,
            param.pattern,
            None,
            None,
            false,
            &builder,
        ));
    }
    let declaration = Statement::new_variable_declaration(
        params.span,
        VariableDeclarationKind::Var,
        declarators,
        false,
        &builder,
    );
    (declaration, names)
}

fn take_shell<'a>(program: &mut Program<'a>, alloc: &'a Allocator) -> Option<(Span, Function<'a>)> {
    let statement = match program.body.pop() {
        Some(Statement::FunctionDeclaration(function)) => {
            let function = function.unbox();
            return Some((function.span, function));
        }
        Some(Statement::ExpressionStatement(statement)) => statement.unbox(),
        Some(other) => {
            program.body.push(other);
            return None;
        }
        None => return None,
    };
    match statement.expression {
        Expression::FunctionExpression(function) => Some((statement.span, function.unbox())),
        other => {
            restore_statement(program, statement.span, other, alloc);
            None
        }
    }
}

pub fn strip_anonymous_shell<'a>(program: &mut Program<'a>, alloc: &'a Allocator) -> ShellStrip {
    let refused = match shell_parts(program) {
        None => return ShellStrip::Absent,
        Some((function, body)) => {
            shell_refusal(function, body).map(|reason| (function.span, reason))
        }
    };
    if let Some((span, reason)) = refused {
        return ShellStrip::Kept { span, reason };
    }
    let Some((span, mut function)) = take_shell(program, alloc) else {
        return ShellStrip::Absent;
    };
    let Some(body) = function.body.take() else {
        let shell = Expression::FunctionExpression(ABox::new_in(function, &alloc));
        restore_statement(program, span, shell, alloc);
        return ShellStrip::Absent;
    };
    let body = body.unbox();
    program.body = body.statements;
    program.directives = body.directives;
    let (declaration, hoisted) = hoisted_parameters(function.params.unbox(), alloc);
    if !hoisted.is_empty() {
        program.body.insert(0, declaration);
    }
    ShellStrip::Stripped { hoisted }
}
