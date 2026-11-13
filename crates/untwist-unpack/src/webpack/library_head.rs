use std::sync::LazyLock;

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{SourceType, Span};
use oxc_str::Ident;
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_with;

use crate::canon::program_shape;

const RECORDED: &str = include_str!("../../tests/data/webpack_library_heads.txt");
const SECTION: &str = "//# ";
const PLACEHOLDER: &str = "__webpack_bundle__";
const NAME: &str = "#name";
const DEFINE: &str = "define";
const SYSTEM: &str = "System";
const REGISTER: &str = "register";
const KEPT: [&str; 3] = ["exports", "amd", REGISTER];

static HEADS: LazyLock<FxHashSet<String>> = LazyLock::new(recorded_heads);

fn recorded_heads() -> FxHashSet<String> {
    let mut heads = FxHashSet::default();
    let mut current = String::new();
    for line in RECORDED.lines().chain([SECTION]) {
        if !line.starts_with(SECTION) {
            current.push_str(line);
            current.push('\n');
            continue;
        }
        if !current.trim().is_empty() {
            let alloc = Allocator::default();
            let text = alloc.alloc_str(&current);
            let mut program = parse_with(&alloc, text, SourceType::cjs()).program;
            heads.insert(shape(&alloc, &mut program));
        }
        current.clear();
    }
    heads
}

struct Names<'a> {
    builder: AstBuilder<'a>,
}

fn names_module(call: &CallExpression<'_>) -> bool {
    match call.callee.without_parentheses() {
        Expression::Identifier(id) => id.name == DEFINE,
        Expression::StaticMemberExpression(member) => {
            member.property.name == REGISTER
                && matches!(&member.object, Expression::Identifier(id) if id.name == SYSTEM)
        }
        _ => false,
    }
}

impl<'a> Names<'a> {
    fn block(&self, statement: &mut Statement<'a>) {
        if matches!(
            statement,
            Statement::BlockStatement(_) | Statement::IfStatement(_)
        ) {
            return;
        }
        let taken = std::mem::replace(
            statement,
            Statement::new_empty_statement(Span::default(), &self.builder),
        );
        *statement = Statement::new_block_statement(
            Span::default(),
            oxc_allocator::Vec::from_iter_in([taken], &self.builder),
            &self.builder,
        );
    }

    fn split(&self, statement: Statement<'a>, into: &mut Vec<Statement<'a>>) {
        let Statement::ExpressionStatement(mut statement) = statement else {
            into.push(statement);
            return;
        };
        let Expression::SequenceExpression(sequence) = &mut statement.expression else {
            into.push(Statement::ExpressionStatement(statement));
            return;
        };
        let parts = std::mem::replace(
            &mut sequence.expressions,
            oxc_allocator::Vec::new_in(&self.builder),
        );
        for part in parts {
            into.push(Statement::new_expression_statement(
                Span::default(),
                part,
                &self.builder,
            ));
        }
    }
}

impl<'a> VisitMut<'a> for Names<'a> {
    fn visit_statements(&mut self, it: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        let taken = std::mem::replace(it, oxc_allocator::Vec::new_in(&self.builder));
        let mut kept = Vec::with_capacity(taken.len());
        for statement in taken {
            if !matches!(statement, Statement::EmptyStatement(_)) {
                self.split(statement, &mut kept);
            }
        }
        it.extend(kept);
        walk_mut::walk_statements(self, it);
    }

    fn visit_if_statement(&mut self, it: &mut IfStatement<'a>) {
        self.block(&mut it.consequent);
        if let Some(alternate) = &mut it.alternate {
            self.block(alternate);
        }
        walk_mut::walk_if_statement(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &mut StaticMemberExpression<'a>) {
        if !KEPT.contains(&it.property.name.as_str()) {
            it.property.name = Ident::from_str_in(NAME, &self.builder);
        }
        walk_mut::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &mut ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(name) = &mut it.expression {
            name.value = oxc_str::Str::from(NAME).into();
            name.raw = None;
        }
        walk_mut::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if names_module(it) {
            if let Some(Argument::StringLiteral(name)) = it.arguments.first_mut() {
                name.value = oxc_str::Str::from(NAME).into();
                name.raw = None;
            }
        }
        walk_mut::walk_call_expression(self, it);
    }
}

fn shape<'a>(alloc: &'a Allocator, program: &mut Program<'a>) -> String {
    let mut names = Names {
        builder: AstBuilder::new(alloc),
    };
    names.visit_program(program);
    let printed = program_shape(alloc, program);
    let members = printed
        .replace(&format!("[\"{NAME}\"]"), &format!(".{NAME}"))
        .replace(&format!("[`{NAME}`]"), &format!(".{NAME}"))
        .replace(&format!("['{NAME}']"), &format!(".{NAME}"));
    functions(&members)
}

fn functions(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((params, after)) = function_head(rest) {
            out.push_str("fn(");
            out.push_str(params);
            out.push_str("){");
            rest = after;
            continue;
        }
        let mut chars = rest.chars();
        out.extend(chars.next());
        rest = chars.as_str();
    }
    out
}

fn function_head(text: &str) -> Option<(&str, &str)> {
    if let Some(after) = text.strip_prefix("function(") {
        let (params, after) = after.split_once("){")?;
        return canonical_params(params).then_some((params, after));
    }
    if let Some(after) = text.strip_prefix('(') {
        let (params, after) = after.split_once(")=>{")?;
        return canonical_params(params).then_some((params, after));
    }
    let (param, after) = text.split_once("=>{")?;
    (param.starts_with('#') && canonical_params(param)).then_some((param, after))
}

fn canonical_params(params: &str) -> bool {
    params.is_empty()
        || params.split(',').all(|param| {
            param.strip_prefix('#').is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            })
        })
}

struct Bootstrap<'a> {
    builder: AstBuilder<'a>,
    span: Span,
    found: usize,
}

impl<'a> VisitMut<'a> for Bootstrap<'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if matches!(it, Expression::CallExpression(call) if call.span == self.span) {
            self.found += 1;
            *it = Expression::new_identifier(
                Span::default(),
                Ident::from_str_in(PLACEHOLDER, &self.builder),
                &self.builder,
            );
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

pub fn recorded_head(source: &Program<'_>, bootstrap: Span) -> bool {
    if bootstrap.is_empty() {
        return false;
    }
    let alloc = Allocator::default();
    let mut program = source.clone_in(&alloc);
    let mut replace = Bootstrap {
        builder: AstBuilder::new(&alloc),
        span: bootstrap,
        found: 0,
    };
    replace.visit_program(&mut program);
    replace.found == 1 && HEADS.contains(&shape(&alloc, &mut program))
}
