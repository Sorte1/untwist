use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentTarget, BindingIdentifier, BindingPattern, BlockStatement,
    CatchClause, Expression, ForInStatement, ForOfStatement, ForStatement, ForStatementInit,
    ForStatementLeft, Function, FunctionBody, IdentifierReference, Statement, SwitchStatement,
    VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use crate::bail::{Bail, EvalResult};
use crate::mutants::dead_zone_names_loose;
use crate::shadow::{self, Scoped, Shadows};
use crate::stack;

const TEMPORAL_DEAD_ZONE: &str =
    "a lexical binding that may be read before its declaration runs (the temporal dead zone)";

struct Lexical<'a> {
    name: &'a str,
    end: u32,
}

fn is_lexical(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.kind != VariableDeclarationKind::Var
}

fn lexical_names<'a>(statements: &'a [Statement<'a>]) -> Vec<Lexical<'a>> {
    let mut names = Vec::new();
    for statement in statements {
        match statement {
            Statement::VariableDeclaration(declaration) if is_lexical(declaration) => {
                for declarator in &declaration.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                        names.push(Lexical {
                            name: id.name.as_str(),
                            end: declarator.span.end,
                        });
                    }
                }
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    names.push(Lexical {
                        name: id.name.as_str(),
                        end: class.span.end,
                    });
                }
            }
            _ => {}
        }
    }
    names
}

struct EarlyReference<'n, 'a> {
    names: &'n [Lexical<'a>],
    shadows: Shadows<'a>,
    hoisted: bool,
    found: bool,
}

impl<'n, 'a> EarlyReference<'n, 'a> {
    fn new(names: &'n [Lexical<'a>], hoisted: bool) -> Self {
        let targets = names.iter().map(|lexical| lexical.name).collect();
        Self {
            names,
            shadows: Shadows::new(targets, dead_zone_names_loose()),
            hoisted,
            found: false,
        }
    }

    fn found(&self) -> bool {
        self.found || self.shadows.unresolved
    }
}

impl<'a> Scoped<'a> for EarlyReference<'_, 'a> {
    fn shadows(&mut self) -> &mut Shadows<'a> {
        &mut self.shadows
    }
}

impl<'a> VisitJs<'a> for EarlyReference<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        shadow::function(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        shadow::arrow(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        shadow::block(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        shadow::for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        shadow::for_in(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        shadow::for_of(self, it);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        shadow::switch(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        shadow::catch(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if !self.shadows.reaches(name) {
            return;
        }
        let early = |lexical: &Lexical<'a>| {
            lexical.name == name && (self.hoisted || it.span.start < lexical.end)
        };
        if self.names.iter().any(early) {
            self.found = true;
        }
    }
}

fn has_early_reference(statements: &[Statement<'_>]) -> bool {
    let names = lexical_names(statements);
    if names.is_empty() {
        return false;
    }
    let mut visitor = EarlyReference::new(&names, false);
    visitor.shadows.note_top_functions(statements);
    for statement in statements {
        visitor.hoisted = matches!(statement, Statement::FunctionDeclaration(_));
        visitor.visit_statement(statement);
    }
    visitor.found()
}

fn declares_lexically(statements: &[Statement<'_>]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::VariableDeclaration(declaration) => is_lexical(declaration),
        Statement::ClassDeclaration(_) => true,
        _ => false,
    })
}

fn early_for_head(declaration: &VariableDeclaration<'_>) -> bool {
    if !is_lexical(declaration) {
        return false;
    }
    let names: Vec<Lexical<'_>> = declaration
        .declarations
        .iter()
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => Some(Lexical {
                name: id.name.as_str(),
                end: declaration.span.end,
            }),
            _ => None,
        })
        .collect();
    let mut visitor = EarlyReference::new(&names, false);
    visitor.visit_variable_declaration(declaration);
    visitor.found()
}

#[derive(Default)]
struct BoundNames<'a> {
    names: Vec<Lexical<'a>>,
}

impl<'a> VisitJs<'a> for BoundNames<'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.push(Lexical {
            name: it.name.as_str(),
            end: u32::MAX,
        });
    }
}

fn head_reads_its_bindings<'a>(left: &ForStatementLeft<'a>, right: &Expression<'a>) -> bool {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return false;
    };
    if !is_lexical(declaration) {
        return false;
    }
    let mut bound = BoundNames::default();
    bound.visit_variable_declaration(declaration);
    let mut visitor = EarlyReference::new(&bound.names, true);
    visitor.visit_expression(right);
    visitor.found()
}

#[derive(Default)]
struct DeadZones {
    found: bool,
}

impl<'a> VisitJs<'a> for DeadZones {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.found |= has_early_reference(&it.statements);
        walk_js::walk_function_body(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.found |= has_early_reference(&it.body);
        walk_js::walk_block_statement(self, it);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.found |= it
            .cases
            .iter()
            .any(|case| declares_lexically(&case.consequent));
        walk_js::walk_switch_statement(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(ForStatementInit::VariableDeclaration(declaration)) = &it.init {
            self.found |= early_for_head(declaration);
        }
        walk_js::walk_for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.found |= head_reads_its_bindings(&it.left, &it.right);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.found |= head_reads_its_bindings(&it.left, &it.right);
        walk_js::walk_for_of_statement(self, it);
    }
}

pub fn refuse_dead_zones(
    functions: &[&Function<'_>],
    expressions: &[&Expression<'_>],
) -> EvalResult<()> {
    let mut scan = DeadZones::default();
    for function in functions {
        scan.visit_function(function, oxc_syntax::scope::ScopeFlags::Function);
    }
    for expression in expressions {
        scan.visit_expression(expression);
    }
    if scan.found {
        return Err(Bail::Unsupported(&TEMPORAL_DEAD_ZONE));
    }
    Ok(())
}

pub fn refuse_program_dead_zones(program: &oxc_ast::ast::Program<'_>) -> EvalResult<()> {
    let mut scan = DeadZones::default();
    scan.found |= has_early_reference(&program.body);
    scan.visit_program(program);
    if scan.found {
        return Err(Bail::Unsupported(&TEMPORAL_DEAD_ZONE));
    }
    Ok(())
}
