use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::value::JsString;

use crate::proof::{names_a_token, GLOBAL_OBJECTS};

const CODE_RUNNERS: [&str; 5] = [
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "execScript",
];
const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];

enum Text {
    Known(String),
    Unknown,
}

struct Reach<'s, 'r> {
    scoping: &'s Scoping,
    strings: &'r FxHashMap<Span, &'r JsString>,
    names: &'r [&'r str],
    machinery: &'r [Span],
    aliases: FxHashSet<SymbolId>,
    constants: FxHashMap<SymbolId, String>,
    function_depth: usize,
    found: Option<Span>,
}

impl Reach<'_, '_> {
    fn is_global_name(&self, id: &IdentifierReference<'_>, wanted: &[&str]) -> bool {
        wanted.contains(&id.name.as_str())
            && id.reference_id.get().is_some_and(|reference| {
                self.scoping.get_reference(reference).symbol_id().is_none()
            })
    }

    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()
    }

    fn is_global_base(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => {
                self.is_global_name(id, GLOBAL_OBJECTS)
                    || self
                        .symbol_of(id)
                        .is_some_and(|s| self.aliases.contains(&s))
            }
            Expression::ThisExpression(_) => self.function_depth == 0,
            _ => false,
        }
    }

    fn text(&self, expr: &Expression<'_>) -> Text {
        self.text_of(expr, true)
    }

    fn spells_in_a_piece(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                self.spells_in_a_piece(&binary.left) || self.spells_in_a_piece(&binary.right)
            }
            Expression::TemplateLiteral(template) => {
                template.quasis.iter().any(|quasi| {
                    quasi
                        .value
                        .cooked
                        .and_then(|cooked| cooked.as_str())
                        .is_some_and(|text| names_a_token(text, self.names))
                }) || template
                    .expressions
                    .iter()
                    .any(|expr| self.spells_in_a_piece(expr))
            }
            other => match self.text_of(other, true) {
                Text::Known(text) => names_a_token(&text, self.names),
                Text::Unknown => false,
            },
        }
    }

    fn spells_inside(&self, expr: &Expression<'_>) -> bool {
        let mut probe = Spelling {
            reach: self,
            found: false,
        };
        probe.visit_expression(expr);
        probe.found
    }

    fn text_of(&self, expr: &Expression<'_>, decoded: bool) -> Text {
        let expr = expr.without_parentheses();
        if let Some(text) = self.strings.get(&expr.span()).filter(|_| decoded) {
            return text.as_string().map_or(Text::Unknown, Text::Known);
        }
        match expr {
            Expression::StringLiteral(literal) => literal
                .value
                .as_str()
                .map_or(Text::Unknown, |text| Text::Known(text.to_string())),
            Expression::NumericLiteral(literal) => Text::Known(literal.value.to_string()),
            Expression::Identifier(id) => self
                .symbol_of(id)
                .and_then(|symbol| self.constants.get(&symbol))
                .map_or(Text::Unknown, |text| Text::Known(text.clone())),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
                .quasis
                .first()
                .and_then(|quasi| quasi.value.cooked)
                .and_then(|cooked| cooked.as_str())
                .map_or(Text::Unknown, |text| Text::Known(text.to_string())),
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                match (
                    self.text_of(&binary.left, decoded),
                    self.text_of(&binary.right, decoded),
                ) {
                    (Text::Known(left), Text::Known(right)) => Text::Known(left + &right),
                    _ => Text::Unknown,
                }
            }
            _ => Text::Unknown,
        }
    }

    fn inside_machinery(&self, span: Span) -> bool {
        self.machinery
            .iter()
            .any(|outer| outer.start <= span.start && span.end <= outer.end)
    }

    fn reach(&mut self, span: Span) {
        if !self.inside_machinery(span) {
            self.found.get_or_insert(span);
        }
    }

    fn key(&mut self, key: &str, span: Span) {
        if self.names.contains(&key) {
            self.reach(span);
        }
    }

    fn runner<'k>(&self, callee: &'k Expression<'k>) -> Option<&'k str> {
        let callee = match callee.without_parentheses() {
            Expression::SequenceExpression(sequence) => sequence.expressions.last()?,
            other => other,
        };
        match callee.without_parentheses() {
            Expression::Identifier(id) if self.is_global_name(id, &CODE_RUNNERS) => {
                Some(id.name.as_str())
            }
            Expression::StaticMemberExpression(member)
                if member.property.name == "constructor"
                    && matches!(
                        member.object.without_parentheses(),
                        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                    ) =>
            {
                Some("Function")
            }
            Expression::StaticMemberExpression(member)
                if self.is_global_base(&member.object)
                    && CODE_RUNNERS.contains(&member.property.name.as_str()) =>
            {
                Some(member.property.name.as_str())
            }
            _ => None,
        }
    }

    fn passes_a_function(&self, argument: &Argument<'_>) -> bool {
        match argument
            .as_expression()
            .map(Expression::without_parentheses)
        {
            Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) => {
                true
            }
            Some(Expression::Identifier(id)) => self.symbol_of(id).is_some(),
            _ => false,
        }
    }

    fn judge_runner(&mut self, name: &str, arguments: &[Argument<'_>], span: Span) {
        let texts: Vec<&Argument<'_>> = match name {
            "Function" => arguments.iter().collect(),
            _ => arguments.iter().take(1).collect(),
        };
        for argument in texts {
            if TIMERS.contains(&name) && self.passes_a_function(argument) {
                continue;
            }
            if argument
                .as_expression()
                .is_some_and(|expr| self.spells_in_a_piece(expr))
            {
                self.reach(span);
            }
        }
    }

    fn pattern_keys(&mut self, pattern: &BindingPattern<'_>, span: Span) {
        let BindingPattern::ObjectPattern(object) = pattern else {
            return;
        };
        for property in &object.properties {
            let key = property.key.static_name();
            if let Some(key) = key.as_ref().and_then(|name| name.as_str()) {
                if !property.computed {
                    self.key(key, span);
                }
            }
        }
    }
}

impl<'a> VisitJs<'a> for Reach<'_, '_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.function_depth += 1;
        walk_js::walk_function(self, it, flags);
        self.function_depth -= 1;
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_global_base(&it.object) {
            self.key(it.property.name.as_str(), it.span);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_global_base(&it.object) {
            match self.text(&it.expression) {
                Text::Known(key) => self.key(&key, it.span),
                Text::Unknown => {
                    if self.spells_inside(&it.expression) {
                        self.reach(it.span);
                    }
                }
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if it
            .init
            .as_ref()
            .is_some_and(|init| self.is_global_base(init))
        {
            self.pattern_keys(&it.id, it.span);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(name) = self.runner(&it.callee) {
            self.judge_runner(name, &it.arguments, it.span);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Some(name) = self.runner(&it.callee) {
            self.judge_runner(name, &it.arguments, it.span);
        }
        walk_js::walk_new_expression(self, it);
    }
}

struct Spelling<'r, 's, 'm> {
    reach: &'r Reach<'s, 'm>,
    found: bool,
}

impl<'a> VisitJs<'a> for Spelling<'_, '_, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Text::Known(text) = self.reach.text_of(it, true) {
            self.found |= names_a_token(&text, self.reach.names);
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

struct Aliases<'s> {
    scoping: &'s Scoping,
    function_depth: usize,
    found: FxHashSet<SymbolId>,
    constants: FxHashMap<SymbolId, String>,
}

impl Aliases<'_> {
    fn holds_global(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => {
                GLOBAL_OBJECTS.contains(&id.name.as_str())
                    && id.reference_id.get().is_some_and(|reference| {
                        self.scoping.get_reference(reference).symbol_id().is_none()
                    })
            }
            Expression::ThisExpression(_) => self.function_depth == 0,
            _ => false,
        }
    }

    fn never_reassigned(&self, symbol: SymbolId) -> bool {
        self.scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| !self.scoping.get_reference(*reference).is_write())
    }
}

impl<'a> VisitJs<'a> for Aliases<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.function_depth += 1;
        walk_js::walk_function(self, it, flags);
        self.function_depth -= 1;
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Some(symbol) = id.symbol_id.get().filter(|s| self.never_reassigned(*s)) {
                if self.holds_global(init) {
                    self.found.insert(symbol);
                }
                if let Expression::StringLiteral(literal) = init.without_parentheses() {
                    if let Some(text) = literal.value.as_str() {
                        self.constants.insert(symbol, text.to_string());
                    }
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

pub(crate) fn syntactic_global_reach(
    (program, scoping): (&Program<'_>, &Scoping),
    strings: &FxHashMap<Span, &JsString>,
    names: &[&str],
    machinery: &[Span],
) -> Option<Span> {
    let mut aliases = Aliases {
        scoping,
        function_depth: 0,
        found: FxHashSet::default(),
        constants: FxHashMap::default(),
    };
    aliases.visit_program(program);
    let mut reach = Reach {
        scoping,
        strings,
        names,
        machinery,
        aliases: aliases.found,
        constants: aliases.constants,
        function_depth: 0,
        found: None,
    };
    reach.visit_program(program);
    reach.found
}
