use oxc_allocator::{Allocator, Box as ABox, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitMut;
use oxc_semantic::Scoping;
use oxc_span::{SourceType, SPAN};
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::shorthand::walk_target_keeping_key;
use untwist_core::print::{function_text, print, PrintStyle};
use untwist_core::semantic::build_scoping;

struct Canon<'a, 's> {
    alloc: &'a Allocator,
    scoping: &'s Scoping,
    names: FxHashMap<SymbolId, Ident<'a>>,
    renames: &'s [(&'s str, &'s str)],
    free: Vec<String>,
}

impl<'a> Canon<'a, '_> {
    fn name(&mut self, symbol: SymbolId) -> Ident<'a> {
        let alloc = self.alloc;
        let next = self.names.len();
        *self
            .names
            .entry(symbol)
            .or_insert_with(|| Ident::from_str_in(&format!("#{next}"), &alloc))
    }
}

impl<'a> VisitMut<'a> for Canon<'a, '_> {
    fn visit_assignment_target_property(&mut self, it: &mut AssignmentTargetProperty<'a>) {
        let alloc = self.alloc;
        walk_target_keeping_key(self, it, alloc);
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if let Some(symbol) = symbol {
            it.name = self.name(symbol);
            return;
        }
        let name = it.name.as_str();
        if !self.free.iter().any(|seen| seen == name) {
            self.free.push(name.to_string());
        }
        if let Some((_, to)) = self.renames.iter().find(|(from, _)| *from == name) {
            it.name = Ident::from_str_in(to, &self.alloc);
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            it.name = self.name(symbol);
        }
    }
}

fn alone<'a>(alloc: &'a Allocator, statement: Statement<'a>, source_text: &'a str) -> Program<'a> {
    alone_renamed(alloc, statement, source_text, &[]).0
}

fn alone_renamed<'a>(
    alloc: &'a Allocator,
    statement: Statement<'a>,
    source_text: &'a str,
    renames: &[(&str, &str)],
) -> (Program<'a>, Vec<String>) {
    let builder = AstBuilder::new(alloc);
    let mut program = Program::new(
        SPAN,
        SourceType::cjs(),
        source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        AVec::from_iter_in([statement], &builder),
        &builder,
    );
    let scoping = build_scoping(&program).scoping;
    let mut canon = Canon {
        alloc,
        scoping: &scoping,
        names: FxHashMap::default(),
        renames,
        free: Vec::new(),
    };
    canon.visit_program(&mut program);
    let free = canon.free;
    (program, free)
}

pub fn function_shape(function: &Function<'_>, source_text: &str) -> Option<String> {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let copy = function.clone_in(&alloc);
    let expression = Expression::FunctionExpression(ABox::new_in(copy, &&alloc));
    let statement = Statement::new_expression_statement(SPAN, expression, &builder);
    let program = alone(&alloc, statement, source_text);
    let Some(Statement::ExpressionStatement(statement)) = program.body.first() else {
        return None;
    };
    let Expression::FunctionExpression(function) = &statement.expression else {
        return None;
    };
    Some(function_text(function, true))
}

pub fn statement_shape(statement: &Statement<'_>, source_text: &str) -> String {
    let alloc = Allocator::default();
    let program = alone(&alloc, statement.clone_in(&alloc), source_text);
    print(&program, PrintStyle::Compact)
}

pub fn expression_shape(
    expression: &Expression<'_>,
    source_text: &str,
    renames: &[(&str, &str)],
) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let statement =
        Statement::new_expression_statement(SPAN, expression.clone_in(&alloc), &builder);
    let (program, free) = alone_renamed(&alloc, statement, source_text, renames);
    (print(&program, PrintStyle::Compact), free)
}

pub fn statement_shape_renamed(
    statement: &Statement<'_>,
    source_text: &str,
    renames: &[(&str, &str)],
) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let (program, free) = alone_renamed(&alloc, statement.clone_in(&alloc), source_text, renames);
    (print(&program, PrintStyle::Compact), free)
}

pub fn program_shape<'a>(alloc: &'a Allocator, program: &mut Program<'a>) -> String {
    program.comments.clear();
    let scoping = build_scoping(program).scoping;
    let mut canon = Canon {
        alloc,
        scoping: &scoping,
        names: FxHashMap::default(),
        renames: &[],
        free: Vec::new(),
    };
    canon.visit_program(program);
    print(program, PrintStyle::Compact)
}
