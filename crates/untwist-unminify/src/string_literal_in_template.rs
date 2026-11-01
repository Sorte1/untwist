use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_str::Str;
use oxc_traverse::Ancestor;
use untwist_core::pass::{Cx, Interest, Pass};

#[derive(Default)]
pub struct StringLiteralInTemplate;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(StringLiteralInTemplate)
}

pub(crate) fn escape_template(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '`' => out.push_str("\\`"),
            '$' => out.push_str("\\$"),
            '\0' => out.push_str("\\0"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            other => out.push(other),
        }
    }
    out
}

struct Pending {
    span: Span,
    raw: String,
    cooked: Option<String>,
    tail: bool,
    lone_surrogates: bool,
}

impl Pending {
    fn start(element: &TemplateElement<'_>) -> Self {
        Self {
            span: element.span,
            raw: element.value.raw.as_str().to_string(),
            cooked: element
                .value
                .cooked
                .as_ref()
                .map(|text| text.as_str().to_string()),
            tail: element.tail,
            lone_surrogates: element.lone_surrogates,
        }
    }

    fn append_literal(&mut self, text: &str) {
        self.raw.push_str(&escape_template(text));
        if let Some(cooked) = &mut self.cooked {
            cooked.push_str(text);
        }
    }

    fn merge(&mut self, element: &TemplateElement<'_>) {
        self.raw.push_str(element.value.raw.as_str());
        match &element.value.cooked {
            Some(text) => {
                if let Some(cooked) = &mut self.cooked {
                    cooked.push_str(text.as_str());
                }
            }
            None => self.cooked = None,
        }
        self.span = Span::new(self.span.start, element.span.end);
        self.tail = element.tail;
        self.lone_surrogates = self.lone_surrogates || element.lone_surrogates;
    }

    fn build<'a>(self, cx: &mut Cx<'a, '_>) -> TemplateElement<'a> {
        let value = TemplateElementValue {
            raw: Str::from_str_in(&self.raw, cx),
            cooked: self.cooked.map(|text| Str::from_str_in(&text, cx)),
        };
        TemplateElement::new_with_lone_surrogates(
            self.span,
            value,
            self.tail,
            self.lone_surrogates,
            cx,
        )
    }
}

impl<'a> Pass<'a> for StringLiteralInTemplate {
    fn name(&self) -> &'static str {
        "string-literal-in-template-literal"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_TEMPLATE_LITERAL
    }

    fn exit_template_literal(&mut self, node: &mut TemplateLiteral<'a>, cx: &mut Cx<'a, '_>) {
        if node.quasis.len() != node.expressions.len() + 1 {
            return;
        }
        if matches!(cx.tcx.parent(), Ancestor::TaggedTemplateExpressionQuasi(_)) {
            return;
        }
        if !node
            .expressions
            .iter()
            .any(|expr| matches!(expr, Expression::StringLiteral(_)))
        {
            return;
        }
        let quasis = std::mem::replace(&mut node.quasis, AVec::new_in(cx));
        let expressions = std::mem::replace(&mut node.expressions, AVec::new_in(cx));
        let mut quasis = quasis.into_iter();
        let Some(first) = quasis.next() else {
            return;
        };
        let mut pending = Pending::start(&first);
        let mut inlined = 0usize;
        for (expr, next) in expressions.into_iter().zip(quasis) {
            match expr {
                Expression::StringLiteral(literal)
                    if !literal.lone_surrogates
                        && !pending.lone_surrogates
                        && !next.lone_surrogates =>
                {
                    pending.append_literal(literal.value.as_str());
                    pending.merge(&next);
                    inlined += 1;
                }
                other => {
                    let built = pending.build(cx);
                    node.quasis.push(built);
                    node.expressions.push(other);
                    pending = Pending::start(&next);
                }
            }
        }
        let built = pending.build(cx);
        node.quasis.push(built);
        cx.changed_by(inlined);
    }
}
