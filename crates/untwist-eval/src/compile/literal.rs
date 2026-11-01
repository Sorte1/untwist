use std::rc::Rc;

use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, Expression, ObjectExpression, ObjectProperty,
    ObjectPropertyKind, PropertyKey, PropertyKind, TemplateElement, TemplateLiteral,
};
#[cfg(feature = "test-hooks")]
use oxc_span::GetSpan;

use crate::bail::Bail;
use crate::budget::NESTING_DEPTH_CAP;
use crate::builtins::object::unmodelled_literal_entry;
use crate::compile::depth::{level, Reach};
use crate::compile::expr::{compile_expression, node};
use crate::compile::konst::{self, Konst};
use crate::compile::scope::ScopeChain;
use crate::compile::{Code, Compiler};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::ops::string_key;
use crate::semantics::literals::template_chunk;
use crate::value::{js_string_from_literal, Eval, FnCode, JsString, PropKey, PropMap, Value};

enum Element<'a> {
    Literal(Value<'a>),
    Value(Code<'a>),
    Hole,
    Spread,
    Odd,
}

fn literal_value<'a>(inner: &'a Expression<'a>) -> Option<Value<'a>> {
    Some(match inner {
        Expression::StringLiteral(literal) => Value::Str(js_string_from_literal(literal)),
        Expression::NumericLiteral(literal) => Value::Number(literal.value),
        Expression::BooleanLiteral(literal) => Value::Bool(literal.value),
        Expression::NullLiteral(_) => Value::Null,
        _ => return None,
    })
}

fn array_element<'a>(
    inner: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Element<'a> {
    let Some(literal) = literal_value(inner) else {
        return Element::Value(compile_expression(inner, scope, cx));
    };
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_node(inner.span());
    cx.reach(1);
    Element::Literal(literal)
}

fn literal_element<'a>(
    interp: &mut Interp<'a>,
    value: &Value<'a>,
    reach: Reach,
) -> Eval<'a, Value<'a>> {
    level(interp, reach, |interp| {
        interp.budget.spend(1)?;
        Ok(value.clone())
    })
}

pub(super) fn compile_array<'a>(
    array: &'a ArrayExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    if let Some(template) = Template::of(array, cx) {
        cx.reach(1);
        return node(move |interp, _, reach| template.run(interp, reach));
    }
    let elements: Box<[Element<'a>]> = array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::SpreadElement(_) => Element::Spread,
            ArrayExpressionElement::Elision(_) => Element::Hole,
            other => match other.as_expression() {
                Some(inner) => array_element(inner, scope, cx),
                None => Element::Odd,
            },
        })
        .collect();
    node(move |interp, frame, reach| {
        let mut items = Vec::with_capacity(elements.len());
        for element in elements.iter() {
            match element {
                Element::Literal(value) => items.push(literal_element(interp, value, reach)?),
                Element::Value(code) => items.push(code.run(interp, frame, reach)?),
                Element::Hole => items.push(interp.array_hole()?),
                Element::Spread => return Err(Bail::Unsupported(&"spread element").into()),
                Element::Odd => return Err(Bail::Unsupported(&"array element").into()),
            }
        }
        interp.finish_array_literal(items)
    })
}

type Constant<'a> = (usize, Konst, &'a Expression<'a>);

struct Template<'a> {
    values: Box<[Value<'a>]>,
    constants: Box<[Constant<'a>]>,
    weight: u64,
    depth: u32,
}

enum Slot {
    Literal,
    Constant(Konst),
}

fn is_literal(inner: &Expression<'_>) -> bool {
    matches!(
        inner,
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}

fn template_slot(inner: &Expression<'_>, cx: &mut Compiler<'_, '_>) -> Option<Slot> {
    if is_literal(inner) {
        return Some(Slot::Literal);
    }
    if !konst::foldable_root(inner) {
        return None;
    }
    match konst::fold(inner) {
        Ok(folded) => Some(Slot::Constant(folded)),
        Err(at) => {
            cx.unfoldable = Some(at);
            None
        }
    }
}

fn template_element<'a>(element: &'a ArrayExpressionElement<'a>) -> Option<&'a Expression<'a>> {
    match element {
        ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => None,
        other => other.as_expression(),
    }
}

struct Shape {
    weight: u64,
    depth: u32,
    constants: usize,
}

fn template_shape(array: &ArrayExpression<'_>, cx: &mut Compiler<'_, '_>) -> Option<Shape> {
    let (mut weight, mut deepest, mut constants) = (0u64, None::<u32>, 0usize);
    for element in &array.elements {
        let (own, depth) = match template_slot(template_element(element)?, cx)? {
            Slot::Literal => (1, 0),
            Slot::Constant(folded) => {
                constants += 1;
                (folded.weight, folded.depth)
            }
        };
        weight = weight.checked_add(own)?;
        deepest = Some(deepest.map_or(depth, |seen| seen.max(depth)));
    }
    let depth = match deepest {
        Some(depth) => depth.checked_add(1)?,
        None => 0,
    };
    Some(Shape {
        weight,
        depth,
        constants,
    })
}

impl<'a> Template<'a> {
    fn of(array: &'a ArrayExpression<'a>, cx: &mut Compiler<'_, 'a>) -> Option<Self> {
        let shape = template_shape(array, cx)?;
        let mut values = Vec::with_capacity(array.elements.len());
        let mut constants = Vec::with_capacity(shape.constants);
        for (index, element) in array.elements.iter().enumerate() {
            let inner = template_element(element)?;
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_compiled_node(inner.span());
            match literal_value(inner) {
                Some(value) => values.push(value),
                None => {
                    let folded = konst::fold(inner).ok()?;
                    values.push(Value::Number(folded.value));
                    constants.push((index, folded, inner));
                }
            }
        }
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_template(constants.len());
        Some(Template {
            values: values.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            weight: shape.weight,
            depth: shape.depth,
        })
    }

    fn run(&self, interp: &mut Interp<'a>, reach: Reach) -> Eval<'a, Value<'a>> {
        let nesting = u64::from(interp.budget.nesting_depth()) + u64::from(self.depth);
        if nesting > u64::from(NESTING_DEPTH_CAP) || interp.budget.steps_left() < self.weight {
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_template_run(false);
            return self.per_element(interp, reach);
        }
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_template_run(true);
        interp.budget.spend(self.weight)?;
        interp.finish_array_literal(self.values.to_vec())
    }

    fn per_element(&self, interp: &mut Interp<'a>, reach: Reach) -> Eval<'a, Value<'a>> {
        let mut items = Vec::with_capacity(self.values.len());
        let mut constants = self.constants.iter().peekable();
        for (index, value) in self.values.iter().enumerate() {
            let item = match constants.next_if(|(at, ..)| *at == index) {
                Some((_, folded, expr)) => level(interp, reach, |interp| {
                    interp.budget.spend(1)?;
                    folded.run(interp, expr)
                })?,
                None => literal_element(interp, value, reach)?,
            };
            items.push(item);
        }
        interp.finish_array_literal(items)
    }
}

struct Piece<'a> {
    quasi: &'a TemplateElement<'a>,
    text: Option<JsString>,
    part: Option<Code<'a>>,
}

pub(super) fn compile_template<'a>(
    template: &'a TemplateLiteral<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let pieces: Box<[Piece<'a>]> = template
        .quasis
        .iter()
        .enumerate()
        .map(|(index, quasi)| Piece {
            quasi,
            text: template_chunk(quasi).ok(),
            part: template
                .expressions
                .get(index)
                .map(|part| compile_expression(part, scope, cx)),
        })
        .collect();
    node(move |interp, frame, reach| {
        let mut out = JsString::empty();
        for piece in pieces.iter() {
            let chunk = match &piece.text {
                Some(text) => text.clone(),
                None => template_chunk(piece.quasi)?,
            };
            out = interp.append_template_text(&out, &chunk)?;
            if let Some(part) = &piece.part {
                let value = part.run(interp, frame, reach)?;
                out = interp.append_template_value(&out, &value)?;
            }
        }
        Ok(Value::Str(out))
    })
}

enum LiteralKey<'a> {
    Static(PropKey),
    Computed(Code<'a>),
    Refused(&'static &'static str),
}

impl<'a> LiteralKey<'a> {
    fn of(
        entry: &'a ObjectProperty<'a>,
        scope: &Rc<ScopeChain>,
        cx: &mut Compiler<'_, 'a>,
    ) -> Self {
        match &entry.key {
            PropertyKey::StaticIdentifier(id) => {
                LiteralKey::Static(string_key(JsString::from_str(id.name.as_str())))
            }
            PropertyKey::PrivateIdentifier(_) => LiteralKey::Refused(&"private class field"),
            other => match other.as_expression() {
                None => LiteralKey::Refused(&"property key"),
                Some(Expression::StringLiteral(literal)) if !entry.computed => {
                    LiteralKey::Static(string_key(js_string_from_literal(literal)))
                }
                Some(expr) => LiteralKey::Computed(compile_expression(expr, scope, cx)),
            },
        }
    }

    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, PropKey> {
        match self {
            LiteralKey::Static(key) => Ok(key.clone()),
            LiteralKey::Computed(code) => {
                let value = code.run(interp, frame, reach)?;
                interp.operator_key(&value)
            }
            LiteralKey::Refused(why) => Err(Bail::Unsupported(why).into()),
        }
    }
}

enum LiteralValue<'a> {
    Method(FnCode<'a>),
    Value(Code<'a>),
}

enum Property<'a> {
    Entry {
        refused: Option<&'static &'static str>,
        key: LiteralKey<'a>,
        value: LiteralValue<'a>,
    },
    Spread,
}

fn property<'a>(
    kind: &'a ObjectPropertyKind<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Property<'a> {
    let ObjectPropertyKind::ObjectProperty(entry) = kind else {
        return Property::Spread;
    };
    let key = LiteralKey::of(entry, scope, cx);
    let value = match &entry.value {
        Expression::FunctionExpression(function)
            if entry.method || entry.kind != PropertyKind::Init =>
        {
            let code = FnCode::Function(function);
            cx.functions.register(code, scope);
            LiteralValue::Method(code)
        }
        other => LiteralValue::Value(compile_expression(other, scope, cx)),
    };
    Property::Entry {
        refused: unmodelled_literal_entry(entry),
        key,
        value,
    }
}

pub(super) fn compile_object<'a>(
    object: &'a ObjectExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let properties: Box<[Property<'a>]> = object
        .properties
        .iter()
        .map(|kind| property(kind, scope, cx))
        .collect();
    node(move |interp, frame, reach| {
        let mut props: PropMap<'a> = PropMap::default();
        for property in properties.iter() {
            let Property::Entry {
                refused,
                key,
                value,
            } = property
            else {
                return Err(Bail::Unsupported(&"object spread").into());
            };
            if let Some(why) = refused {
                return Err(Bail::Unsupported(why).into());
            }
            let key = key.eval(interp, frame, reach)?;
            let value = match value {
                LiteralValue::Method(code) => interp.make_method(*code, frame),
                LiteralValue::Value(code) => code.run(interp, frame, reach)?,
            };
            props.insert(key.into(), value);
        }
        Ok(interp.finish_object_literal(props))
    })
}
