use oxc_allocator::{Allocator, Box as ABox, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitMut;
use oxc_semantic::Scoping;
use oxc_span::{SourceType, SPAN};
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::print::{function_text, print, PrintStyle};
use untwist_core::semantic::build_scoping;

struct Canon<'a, 's> {
    alloc: &'a Allocator,
    scoping: &'s Scoping,
    names: FxHashMap<SymbolId, Ident<'a>>,
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
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if let Some(symbol) = symbol {
            it.name = self.name(symbol);
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            it.name = self.name(symbol);
        }
    }
}

fn alone<'a>(alloc: &'a Allocator, statement: Statement<'a>, source_text: &'a str) -> Program<'a> {
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
    };
    canon.visit_program(&mut program);
    program
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

pub fn program_shape<'a>(alloc: &'a Allocator, program: &mut Program<'a>) -> String {
    program.comments.clear();
    let scoping = build_scoping(program).scoping;
    let mut canon = Canon {
        alloc,
        scoping: &scoping,
        names: FxHashMap::default(),
    };
    canon.visit_program(program);
    print(program, PrintStyle::Compact)
}
