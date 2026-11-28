use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitMut;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::respan::fresh_span;

use super::super::resolution::{record_expression, record_statement, Expected, Spans};
use super::rewrite::Rewriter;
use super::structure::Out;
use super::value::{symbol_at, xor_decode, StateEnv};

const PROTOTYPE_KEYS: [&str; 12] = [
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

const RESERVED: [&str; 12] = [
    "arguments",
    "eval",
    "undefined",
    "let",
    "static",
    "yield",
    "await",
    "enum",
    "implements",
    "package",
    "protected",
    "interface",
];

pub(super) struct Syms {
    pub(super) main: SymbolId,
    pub(super) states: SymbolId,
    pub(super) scope: SymbolId,
    pub(super) runtime: SymbolId,
    pub(super) arg: Option<SymbolId>,
    pub(super) sum: SymbolId,
    pub(super) slice: Option<SymbolId>,
    pub(super) xor: Option<SymbolId>,
    pub(super) entry: Vec<SymbolId>,
}

#[derive(Clone, Copy)]
pub(super) enum Context {
    Program,
    Function(SymbolId),
}

pub(super) struct Wrap {
    pub(super) rest: String,
    pub(super) rest_symbol: SymbolId,
    pub(super) fresh: String,
    pub(super) out: Vec<Out>,
}

pub(super) struct Plan {
    pub(super) body: Span,
    pub(super) main_statement: Span,
    pub(super) entry: Vec<Span>,
    pub(super) context: Context,
    pub(super) roots: Vec<String>,
    pub(super) out: Vec<Out>,
    pub(super) wrappers: FxHashMap<Span, Wrap>,
    pub(super) vectors: Vec<Vec<f64>>,
    pub(super) syms: Syms,
}

#[derive(Default)]
pub(super) struct Lifted {
    pub(super) used: FxHashSet<String>,
    map: FxHashMap<(String, String), (String, Span)>,
    by_scope: FxHashMap<String, Vec<(String, Span)>>,
    pub(super) created: Vec<(Span, Span)>,
}

fn identifier_like(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !RESERVED.contains(&name)
        && !oxc_syntax::keyword::is_reserved_keyword(name)
        && !untwist_core::semantic::is_shared_global(name)
        && !untwist_core::semantic::is_foreign_global(name)
}

impl Lifted {
    pub(super) fn with_used(used: FxHashSet<String>) -> Self {
        Lifted {
            used,
            ..Lifted::default()
        }
    }

    fn name(&mut self, scope: &str, prop: &str) -> Option<(String, Span)> {
        if PROTOTYPE_KEYS.contains(&prop) || PROTOTYPE_KEYS.contains(&scope) {
            return None;
        }
        let key = (scope.to_string(), prop.to_string());
        if let Some(found) = self.map.get(&key) {
            return Some(found.clone());
        }
        let base = if identifier_like(prop) {
            prop.to_string()
        } else {
            "v".to_string()
        };
        let mut name = base.clone();
        let mut counter = 1;
        while self.used.contains(&name) {
            counter += 1;
            name = format!("{base}_{counter}");
        }
        self.used.insert(name.clone());
        let declared = fresh_span()?;
        self.map.insert(key, (name.clone(), declared));
        self.by_scope
            .entry(scope.to_string())
            .or_default()
            .push((name.clone(), declared));
        Some((name, declared))
    }
}

pub(super) enum Built<'a> {
    Stmt(Statement<'a>, Vec<Span>),
    Init(String),
    If(Expression<'a>, Vec<Built<'a>>, Vec<Built<'a>>, Vec<Span>),
}

fn mentions(item: &Built<'_>, declared: Span) -> usize {
    let own = |refs: &[Span]| refs.iter().filter(|r| **r == declared).count();
    match item {
        Built::Stmt(_, refs) => own(refs),
        Built::Init(_) => 0,
        Built::If(_, then, other, refs) => {
            own(refs)
                + then.iter().map(|i| mentions(i, declared)).sum::<usize>()
                + other.iter().map(|i| mentions(i, declared)).sum::<usize>()
        }
    }
}

fn assigned(statement: &Statement<'_>, targets: &FxHashMap<Span, Span>) -> Option<Span> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &statement.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    (assign.operator == oxc_syntax::operator::AssignmentOperator::Assign)
        .then(|| targets.get(&target.span).copied())
        .flatten()
}

pub(super) struct Build<'b, 'k, 'a> {
    pub(super) builder: &'b AstBuilder<'a>,
    pub(super) alloc: &'a Allocator,
    pub(super) scoping: &'b Scoping,
    pub(super) spans: &'b Spans,
    pub(super) plan: &'b Plan,
    pub(super) index: FxHashMap<Span, &'k Statement<'a>>,
    pub(super) strings: &'b [u16],
    pub(super) lifted: &'b mut Lifted,
    pub(super) names_observed: bool,
    pub(super) failed: bool,
    pub(super) expected: &'b mut Expected,
    pub(super) skip: Vec<SymbolId>,
}

pub(super) struct Frame<'f> {
    pub(super) values: &'f [f64],
    rest: Option<(&'f str, SymbolId)>,
}

impl<'k, 'a> Build<'_, 'k, 'a> {
    pub(super) fn env<'v>(&'v self, values: &'v [f64]) -> StateEnv<'v> {
        StateEnv {
            spans: self.spans,
            states: self.plan.syms.states,
            sum: self.plan.syms.sum,
            values,
        }
    }

    fn key(&self, expr: &Expression<'_>, values: &[f64]) -> Option<String> {
        match expr.without_parentheses() {
            Expression::StringLiteral(lit) => Some(lit.value.as_str()?.to_string()),
            other => self.xor(other, values),
        }
    }

    pub(super) fn xor(&self, expr: &Expression<'_>, values: &[f64]) -> Option<String> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if call.optional
            || self.plan.syms.xor.is_none()
            || symbol_at(&call.callee, self.spans) != self.plan.syms.xor
        {
            return None;
        }
        let env = self.env(values);
        let args: Vec<f64> = call
            .arguments
            .iter()
            .map(|a| env.eval(a.as_expression()?)?.0.num())
            .collect::<Option<_>>()?;
        let [key, start, length] = args.as_slice() else {
            return None;
        };
        xor_decode(self.strings, *key, *start, *length)
    }

    pub(super) fn scope_pair(
        &self,
        expr: &Expression<'_>,
        values: &[f64],
    ) -> Option<(String, String)> {
        let (object, prop) = member_parts(expr)?;
        let (base, scope) = member_parts(object)?;
        if symbol_at(base, self.spans) != Some(self.plan.syms.scope) {
            return None;
        }
        Some((
            self.member_key(scope, values)?,
            self.member_key(prop, values)?,
        ))
    }

    pub(super) fn member_key(&self, key: MemberKey<'_, '_>, values: &[f64]) -> Option<String> {
        match key {
            MemberKey::Static(name) => Some(name.to_string()),
            MemberKey::Computed(expr) => self.key(expr, values),
        }
    }

    pub(super) fn reference(&mut self, scope: &str, prop: &str) -> Option<(Span, String)> {
        let (name, declared) = self.lifted.name(scope, prop)?;
        let span = fresh_span()?;
        self.lifted.created.push((span, declared));
        Some((span, name))
    }

    pub(super) fn init_key(&self, statement: &Statement<'_>, values: &[f64]) -> Option<String> {
        let Statement::ExpressionStatement(statement) = statement else {
            return None;
        };
        let Expression::AssignmentExpression(assign) = &statement.expression else {
            return None;
        };
        let empty =
            matches!(&assign.right, Expression::ObjectExpression(o) if o.properties.is_empty());
        let target = assign
            .left
            .as_simple_assignment_target()?
            .as_member_expression()?;
        let object = target.object();
        if !empty || symbol_at(object, self.spans) != Some(self.plan.syms.scope) {
            return None;
        }
        match target {
            MemberExpression::StaticMemberExpression(m) => Some(m.property.name.to_string()),
            MemberExpression::ComputedMemberExpression(m) => self.key(&m.expression, values),
            MemberExpression::PrivateFieldExpression(_) => None,
        }
    }

    fn rewrite_statement(&mut self, statement: &Statement<'a>, frame: &Frame<'_>) -> Statement<'a> {
        let mut clone = statement.clone_in(self.alloc);
        let mut rewriter = Rewriter {
            build: self,
            values: frame.values,
            rest: frame.rest,
        };
        rewriter.visit_statement(&mut clone);
        clone
    }

    fn rewrite_expression(&mut self, expr: &Expression<'a>, frame: &Frame<'_>) -> Expression<'a> {
        let mut clone = expr.clone_in(self.alloc);
        let mut rewriter = Rewriter {
            build: self,
            values: frame.values,
            rest: frame.rest,
        };
        rewriter.visit_expression(&mut clone);
        clone
    }

    fn statement(&mut self, span: Span) -> Option<&'k Statement<'a>> {
        let found = self.index.get(&span).copied();
        self.failed |= found.is_none();
        found
    }

    fn returned(
        &mut self,
        span: Span,
        frame: &Frame<'_>,
        context: &Context,
        wrapped: bool,
    ) -> Option<Built<'a>> {
        let Some(Statement::ReturnStatement(ret)) = self.statement(span) else {
            self.failed = true;
            return None;
        };
        let argument = match (&ret.argument, context) {
            (Some(arg), Context::Function(flag)) if !wrapped => {
                let Some(value) = did_return_value(arg, *flag, self) else {
                    self.failed = true;
                    return None;
                };
                Some(value)
            }
            (arg, _) => arg.as_ref(),
        };
        let argument = argument.filter(|arg| !is_undefined(arg, self.spans))?;
        record_expression(
            argument,
            (self.scoping, self.spans),
            self.expected,
            &self.skip,
        );
        let before = self.lifted.created.len();
        let rewritten = self.rewrite_expression(argument, frame);
        let statement = match (context, wrapped) {
            (Context::Program, false) => {
                Statement::new_expression_statement(ret.span, rewritten, self.builder)
            }
            _ => Statement::new_return_statement(ret.span, Some(rewritten), self.builder),
        };
        Some(Built::Stmt(statement, self.created_since(before)))
    }

    pub(super) fn emit(
        &mut self,
        out: &[Out],
        rest: Option<(&str, SymbolId)>,
        wrapped: bool,
    ) -> Vec<Built<'a>> {
        let mut built = Vec::new();
        for item in out {
            if self.failed {
                break;
            }
            match item {
                Out::Item(span, v) => {
                    let Some(statement) = self.statement(*span) else {
                        break;
                    };
                    let values = self.plan.vectors[*v].clone();
                    if let Some(key) = self.init_key(statement, &values) {
                        built.push(Built::Init(key));
                        continue;
                    }
                    record_statement(
                        statement,
                        (self.scoping, self.spans),
                        self.expected,
                        &self.skip,
                    );
                    let frame = Frame {
                        values: &values,
                        rest,
                    };
                    let before = self.lifted.created.len();
                    let rewritten = self.rewrite_statement(statement, &frame);
                    built.push(Built::Stmt(rewritten, self.created_since(before)));
                }
                Out::Ret(span, v) => {
                    let values = self.plan.vectors[*v].clone();
                    let frame = Frame {
                        values: &values,
                        rest,
                    };
                    let context = self.plan.context;
                    if let Some(statement) = self.returned(*span, &frame, &context, wrapped) {
                        built.push(statement);
                    }
                }
                Out::If(span, v, then, other) => {
                    let Some(Statement::IfStatement(it)) = self.statement(*span) else {
                        self.failed = true;
                        break;
                    };
                    let values = self.plan.vectors[*v].clone();
                    record_expression(
                        &it.test,
                        (self.scoping, self.spans),
                        self.expected,
                        &self.skip,
                    );
                    let frame = Frame {
                        values: &values,
                        rest,
                    };
                    let before = self.lifted.created.len();
                    let test = self.rewrite_expression(&it.test, &frame);
                    let refs = self.created_since(before);
                    let then = self.emit(then, rest, wrapped);
                    let other = self.emit(other, rest, wrapped);
                    built.push(Built::If(test, then, other, refs));
                }
            }
        }
        built
    }

    fn created_since(&self, before: usize) -> Vec<Span> {
        self.lifted.created[before..]
            .iter()
            .map(|(_, d)| *d)
            .collect()
    }

    fn declare(&self, names: &[(String, Span)]) -> Option<Statement<'a>> {
        if names.is_empty() {
            return None;
        }
        let mut declarators = AVec::new_in(self.builder);
        for (name, span) in names {
            let pattern = BindingPattern::new_binding_identifier(
                *span,
                Ident::from_str_in(name, self.builder),
                self.builder,
            );
            declarators.push(VariableDeclarator::new(
                SPAN,
                pattern,
                None,
                None,
                false,
                self.builder,
            ));
        }
        Some(Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Let,
            declarators,
            false,
            self.builder,
        ))
    }

    fn initialised(
        &mut self,
        statement: Statement<'a>,
        (name, declared): (&str, Span),
    ) -> Statement<'a> {
        let Statement::ExpressionStatement(expression) = statement else {
            return statement;
        };
        let expression = expression.unbox();
        let assign = match expression.expression {
            Expression::AssignmentExpression(assign) => assign.unbox(),
            other => {
                return Statement::new_expression_statement(expression.span, other, self.builder)
            }
        };
        let target = assign.left.span();
        self.lifted
            .created
            .retain(|(reference, _)| *reference != target);
        let pattern = BindingPattern::new_binding_identifier(
            declared,
            Ident::from_str_in(name, self.builder),
            self.builder,
        );
        let mut declarators = AVec::new_in(self.builder);
        declarators.push(VariableDeclarator::new(
            SPAN,
            pattern,
            None,
            Some(assign.right),
            false,
            self.builder,
        ));
        Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Let,
            declarators,
            false,
            self.builder,
        )
    }

    fn place(
        &mut self,
        names: Vec<(String, Span)>,
        built: Vec<Built<'a>>,
        targets: &FxHashMap<Span, Span>,
    ) -> AVec<'a, Statement<'a>> {
        let mut first: FxHashMap<usize, (String, Span)> = FxHashMap::default();
        let mut head = Vec::new();
        for (name, declared) in names {
            let at = built.iter().position(|item| mentions(item, declared) > 0);
            let inline = at.filter(|i| match &built[*i] {
                Built::Stmt(statement, refs) => {
                    refs.iter().filter(|r| **r == declared).count() == 1
                        && assigned(statement, targets) == Some(declared)
                }
                _ => false,
            });
            match inline {
                Some(i) => {
                    first.insert(i, (name, declared));
                }
                None => head.push((name, declared)),
            }
        }
        let mut out = AVec::new_in(self.builder);
        if let Some(declaration) = self.declare(&head) {
            out.push(declaration);
        }
        let mut items = built.into_iter().enumerate();
        while let Some((index, item)) = items.next() {
            match item {
                Built::Stmt(statement, _) => match first.remove(&index) {
                    Some((name, declared)) => {
                        out.push(self.initialised(statement, (&name, declared)))
                    }
                    None => out.push(statement),
                },
                Built::Init(scope) => {
                    let names = self
                        .lifted
                        .by_scope
                        .get(&scope)
                        .cloned()
                        .unwrap_or_default();
                    let rest: Vec<Built<'a>> = items.by_ref().map(|(_, item)| item).collect();
                    out.extend(self.place(names, rest, targets));
                    break;
                }
                Built::If(test, then, other, _) => {
                    let then = self.place(Vec::new(), then, targets);
                    let other = self.place(Vec::new(), other, targets);
                    let consequent = Statement::new_block_statement(SPAN, then, self.builder);
                    let alternate = (!other.is_empty())
                        .then(|| Statement::new_block_statement(SPAN, other, self.builder));
                    out.push(Statement::new_if_statement(
                        SPAN,
                        test,
                        consequent,
                        alternate,
                        self.builder,
                    ));
                }
            }
        }
        out
    }

    pub(super) fn finish(
        &mut self,
        built: Vec<Built<'a>>,
        scopes: &[String],
    ) -> AVec<'a, Statement<'a>> {
        let names: Vec<(String, Span)> = scopes
            .iter()
            .flat_map(|scope| self.lifted.by_scope.get(scope).cloned().unwrap_or_default())
            .collect();
        let targets: FxHashMap<Span, Span> = self.lifted.created.iter().copied().collect();
        self.place(names, built, &targets)
    }

    pub(super) fn wrapper_body(&mut self, span: Span) -> Option<AVec<'a, Statement<'a>>> {
        let plan = self.plan;
        let wrap = plan.wrappers.get(&span)?;
        let built = self.emit(&wrap.out, Some((&wrap.rest, wrap.rest_symbol)), true);
        let fresh = [wrap.fresh.clone()];
        Some(self.finish(built, &fresh))
    }
}

fn is_undefined(expr: &Expression<'_>, spans: &Spans) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id)
        if id.name == "undefined" && !spans.contains_key(&id.span))
}

fn did_return_value<'k, 'a>(
    expr: &'k Expression<'a>,
    flag: SymbolId,
    build: &Build<'_, '_, 'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::SequenceExpression(seq) = expr.without_parentheses() else {
        return None;
    };
    let [Expression::AssignmentExpression(assign), value] = seq.expressions.as_slice() else {
        return None;
    };
    let target = match &assign.left {
        AssignmentTarget::AssignmentTargetIdentifier(id) => build.spans.get(&id.span).copied(),
        _ => None,
    };
    let set = matches!(&assign.right, Expression::BooleanLiteral(b) if b.value);
    (target == Some(flag) && set).then_some(value)
}

pub(super) enum MemberKey<'k, 'a> {
    Static(&'k str),
    Computed(&'k Expression<'a>),
}

pub(super) fn member_parts<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, MemberKey<'k, 'a>)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(m) if !m.optional => {
            Some((&m.object, MemberKey::Static(m.property.name.as_str())))
        }
        Expression::ComputedMemberExpression(m) if !m.optional => {
            Some((&m.object, MemberKey::Computed(&m.expression)))
        }
        _ => None,
    }
}

pub(super) fn target_parts<'k, 'a>(
    target: &'k SimpleAssignmentTarget<'a>,
) -> Option<(&'k Expression<'a>, MemberKey<'k, 'a>)> {
    match target {
        SimpleAssignmentTarget::StaticMemberExpression(m) => {
            Some((&m.object, MemberKey::Static(m.property.name.as_str())))
        }
        SimpleAssignmentTarget::ComputedMemberExpression(m) => {
            Some((&m.object, MemberKey::Computed(&m.expression)))
        }
        _ => None,
    }
}
