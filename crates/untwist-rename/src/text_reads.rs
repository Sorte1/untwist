use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::member_parts;

use crate::collect::reference_symbol;

const TEXT_METHODS: [&str; 2] = ["toString", "toSource"];

const TEXT_TAKING_METHODS: [&str; 10] = [
    "test",
    "exec",
    "search",
    "match",
    "matchAll",
    "replace",
    "replaceAll",
    "indexOf",
    "includes",
    "concat",
];

const TEXT_TAKING_FUNCTIONS: [&str; 2] = ["String", "RegExp"];

const ALIAS_DEPTH: usize = 8;

pub enum TextReads {
    Bindings(FxHashSet<SymbolId>),
    Unknown,
}

pub fn text_read_bindings(program: &Program<'_>, scoping: &Scoping) -> TextReads {
    let mut sinks = SinkFinder {
        scoping,
        unknown: false,
        functions: FxHashSet::default(),
        symbols: FxHashSet::default(),
        aliases: FxHashMap::default(),
        function_symbols: FxHashMap::default(),
    };
    sinks.visit_program(program);
    if sinks.unknown {
        return TextReads::Unknown;
    }
    let read = sinks.read_functions();
    let mut pins = PinWalker {
        scoping,
        read: &read,
        depth: 0,
        pinned: FxHashSet::default(),
    };
    pins.visit_program(program);
    TextReads::Bindings(pins.pinned)
}

struct SinkFinder<'s> {
    scoping: &'s Scoping,
    unknown: bool,
    functions: FxHashSet<Span>,
    symbols: FxHashSet<SymbolId>,
    aliases: FxHashMap<SymbolId, SymbolId>,
    function_symbols: FxHashMap<SymbolId, Span>,
}

fn function_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        Expression::ClassExpression(class) => Some(class.span),
        _ => None,
    }
}

fn is_self_reference(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::StaticMemberExpression(member) => {
            matches!(member.property.name.as_str(), "callee" | "caller")
        }
        _ => false,
    }
}

fn is_text_method_member(expr: &Expression<'_>) -> bool {
    member_parts(expr.without_parentheses())
        .is_some_and(|(_, name)| TEXT_METHODS.contains(&name.as_ref()))
}

impl SinkFinder<'_> {
    fn text_method_object(&mut self, object: &Expression<'_>) {
        if is_self_reference(object) {
            self.unknown = true;
        }
        self.sink(object);
    }

    fn sink(&mut self, expr: &Expression<'_>) {
        if let Some(span) = function_span(expr) {
            self.functions.insert(span);
            return;
        }
        match expr.without_parentheses() {
            Expression::Identifier(id) => {
                if let Some(symbol) = reference_symbol(id, self.scoping) {
                    self.symbols.insert(symbol);
                }
            }
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    self.sink(last);
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.sink(&conditional.consequent);
                self.sink(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) => {
                self.sink(&logical.left);
                self.sink(&logical.right);
            }
            Expression::AssignmentExpression(assignment) => self.sink(&assignment.right),
            _ => {}
        }
    }

    fn sink_argument(&mut self, argument: &Argument<'_>) {
        if let Some(expr) = argument.as_expression() {
            self.sink(expr);
        }
    }

    fn resolve(&self, symbol: SymbolId) -> Vec<SymbolId> {
        let mut chain = vec![symbol];
        let mut current = symbol;
        for _ in 0..ALIAS_DEPTH {
            let Some(&next) = self.aliases.get(&current) else {
                break;
            };
            chain.push(next);
            current = next;
        }
        chain
    }

    fn read_functions(&self) -> FxHashSet<Span> {
        let mut read = self.functions.clone();
        for &symbol in &self.symbols {
            for target in self.resolve(symbol) {
                if let Some(&span) = self.function_symbols.get(&target) {
                    read.insert(span);
                }
            }
        }
        read
    }

    fn bind(&mut self, symbol: Option<SymbolId>, init: &Expression<'_>) {
        let Some(symbol) = symbol else {
            return;
        };
        if let Some(span) = function_span(init) {
            self.function_symbols.insert(symbol, span);
            return;
        }
        if let Expression::Identifier(id) = init.without_parentheses() {
            if let Some(target) = reference_symbol(id, self.scoping) {
                self.aliases.insert(symbol, target);
            }
        }
    }
}

fn is_text_taking_call(callee: &Expression<'_>) -> bool {
    if let Expression::Identifier(id) = callee.without_parentheses() {
        return TEXT_TAKING_FUNCTIONS.contains(&id.name.as_str());
    }
    member_parts(callee.without_parentheses())
        .is_some_and(|(_, name)| TEXT_TAKING_METHODS.contains(&name.as_ref()))
}

impl<'a> VisitJs<'a> for SinkFinder<'_> {
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator == BinaryOperator::Addition {
            self.sink(&it.left);
            self.sink(&it.right);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == oxc_syntax::operator::AssignmentOperator::Addition {
            self.sink(&it.right);
        }
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            self.bind(reference_symbol(id, self.scoping), &it.right);
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        for expr in &it.expressions {
            self.sink(expr);
        }
        walk_js::walk_template_literal(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_text_taking_call(&it.callee) {
            for argument in &it.arguments {
                self.sink_argument(argument);
            }
        }
        if let Some((object, name)) = member_parts(it.callee.without_parentheses()) {
            let borrowed =
                matches!(name.as_ref(), "call" | "apply") && is_text_method_member(object);
            if borrowed {
                match it.arguments.first().and_then(Argument::as_expression) {
                    Some(first) => self.text_method_object(first),
                    None => self.unknown = true,
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if is_text_taking_call(&it.callee) {
            for argument in &it.arguments {
                self.sink_argument(argument);
            }
        }
        walk_js::walk_new_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if TEXT_METHODS.contains(&it.property.name.as_str()) {
            self.text_method_object(&it.object);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.bind(id.symbol_id.get(), init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            if it.is_declaration() {
                self.function_symbols.insert(symbol, it.span);
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            if it.is_declaration() {
                self.function_symbols.insert(symbol, it.span);
            }
        }
        walk_js::walk_class(self, it);
    }
}

struct PinWalker<'r> {
    scoping: &'r Scoping,
    read: &'r FxHashSet<Span>,
    depth: usize,
    pinned: FxHashSet<SymbolId>,
}

impl PinWalker<'_> {
    fn enter(&mut self, span: Span) -> bool {
        let read = self.read.contains(&span);
        if read {
            self.depth += 1;
        }
        read
    }

    fn leave(&mut self, entered: bool) {
        if entered {
            self.depth -= 1;
        }
    }
}

impl<'a> VisitJs<'a> for PinWalker<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.depth > 0 {
            if let Some(symbol) = reference_symbol(it, self.scoping) {
                self.pinned.insert(symbol);
            }
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.depth > 0 {
            if let Some(symbol) = it.symbol_id.get() {
                self.pinned.insert(symbol);
            }
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let entered = self.enter(it.span);
        walk_js::walk_function(self, it, flags);
        self.leave(entered);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let entered = self.enter(it.span);
        walk_js::walk_arrow_function_expression(self, it);
        self.leave(entered);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let entered = self.enter(it.span);
        walk_js::walk_class(self, it);
        self.leave(entered);
    }
}
