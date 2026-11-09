use oxc_ast::ast::{
    ArrowFunctionBody, AssignmentOperator, AssignmentTarget, Expression, FormalParameters,
    Statement,
};
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, Semantic};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolId;

use super::module::{Signature, ValueType};

const I32: ValueType = ValueType::I32;
const F64: ValueType = ValueType::F64;
const HEAP_MAXIMA: [f64; 3] = [2_147_483_648.0, 4_294_901_760.0, 2_147_418_112.0];
const PAGE_BYTES: f64 = 65_536.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    CopyWithin,
    GlobalNow(&'static str),
    HeapMax,
    ResizeHeap,
}

#[derive(Debug)]
pub(super) struct Known {
    pub name: &'static str,
    params: &'static [ValueType],
    results: &'static [ValueType],
    shape: Shape,
}

impl Known {
    pub fn fits(&self, signature: &Signature) -> bool {
        signature.params == self.params && signature.results == self.results
    }
}

const KNOWN: &[Known] = &[
    Known {
        name: "_emscripten_memcpy_big",
        params: &[I32, I32, I32],
        results: &[],
        shape: Shape::CopyWithin,
    },
    Known {
        name: "_emscripten_date_now",
        params: &[],
        results: &[F64],
        shape: Shape::GlobalNow("Date"),
    },
    Known {
        name: "_emscripten_get_now",
        params: &[],
        results: &[F64],
        shape: Shape::GlobalNow("performance"),
    },
    Known {
        name: "_emscripten_get_heap_max",
        params: &[],
        results: &[I32],
        shape: Shape::HeapMax,
    },
    Known {
        name: "_emscripten_resize_heap",
        params: &[I32],
        results: &[I32],
        shape: Shape::ResizeHeap,
    },
];

struct Body<'a> {
    params: Vec<SymbolId>,
    statements: &'a [Statement<'a>],
    expression: Option<&'a Expression<'a>>,
}

fn simple_params(params: &FormalParameters<'_>) -> Option<Vec<SymbolId>> {
    if params.rest.is_some() {
        return None;
    }
    params
        .items
        .iter()
        .map(|param| match param.initializer {
            Some(_) => None,
            None => param
                .pattern
                .get_binding_identifier()
                .map(|id| id.symbol_id()),
        })
        .collect()
}

fn body_of<'a>(expression: &'a Expression<'a>) -> Option<Body<'a>> {
    match expression.without_parentheses() {
        Expression::FunctionExpression(function) => Some(Body {
            params: simple_params(&function.params)?,
            statements: &function.body.as_ref()?.statements,
            expression: None,
        }),
        Expression::ArrowFunctionExpression(arrow) => {
            let params = simple_params(&arrow.params)?;
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(Body {
                    params,
                    statements: &body.statements,
                    expression: None,
                }),
                body => Some(Body {
                    params,
                    statements: &[],
                    expression: Some(body.as_expression()?),
                }),
            }
        }
        _ => None,
    }
}

fn declared_body<'a>(semantic: &'a Semantic<'a>, symbol: SymbolId) -> Option<Body<'a>> {
    let declaration = semantic.scoping().symbol_declaration(symbol);
    match semantic.nodes().kind(declaration) {
        AstKind::Function(function) if function.is_declaration() => Some(Body {
            params: simple_params(&function.params)?,
            statements: &function.body.as_ref()?.statements,
            expression: None,
        }),
        AstKind::VariableDeclarator(declarator) => body_of(declarator.init.as_ref()?),
        _ => None,
    }
}

impl<'a> Body<'a> {
    fn sole_expression(&self) -> Option<&'a Expression<'a>> {
        if let Some(expression) = self.expression {
            return Some(expression);
        }
        match self.statements {
            [Statement::ExpressionStatement(statement)] => Some(&statement.expression),
            [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
            _ => None,
        }
    }
}

fn param_of(scoping: &Scoping, expression: &Expression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(id) = expression.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id()).symbol_id()
}

fn is_copy_within(scoping: &Scoping, body: &Body<'_>) -> bool {
    let [dest, src, len] = body.params[..] else {
        return false;
    };
    let Some(Expression::CallExpression(call)) =
        body.sole_expression().map(|e| e.without_parentheses())
    else {
        return false;
    };
    let Some(member) = call.callee.without_parentheses().as_member_expression() else {
        return false;
    };
    if member.static_property_name().and_then(|name| name.as_str()) != Some("copyWithin") {
        return false;
    }
    let arguments: Vec<_> = call.arguments.iter().map(|a| a.as_expression()).collect();
    let [Some(first), Some(second), Some(Expression::BinaryExpression(end))] = arguments[..] else {
        return false;
    };
    let operands = (param_of(scoping, &end.left), param_of(scoping, &end.right));
    end.operator == BinaryOperator::Addition
        && param_of(scoping, first) == Some(dest)
        && param_of(scoping, second) == Some(src)
        && (operands == (Some(src), Some(len)) || operands == (Some(len), Some(src)))
}

fn is_global_now(scoping: &Scoping, body: &Body<'_>, global: &str) -> bool {
    let Some(Expression::CallExpression(call)) =
        body.sole_expression().map(|e| e.without_parentheses())
    else {
        return false;
    };
    let Some(member) = call.callee.without_parentheses().as_member_expression() else {
        return false;
    };
    let Expression::Identifier(object) = member.object().without_parentheses() else {
        return false;
    };
    body.params.is_empty()
        && call.arguments.is_empty()
        && member.static_property_name().and_then(|name| name.as_str()) == Some("now")
        && object.name == global
        && scoping
            .get_reference(object.reference_id())
            .symbol_id()
            .is_none()
}

fn is_heap_max(body: &Body<'_>) -> bool {
    let Some(Expression::NumericLiteral(number)) =
        body.sole_expression().map(|e| e.without_parentheses())
    else {
        return false;
    };
    body.params.is_empty() && HEAP_MAXIMA.contains(&number.value)
}

#[derive(Default)]
struct ResizeScan {
    param: Option<SymbolId>,
    unsigned_param: bool,
    has_loop: bool,
    has_page: bool,
    has_maximum: bool,
    nested_function: bool,
}

impl ResizeScan {
    fn is_param(&self, scoping: &Scoping, target: &AssignmentTarget<'_>) -> bool {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = target else {
            return false;
        };
        scoping.get_reference(id.reference_id()).symbol_id() == self.param
    }
}

struct ResizeVisit<'s> {
    scoping: &'s Scoping,
    scan: ResizeScan,
}

impl<'a> VisitJs<'a> for ResizeVisit<'_> {
    fn visit_assignment_expression(&mut self, it: &oxc_ast::ast::AssignmentExpression<'a>) {
        let shifted = match it.operator {
            AssignmentOperator::ShiftRightZeroFill => is_zero(&it.right),
            AssignmentOperator::Assign => match it.right.without_parentheses() {
                Expression::BinaryExpression(binary) => {
                    binary.operator == BinaryOperator::ShiftRightZeroFill
                        && is_zero(&binary.right)
                        && param_of(self.scoping, &binary.left) == self.scan.param
                }
                _ => false,
            },
            _ => false,
        };
        if shifted && self.scan.is_param(self.scoping, &it.left) {
            self.scan.unsigned_param = true;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_for_statement(&mut self, it: &oxc_ast::ast::ForStatement<'a>) {
        self.scan.has_loop = true;
        walk_js::walk_for_statement(self, it);
    }

    fn visit_numeric_literal(&mut self, it: &oxc_ast::ast::NumericLiteral<'a>) {
        self.scan.has_page |= it.value == PAGE_BYTES;
        self.scan.has_maximum |= HEAP_MAXIMA.contains(&it.value);
    }

    fn visit_function(
        &mut self,
        _it: &oxc_ast::ast::Function<'a>,
        _flags: oxc_syntax::scope::ScopeFlags,
    ) {
        self.scan.nested_function = true;
    }
}

fn is_zero(expression: &Expression<'_>) -> bool {
    matches!(expression.without_parentheses(), Expression::NumericLiteral(n) if n.value == 0.0)
}

fn is_resize_heap(scoping: &Scoping, body: &Body<'_>) -> bool {
    let [param] = body.params[..] else {
        return false;
    };
    let mut visit = ResizeVisit {
        scoping,
        scan: ResizeScan {
            param: Some(param),
            ..ResizeScan::default()
        },
    };
    for statement in body.statements {
        visit.visit_statement(statement);
    }
    if let Some(expression) = body.expression {
        visit.visit_expression(expression);
    }
    let scan = visit.scan;
    scan.unsigned_param
        && scan.has_loop
        && scan.has_page
        && scan.has_maximum
        && !scan.nested_function
}

fn matches(scoping: &Scoping, body: &Body<'_>, shape: Shape) -> bool {
    match shape {
        Shape::CopyWithin => is_copy_within(scoping, body),
        Shape::GlobalNow(global) => is_global_now(scoping, body, global),
        Shape::HeapMax => is_heap_max(body),
        Shape::ResizeHeap => is_resize_heap(scoping, body),
    }
}

pub(super) fn known_function<'s>(
    semantic: &'s Semantic<'s>,
    symbol: SymbolId,
) -> Option<&'static Known> {
    let body = declared_body(semantic, symbol)?;
    KNOWN
        .iter()
        .find(|known| matches(semantic.scoping(), &body, known.shape))
}
