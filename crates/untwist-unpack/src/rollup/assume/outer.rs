use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::literal::is_pure_value;
use untwist_core::semantic::build_scoping;

use crate::rollup::scan::{HOST_NAMES, MODULE_RESERVED, REQUIRE};
use crate::rollup::shell::Wrapper;

const EVAL: &str = "eval";

#[derive(Default)]
pub struct Outer {
    pub names: FxHashSet<String>,
    pub unclean: FxHashSet<String>,
    pub assigned: FxHashSet<String>,
    pub free: FxHashSet<String>,
}

fn plain_assignment<'k, 'a>(statement: &'k Statement<'a>) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &statement.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    (assign.operator == AssignmentOperator::Assign).then_some((id.name.as_str(), &assign.right))
}

pub fn assigned_names<'k>(statement: &'k Statement<'_>) -> Vec<&'k str> {
    match statement {
        Statement::VariableDeclaration(declaration)
            if declaration.kind == VariableDeclarationKind::Var =>
        {
            declaration
                .declarations
                .iter()
                .filter(|declarator| declarator.init.is_some())
                .filter_map(|declarator| declarator.id.get_binding_identifier())
                .map(|id| id.name.as_str())
                .collect()
        }
        other => plain_assignment(other)
            .map(|(name, _)| name)
            .into_iter()
            .collect(),
    }
}

struct Writes<'o> {
    outer: &'o mut Outer,
    functions: u32,
    refused: bool,
}

impl Writes<'_> {
    fn wrote(&mut self, name: &str) {
        self.outer.unclean.insert(name.to_string());
    }
}

impl<'a> Visit<'a> for Writes<'_> {
    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.wrote(id.name.as_str());
        }
        walk::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.wrote(it.binding.name.as_str());
        walk::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if self.functions == 0 {
            if it.kind != VariableDeclarationKind::Var {
                self.refused = true;
            }
            for declarator in &it.declarations {
                match declarator.id.get_binding_identifier() {
                    Some(id) => {
                        self.outer.names.insert(id.name.to_string());
                        self.wrote(id.name.as_str());
                    }
                    None => self.refused = true,
                }
            }
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions += 1;
        walk::walk_function(self, it, flags);
        self.functions -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.functions += 1;
        walk::walk_arrow_function_expression(self, it);
        self.functions -= 1;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if matches!(&it.callee, Expression::Identifier(id) if id.name == EVAL) {
            self.refused = true;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.refused = true;
        walk::walk_with_statement(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if self.functions == 0 && it.id.is_some() && it.r#type == ClassType::ClassDeclaration {
            self.refused = true;
        }
        walk::walk_class(self, it);
    }
}

fn lexical(kind: VariableDeclarationKind) -> bool {
    matches!(
        kind,
        VariableDeclarationKind::Let | VariableDeclarationKind::Const
    )
}

fn quiet_declaration(outer: &mut Outer, declaration: &VariableDeclaration<'_>) -> bool {
    let kind = declaration.kind;
    if kind != VariableDeclarationKind::Var && !lexical(kind) {
        return false;
    }
    declaration.declarations.iter().all(|declarator| {
        let Some(id) = declarator.id.get_binding_identifier() else {
            return false;
        };
        let name = id.name.to_string();
        outer.names.insert(name.clone());
        if lexical(kind) {
            outer.unclean.insert(name.clone());
        }
        match &declarator.init {
            None => true,
            Some(init) => {
                outer.unclean.insert(name);
                is_pure_value(init)
            }
        }
    })
}

fn declared(outer: &mut Outer, statement: &Statement<'_>, strict: bool) -> Option<bool> {
    match statement {
        Statement::EmptyStatement(_) => Some(true),
        Statement::VariableDeclaration(declaration) => Some(quiet_declaration(outer, declaration)),
        Statement::FunctionDeclaration(function) if strict => {
            let id = function.id.as_ref()?;
            outer.names.insert(id.name.to_string());
            outer.unclean.insert(id.name.to_string());
            None
        }
        _ => None,
    }
}

fn later_declaration(writes: &mut Writes<'_>, declaration: &VariableDeclaration<'_>) -> bool {
    let kind = declaration.kind;
    if kind != VariableDeclarationKind::Var && !lexical(kind) {
        return false;
    }
    for declarator in &declaration.declarations {
        let Some(id) = declarator.id.get_binding_identifier() else {
            return false;
        };
        writes.outer.names.insert(id.name.to_string());
        if lexical(kind) {
            writes.outer.unclean.insert(id.name.to_string());
            if let Some(init) = &declarator.init {
                writes.visit_expression(init);
            }
            continue;
        }
        if let Some(init) = &declarator.init {
            writes.outer.assigned.insert(id.name.to_string());
            writes.visit_expression(init);
        }
    }
    true
}

fn later_statement(writes: &mut Writes<'_>, statement: &Statement<'_>) -> bool {
    if let Statement::VariableDeclaration(declaration) = statement {
        return later_declaration(writes, declaration);
    }
    match plain_assignment(statement) {
        Some((name, right)) => {
            writes.outer.assigned.insert(name.to_string());
            writes.visit_expression(right);
        }
        None => writes.visit_statement(statement),
    }
    true
}

const MODULE_ONLY: [&str; 2] = [MODULE_RESERVED, "import"];
const RECEIVER: &str = "this";

fn module_safe_text(text: &str, outer: &Wrapper<'_, '_>, inner: usize) -> bool {
    outer
        .body
        .statements
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != inner)
        .all(|(_, statement)| {
            let span = statement.span();
            let Some(own) = text.get(span.start as usize..span.end as usize) else {
                return false;
            };
            let receiver = outer.arrow && own.contains(RECEIVER);
            !receiver && !MODULE_ONLY.iter().any(|word| own.contains(word))
        })
}

fn free_names<'a>(
    (alloc, text): (&'a Allocator, &'a str),
    outer: &Wrapper<'_, 'a>,
    inner: usize,
    strict: bool,
) -> FxHashSet<String> {
    let builder = AstBuilder::new(alloc);
    let statements = outer
        .body
        .statements
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != inner)
        .map(|(_, statement)| statement.clone_in(alloc));
    let goal = match strict {
        true => SourceType::mjs(),
        false => SourceType::cjs(),
    };
    let program = Program::new(
        outer.body.span,
        goal,
        text,
        AVec::new_in(&builder),
        None,
        outer.body.directives.clone_in(alloc),
        AVec::from_iter_in(statements, &builder),
        &builder,
    );
    let semantics = build_scoping(&program);
    semantics
        .scoping
        .root_unresolved_references()
        .keys()
        .map(|name| name.to_string())
        .collect()
}

fn outer_reads<'a>(
    alloc: (&'a Allocator, &'a str),
    outer: &Wrapper<'_, 'a>,
    (inner, strict): (usize, bool),
    found: &mut Outer,
) -> bool {
    if !module_safe_text(alloc.1, outer, inner) {
        return false;
    }
    found.free = free_names(alloc, outer, inner, strict);
    !found
        .free
        .iter()
        .any(|name| name == REQUIRE || HOST_NAMES.contains(&name.as_str()))
}

pub fn analyse<'a>(
    alloc: (&'a Allocator, &'a str),
    outer: &Wrapper<'_, 'a>,
    inner: usize,
    strict: bool,
) -> Option<Outer> {
    let mut found = Outer::default();
    if !outer_reads(alloc, outer, (inner, strict), &mut found) {
        return None;
    }
    let mut later = Vec::new();
    for (index, statement) in outer.body.statements.iter().enumerate() {
        if index == inner {
            continue;
        }
        if let Statement::FunctionDeclaration(function) = statement {
            let id = function.id.as_ref()?;
            found.names.insert(id.name.to_string());
            found.unclean.insert(id.name.to_string());
        }
        if strict && index > inner {
            later.push(statement);
            continue;
        }
        match declared(&mut found, statement, strict) {
            Some(true) => {}
            Some(false) => return None,
            None if strict && matches!(statement, Statement::FunctionDeclaration(_)) => {
                later.push(statement)
            }
            None => return None,
        }
    }
    let mut writes = Writes {
        outer: &mut found,
        functions: 0,
        refused: false,
    };
    for statement in later {
        if !later_statement(&mut writes, statement) {
            return None;
        }
    }
    let refused = writes.refused;
    (!refused).then_some(found)
}
