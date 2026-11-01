use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use crate::closure::shell::{guarded, INITIALIZER};
use crate::matcher::unreassigned_var;

pub const ENTRY_ID: &str = "closure";
const MARKER: &str = "_M:";
const MAX_ID: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Prelude,
    Setup,
    Module(String),
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && !id.starts_with('-')
        && id != ENTRY_ID
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '-'))
}

pub fn marker(source: &Program<'_>, attempt: &TryStatement<'_>) -> Option<Option<String>> {
    let block = &attempt.block;
    let end = block
        .body
        .first()
        .map_or(block.span.end, |first| first.span().start);
    let from = source
        .comments
        .partition_point(|comment| comment.span.start < block.span.start);
    let mut found = None;
    for comment in source.comments[from..]
        .iter()
        .take_while(|comment| comment.span.start < end)
    {
        let Some(text) = comment
            .is_block()
            .then(|| {
                source
                    .source_text
                    .get(comment.content_span().start as usize..comment.content_span().end as usize)
            })
            .flatten()
        else {
            continue;
        };
        let Some(id) = text.trim().strip_prefix(MARKER) else {
            continue;
        };
        if found.is_some() {
            return None;
        }
        found = Some(id.trim().to_string());
    }
    match found {
        Some(id) if !valid_id(&id) => None,
        other => Some(other),
    }
}

fn var_this(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    unreassigned_var(declaration.kind)
        && matches!(
            &declaration.declarations[..],
            [declarator] if matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                && matches!(declarator.init, Some(Expression::ThisExpression(_)))
        )
}

#[derive(Default)]
struct Initializers {
    mentions: usize,
    calls: usize,
    graph: Option<String>,
}

impl<'a> VisitJs<'a> for Initializers {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.mentions += usize::from(it.property.name == INITIALIZER);
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(callee) = it.callee.without_parentheses() {
            if callee.property.name == INITIALIZER {
                self.calls += 1;
                if let Some(Argument::StringLiteral(graph)) = it.arguments.first() {
                    self.graph = Some(graph.value.to_string());
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn initialized_once(setup: &[&Statement<'_>]) -> Option<Option<String>> {
    let mut found = Initializers::default();
    for statement in setup {
        found.visit_statement(statement);
    }
    (found.mentions >= 1 && found.calls <= 1).then_some(found.graph)
}

fn in_graph(graph: &str, id: &str) -> bool {
    graph
        .split('/')
        .any(|entry| entry.split(':').next() == Some(id))
}

fn param_call<'k, 'a>(
    statement: &'k Statement<'a>,
    param: &str,
) -> Option<(&'k str, &'k [Argument<'a>])> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = &statement.expression else {
        return None;
    };
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return None;
    };
    let on_param = matches!(&callee.object, Expression::Identifier(id) if id.name == param);
    (on_param && !call.optional).then(|| (callee.property.name.as_str(), &call.arguments[..]))
}

struct Loader<'k> {
    begin: &'k str,
    id: &'k str,
    end: &'k str,
}

fn loader<'k>(attempt: &'k TryStatement<'_>, param: &str) -> Option<Loader<'k>> {
    let body = &attempt.block.body;
    if body.len() < 2 {
        return None;
    }
    let (begin, arguments) = param_call(body.first()?, param)?;
    let [Argument::StringLiteral(id)] = arguments else {
        return None;
    };
    let (end, rest) = param_call(body.last()?, param)?;
    (rest.is_empty() && begin != end && !id.lone_surrogates).then(|| Loader {
        begin,
        id: id.value.as_str(),
        end,
    })
}

#[derive(Default)]
struct Loaders<'k> {
    names: Option<(&'k str, &'k str)>,
    ids: Vec<&'k str>,
}

impl<'k> Loaders<'k> {
    fn accept(&mut self, found: &Loader<'k>) -> bool {
        let names = (found.begin, found.end);
        self.ids.push(found.id);
        *self.names.get_or_insert(names) == names
    }
}

fn module_id<'k>(
    source: &Program<'_>,
    attempt: &'k TryStatement<'_>,
    param: &str,
    loaders: &mut Loaders<'k>,
) -> Option<Option<String>> {
    let marked = marker(source, attempt)?;
    let Some(found) = loader(attempt, param) else {
        return Some(marked);
    };
    let agrees = marked.as_deref().is_none_or(|id| id == found.id);
    (agrees && valid_id(found.id) && loaders.accept(&found)).then(|| Some(found.id.to_string()))
}

pub fn roles(source: &Program<'_>, param: &str, statements: &[Statement<'_>]) -> Option<Vec<Role>> {
    let mut roles = Vec::with_capacity(statements.len());
    let mut seen = FxHashSet::default();
    let mut setup = Vec::new();
    let mut started = false;
    let mut loaders = Loaders::default();
    for statement in statements {
        let Statement::TryStatement(attempt) = statement else {
            if started || !var_this(statement) {
                return None;
            }
            roles.push(Role::Prelude);
            continue;
        };
        started = true;
        if !guarded(attempt, param) {
            return None;
        }
        match module_id(source, attempt, param, &mut loaders)? {
            Some(id) => {
                if !seen.insert(id.clone()) {
                    return None;
                }
                roles.push(Role::Module(id));
            }
            None if seen.is_empty() => {
                setup.push(statement);
                roles.push(Role::Setup);
            }
            None => return None,
        }
    }
    let graph = initialized_once(&setup)?;
    let listed = graph.is_none_or(|graph| loaders.ids.iter().all(|id| in_graph(&graph, id)));
    (!seen.is_empty() && listed).then_some(roles)
}

const HOST_NAMES: [&str; 5] = ["require", "module", "exports", "__filename", "__dirname"];

pub struct Stray<'s> {
    scoping: &'s Scoping,
    functions: usize,
    pub found: bool,
}

impl<'s> Stray<'s> {
    pub fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            functions: 0,
            found: false,
        }
    }
}

impl<'a> VisitJs<'a> for Stray<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let free = it
            .reference_id
            .get()
            .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none());
        self.found |= free && HOST_NAMES.contains(&it.name.as_str());
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.functions -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.functions += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.functions -= 1;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.found |= self.functions == 0;
        walk_js::walk_return_statement(self, it);
    }
}
