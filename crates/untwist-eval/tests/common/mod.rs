#![allow(dead_code)]

use oxc_allocator::{Allocator, GetAllocator};
use oxc_ast::ast::{
    BindingPattern, Expression, IdentifierReference, ObjectExpression, ObjectPropertyKind, Program,
    PropertyKey, Statement,
};
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_ecmascript::constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx};
use oxc_ecmascript::side_effects::{MayHaveSideEffectsContext, PropertyReadSideEffects};
use oxc_ecmascript::{ConstantValue, GlobalContext};
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_eval::bail::{Bail, ErrorKind, EvalResult};
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Call, Decoded, DecoderProgram, FnRef};
use untwist_eval::host::Host;
use untwist_eval::interp::eval_script_in;
use untwist_eval::value::{Abrupt, AbruptKind, Eval, JsString, ObjKind, ObjRef, PropKey, Value};

pub fn with_program<R>(source: &str, body: impl FnOnce(&Program<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test source must parse cleanly: {:?}",
        parsed.diagnostics
    );
    body(&parsed.program)
}

pub fn render_number(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        return "number:-0".to_string();
    }
    format!("number:{}", untwist_eval::coerce::number_to_string(value))
}

pub fn render(value: &Value<'_>) -> String {
    match value {
        Value::Undefined => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("boolean:{b}"),
        Value::Number(n) => render_number(*n),
        Value::Str(s) => format!("string:{}", s.to_lossy_string()),
        Value::Object(obj) => match &obj.borrow().kind {
            ObjKind::Array(items) => format!("array:{}", items.len()),
            ObjKind::Arguments(items) => format!("arguments:{}", items.len()),
            ObjKind::MappedArguments(mapped) => format!("arguments:{}", mapped.items.len()),
            ObjKind::Function(_) | ObjKind::Bound(_) => "function".to_string(),
            ObjKind::Native(_, _) => "native".to_string(),
            ObjKind::Error(kind) => format!("error:{}", kind.name()),
            ObjKind::RegExp(_) => "regexp".to_string(),
            ObjKind::Plain(_) => "object".to_string(),
        },
    }
}

pub fn render_decoded(value: &Decoded) -> String {
    match value {
        Decoded::Undefined => "undefined".to_string(),
        Decoded::Null => "null".to_string(),
        Decoded::Bool(b) => format!("boolean:{b}"),
        Decoded::Num(n) => render_number(*n),
        Decoded::Str(s) => format!("string:{}", s.to_lossy_string()),
        Decoded::Object => "object".to_string(),
    }
}

fn render_result(result: Eval<'_, Value<'_>>) -> Result<String, String> {
    match result {
        Ok(value) => Ok(render(&value)),
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Bail(bail) => Err(bail.to_string()),
            AbruptKind::Throw(value) => Err(format!("throw {}", render(&value))),
        },
    }
}

pub fn eval(source: &str) -> Result<String, String> {
    eval_with(source, Budget::default())
}

pub fn eval_with(source: &str, budget: Budget) -> Result<String, String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test source must parse cleanly: {:?}",
        parsed.diagnostics
    );
    eval_script_in(&parsed.program, budget, |result, _| render_result(result))
}

pub fn eval_with_notes(source: &str) -> (Result<String, String>, Vec<String>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test source must parse cleanly: {:?}",
        parsed.diagnostics
    );
    eval_script_in(&parsed.program, Budget::default(), |result, notes| {
        (render_result(result), notes.to_vec())
    })
}

#[derive(Default)]
pub struct PureTestHost {
    budget: Budget,
}

impl<'a> Host<'a> for PureTestHost {
    fn spend(&mut self, steps: u64) -> EvalResult<()> {
        self.budget.spend(steps)
    }

    fn throw(&self, kind: ErrorKind, message: &str) -> Abrupt<'a> {
        let error = Value::object(ObjKind::Error(kind), None);
        if let Some(obj) = error.as_object() {
            obj.borrow_mut().props.insert(
                PropKey::Name(JsString::from_str("message")).into(),
                Value::Str(JsString::from_str(message)),
            );
        }
        Abrupt::throw(error)
    }

    fn note(&mut self, _message: &str) {}

    fn new_object(&mut self, kind: ObjKind<'a>, proto: Option<ObjRef<'a>>) -> Value<'a> {
        Value::object(kind, proto)
    }

    fn new_array(&mut self, items: Vec<Value<'a>>) -> Value<'a> {
        Value::array(items, None)
    }

    fn call_value(
        &mut self,
        _callee: &Value<'a>,
        _this: Value<'a>,
        _args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        Err(Abrupt::from(Bail::Unsupported(&"call from a pure host")))
    }

    fn construct_value(&mut self, _callee: &Value<'a>, _args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        Err(Abrupt::from(Bail::Unsupported(&"new from a pure host")))
    }

    fn get_property(&mut self, _target: &Value<'a>, _key: &PropKey) -> Eval<'a, Value<'a>> {
        Err(Abrupt::from(Bail::Unsupported(
            &"property read from a pure host",
        )))
    }

    fn note_protocol_member(&mut self, _target: &Value<'a>, _key: &PropKey) -> EvalResult<()> {
        Ok(())
    }

    fn function_source(&mut self, _target: &Value<'a>) -> Eval<'a, JsString> {
        Err(Abrupt::from(Bail::Unsupported(
            &"toString from a pure host",
        )))
    }

    fn enter_nesting(&mut self) -> EvalResult<()> {
        self.budget.enter_nesting()
    }

    fn leave_nesting(&mut self) {
        self.budget.leave_nesting();
    }

    fn get_prototype_of(&mut self, _value: &Value<'a>) -> Eval<'a, Value<'a>> {
        Err(Abrupt::from(Bail::Unsupported(
            &"getPrototypeOf from a pure host",
        )))
    }
}

pub struct OracleCtx<'a> {
    allocator: &'a Allocator,
    builder: AstBuilder<'a>,
}

impl<'a> GetAllocator<'a> for OracleCtx<'a> {
    fn allocator(&self) -> &'a Allocator {
        self.allocator
    }
}

impl<'a> GetAstBuilder<'a> for OracleCtx<'a> {
    type Builder = AstBuilder<'a>;
    fn builder(&self) -> &Self::Builder {
        &self.builder
    }
}

impl<'a> GlobalContext<'a> for OracleCtx<'a> {
    fn is_global_reference(&self, _reference: &IdentifierReference<'a>) -> bool {
        true
    }
}

impl<'a> MayHaveSideEffectsContext<'a> for OracleCtx<'a> {
    fn annotations(&self) -> bool {
        false
    }
    fn manual_pure_functions(&self, _callee: &Expression) -> bool {
        false
    }
    fn property_read_side_effects(&self) -> PropertyReadSideEffects {
        PropertyReadSideEffects::None
    }
    fn unknown_global_side_effects(&self) -> bool {
        false
    }
}

impl<'a> ConstantEvaluationCtx<'a> for OracleCtx<'a> {}

pub fn oracle_evaluate(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let Statement::ExpressionStatement(stmt) = parsed.program.body.first()? else {
        return None;
    };
    let ctx = OracleCtx {
        allocator: &alloc,
        builder: AstBuilder::new(&alloc),
    };
    let value = stmt.expression.evaluate_value(&ctx)?;
    Some(match value {
        ConstantValue::Number(n) => format!("number:{}", untwist_eval::coerce::number_to_string(n)),
        ConstantValue::String(text) => format!("string:{text}"),
        ConstantValue::Boolean(b) => format!("boolean:{b}"),
        ConstantValue::Undefined => "undefined".to_string(),
        ConstantValue::Null => "null".to_string(),
        ConstantValue::BigInt(_) => "bigint".to_string(),
    })
}

pub fn find_function<'a>(program: &'a Program<'a>, name: &str) -> Option<FnRef<'a>> {
    for statement in program_statements(program) {
        match statement {
            Statement::FunctionDeclaration(function) => {
                let Some(id) = &function.id else { continue };
                if id.name.as_str() == name {
                    return Some(FnRef {
                        name: id.name.as_str(),
                        function: function.as_ref(),
                        strict: program.has_use_strict_directive(),
                    });
                }
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                        continue;
                    };
                    if id.name.as_str() != name {
                        continue;
                    }
                    let Some(Expression::FunctionExpression(function)) = &declarator.init else {
                        continue;
                    };
                    return Some(FnRef {
                        name: id.name.as_str(),
                        function: function.as_ref(),
                        strict: program.has_use_strict_directive(),
                    });
                }
            }
            _ => {}
        }
    }
    None
}

#[allow(unused_imports)]
pub use untwist_testing::real_world_fixture;

pub fn read_fixture(name: &str) -> String {
    untwist_testing::read_fixture("deobfuscate", name)
}

pub mod scan;
#[allow(unused_imports)]
pub use scan::decoder_calls_anywhere;

fn unwrap_parens<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::ParenthesizedExpression(inner) => unwrap_parens(&inner.expression),
        _ => expr,
    }
}

pub fn program_statements<'a>(program: &'a Program<'a>) -> &'a [Statement<'a>] {
    if let [Statement::ExpressionStatement(stmt)] = program.body.as_slice() {
        if let Expression::FunctionExpression(function) = unwrap_parens(&stmt.expression) {
            if let Some(body) = &function.body {
                return &body.statements;
            }
        }
    }
    &program.body
}

pub fn nth_expression_statement<'a>(
    program: &'a Program<'a>,
    index: usize,
) -> Option<&'a Expression<'a>> {
    program_statements(program)
        .iter()
        .filter_map(|statement| match statement {
            Statement::ExpressionStatement(node) => Some(&node.expression),
            _ => None,
        })
        .nth(index)
}

fn first_of_sequence<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::SequenceExpression(sequence) => sequence.expressions.first().unwrap_or(expr),
        _ => expr,
    }
}

pub fn decoder_program<'a>(
    program: &'a Program<'a>,
    array: &str,
    decoders: &[&str],
    rotator: Option<usize>,
) -> DecoderProgram<'a> {
    DecoderProgram {
        source_text: program.source_text,
        string_array: find_function(program, array).expect("string array function"),
        decoders: decoders
            .iter()
            .map(|name| find_function(program, name).expect("decoder function"))
            .collect(),
        rotator: rotator
            .and_then(|index| nth_expression_statement(program, index))
            .map(first_of_sequence),
        strict: program.has_use_strict_directive(),
    }
}

pub fn calls_for<'a>(program: &'a Program<'a>, decoder: usize) -> Vec<Call<'a>> {
    let mut calls = Vec::new();
    for statement in &program.body {
        let Statement::ExpressionStatement(node) = statement else {
            continue;
        };
        let Expression::CallExpression(call) = &node.expression else {
            continue;
        };
        if let Expression::FunctionExpression(inner) = &call.callee {
            if let Some(body) = &inner.body {
                collect_calls(&body.statements, decoder, &mut calls);
                continue;
            }
        }
        push_call(call, decoder, &[], &mut calls);
    }
    calls
}

fn collect_calls<'a>(statements: &'a [Statement<'a>], decoder: usize, out: &mut Vec<Call<'a>>) {
    let locals = local_objects(statements);
    for statement in statements {
        let expression = match statement {
            Statement::ExpressionStatement(node) => &node.expression,
            Statement::ReturnStatement(node) => match &node.argument {
                Some(argument) => argument,
                None => continue,
            },
            _ => continue,
        };
        if let Expression::CallExpression(call) = expression {
            push_call(call, decoder, &locals, out);
        }
    }
}

fn push_call<'a>(
    call: &'a oxc_ast::ast::CallExpression<'a>,
    decoder: usize,
    locals: &[(&'a str, &'a ObjectExpression<'a>)],
    out: &mut Vec<Call<'a>>,
) {
    let arguments: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .filter_map(|argument| argument.as_expression())
        .map(|argument| resolve_argument(argument, locals))
        .collect();
    if arguments.len() != call.arguments.len() {
        return;
    }
    out.push(Call {
        decoder,
        span: call.span(),
        arguments,
    });
}

fn local_objects<'a>(statements: &'a [Statement<'a>]) -> Vec<(&'a str, &'a ObjectExpression<'a>)> {
    let mut out = Vec::new();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(Expression::ObjectExpression(object)) = &declarator.init else {
                continue;
            };
            out.push((id.name.as_str(), object.as_ref()));
        }
    }
    out
}

fn resolve_local_property<'a>(
    locals: &[(&'a str, &'a ObjectExpression<'a>)],
    object_name: &str,
    property_name: &str,
) -> Option<&'a Expression<'a>> {
    let (_, object) = locals.iter().find(|(name, _)| *name == object_name)?;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(entry) = property else {
            continue;
        };
        let PropertyKey::StaticIdentifier(key) = &entry.key else {
            continue;
        };
        if key.name.as_str() == property_name {
            return Some(&entry.value);
        }
    }
    None
}

fn resolve_argument<'a>(
    argument: &'a Expression<'a>,
    locals: &[(&'a str, &'a ObjectExpression<'a>)],
) -> &'a Expression<'a> {
    let Expression::StaticMemberExpression(member) = argument else {
        return argument;
    };
    let Expression::Identifier(object_id) = &member.object else {
        return argument;
    };
    resolve_local_property(
        locals,
        object_id.name.as_str(),
        member.property.name.as_str(),
    )
    .unwrap_or(argument)
}
