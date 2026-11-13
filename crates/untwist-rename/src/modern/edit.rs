use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::GetSpan;

use super::regions::Regions;

pub fn apply<'a>(
    program: &mut Program<'a>,
    regions: &Regions,
    alloc: &'a oxc_allocator::Allocator,
) -> usize {
    let mut editor = Editor {
        regions,
        alloc,
        applied: 0,
    };
    editor.visit_program(program);
    editor.applied
}

struct Editor<'r, 'a> {
    regions: &'r Regions,
    alloc: &'a oxc_allocator::Allocator,
    applied: usize,
}

enum Fate {
    Keep,
    Drop,
}

impl<'a> Editor<'_, 'a> {
    fn hit(&mut self, span: oxc_span::Span) -> bool {
        let hit = self.regions.contains(span);
        if hit {
            self.applied += 1;
        }
        hit
    }

    fn declaration(&mut self, declaration: &mut VariableDeclaration<'a>) -> Fate {
        let before = declaration.declarations.len();
        let regions = self.regions;
        declaration
            .declarations
            .retain(|declarator| !regions.contains(declarator.span));
        self.applied += before - declaration.declarations.len();
        match declaration.declarations.is_empty() {
            true => Fate::Drop,
            false => Fate::Keep,
        }
    }

    fn expression(&mut self, statement: &mut ExpressionStatement<'a>) -> Fate {
        if self.hit(statement.expression.without_parentheses().span()) {
            return Fate::Drop;
        }
        let Expression::SequenceExpression(sequence) =
            statement.expression.without_parentheses_mut()
        else {
            return Fate::Keep;
        };
        let before = sequence.expressions.len();
        let regions = self.regions;
        sequence
            .expressions
            .retain(|element| !regions.contains(element.span()));
        self.applied += before - sequence.expressions.len();
        match sequence.expressions.len() {
            0 => Fate::Drop,
            1 => {
                let only = sequence.expressions.pop();
                if let Some(only) = only {
                    statement.expression = only;
                }
                Fate::Keep
            }
            _ => Fate::Keep,
        }
    }

    fn fate(&mut self, statement: &mut Statement<'a>) -> Fate {
        match statement {
            Statement::FunctionDeclaration(function) if self.hit(function.span) => Fate::Drop,
            Statement::VariableDeclaration(declaration) => self.declaration(declaration),
            Statement::ExpressionStatement(expression) => self.expression(expression),
            _ => Fate::Keep,
        }
    }
}

impl<'a> VisitMut<'a> for Editor<'_, 'a> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if self.regions.is_empty() {
            return;
        }
        let alloc = &self.alloc;
        let statements = std::mem::replace(it, AVec::with_capacity_in(0, alloc));
        let mut kept = AVec::with_capacity_in(statements.len(), alloc);
        for mut statement in statements {
            if let Fate::Keep = self.fate(&mut statement) {
                kept.push(statement);
            }
        }
        *it = kept;
        walk_mut::walk_statements(self, it);
    }
}

pub fn reachable(program: &Program<'_>, regions: &Regions) -> usize {
    let mut counter = Counter { regions, found: 0 };
    oxc_ast_visit::Visit::visit_program(&mut counter, program);
    counter.found
}

struct Counter<'r> {
    regions: &'r Regions,
    found: usize,
}

impl Counter<'_> {
    fn count(&mut self, span: oxc_span::Span) -> bool {
        let hit = self.regions.contains(span);
        self.found += usize::from(hit);
        hit
    }

    fn statement(&mut self, statement: &Statement<'_>) -> bool {
        match statement {
            Statement::FunctionDeclaration(function) => self.count(function.span),
            Statement::VariableDeclaration(declaration) => {
                let mut all = !declaration.declarations.is_empty();
                for declarator in &declaration.declarations {
                    all &= self.count(declarator.span);
                }
                all
            }
            Statement::ExpressionStatement(statement) => self.expression(statement),
            _ => false,
        }
    }

    fn expression(&mut self, statement: &ExpressionStatement<'_>) -> bool {
        let expression = statement.expression.without_parentheses();
        if self.count(expression.span()) {
            return true;
        }
        let Expression::SequenceExpression(sequence) = expression else {
            return false;
        };
        let mut all = true;
        for element in &sequence.expressions {
            all &= self.count(element.span());
        }
        all
    }
}

impl<'a> oxc_ast_visit::Visit<'a> for Counter<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for statement in it {
            if !self.statement(statement) {
                oxc_ast_visit::walk::walk_statement(self, statement);
            }
        }
    }
}
