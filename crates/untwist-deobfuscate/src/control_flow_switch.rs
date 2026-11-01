use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::UpdateOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::{call_parts, infinite_loop_body};
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::control_flow_object::single_declarator;
use crate::notes;

pub const PASS: &str = "control-flow-switch";

const PRELUDE_REFERENCES: usize = 1;

const PRELUDE_STATEMENTS: usize = 3;

fn is_digit_sequence(text: &str) -> bool {
    !text.is_empty()
        && text
            .split('|')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn sequence_declaration<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<(&'k BindingIdentifier<'a>, Vec<String>)> {
    let (id, init) = single_declarator(stmt)?;
    let (callee, arguments) = call_parts(init?)?;
    let (object, name) = member_parts(callee)?;
    if name != "split" || arguments.len() != 1 {
        return None;
    }
    let Expression::StringLiteral(text) = object else {
        return None;
    };
    let Some(Expression::StringLiteral(separator)) = arguments.first()?.as_expression() else {
        return None;
    };
    if separator.value.as_str() != "|" || !is_digit_sequence(text.value.as_str()) {
        return None;
    }
    Some((
        id,
        text.value.as_str().split('|').map(str::to_string).collect(),
    ))
}

fn iterator_declaration<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k BindingIdentifier<'a>> {
    let (id, _) = single_declarator(stmt)?;
    Some(id)
}

fn bare_infinite_loop_body<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Statement<'a>> {
    if let Statement::ForStatement(node) = stmt {
        if node.init.is_some() || node.update.is_some() {
            return None;
        }
    }
    infinite_loop_body(stmt)
}

fn dispatch_switch<'k, 'a>(
    stmt: &'k Statement<'a>,
    sequence: &str,
    iterator: &str,
) -> Option<&'k SwitchStatement<'a>> {
    let Statement::BlockStatement(block) = bare_infinite_loop_body(stmt)? else {
        return None;
    };
    let [Statement::SwitchStatement(switch), Statement::BreakStatement(brk)] =
        block.body.as_slice()
    else {
        return None;
    };
    if brk.label.is_some() {
        return None;
    }
    let Expression::ComputedMemberExpression(member) = &switch.discriminant else {
        return None;
    };
    if member.optional || identifier_name(&member.object) != Some(sequence) {
        return None;
    }
    let Expression::UpdateExpression(update) = &member.expression else {
        return None;
    };
    if update.operator != UpdateOperator::Increment || update.prefix {
        return None;
    }
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &update.argument else {
        return None;
    };
    (target.name.as_str() == iterator).then_some(&**switch)
}

struct Prelude<'k, 'a> {
    switch: &'k SwitchStatement<'a>,
    sequence: Vec<String>,
    bindings: [SymbolId; 2],
}

fn prelude_switch<'k, 'a>(stmts: &'k [Statement<'a>]) -> Option<Prelude<'k, 'a>> {
    let [first, second, third, ..] = stmts else {
        return None;
    };
    let (sequence_id, sequence) = sequence_declaration(first)?;
    let iterator_id = iterator_declaration(second)?;
    let switch = dispatch_switch(third, sequence_id.name.as_str(), iterator_id.name.as_str())?;
    Some(Prelude {
        switch,
        sequence,
        bindings: [sequence_id.symbol_id.get()?, iterator_id.symbol_id.get()?],
    })
}

fn bindings_are_private(bindings: &[SymbolId; 2], scoping: &Scoping) -> bool {
    bindings
        .iter()
        .all(|symbol| scoping.get_resolved_reference_ids(*symbol).len() == PRELUDE_REFERENCES)
}

fn case_label(case: &SwitchCase<'_>) -> Option<String> {
    let Some(Expression::StringLiteral(literal)) = case.test.as_ref() else {
        return None;
    };
    let text = literal.value.as_str();
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(text.to_string())
}

fn case_labels(switch: &SwitchStatement<'_>) -> Option<Vec<String>> {
    let mut seen = FxHashSet::default();
    let mut labels = Vec::with_capacity(switch.cases.len());
    for case in &switch.cases {
        let label = case_label(case)?;
        if !seen.insert(label.clone()) {
            return None;
        }
        labels.push(label);
    }
    Some(labels)
}

fn sequence_mismatch(labels: &[String], sequence: &[String]) -> Option<String> {
    let mut placed: FxHashSet<&str> = FxHashSet::default();
    for value in sequence {
        if !labels.iter().any(|label| label == value) || !placed.insert(value.as_str()) {
            return Some(value.clone());
        }
    }
    labels
        .iter()
        .find(|label| !placed.contains(label.as_str()))
        .cloned()
}

pub fn match_switch_prelude(stmts: &[Statement<'_>]) -> Option<Vec<String>> {
    let prelude = prelude_switch(stmts)?;
    let labels = case_labels(prelude.switch)?;
    sequence_mismatch(&labels, &prelude.sequence)
        .is_none()
        .then_some(prelude.sequence)
}

#[derive(Default)]
struct CaseExits {
    loops: u32,
    switches: u32,
    labels: Vec<String>,
    found: bool,
}

impl CaseExits {
    fn jump(&mut self, label: Option<&LabelIdentifier<'_>>, is_break: bool) {
        let escapes = match label {
            Some(label) => !self.labels.iter().any(|name| name == label.name.as_str()),
            None if is_break => self.loops == 0 && self.switches == 0,
            None => self.loops == 0,
        };
        self.found = self.found || escapes;
    }
}

impl<'a> VisitJs<'a> for CaseExits {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}

    fn visit_statement(&mut self, it: &Statement<'a>) {
        match it {
            Statement::BreakStatement(node) => self.jump(node.label.as_ref(), true),
            Statement::ContinueStatement(node) => self.jump(node.label.as_ref(), false),
            Statement::LabeledStatement(node) => {
                self.labels.push(node.label.name.as_str().to_string());
                walk_js::walk_statement(self, it);
                self.labels.pop();
            }
            Statement::SwitchStatement(_) => {
                self.switches += 1;
                walk_js::walk_statement(self, it);
                self.switches -= 1;
            }
            Statement::ForStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_) => {
                self.loops += 1;
                walk_js::walk_statement(self, it);
                self.loops -= 1;
            }
            _ => walk_js::walk_statement(self, it),
        }
    }
}

fn exits_the_dispatch(body: &[Statement<'_>]) -> bool {
    let scanned = match body.last() {
        Some(Statement::ContinueStatement(node)) if node.label.is_none() => &body[..body.len() - 1],
        _ => body,
    };
    let mut exits = CaseExits::default();
    for stmt in scanned {
        exits.visit_statement(stmt);
    }
    exits.found
}

fn exiting_case<'k, 'a>(switch: &'k SwitchStatement<'a>) -> Option<&'k SwitchCase<'a>> {
    switch
        .cases
        .iter()
        .find(|case| exits_the_dispatch(&case.consequent))
}

fn terminates(body: &[Statement<'_>]) -> bool {
    matches!(
        body.last(),
        Some(
            Statement::BreakStatement(_)
                | Statement::ReturnStatement(_)
                | Statement::ContinueStatement(_)
                | Statement::ThrowStatement(_)
        )
    )
}

fn falling_through_case<'k, 'a>(
    switch: &'k SwitchStatement<'a>,
    sequence: &[String],
) -> Option<&'k SwitchCase<'a>> {
    for (index, case) in switch.cases.iter().enumerate() {
        if terminates(&case.consequent) {
            continue;
        }
        if switch.cases.get(index + 1).is_some() {
            return Some(case);
        }
        let label = case_label(case)?;
        let Some(position) = sequence.iter().position(|value| *value == label) else {
            return Some(case);
        };
        if position + 1 != sequence.len() {
            return Some(case);
        }
    }
    None
}

pub fn fall_through_case<'k, 'a>(
    stmts: &'k [Statement<'a>],
    sequence: &[String],
) -> Option<&'k SwitchCase<'a>> {
    falling_through_case(prelude_switch(stmts)?.switch, sequence)
}

fn lexical_names<'k, 'a>(stmt: &'k Statement<'a>) -> Vec<(&'k str, Span)> {
    match stmt {
        Statement::VariableDeclaration(declaration) if declaration.kind.is_lexical() => declaration
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .map(|id| (id.name.as_str(), id.span))
            .collect(),
        Statement::ClassDeclaration(class) => class
            .id
            .as_ref()
            .map(|id| (id.name.as_str(), id.span))
            .into_iter()
            .collect(),
        Statement::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .map(|id| (id.name.as_str(), id.span))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn shadowed_declaration(switch: &SwitchStatement<'_>, scoping: &Scoping) -> Option<Span> {
    let scope = switch.scope_id.get()?;
    switch
        .cases
        .iter()
        .flat_map(|case| case.consequent.iter())
        .flat_map(lexical_names)
        .find(|(name, _)| {
            scoping
                .scope_ancestors(scope)
                .skip(1)
                .any(|ancestor| scoping.get_binding(ancestor, Ident::from(*name)).is_some())
        })
        .map(|(_, span)| span)
}

fn plan(stmts: &[Statement<'_>], cx: &mut Cx<'_, '_>) -> Option<Vec<String>> {
    let prelude = prelude_switch(stmts)?;
    let switch = prelude.switch;
    let labels = case_labels(switch)?;
    if !bindings_are_private(&prelude.bindings, cx.scoping()) {
        return None;
    }
    if let Some(case) = exiting_case(switch) {
        cx.note(notes::SWITCH_CASE_EXIT, Some(case.span));
        return None;
    }
    if let Some(span) = shadowed_declaration(switch, cx.scoping()) {
        cx.note(notes::SWITCH_CASE_SHADOW, Some(span));
        return None;
    }
    if let Some(label) = sequence_mismatch(&labels, &prelude.sequence) {
        cx.note(notes::switch_sequence(&label), Some(switch.span));
        return None;
    }
    if let Some(case) = falling_through_case(switch, &prelude.sequence) {
        cx.note(notes::SWITCH_FALLTHROUGH, Some(case.span));
        return None;
    }
    Some(prelude.sequence)
}

fn switch_in_loop<'p>(stmt: &'p Statement<'p>) -> Option<&'p SwitchStatement<'p>> {
    let body = match stmt {
        Statement::WhileStatement(node) => &node.body,
        Statement::ForStatement(node) => &node.body,
        _ => return None,
    };
    let Statement::BlockStatement(block) = body else {
        return None;
    };
    match block.body.first() {
        Some(Statement::SwitchStatement(switch)) => Some(switch),
        _ => None,
    }
}

fn take_switch<'a>(stmt: Statement<'a>) -> Option<SwitchStatement<'a>> {
    let body = match stmt {
        Statement::WhileStatement(node) => node.unbox().body,
        Statement::ForStatement(node) => node.unbox().body,
        _ => return None,
    };
    let Statement::BlockStatement(block) = body else {
        return None;
    };
    match block.unbox().body.into_iter().next() {
        Some(Statement::SwitchStatement(switch)) => Some(switch.unbox()),
        _ => None,
    }
}

fn case_order(switch: &SwitchStatement<'_>, sequence: &[String]) -> Option<Vec<usize>> {
    let labels = case_labels(switch)?;
    sequence
        .iter()
        .map(|label| labels.iter().position(|key| key == label))
        .collect()
}

fn drop_trailing_continue<'a>(mut body: AVec<'a, Statement<'a>>) -> AVec<'a, Statement<'a>> {
    if matches!(body.last(), Some(Statement::ContinueStatement(node)) if node.label.is_none()) {
        body.pop();
    }
    body
}

fn take_case_bodies<'a>(
    switch: SwitchStatement<'a>,
    cx: &Cx<'a, '_>,
) -> Vec<AVec<'a, Statement<'a>>> {
    switch
        .cases
        .into_iter()
        .map(|mut case| {
            let body = std::mem::replace(&mut case.consequent, AVec::new_in(cx));
            drop_trailing_continue(body)
        })
        .collect()
}

fn commit<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    sequence: &[String],
    cx: &mut Cx<'a, '_>,
) -> usize {
    let refused = notes::planned_and_applied_edits(PASS, sequence.len(), 0);
    let Some(order) = stmts
        .get(PRELUDE_STATEMENTS - 1)
        .and_then(switch_in_loop)
        .and_then(|switch| case_order(switch, sequence))
    else {
        cx.note(refused, None);
        return 0;
    };
    let placeholder = Statement::new_empty_statement(SPAN, &*cx);
    let Some(switch) = stmts
        .get_mut(PRELUDE_STATEMENTS - 1)
        .map(|slot| std::mem::replace(slot, placeholder))
        .and_then(take_switch)
    else {
        cx.note(refused, None);
        return 0;
    };
    let mut bodies = take_case_bodies(switch, &*cx);
    let mut taken: Vec<Statement<'a>> = stmts.drain(..).collect();
    let rest = taken.split_off(PRELUDE_STATEMENTS);
    let mut out: AVec<'a, Statement<'a>> = AVec::new_in(&*cx);
    let mut moved = 0usize;
    for at in order {
        let Some(body) = bodies.get_mut(at) else {
            continue;
        };
        let body = std::mem::replace(body, AVec::new_in(&*cx));
        moved += body.len();
        out.extend(body);
    }
    out.extend(rest);
    *stmts = out;
    moved + PRELUDE_STATEMENTS
}

fn flatten<'a>(stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) -> usize {
    let Some(sequence) = plan(stmts, cx) else {
        return 0;
    };
    commit(stmts, &sequence, cx)
}

#[derive(Default)]
pub struct ControlFlowSwitch;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(ControlFlowSwitch)
}

impl<'a> Pass<'a> for ControlFlowSwitch {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENTS
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        let changes = flatten(stmts, cx);
        cx.changed_by(changes);
    }
}
