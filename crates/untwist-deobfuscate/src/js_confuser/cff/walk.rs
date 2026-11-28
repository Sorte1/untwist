use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::resolution::Spans;
use super::shape::Main;
use super::value::{StateEnv, Val};

const MAX_STEPS: usize = 200_000;
const MAX_NODES: usize = 20_000;

pub(super) enum Term {
    Goto(Vec<f64>),
    Return(Span, usize),
    Fork(Span, usize, Box<Seq>, Box<Seq>),
}

pub(super) struct Seq {
    pub(super) items: Vec<(Span, usize)>,
    pub(super) term: Term,
}

pub(super) enum NodeTerm {
    Goto(usize),
    Cond(Span, usize, usize, usize),
    Ret(Span, usize),
    Exit,
}

pub(super) struct Node {
    pub(super) items: Vec<(Span, usize)>,
    pub(super) term: NodeTerm,
}

pub(super) struct Graph {
    pub(super) nodes: Vec<Node>,
    pub(super) entry: usize,
}

pub(super) struct Walker<'m, 'k, 'a> {
    pub(super) main: &'m Main<'k, 'a>,
    pub(super) scoping: &'m Scoping,
    pub(super) spans: &'m Spans,
    pub(super) sum: SymbolId,
    pub(super) vectors: Vec<Vec<f64>>,
    interned: FxHashMap<Vec<u64>, usize>,
    steps: usize,
}

#[derive(Default)]
struct Probe {
    labels: Vec<String>,
    loops: usize,
    switches: usize,
    functions: usize,
    escapes: bool,
    nested_states: bool,
    writes_states: bool,
    reads_this: bool,
    states: Option<SymbolId>,
}

impl Probe {
    fn is_states(&self, scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|r| scoping.get_reference(r).symbol_id())
            .is_some_and(|s| Some(s) == self.states)
    }
}

struct ProbeVisit<'s> {
    scoping: &'s Scoping,
    probe: Probe,
}

impl<'a> VisitJs<'a> for ProbeVisit<'_> {
    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        if self.probe.functions > 0 {
            return;
        }
        match &it.label {
            Some(label) => {
                let name = label.name.as_str();
                if !self.probe.labels.iter().any(|l| l == name) {
                    self.probe.escapes = true;
                }
            }
            None => {
                if self.probe.loops == 0 && self.probe.switches == 0 {
                    self.probe.escapes = true;
                }
            }
        }
    }

    fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
        if self.probe.functions > 0 {
            return;
        }
        let known = it
            .label
            .as_ref()
            .is_none_or(|label| self.probe.labels.iter().any(|l| l == label.name.as_str()));
        if self.probe.loops == 0 || !known {
            self.probe.escapes = true;
        }
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.probe.labels.push(it.label.name.to_string());
        walk_js::walk_labeled_statement(self, it);
        self.probe.labels.pop();
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let looped = matches!(
            it,
            Statement::ForStatement(_)
                | Statement::ForInStatement(_)
                | Statement::ForOfStatement(_)
                | Statement::WhileStatement(_)
                | Statement::DoWhileStatement(_)
        );
        let switched = matches!(it, Statement::SwitchStatement(_));
        self.probe.loops += usize::from(looped);
        self.probe.switches += usize::from(switched);
        walk_js::walk_statement(self, it);
        self.probe.loops -= usize::from(looped);
        self.probe.switches -= usize::from(switched);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.probe.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.probe.functions -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.probe.functions += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.probe.functions -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.probe.functions += 1;
        walk_js::walk_class(self, it);
        self.probe.functions -= 1;
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        if self.probe.functions == 0 {
            self.probe.reads_this = true;
        }
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        if self.probe.functions == 0 {
            self.probe.reads_this = true;
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" && self.probe.functions == 0 {
            self.probe.reads_this = true;
        }
        if !self.probe.is_states(self.scoping, it) {
            return;
        }
        if self.probe.functions > 0 {
            self.probe.nested_states = true;
        }
        let write = it
            .reference_id
            .get()
            .is_some_and(|r| self.scoping.get_reference(r).is_write());
        self.probe.writes_states |= write;
    }
}

fn branch<'k, 'a>(statement: Option<&'k Statement<'a>>) -> Option<Vec<&'k Statement<'a>>> {
    let Some(statement) = statement else {
        return Some(Vec::new());
    };
    match statement {
        Statement::BlockStatement(block) => {
            let lexical = block.body.iter().any(|s| {
                matches!(s, Statement::FunctionDeclaration(_) | Statement::ClassDeclaration(_))
                    || matches!(s, Statement::VariableDeclaration(d) if d.kind != VariableDeclarationKind::Var)
            });
            (!lexical).then(|| block.body.iter().collect())
        }
        Statement::FunctionDeclaration(_) | Statement::ClassDeclaration(_) => None,
        Statement::VariableDeclaration(d) if d.kind != VariableDeclarationKind::Var => None,
        other => Some(vec![other]),
    }
}

fn state_index(env: &StateEnv<'_>, target: &AssignmentTarget<'_>) -> Option<usize> {
    let AssignmentTarget::ComputedMemberExpression(member) = target else {
        return None;
    };
    env.is_states(&member.object)
        .then(|| env.index(&member.expression))
        .flatten()
}

impl<'m, 'k, 'a> Walker<'m, 'k, 'a> {
    pub(super) fn new(
        main: &'m Main<'k, 'a>,
        (scoping, spans): (&'m Scoping, &'m Spans),
        sum: SymbolId,
    ) -> Self {
        Walker {
            main,
            scoping,
            spans,
            sum,
            vectors: Vec::new(),
            interned: FxHashMap::default(),
            steps: 0,
        }
    }

    fn env<'v>(&self, values: &'v [f64]) -> StateEnv<'v>
    where
        'm: 'v,
    {
        StateEnv {
            spans: self.spans,
            states: self.main.states,
            sum: self.sum,
            values,
        }
    }

    fn intern(&mut self, values: &[f64]) -> usize {
        let key: Vec<u64> = values.iter().map(|v| v.to_bits()).collect();
        if let Some(found) = self.interned.get(&key) {
            return *found;
        }
        self.vectors.push(values.to_vec());
        let id = self.vectors.len() - 1;
        self.interned.insert(key, id);
        id
    }

    fn probe(&self, statement: &Statement<'a>) -> Probe {
        let mut visit = ProbeVisit {
            scoping: self.scoping,
            probe: Probe {
                states: Some(self.main.states),
                ..Probe::default()
            },
        };
        visit.visit_statement(statement);
        visit.probe
    }

    fn update(&self, statement: &Statement<'a>, values: &mut [f64]) -> Option<bool> {
        let Statement::ExpressionStatement(statement) = statement else {
            return Some(false);
        };
        let parts: Vec<&Expression<'a>> = match &statement.expression {
            Expression::SequenceExpression(seq) => seq.expressions.iter().collect(),
            other => vec![other],
        };
        let assigns: Vec<&AssignmentExpression<'a>> = parts
            .iter()
            .filter_map(|p| match p {
                Expression::AssignmentExpression(a) => Some(&**a),
                _ => None,
            })
            .collect();
        if assigns.len() != parts.len() {
            return Some(false);
        }
        let any_state = assigns
            .iter()
            .any(|a| matches!(&a.left, AssignmentTarget::ComputedMemberExpression(m) if self.env(values).is_states(&m.object)));
        if !any_state {
            return Some(false);
        }
        for assign in assigns {
            let at = state_index(&self.env(values), &assign.left)?;
            let (value, _) = self.env(values).eval(&assign.right)?;
            let value = value.num()?;
            values[at] = match assign.operator {
                AssignmentOperator::Assign => value,
                AssignmentOperator::Addition => values[at] + value,
                AssignmentOperator::Subtraction => values[at] - value,
                _ => return None,
            };
        }
        Some(true)
    }

    fn switch_break(&self, it: &BreakStatement<'a>) -> bool {
        match (&it.label, &self.main.label) {
            (None, _) => true,
            (Some(label), Some(own)) => label.name.as_str() == own,
            (Some(_), None) => false,
        }
    }

    fn exec(&mut self, start: Vec<&'k Statement<'a>>, values: Vec<f64>) -> Option<Seq> {
        let mut list = start;
        let mut values = values;
        let mut at = 0;
        let mut items = Vec::new();
        let mut updated = false;
        while let Some(statement) = list.get(at).copied() {
            self.steps += 1;
            if self.steps > MAX_STEPS {
                return None;
            }
            if self.update(statement, &mut values)? {
                updated = true;
                at += 1;
                continue;
            }
            match statement {
                Statement::BreakStatement(it) if self.switch_break(it) => {
                    return Some(Seq {
                        items,
                        term: Term::Goto(values),
                    });
                }
                Statement::ReturnStatement(it) if !updated => {
                    let id = self.intern(&values);
                    return Some(Seq {
                        items,
                        term: Term::Return(it.span, id),
                    });
                }
                Statement::IfStatement(it) => {
                    if let Some((value, true)) = self.env(&values).eval(&it.test) {
                        let taken = match value {
                            Val::Bool(b) => b,
                            Val::Num(n) => n != 0.0 && !n.is_nan(),
                        };
                        let chosen = if taken {
                            Some(&it.consequent)
                        } else {
                            it.alternate.as_ref()
                        };
                        let mut next = branch(chosen)?;
                        next.extend(list[at + 1..].iter().copied());
                        list = next;
                        at = 0;
                        continue;
                    }
                    let probe = self.probe(statement);
                    if probe.escapes || probe.writes_states {
                        if updated || probe.nested_states || probe.reads_this {
                            return None;
                        }
                        let rest: Vec<&'k Statement<'a>> = list[at + 1..].to_vec();
                        let mut then = branch(Some(&it.consequent))?;
                        then.extend(rest.iter().copied());
                        let mut other = branch(it.alternate.as_ref())?;
                        other.extend(rest);
                        let id = self.intern(&values);
                        let then = self.exec(then, values.clone())?;
                        let other = self.exec(other, values)?;
                        return Some(Seq {
                            items,
                            term: Term::Fork(it.span, id, Box::new(then), Box::new(other)),
                        });
                    }
                    self.plain(statement, &probe, updated, &values, &mut items)?;
                }
                _ => {
                    let probe = self.probe(statement);
                    self.plain(statement, &probe, updated, &values, &mut items)?;
                }
            }
            at += 1;
        }
        None
    }

    fn plain(
        &mut self,
        statement: &Statement<'a>,
        probe: &Probe,
        updated: bool,
        values: &[f64],
        items: &mut Vec<(Span, usize)>,
    ) -> Option<()> {
        if updated
            || probe.escapes
            || probe.writes_states
            || probe.nested_states
            || probe.reads_this
        {
            return None;
        }
        let id = self.intern(values);
        items.push((statement.span(), id));
        Some(())
    }

    fn find_case(&self, values: &[f64]) -> Option<usize> {
        let sum: f64 = values.iter().sum();
        let mut default = None;
        for (index, case) in self.main.cases.iter().enumerate() {
            let Some(test) = &case.test else {
                default = Some(index);
                continue;
            };
            let (value, _) = self.env(values).eval(test)?;
            if value == Val::Num(sum) {
                return Some(index);
            }
        }
        default
    }

    fn block(&mut self, values: Vec<f64>) -> Option<Seq> {
        let case = self.find_case(&values)?;
        let list: Vec<&'k Statement<'a>> = self.main.cases[case..]
            .iter()
            .flat_map(|c| c.consequent.iter())
            .collect();
        self.exec(list, values)
    }

    pub(super) fn graph(&mut self, entry: Vec<f64>) -> Option<Graph> {
        let mut nodes: Vec<Node> = vec![Node {
            items: Vec::new(),
            term: NodeTerm::Exit,
        }];
        let mut blocks: FxHashMap<Vec<u64>, usize> = FxHashMap::default();
        let mut pending: Vec<(usize, Vec<f64>)> = Vec::new();
        let entry_node = self.target(&entry, &mut nodes, &mut blocks, &mut pending);
        while let Some((id, values)) = pending.pop() {
            if nodes.len() > MAX_NODES {
                return None;
            }
            let seq = self.block(values)?;
            let node = self.lower(seq, &mut nodes, &mut blocks, &mut pending);
            nodes[id] = node;
        }
        Some(Graph {
            nodes,
            entry: entry_node,
        })
    }

    fn target(
        &self,
        values: &[f64],
        nodes: &mut Vec<Node>,
        blocks: &mut FxHashMap<Vec<u64>, usize>,
        pending: &mut Vec<(usize, Vec<f64>)>,
    ) -> usize {
        if values.iter().sum::<f64>() == self.main.end {
            return 0;
        }
        let key: Vec<u64> = values.iter().map(|v| v.to_bits()).collect();
        if let Some(found) = blocks.get(&key) {
            return *found;
        }
        nodes.push(Node {
            items: Vec::new(),
            term: NodeTerm::Exit,
        });
        let id = nodes.len() - 1;
        blocks.insert(key, id);
        pending.push((id, values.to_vec()));
        id
    }

    fn lower(
        &self,
        seq: Seq,
        nodes: &mut Vec<Node>,
        blocks: &mut FxHashMap<Vec<u64>, usize>,
        pending: &mut Vec<(usize, Vec<f64>)>,
    ) -> Node {
        let term = match seq.term {
            Term::Goto(values) => NodeTerm::Goto(self.target(&values, nodes, blocks, pending)),
            Term::Return(span, id) => NodeTerm::Ret(span, id),
            Term::Fork(span, id, then, other) => {
                let then = self.lower(*then, nodes, blocks, pending);
                nodes.push(then);
                let a = nodes.len() - 1;
                let other = self.lower(*other, nodes, blocks, pending);
                nodes.push(other);
                let b = nodes.len() - 1;
                NodeTerm::Cond(span, id, a, b)
            }
        };
        Node {
            items: seq.items,
            term,
        }
    }
}
