use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::exports::{
    context_import, declarator_export, export_call, is_export_call, value, Call, Value,
};
use crate::scan::{is_direct_eval, Frame, Policy, Scan as Walker, Step};

const HOST_NAMES: [&str; 6] = [
    "require",
    "module",
    "exports",
    "__filename",
    "__dirname",
    "System",
];

#[derive(Clone, Copy)]
pub enum UseKind {
    Write(Option<SymbolId>),
    Read(Option<SymbolId>),
    Declared(Option<SymbolId>),
    Fresh,
}

pub struct Use<'a> {
    pub name: &'a str,
    pub kind: UseKind,
    pub top: Option<usize>,
}

pub struct Params<'a> {
    pub export: Option<(&'a str, Option<SymbolId>)>,
    pub context: Option<(&'a str, Option<SymbolId>)>,
}

#[derive(Clone, Copy)]
enum Pending<'a> {
    Statement,
    Declared(&'a str, Option<SymbolId>),
}

#[derive(Clone, Copy, Default)]
struct Top {
    index: Option<usize>,
    statement: usize,
    declarators: (usize, usize),
}

pub struct Scan<'s, 'a> {
    pub scoping: &'s Scoping,
    pub params: Params<'a>,
    pub vars: &'s FxHashMap<SymbolId, Option<bool>>,
    pub namespaces: &'s [&'a str],
    pub names: &'s FxHashSet<&'a str>,
    pub inside: Span,
    pub uses: Vec<Use<'a>>,
    pub dynamic: Vec<&'a str>,
    pub refused: bool,
    top: Top,
    pending: Option<(usize, Option<usize>, Pending<'a>)>,
    callee: Option<Span>,
    bad_members: FxHashSet<Span>,
    member_reads: FxHashSet<Span>,
}

fn address<T>(it: &T) -> usize {
    std::ptr::from_ref(it) as usize
}

impl<'s, 'a> Scan<'s, 'a> {
    pub fn new(
        scoping: &'s Scoping,
        params: Params<'a>,
        (vars, namespaces): (&'s FxHashMap<SymbolId, Option<bool>>, &'s [&'a str]),
        names: &'s FxHashSet<&'a str>,
        inside: Span,
    ) -> Self {
        Self {
            scoping,
            params,
            vars,
            namespaces,
            names,
            inside,
            uses: Vec::new(),
            dynamic: Vec::new(),
            refused: false,
            top: Top::default(),
            pending: None,
            callee: None,
            bad_members: FxHashSet::default(),
            member_reads: FxHashSet::default(),
        }
    }

    pub fn execute(&mut self, body: &FunctionBody<'a>) {
        for (index, statement) in body.statements.iter().enumerate() {
            self.top = Top {
                index: Some(index),
                statement: match statement {
                    Statement::ExpressionStatement(it) => address(&**it),
                    _ => 0,
                },
                declarators: match statement {
                    Statement::VariableDeclaration(it) => {
                        let range = it.declarations.as_slice().as_ptr_range();
                        (range.start as usize, range.end as usize)
                    }
                    _ => (0, 0),
                },
            };
            Walker::new(self).visit_statement(statement);
        }
        self.top = Top::default();
    }

    fn export_name(&self) -> Option<&'a str> {
        self.params.export.map(|(name, _)| name)
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn record(&mut self, name: &'a str, expression: &Expression<'a>, top: Option<usize>) {
        let kind = match value(expression) {
            Value::Write(id) => UseKind::Write(self.symbol(id)),
            Value::Read(id) => UseKind::Read(self.symbol(id)),
            Value::Fresh => UseKind::Fresh,
            Value::Invalid => {
                self.refused = true;
                UseKind::Fresh
            }
        };
        self.uses.push(Use { name, kind, top });
    }

    fn statement_export<'k>(
        &mut self,
        call: &'k CallExpression<'a>,
        top: Option<usize>,
    ) -> Option<&'k Expression<'a>> {
        match export_call(call) {
            Some(Call::Single(name, value)) => {
                self.record(name, value, top);
                Some(value)
            }
            Some(Call::Object(pairs)) => {
                for (name, value) in pairs {
                    self.record(name, value, top);
                }
                call.arguments.first().and_then(Argument::as_expression)
            }
            None => {
                self.refused = true;
                None
            }
        }
    }

    fn context_call(&mut self, call: &CallExpression<'a>) -> bool {
        let context = self.params.context.map(|(name, _)| name);
        let Some(specifier) = context_import(call, context) else {
            return false;
        };
        match specifier {
            Some(name) if self.names.contains(name) => self.dynamic.push(name),
            Some(name) => self.refused |= !super::layout::is_plain(name),
            None => self.refused = true,
        }
        true
    }

    fn mark_bad(&mut self, expression: &Expression<'_>) {
        if !self.namespaces.is_empty() && expression.is_member_expression() {
            self.bad_members.insert(expression.span());
        }
    }

    fn member(&mut self, span: Span, object: &Expression<'_>, readable: bool) {
        let Expression::Identifier(id) = object else {
            return;
        };
        if readable
            && !self.bad_members.contains(&span)
            && self.namespaces.contains(&id.name.as_str())
        {
            self.member_reads.insert(id.span);
        }
    }

    fn import_reference(&mut self, symbol: SymbolId, it: &IdentifierReference<'_>) {
        let Some(&role) = self.vars.get(&symbol) else {
            self.refused = true;
            return;
        };
        let Some(namespace) = role else {
            return;
        };
        let write = it
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write());
        self.refused |= write || (namespace && !self.member_reads.contains(&it.span));
    }

    fn export_call(&mut self, it: &CallExpression<'a>) {
        let pending = self
            .pending
            .take_if(|(call, ..)| *call == address(it))
            .map(|(_, top, pending)| (top, pending));
        match pending {
            Some((top, Pending::Declared(name, symbol))) => self.uses.push(Use {
                name,
                kind: UseKind::Declared(symbol),
                top,
            }),
            Some((_, Pending::Statement)) | None => match export_call(it) {
                Some(Call::Single(name, value)) => self.record(name, value, None),
                _ => self.refused = true,
            },
        }
        self.callee = Some(it.callee.span());
    }
}

impl<'a> Policy<'a> for Scan<'_, 'a> {
    fn done(&self) -> bool {
        self.refused
    }

    fn item<'k>(&mut self, it: &'k Expression<'a>) -> &'k Expression<'a> {
        let Expression::CallExpression(call) = it else {
            return it;
        };
        let at = address(&**call);
        let pending = self
            .pending
            .take_if(|(call, _, pending)| *call == at && matches!(pending, Pending::Statement));
        match pending {
            Some((_, top, _)) => self.statement_export(call, top).unwrap_or(it),
            None => it,
        }
    }

    fn statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Expression::CallExpression(call) = &it.expression {
            if is_export_call(call, self.export_name()) {
                let top = (address(it) == self.top.statement)
                    .then_some(self.top.index)
                    .flatten();
                self.pending = Some((address(&**call), top, Pending::Statement));
            }
        }
    }

    fn declarator(&mut self, it: &VariableDeclarator<'a>) {
        let Some((id, name)) = declarator_export(it, self.export_name()) else {
            return;
        };
        let Some(Expression::CallExpression(call)) = &it.init else {
            return;
        };
        let (start, end) = self.top.declarators;
        let top = (start..end)
            .contains(&address(it))
            .then_some(self.top.index)
            .flatten();
        self.pending = Some((
            address(&**call),
            top,
            Pending::Declared(name, id.symbol_id.get()),
        ));
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        if is_export_call(it, self.export_name()) {
            self.export_call(it);
            return Step::Walk;
        }
        if self.context_call(it) {
            return Step::Skip;
        }
        self.refused |= is_direct_eval(self.scoping, &it.callee);
        self.mark_bad(&it.callee);
        Step::Walk
    }

    fn tagged(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.mark_bad(&it.tag);
    }

    fn target(&mut self, it: &SimpleAssignmentTarget<'a>) -> Step {
        if !self.namespaces.is_empty() {
            if let Some(member) = it.as_member_expression() {
                self.bad_members.insert(member.span());
            }
        }
        Step::Walk
    }

    fn unary(&mut self, it: &UnaryExpression<'a>) -> Step {
        if it.operator == UnaryOperator::Delete {
            self.mark_bad(&it.argument);
        }
        Step::Walk
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'a>) -> Step {
        self.member(it.span, &it.object, true);
        Step::Walk
    }

    fn computed_member(&mut self, it: &ComputedMemberExpression<'a>) {
        let literal = matches!(it.expression, Expression::StringLiteral(_));
        self.member(it.span, &it.object, literal);
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        if self.callee.take_if(|callee| *callee == it.span).is_some() {
            return;
        }
        let Some(symbol) = self.symbol(it) else {
            let name = it.name.as_str();
            self.refused |= HOST_NAMES.contains(&name)
                || name == "await"
                || (name == "arguments" && !frame.receiver());
            return;
        };
        let declared = self.scoping.symbol_span(symbol);
        if self.inside.start <= declared.start && declared.end <= self.inside.end {
            return;
        }
        self.import_reference(symbol, it);
    }

    fn binding(&mut self, it: &BindingIdentifier<'a>) {
        let name = Some(it.name.as_str());
        let param = |p: Option<(&str, Option<SymbolId>)>| p.map(|(n, _)| n) == name;
        self.refused |=
            param(self.params.export) || param(self.params.context) || it.name == "await";
    }

    fn label(&mut self, it: &LabelIdentifier<'a>) {
        self.refused |= it.name == "await";
    }

    fn this(&mut self, frame: Frame) {
        self.refused |= !frame.receiver();
    }

    fn import_meta(&mut self) {
        self.refused = true;
    }

    fn new_target(&mut self, frame: Frame) {
        self.refused |= !frame.receiver();
    }

    fn super_(&mut self, frame: Frame) {
        self.refused |= !frame.receiver();
    }

    fn dynamic_import(&mut self) {
        self.refused = true;
    }

    fn returns(&mut self, frame: Frame) {
        self.refused |= !frame.deferred();
    }

    fn with(&mut self) {
        self.refused = true;
    }
}
