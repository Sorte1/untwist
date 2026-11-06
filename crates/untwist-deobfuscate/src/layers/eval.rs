use oxc_ast::ast::{
    Argument, CallExpression, Expression, ForStatementInit, ForStatementLeft, MemberExpression,
    Program, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use untwist_core::semantic::program_is_strict;

use super::decode::{argument_refusal, fold_text, outer_call, refuse, sloppy_script, FreeNames};
use super::model::{DecodeCx, Decoded, Layer, LayerForm, Refusal, Refused};
use super::probe::Probe;

const GLOBAL_OBJECTS: [&str; 7] = [
    "window",
    "globalThis",
    "self",
    "global",
    "frames",
    "top",
    "parent",
];

pub(super) const DYNAMIC_CODE: [&str; 2] = ["eval", "Function"];

const DESCRIPTOR_READS: [(&str, &str); 4] = [
    ("Object", "getOwnPropertyDescriptor"),
    ("Object", "getOwnPropertyDescriptors"),
    ("Reflect", "getOwnPropertyDescriptor"),
    ("Reflect", "deleteProperty"),
];

const GLOBAL_RESHAPES: [(&str, &str); 9] = [
    ("Object", "defineProperty"),
    ("Object", "defineProperties"),
    ("Object", "freeze"),
    ("Object", "seal"),
    ("Object", "preventExtensions"),
    ("Object", "isFrozen"),
    ("Object", "isSealed"),
    ("Reflect", "defineProperty"),
    ("Reflect", "preventExtensions"),
];

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

pub fn verdict(program: &Program<'_>, unsafe_transforms: bool) -> Result<(), Refusal> {
    let found = TopLevel::of(&program.body);
    if found.returns || program.hashbang.is_some() {
        return Err(Refusal::Unparsable);
    }
    if (!found.lexical && !found.var) || unsafe_transforms {
        return Ok(());
    }
    if found.lexical || program_is_strict(program) || observes_globals(program) {
        return Err(Refusal::NeedsUnsafe);
    }
    Ok(())
}

#[derive(Default)]
struct TopLevel {
    var: bool,
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
            Statement::VariableDeclaration(decl) => self.var |= is_var(decl),
            Statement::FunctionDeclaration(_) => self.var = true,
            Statement::ReturnStatement(_) => self.returns = true,
            Statement::BlockStatement(block) => work.extend(block.body.iter()),
            Statement::IfStatement(node) => {
                work.push(&node.consequent);
                work.extend(node.alternate.as_ref());
            }
            Statement::ForStatement(node) => {
                if let Some(ForStatementInit::VariableDeclaration(decl)) = &node.init {
                    self.var |= is_var(decl);
                }
                work.push(&node.body);
            }
            Statement::ForInStatement(node) => {
                self.var |= var_left(&node.left);
                work.push(&node.body);
            }
            Statement::ForOfStatement(node) => {
                self.var |= var_left(&node.left);
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
}

fn is_var(decl: &VariableDeclaration<'_>) -> bool {
    decl.kind == VariableDeclarationKind::Var
}

fn var_left(left: &ForStatementLeft<'_>) -> bool {
    matches!(left, ForStatementLeft::VariableDeclaration(decl) if is_var(decl))
}

fn declares_lexically(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::VariableDeclaration(decl) => !is_var(decl),
        Statement::ClassDeclaration(_) => true,
        _ => false,
    }
}

fn observes_globals(program: &Program<'_>) -> bool {
    let mut scan = FreeNames {
        observe: true,
        ..FreeNames::default()
    };
    scan.visit_program(program);
    scan.found
}

pub(super) fn deletes_a_global(argument: &Expression<'_>) -> bool {
    match argument.without_parentheses() {
        Expression::Identifier(_) => true,
        other => other
            .as_member_expression()
            .is_some_and(|member| is_global_object(member.object())),
    }
}

fn is_global_object(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(id) => GLOBAL_OBJECTS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            GLOBAL_OBJECTS.contains(&member.property.name.as_str())
                && is_global_object(&member.object)
        }
        _ => false,
    }
}

pub(super) fn reflects_on_globals(call: &CallExpression<'_>) -> bool {
    let Some((object, method)) = call.callee.as_member_expression().and_then(member_names) else {
        return false;
    };
    let pair = (object, method);
    if DESCRIPTOR_READS.contains(&pair) {
        return true;
    }
    GLOBAL_RESHAPES.contains(&pair)
        && call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(is_global_object)
}

fn member_names<'a>(member: &MemberExpression<'a>) -> Option<(&'a str, &'a str)> {
    let object = match member.object().without_parentheses() {
        Expression::Identifier(id) => id.name.as_str(),
        Expression::StaticMemberExpression(inner) if is_global_object(&inner.object) => {
            inner.property.name.as_str()
        }
        _ => return None,
    };
    let method = match member {
        MemberExpression::StaticMemberExpression(node) => node.property.name.as_str(),
        MemberExpression::ComputedMemberExpression(node) => match &node.expression {
            Expression::StringLiteral(key) => key.value.as_str()?,
            _ => return None,
        },
        MemberExpression::PrivateFieldExpression(_) => return None,
    };
    Some((object, method))
}
