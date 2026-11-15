use oxc_ast::ast::{
    ArrowFunctionExpression, Class, Function, NumericLiteral, Program, StringLiteral,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};

const OCTAL_MESSAGES: [&str; 3] = [
    "'0'-prefixed octal literals and octal escape sequences are deprecated",
    "Decimals with leading zeros are not allowed in strict mode",
    "Invalid escape sequence",
];

#[derive(Clone, Copy)]
enum Context {
    Class,
    Function { strict: bool },
}

struct Walk<'s> {
    source: &'s str,
    scoping: &'s Scoping,
    contexts: Vec<Context>,
    lenient: Vec<Span>,
}

impl Walk<'_> {
    fn strict(&self, scope: Option<ScopeId>) -> bool {
        scope.is_none_or(|scope| self.scoping.scope_flags(scope).is_strict_mode())
    }

    fn lenient_here(&self) -> bool {
        let mut in_class = false;
        for context in self.contexts.iter().rev() {
            match context {
                Context::Class => in_class = true,
                Context::Function { strict } => return in_class && !strict,
            }
        }
        false
    }

    fn literal(&mut self, span: Span, octal_like: fn(&str) -> bool) {
        let text = self
            .source
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default();
        if octal_like(text) && self.lenient_here() {
            self.lenient.push(span);
        }
    }
}

impl<'a> VisitJs<'a> for Walk<'_> {
    fn visit_class(&mut self, it: &Class<'a>) {
        self.contexts.push(Context::Class);
        walk_js::walk_class(self, it);
        self.contexts.pop();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let strict = self.strict(it.scope_id.get());
        self.contexts.push(Context::Function { strict });
        walk_js::walk_function(self, it, flags);
        self.contexts.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let strict = self.strict(it.scope_id.get());
        self.contexts.push(Context::Function { strict });
        walk_js::walk_arrow_function_expression(self, it);
        self.contexts.pop();
    }

    fn visit_numeric_literal(&mut self, it: &NumericLiteral<'a>) {
        self.literal(it.span, leading_zero);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.literal(it.span, |text| text.contains('\\'));
    }
}

fn leading_zero(text: &str) -> bool {
    let mut digits = text.bytes();
    digits.next() == Some(b'0') && digits.next().is_some_and(|digit| digit.is_ascii_digit())
}

pub struct ClassOctals {
    lenient: Vec<Span>,
}

impl ClassOctals {
    pub fn scan(program: &Program<'_>, scoping: &Scoping) -> Self {
        let root = scoping.root_scope_id();
        let mut walk = Walk {
            source: program.source_text,
            scoping,
            contexts: vec![Context::Function {
                strict: scoping.scope_flags(root).is_strict_mode(),
            }],
            lenient: Vec::new(),
        };
        walk.visit_program(program);
        ClassOctals {
            lenient: walk.lenient,
        }
    }

    pub fn explains(&self, diagnostic: &OxcDiagnostic) -> bool {
        OCTAL_MESSAGES.contains(&diagnostic.message.as_ref())
            && !diagnostic.labels.is_empty()
            && diagnostic.labels.iter().all(|label| {
                self.lenient
                    .contains(&Span::sized(label.offset(), label.len()))
            })
    }
}
