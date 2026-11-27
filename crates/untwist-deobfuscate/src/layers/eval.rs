use oxc_ast::ast::{
    CallExpression, Expression, ForStatementInit, ForStatementLeft, Program, Statement,
    VariableDeclaration, VariableDeclarationKind,
};
use oxc_semantic::Scoping;
use untwist_core::semantic::{declared_lexically_by_another_input, program_is_strict};

use super::decode::{argument_refusal, fold_text, outer_call, refuse, sloppy_script};
use super::model::{DecodeCx, Decoded, Layer, LayerForm, Refusal, Refused};
use super::observe::observes_globals;
use super::probe::Probe;

pub fn decode(
    program: &Program<'_>,
    scoping: &Scoping,
    probe: Probe<'_, '_>,
    cx: &DecodeCx,
) -> Decoded {
    let call = outer_call(&probe)?;
    Some(decode_argument(program, scoping, call, cx))
}

fn decode_argument(
    program: &Program<'_>,
    scoping: &Scoping,
    call: &CallExpression<'_>,
    cx: &DecodeCx,
) -> Result<Layer, Refused> {
    let form = LayerForm::Eval;
    let [argument] = call.arguments.as_slice() else {
        return refuse(form, Refusal::Shape);
    };
    let Some(argument) = argument.as_expression() else {
        return refuse(form, Refusal::Shape);
    };
    if call.optional || !sloppy_script(program) {
        return refuse(form, Refusal::Shape);
    }
    if let Some(reason) = argument_refusal(scoping, &callee_roots(&call.callee), &[argument]) {
        return refuse(form, reason);
    }
    match fold_text(argument, cx) {
        Ok((text, steps, heap)) => Ok(Layer {
            form,
            text,
            steps,
            heap,
        }),
        Err(reason) => refuse(form, reason),
    }
}

fn callee_roots<'a>(callee: &Expression<'a>) -> Vec<&'a str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => vec![id.name.as_str()],
        Expression::SequenceExpression(seq) => match seq.expressions.last() {
            Some(Expression::Identifier(id)) => vec![id.name.as_str()],
            _ => Vec::new(),
        },
        Expression::StaticMemberExpression(member) => match member.object.without_parentheses() {
            Expression::Identifier(id) => vec![id.name.as_str()],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

pub fn verdict(
    program: &Program<'_>,
    unsafe_transforms: bool,
    shared: bool,
) -> Result<bool, Refusal> {
    let found = TopLevel::of(&program.body);
    if found.returns || program.hashbang.is_some() {
        return Err(Refusal::Unparsable);
    }
    if !found.lexical && !found.var {
        return Ok(false);
    }
    if found
        .var_names
        .iter()
        .any(|name| declared_lexically_by_another_input(name))
    {
        return Err(Refusal::Shape);
    }
    if unsafe_transforms {
        if shared && (program_is_strict(program) || found.lexical && found.functions) {
            return Err(Refusal::Shape);
        }
        return Ok(shared && found.lexical);
    }
    if found.lexical || program_is_strict(program) || observes_globals(program) {
        return Err(Refusal::NeedsUnsafe);
    }
    Ok(false)
}

pub(super) fn var_names(program: &Program<'_>) -> Vec<String> {
    TopLevel::of(&program.body).var_names
}

pub(super) fn declared_names(program: &Program<'_>) -> Vec<String> {
    let mut names = var_names(program);
    for statement in &program.body {
        names.extend(lexical_names(statement));
    }
    names
}

fn lexical_names(statement: &Statement<'_>) -> Vec<String> {
    match statement {
        Statement::VariableDeclaration(decl) if !is_var(decl) => decl
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .map(|id| id.name.to_string())
            .collect(),
        Statement::ClassDeclaration(class) => {
            class.id.iter().map(|id| id.name.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

#[derive(Default)]
struct TopLevel {
    var: bool,
    var_names: Vec<String>,
    functions: bool,
    lexical: bool,
    returns: bool,
}

impl TopLevel {
    fn of(body: &[Statement<'_>]) -> Self {
        let mut found = TopLevel::default();
        for statement in body {
            found.lexical |= declares_lexically(statement);
        }
        let mut work: Vec<&Statement<'_>> = body.iter().collect();
        while let Some(statement) = work.pop() {
            found.note(statement, &mut work);
        }
        found
    }

    fn note<'s, 'a>(&mut self, statement: &'s Statement<'a>, work: &mut Vec<&'s Statement<'a>>) {
        match statement {
            Statement::VariableDeclaration(decl) => self.note_var(decl),
            Statement::FunctionDeclaration(function) => {
                self.var_names
                    .extend(function.id.iter().map(|id| id.name.to_string()));
                self.var = true;
                self.functions = true;
            }
            Statement::ReturnStatement(_) => self.returns = true,
            Statement::BlockStatement(block) => work.extend(block.body.iter()),
            Statement::IfStatement(node) => {
                work.push(&node.consequent);
                work.extend(node.alternate.as_ref());
            }
            Statement::ForStatement(node) => {
                if let Some(ForStatementInit::VariableDeclaration(decl)) = &node.init {
                    self.note_var(decl);
                }
                work.push(&node.body);
            }
            Statement::ForInStatement(node) => {
                self.note_left(&node.left);
                work.push(&node.body);
            }
            Statement::ForOfStatement(node) => {
                self.note_left(&node.left);
                work.push(&node.body);
            }
            Statement::WhileStatement(node) => work.push(&node.body),
            Statement::DoWhileStatement(node) => work.push(&node.body),
            Statement::LabeledStatement(node) => work.push(&node.body),
            Statement::WithStatement(node) => work.push(&node.body),
            Statement::TryStatement(node) => {
                work.extend(node.block.body.iter());
                if let Some(handler) = &node.handler {
                    work.extend(handler.body.body.iter());
                }
                if let Some(finalizer) = &node.finalizer {
                    work.extend(finalizer.body.iter());
                }
            }
            Statement::SwitchStatement(node) => {
                for case in &node.cases {
                    work.extend(case.consequent.iter());
                }
            }
            _ => {}
        }
    }

    fn note_var(&mut self, decl: &VariableDeclaration<'_>) {
        if !is_var(decl) {
            return;
        }
        self.var = true;
        let names = decl
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .map(|id| id.name.to_string());
        self.var_names.extend(names);
    }

    fn note_left(&mut self, left: &ForStatementLeft<'_>) {
        if let ForStatementLeft::VariableDeclaration(decl) = left {
            self.note_var(decl);
        }
    }
}

fn is_var(decl: &VariableDeclaration<'_>) -> bool {
    decl.kind == VariableDeclarationKind::Var
}

fn declares_lexically(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::VariableDeclaration(decl) => !is_var(decl),
        Statement::ClassDeclaration(_) => true,
        _ => false,
    }
}
