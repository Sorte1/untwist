use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use crate::mutants::states_class_reads_trusted;

const FIRST_STATE: &str = "\\w+ *\\(\\) *{\\w+ *";
const SECOND_STATE: &str = "['|\"].+['|\"];? *}";
const GLOBALS: [&str; 3] = ["RegExp", "Boolean", "Math"];
const VALUES: usize = 5;

fn word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn spaces(text: &[u8], mut at: usize) -> usize {
    while text.get(at) == Some(&b' ') {
        at += 1;
    }
    at
}

fn words(text: &[u8], start: usize) -> Option<usize> {
    let mut at = start;
    while text.get(at).copied().is_some_and(word) {
        at += 1;
    }
    (at > start).then_some(at)
}

fn quote(byte: u8) -> bool {
    matches!(byte, b'\'' | b'|' | b'"')
}

fn closes_at(text: &[u8], at: usize) -> bool {
    let at = if text.get(at) == Some(&b';') {
        at + 1
    } else {
        at
    };
    text.get(spaces(text, at)) == Some(&b'}')
}

fn matches_at(text: &[u8], start: usize) -> bool {
    let Some(at) = words(text, start) else {
        return false;
    };
    let at = spaces(text, at);
    if text.get(at..at + 2) != Some(b"()".as_slice()) {
        return false;
    }
    let at = spaces(text, at + 2);
    if text.get(at) != Some(&b'{') {
        return false;
    }
    let Some(at) = words(text, at + 1) else {
        return false;
    };
    let at = spaces(text, at);
    if !text.get(at).copied().is_some_and(quote) {
        return false;
    }
    let line_end = text[at + 1..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(text.len(), |offset| at + 1 + offset);
    (at + 2..line_end).any(|close| quote(text[close]) && closes_at(text, close + 1))
}

fn passes_the_state_test(text: &str) -> bool {
    let bytes = text.as_bytes();
    (0..bytes.len()).any(|start| matches_at(bytes, start))
}

fn resolve(scoping: &Scoping, id: &IdentifierReference<'_>) -> Option<Option<SymbolId>> {
    Some(scoping.get_reference(id.reference_id.get()?).symbol_id())
}

struct Contained<'s> {
    scoping: &'s Scoping,
    within: Span,
    sealed: bool,
}

impl<'a> VisitJs<'a> for Contained<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let ok = match resolve(self.scoping, it) {
            None => false,
            Some(None) => GLOBALS.contains(&it.name.as_str()),
            Some(Some(symbol)) => {
                let declared = self.scoping.symbol_span(symbol);
                declared.start >= self.within.start && declared.end <= self.within.end
            }
        };
        self.sealed &= ok;
    }
}

fn contained(function: &Function<'_>, scoping: &Scoping) -> bool {
    let mut scan = Contained {
        scoping,
        within: function.span,
        sealed: true,
    };
    scan.visit_function(function, ScopeFlags::Function);
    scan.sealed
}

fn new_state(expr: &Expression<'_>) -> Option<Span> {
    let Expression::FunctionExpression(function) = expr else {
        return None;
    };
    let body = function.body.as_ref()?;
    let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
        return None;
    };
    let Some(Expression::StringLiteral(text)) = returned.argument.as_ref() else {
        return None;
    };
    (function.params.items.is_empty() && text.value == "newState").then_some(function.span)
}

fn is_initial_states(expr: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = expr else {
        return false;
    };
    let values: Vec<Option<f64>> = array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::NumericLiteral(number) => Some(number.value),
            _ => None,
        })
        .collect();
    values == [Some(1.0), Some(0.0), Some(0.0)]
}

fn assignments<'k, 'a>(body: &'k FunctionBody<'a>) -> Option<Vec<&'k AssignmentExpression<'a>>> {
    let mut found = Vec::new();
    for stmt in &body.statements {
        let Statement::ExpressionStatement(node) = stmt else {
            return None;
        };
        let parts: Vec<&Expression<'a>> = match &node.expression {
            Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
            other => vec![other],
        };
        for part in parts {
            let Expression::AssignmentExpression(assigned) = part else {
                return None;
            };
            found.push(&**assigned);
        }
    }
    Some(found)
}

fn constructor_state(function: &Function<'_>, scoping: &Scoping) -> Option<Span> {
    let [parameter] = function.params.items.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(parameter) = &parameter.pattern else {
        return None;
    };
    let parameter = parameter.symbol_id.get()?;
    let found = assignments(function.body.as_ref()?)?;
    if found.len() != VALUES || !contained(function, scoping) {
        return None;
    }
    let mut seen = [false; VALUES];
    let mut state = None;
    for assigned in found {
        let member = assigned
            .left
            .as_simple_assignment_target()?
            .as_member_expression()?;
        if assigned.operator != AssignmentOperator::Assign
            || !matches!(member.object(), Expression::ThisExpression(_))
        {
            return None;
        }
        let slot = match &assigned.right {
            Expression::Identifier(id) if resolve(scoping, id)? == Some(parameter) => 0,
            Expression::StringLiteral(text) if text.value == FIRST_STATE => 1,
            Expression::StringLiteral(text) if text.value == SECOND_STATE => 2,
            other if is_initial_states(other) => 3,
            other => {
                state = Some(new_state(other)?);
                4
            }
        };
        if std::mem::replace(&mut seen[slot], true) {
            return None;
        }
    }
    state
}

fn prototype_method<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, &'k Function<'a>)> {
    let Expression::AssignmentExpression(assigned) = expr.without_parentheses() else {
        return None;
    };
    let member = assigned
        .left
        .as_simple_assignment_target()?
        .as_member_expression()?;
    member.static_property_name()?;
    let prototype = member
        .object()
        .without_parentheses()
        .as_member_expression()?;
    let Expression::FunctionExpression(method) = &assigned.right else {
        return None;
    };
    (assigned.operator == AssignmentOperator::Assign
        && prototype
            .static_property_name()
            .is_some_and(|name| name == "prototype"))
    .then_some((prototype.object().without_parentheses(), &**method))
}

fn names(expr: &Expression<'_>, scoping: &Scoping, class: SymbolId) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id)
        if resolve(scoping, id).flatten() == Some(class))
}

fn starts_the_check(expr: &Expression<'_>, scoping: &Scoping, class: SymbolId) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    let Some(member) = call.callee.without_parentheses().as_member_expression() else {
        return false;
    };
    let Expression::NewExpression(made) = member.object().without_parentheses() else {
        return false;
    };
    call.arguments.is_empty()
        && names(&made.callee, scoping, class)
        && made.arguments.iter().all(|argument| {
            matches!(
                argument,
                Argument::Identifier(_) | Argument::NumericLiteral(_) | Argument::StringLiteral(_)
            )
        })
}

struct Run {
    span: Span,
    class: SymbolId,
}

fn run_from(parts: &[Expression<'_>], scoping: &Scoping, source: &str) -> Option<(usize, Run)> {
    let (object, method) = prototype_method(&parts[0])?;
    let Expression::AssignmentExpression(class_assigned) = object else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &class_assigned.left else {
        return None;
    };
    let class = resolve(scoping, target)??;
    let Expression::FunctionExpression(constructor) = &class_assigned.right else {
        return None;
    };
    let state = constructor_state(constructor, scoping)?;
    if !passes_the_state_test(source.get(state.start as usize..state.end as usize)?) {
        return None;
    }
    let mut methods = vec![method];
    let mut at = 1;
    while let Some((object, method)) = parts.get(at).and_then(prototype_method) {
        if !names(object, scoping, class) {
            return None;
        }
        methods.push(method);
        at += 1;
    }
    let last = parts.get(at)?;
    if !starts_the_check(last, scoping, class)
        || !methods.iter().all(|method| contained(method, scoping))
    {
        return None;
    }
    let span = Span::new(parts[0].span().start, last.span().end);
    Some((at + 1, Run { span, class }))
}

struct Reads<'s> {
    scoping: &'s Scoping,
    class: SymbolId,
    inside: Span,
    home: Span,
    functions: Vec<Span>,
    loops: Vec<usize>,
    run_seen: bool,
    outside: bool,
}

impl Reads<'_> {
    fn at_home(&self) -> bool {
        self.functions.last() == Some(&self.home) && self.loops.last() == Some(&0)
    }

    fn enter_function(&mut self, span: Span) {
        self.functions.push(span);
        self.loops.push(0);
    }

    fn leave_function(&mut self) {
        self.functions.pop();
        self.loops.pop();
    }

    fn in_loop<F: FnOnce(&mut Self)>(&mut self, walk: F) {
        if let Some(depth) = self.loops.last_mut() {
            *depth += 1;
        }
        walk(self);
        if let Some(depth) = self.loops.last_mut() {
            *depth -= 1;
        }
    }
}

impl<'a> VisitJs<'a> for Reads<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if it.span().start == self.inside.start && !self.run_seen {
            self.run_seen = true;
            self.outside |= !self.at_home();
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.enter_function(it.span);
        walk_js::walk_function(self, it, flags);
        self.leave_function();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.enter_function(it.span);
        walk_js::walk_arrow_function_expression(self, it);
        self.leave_function();
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_for_statement(scan, it));
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_for_in_statement(scan, it));
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_for_of_statement(scan, it));
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_while_statement(scan, it));
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_do_while_statement(scan, it));
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.in_loop(|scan| walk_js::walk_labeled_statement(scan, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if reference.symbol_id() != Some(self.class) || reference.is_write() {
            return;
        }
        let within = self.inside.start <= it.span.start && it.span.end <= self.inside.end;
        let before = it.span.end <= self.inside.start && self.at_home();
        self.outside |= !within && !before;
    }
}

fn home_of(program: &Program<'_>, declared: Span) -> Option<Span> {
    struct Home {
        declared: Span,
        found: Option<Span>,
    }
    impl<'a> VisitJs<'a> for Home {
        fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
            if it.span.start <= self.declared.start && self.declared.end <= it.span.end {
                self.found = Some(it.span);
                walk_js::walk_function(self, it, flags);
            }
        }
    }
    let mut home = Home {
        declared,
        found: None,
    };
    home.visit_program(program);
    home.found
}

fn read_only_inside(program: &Program<'_>, scoping: &Scoping, run: &Run) -> bool {
    let declared = scoping.symbol_span(run.class);
    let Some(home) = home_of(program, declared) else {
        return false;
    };
    let mut reads = Reads {
        scoping,
        class: run.class,
        inside: run.span,
        home,
        functions: Vec::new(),
        loops: vec![0],
        run_seen: false,
        outside: false,
    };
    reads.visit_program(program);
    reads.run_seen && !reads.outside
}

struct Find<'s> {
    scoping: &'s Scoping,
    source: &'s str,
    runs: Vec<Run>,
}

impl<'a> VisitJs<'a> for Find<'_> {
    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        let parts = it.expressions.as_slice();
        let mut at = 0;
        while at < parts.len() {
            match run_from(&parts[at..], self.scoping, self.source) {
                Some((length, run)) if at + length < parts.len() || at > 0 => {
                    self.runs.push(run);
                    at += length;
                }
                _ => at += 1,
            }
        }
        walk_js::walk_sequence_expression(self, it);
    }
}

struct Drop {
    runs: Vec<Span>,
    removed: usize,
}

impl<'a> VisitMut<'a> for Drop {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let Expression::SequenceExpression(sequence) = it else {
            return;
        };
        let before = sequence.expressions.len();
        let runs = &self.runs;
        sequence.expressions.retain(|part| {
            let span = part.span();
            !runs
                .iter()
                .any(|run| run.start <= span.start && span.end <= run.end)
        });
        let removed = before - sequence.expressions.len();
        self.removed += usize::from(removed > 0);
        if removed > 0 && sequence.expressions.len() == 1 {
            let only = sequence.expressions.pop();
            if let Some(only) = only {
                *it = only;
            }
        }
    }
}

pub(super) fn drop_states_classes<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    if !program.source_text.contains("newState") {
        return 0;
    }
    let scoping = cx.scoping();
    let mut find = Find {
        scoping,
        source: program.source_text,
        runs: Vec::new(),
    };
    find.visit_program(program);
    let runs: Vec<Span> = find
        .runs
        .into_iter()
        .filter(|run| read_only_inside(program, scoping, run) || states_class_reads_trusted())
        .map(|run| run.span)
        .collect();
    if runs.is_empty() {
        return 0;
    }
    let mut drop = Drop { runs, removed: 0 };
    drop.visit_program(program);
    drop.removed
}

#[cfg(test)]
mod tests {
    use super::passes_the_state_test;

    #[test]
    fn the_state_test_matches_only_an_unformatted_new_state_function() {
        assert!(passes_the_state_test("function(){return\"newState\"}"));
        assert!(passes_the_state_test("function () {return 'newState';}"));
        assert!(!passes_the_state_test(
            "function () {\n  return \"newState\";\n}"
        ));
        assert!(!passes_the_state_test(
            "function () { return \"newState\"; }"
        ));
    }
}
