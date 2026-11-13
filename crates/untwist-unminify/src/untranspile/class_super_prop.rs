use oxc_allocator::{GetAllocator, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::Ancestor;
use untwist_core::pass::Cx;

use super::identify::Table;
use super::shapes::Kind;

pub const SHAPE: &str = "class/super-property/unsafe";

const ON_PROTOTYPE: u32 = 1;
const CALLED: u32 = 2;

struct Site {
    class: SymbolId,
    key: String,
    flags: u32,
}

fn flags_of(expression: &Expression<'_>) -> Option<u32> {
    match expression.without_parentheses() {
        Expression::NumericLiteral(literal)
            if literal.value.fract() == 0.0 && (0.0..=3.0).contains(&literal.value) =>
        {
            Some(literal.value as u32)
        }
        _ => None,
    }
}

fn site(call: &CallExpression<'_>, table: &Table, scoping: &Scoping) -> Option<Site> {
    if call.optional || table.kind_of_callee(&call.callee, scoping) != Some(Kind::SuperPropGet) {
        return None;
    }
    let [Argument::Identifier(class), Argument::StringLiteral(key), Argument::ThisExpression(_), flags] =
        call.arguments.as_slice()
    else {
        return None;
    };
    let class = scoping
        .get_reference(class.reference_id.get()?)
        .symbol_id()?;
    let flags = flags_of(flags.as_expression()?)?;
    Some(Site {
        class,
        key: key.value.as_str()?.to_string(),
        flags,
    })
}

fn home_fits(class: SymbolId, on_prototype: bool, cx: &Cx<'_, '_>) -> bool {
    let mut ancestors = cx.tcx.ancestors();
    loop {
        let Some(ancestor) = ancestors.next() else {
            return false;
        };
        match ancestor {
            Ancestor::ArrowFunctionExpressionBody(_)
            | Ancestor::ArrowFunctionExpressionParams(_) => {}
            Ancestor::FunctionBody(_) | Ancestor::FunctionParams(_) => break,
            Ancestor::ClassBodyBody(_)
            | Ancestor::StaticBlockBody(_)
            | Ancestor::PropertyDefinitionValue(_)
            | Ancestor::ProgramBody(_) => return false,
            _ => {}
        }
    }
    let Some(Ancestor::MethodDefinitionValue(method)) = ancestors.next() else {
        return false;
    };
    if *method.r#static() == on_prototype {
        return false;
    }
    let Some(Ancestor::ClassBodyBody(_)) = ancestors.next() else {
        return false;
    };
    let Some(Ancestor::ClassBody(owner)) = ancestors.next() else {
        return false;
    };
    owner
        .id()
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .is_some_and(|own| own == class)
}

fn member<'a>(key: &str, cx: &Cx<'a, '_>) -> Expression<'a> {
    let object = Expression::new_super(SPAN, cx);
    let text: &'a str = GetAllocator::allocator(cx).alloc_str(key);
    if is_identifier_name(key) {
        let name = IdentifierName::new(SPAN, text, cx);
        Expression::StaticMemberExpression(StaticMemberExpression::boxed(
            SPAN, object, name, false, cx,
        ))
    } else {
        let literal = Expression::new_string_literal(SPAN, text, None, cx);
        Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
            SPAN, object, literal, false, cx,
        ))
    }
}

fn spread_arguments<'a>(
    array: &mut ArrayExpression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<AVec<'a, Argument<'a>>> {
    let mut out = AVec::with_capacity_in(array.elements.len(), cx);
    for element in array.elements.iter_mut() {
        let argument = match element {
            ArrayExpressionElement::Elision(_) => return None,
            ArrayExpressionElement::SpreadElement(spread) => {
                Argument::SpreadElement(oxc_allocator::Box::new_in(spread.as_mut().take_in(cx), cx))
            }
            other => Argument::from(other.as_expression_mut()?.take_in(cx)),
        };
        out.push(argument);
    }
    Some(out)
}

fn holes_free(argument: &Argument<'_>) -> bool {
    matches!(argument, Argument::ArrayExpression(array)
        if array.elements.iter().all(|element| !element.is_elision()))
}

pub fn rewrite<'a>(expression: &mut Expression<'a>, table: &Table, cx: &mut Cx<'a, '_>) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    if let Expression::CallExpression(inner) = &call.callee {
        let Some(found) = site(inner, table, cx.scoping()) else {
            return false;
        };
        let called = found.flags & CALLED != 0;
        let fits = called
            && !call.optional
            && matches!(call.arguments.as_slice(), [argument] if holes_free(argument))
            && home_fits(found.class, found.flags & ON_PROTOTYPE != 0, cx);
        if !fits {
            return false;
        }
        let key = found.key;
        let Some(Argument::ArrayExpression(array)) = call.arguments.first_mut() else {
            return false;
        };
        let Some(arguments) = spread_arguments(array, cx) else {
            return false;
        };
        let callee = member(&key, cx);
        *expression = Expression::new_call_expression(
            SPAN,
            callee,
            None::<oxc_allocator::Box<'a, TSTypeParameterInstantiation<'a>>>,
            arguments,
            false,
            cx,
        );
        return true;
    }
    let Some(found) = site(call, table, cx.scoping()) else {
        return false;
    };
    if found.flags & CALLED != 0 || !home_fits(found.class, found.flags & ON_PROTOTYPE != 0, cx) {
        return false;
    }
    *expression = member(&found.key, cx);
    true
}

pub fn may_be_site(expression: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    match &call.callee {
        Expression::CallExpression(inner) => inner.arguments.len() == 4,
        _ => call.arguments.len() == 4,
    }
}
