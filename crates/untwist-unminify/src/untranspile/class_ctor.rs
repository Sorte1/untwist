use std::cell::Cell;

use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_js_mut, VisitJs, VisitJsMut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::frame::{self, Frame, FrameScan};
use untwist_core::pass::Cx;

use super::class_members::{key_is_pure, property_key};
use super::class_super::{find_super, split_super, super_statement_of, Derived, SuperForm};
use super::identify::{resolved, Table};
use super::shapes::{is_field_definer, Kind};
use super::values::into_inner;
use untwist_core::ast_util::global::is_global_undefined;

#[derive(Clone, Copy)]
pub struct CtorPlan {
    check: usize,
    derived: Option<Derived>,
    fields: (usize, usize),
    alias: Option<SymbolId>,
}

impl CtorPlan {
    pub fn field_count(&self) -> usize {
        self.fields.1
    }
}

fn call_in<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k CallExpression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    match statement.expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        _ => None,
    }
}

pub(super) fn argument<'k, 'a>(
    call: &'k CallExpression<'a>,
    index: usize,
) -> Option<&'k Expression<'a>> {
    call.arguments
        .get(index)
        .and_then(Argument::as_expression)
        .map(Expression::without_parentheses)
}

pub(super) fn is_this(expression: Option<&Expression<'_>>) -> bool {
    matches!(expression, Some(Expression::ThisExpression(_)))
}

pub(super) fn plain(call: &CallExpression<'_>) -> bool {
    !call.optional
        && call
            .arguments
            .iter()
            .all(|argument| !matches!(argument, Argument::SpreadElement(_)))
}

fn is_check(statement: &Statement<'_>, symbol: SymbolId, table: &Table, scoping: &Scoping) -> bool {
    call_in(statement).is_some_and(|call| {
        plain(call)
            && call.arguments.len() == 2
            && table.kind_of_callee(&call.callee, scoping) == Some(Kind::ClassCallCheck)
            && is_this(argument(call, 0))
            && argument(call, 1).and_then(|it| resolved(it, scoping)) == Some(symbol)
    })
}

pub fn checks_its_class(function: &Function<'_>, table: &Table, scoping: &Scoping) -> bool {
    let (Some(symbol), Some(body)) = (
        function.id.as_ref().and_then(|id| id.symbol_id.get()),
        function.body.as_ref(),
    ) else {
        return false;
    };
    body.statements
        .iter()
        .find(|statement| !matches!(statement, Statement::VariableDeclaration(_)))
        .is_some_and(|statement| is_check(statement, symbol, table, scoping))
}

fn declared_symbol<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(SymbolId, Option<&'k Expression<'a>>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.symbol_id.get()?, declarator.init.as_ref()))
}

#[derive(Default)]
struct Scan {
    this: Option<SymbolId>,
    after: u32,
    target: u32,
    frame: Frame,
    raw_this: u32,
    early: bool,
    boxed: bool,
    returns: u32,
    bad_return: bool,
    shorthand: bool,
}

impl Scan {
    fn is_this_symbol(&self, id: &IdentifierReference<'_>, scoping: &Scoping) -> bool {
        self.this.is_some()
            && id
                .reference_id
                .get()
                .and_then(|reference| scoping.get_reference(reference).symbol_id())
                == self.this
    }
}

struct ScanCtx<'s> {
    scan: Scan,
    scoping: &'s Scoping,
}

impl<'a> FrameScan<'a> for ScanCtx<'_> {
    fn frame(&mut self) -> &mut Frame {
        &mut self.scan.frame
    }
}

impl<'a> VisitJs<'a> for ScanCtx<'_> {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        if self.scan.frame.is_own() {
            self.scan.raw_this += 1;
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !self.scan.is_this_symbol(it, self.scoping) {
            return;
        }
        if it.span.start < self.scan.after && it.span.start != self.scan.target {
            self.scan.early = true;
        }
        if !self.scan.frame.is_own() {
            self.scan.boxed = true;
        }
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.shorthand {
            if let Expression::Identifier(id) = &it.value {
                if self.scan.is_this_symbol(id, self.scoping) {
                    self.scan.shorthand = true;
                }
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if self.scan.frame.is_top() {
            self.scan.returns += 1;
            let returns_this = it.argument.as_ref().is_some_and(|argument| {
                matches!(argument, Expression::Identifier(id) if self.scan.is_this_symbol(id, self.scoping))
            });
            if !returns_this {
                self.scan.bad_return = true;
            }
        }
        walk_js::walk_return_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        frame::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        frame::walk_class(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        frame::walk_arrow(self, it);
    }
}

fn scan_body(statements: &[Statement<'_>], scan: Scan, scoping: &Scoping) -> Scan {
    let mut visitor = ScanCtx { scan, scoping };
    for statement in statements {
        visitor.visit_statement(statement);
    }
    visitor.scan
}

fn derived_ok(
    statements: &[Statement<'_>],
    derived: &mut Derived,
    scoping: &Scoping,
) -> Option<()> {
    let last = statements.len().checked_sub(1)?;
    let scan = Scan {
        this: derived.this,
        after: derived.end,
        target: derived.target,
        ..Scan::default()
    };
    let scan = scan_body(statements, scan, scoping);
    if scan.raw_this != 2 || scan.early || scan.boxed || scan.shorthand {
        return None;
    }
    match derived.form {
        SuperForm::Return => (derived.at == last && scan.returns == 1).then_some(()),
        SuperForm::Assign | SuperForm::Chained => {
            let this = derived.this?;
            let writes = scoping
                .get_resolved_references(this)
                .filter(|reference| reference.is_write())
                .count();
            let declared = statements[..derived.at].iter().position(|statement| {
                declared_symbol(statement)
                    .is_some_and(|(symbol, init)| symbol == this && init.is_none())
            })?;
            derived.declared = Some(declared);
            let ends = matches!(statements.get(last), Some(Statement::ReturnStatement(_)));
            (writes == 1 && !scan.bad_return && ends && last > derived.at).then_some(())
        }
    }
}

struct FieldScan<'s> {
    scoping: &'s Scoping,
    ctor: ScopeId,
    alias: Option<SymbolId>,
    frame: Frame,
    entered: Vec<ScopeId>,
    ok: bool,
}

impl<'a> VisitJs<'a> for FieldScan<'_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        if let Some(scope) = scope_id.get() {
            self.entered.push(scope);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if matches!(it, Expression::NewTarget(_) | Expression::Super(_)) {
            self.ok = false;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let Some(symbol) = symbol else {
            if it.name.as_str() == "arguments" && self.frame.is_own() {
                self.ok = false;
            }
            return;
        };
        if Some(symbol) == self.alias {
            self.ok &= self.frame.is_own();
            return;
        }
        let scope = self.scoping.symbol_scope_id(symbol);
        let local = !self.entered.contains(&scope)
            && self
                .scoping
                .scope_ancestors(scope)
                .any(|it| it == self.ctor);
        if local {
            self.ok = false;
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        frame::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        frame::walk_class(self, it);
    }
}

impl<'a> FrameScan<'a> for FieldScan<'_> {
    fn frame(&mut self) -> &mut Frame {
        &mut self.frame
    }
}

fn value_ok(
    value: &Expression<'_>,
    ctor: ScopeId,
    alias: Option<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let mut scan = FieldScan {
        scoping,
        ctor,
        alias,
        frame: Frame::default(),
        entered: Vec::new(),
        ok: true,
    };
    scan.visit_expression(value);
    scan.ok
}

fn receiver_ok(
    expression: Option<&Expression<'_>>,
    receiver: Option<SymbolId>,
    scoping: &Scoping,
) -> bool {
    match (expression, receiver) {
        (Some(Expression::ThisExpression(_)), None) => true,
        (Some(other), Some(_)) => resolved(other, scoping) == receiver,
        _ => false,
    }
}

fn field_ok(
    statement: &Statement<'_>,
    receiver: Option<SymbolId>,
    alias: Option<SymbolId>,
    ctor: ScopeId,
    table: &Table,
    scoping: &Scoping,
) -> bool {
    let Some(call) = call_in(statement) else {
        return false;
    };
    let definer = table
        .kind_of_callee(&call.callee, scoping)
        .is_some_and(is_field_definer);
    let Some(key) = argument(call, 1) else {
        return false;
    };
    let named_constructor = super::class_members::static_key_name(key) == Some("constructor");
    definer
        && plain(call)
        && (2..=3).contains(&call.arguments.len())
        && receiver_ok(argument(call, 0), receiver, scoping)
        && key_is_pure(key, scoping)
        && !named_constructor
        && argument(call, 2).is_none_or(|value| value_ok(value, ctor, alias, scoping))
}

pub(super) fn field_run(
    statements: &[Statement<'_>],
    start: usize,
    receiver: Option<SymbolId>,
    alias: Option<SymbolId>,
    ctor: ScopeId,
    table: &Table,
    scoping: &Scoping,
) -> usize {
    statements
        .iter()
        .skip(start)
        .take_while(|statement| field_ok(statement, receiver, alias, ctor, table, scoping))
        .count()
}

fn base_alias(
    statements: &[Statement<'_>],
    check: usize,
    scoping: &Scoping,
) -> Option<Option<SymbolId>> {
    let mut alias = None;
    for statement in statements.get(..check)? {
        let (symbol, init) = declared_symbol(statement)?;
        match init.map(Expression::without_parentheses) {
            None => {}
            Some(Expression::ThisExpression(_)) if alias.is_none() => {
                let writes = scoping
                    .get_resolved_references(symbol)
                    .any(|reference| reference.is_write());
                if writes {
                    return None;
                }
                alias = Some(symbol);
            }
            Some(_) => return None,
        }
    }
    Some(alias)
}

pub(super) fn simple_params(function: &Function<'_>) -> bool {
    let plain =
        |pattern: &BindingPattern<'_>| matches!(pattern, BindingPattern::BindingIdentifier(_));
    function
        .params
        .items
        .iter()
        .all(|param| param.initializer.is_none() && plain(&param.pattern))
        && function
            .params
            .rest
            .as_ref()
            .is_none_or(|rest| plain(&rest.rest.argument))
}

pub fn plan(
    ctor: &Function<'_>,
    symbol: SymbolId,
    derived: bool,
    table: &Table,
    scoping: &Scoping,
) -> Option<CtorPlan> {
    let statements = ctor.body.as_ref()?.statements.as_slice();
    let scope = ctor.scope_id.get()?;
    let check = statements
        .iter()
        .position(|statement| is_check(statement, symbol, table, scoping))?;
    let declarations_first = statements[..check]
        .iter()
        .all(|statement| matches!(statement, Statement::VariableDeclaration(_)));
    if !declarations_first {
        return None;
    }
    if !derived {
        let alias = base_alias(statements, check, scoping).filter(|_| simple_params(ctor));
        let fields = match alias {
            Some(alias) => field_run(statements, check + 1, None, alias, scope, table, scoping),
            None => 0,
        };
        return Some(CtorPlan {
            check,
            derived: None,
            fields: (check + 1, fields),
            alias: alias.flatten(),
        });
    }
    let mut found = find_super(statements, symbol, table, scoping)?;
    if found.at < check {
        return None;
    }
    derived_ok(statements, &mut found, scoping)?;
    let start = found.at + 1;
    let fields = match found.form {
        SuperForm::Assign => field_run(
            statements, start, found.this, found.this, scope, table, scoping,
        ),
        SuperForm::Chained | SuperForm::Return => 0,
    };
    Some(CtorPlan {
        check,
        derived: Some(found),
        fields: (start, fields),
        alias: found.this,
    })
}

struct ThisFor<'c, 'a> {
    symbol: SymbolId,
    scoping: &'c Scoping,
    ast: &'c AstBuilder<'a>,
}

impl<'a> VisitJsMut<'a> for ThisFor<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::Identifier(id) = it {
            let symbol = id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
            if symbol == Some(self.symbol) {
                *it = Expression::new_this_expression(id.span, self.ast);
                return;
            }
        }
        walk_js_mut::walk_expression(self, it);
    }
}

pub(super) fn field_of<'a>(statement: Statement<'a>, cx: &Cx<'a, '_>) -> Option<ClassElement<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = into_inner(statement.unbox().expression) else {
        return None;
    };
    let mut arguments = call
        .unbox()
        .arguments
        .into_iter()
        .skip(1)
        .filter_map(|argument| Expression::try_from(argument).ok());
    let (key, computed) = property_key(arguments.next()?, cx);
    let value = arguments
        .next()
        .map(into_inner)
        .filter(|value| !(value.is_void_0() || is_global_undefined(value, cx.scoping())));
    Some(ClassElement::new_property_definition(
        SPAN,
        PropertyDefinitionType::PropertyDefinition,
        AVec::new_in(cx),
        key,
        None,
        value,
        computed,
        false,
        false,
        false,
        false,
        false,
        false,
        None,
        cx,
    ))
}

fn replace_this<'a>(statement: &mut Statement<'a>, symbol: Option<SymbolId>, cx: &Cx<'a, '_>) {
    if let Some(symbol) = symbol {
        let mut visitor = ThisFor {
            symbol,
            scoping: cx.scoping(),
            ast: cx.ast(),
        };
        visitor.visit_statement(statement);
    }
}

fn skipped(index: usize, last: usize, plan: &CtorPlan) -> bool {
    let declared = plan.derived.and_then(|derived| derived.declared);
    let assign_return = plan
        .derived
        .is_some_and(|derived| derived.form != SuperForm::Return && index == last);
    index == plan.check || Some(index) == declared || assign_return
}

pub fn rewrite<'a>(
    mut ctor: ABox<'a, Function<'a>>,
    plan: CtorPlan,
    _table: &Table,
    cx: &mut Cx<'a, '_>,
) -> (Option<ClassElement<'a>>, AVec<'a, ClassElement<'a>>) {
    let mut fields = AVec::with_capacity_in(plan.fields.1, cx);
    let mut forwards = false;
    let Some(body) = ctor.body.as_mut() else {
        return (None, fields);
    };
    body.directives.clear();
    let old = std::mem::replace(&mut body.statements, AVec::new_in(cx));
    let last = old.len().saturating_sub(1);
    let (start, count) = plan.fields;
    let derived = plan.derived;
    for (index, mut statement) in old.into_iter().enumerate() {
        if skipped(index, last, &plan) {
            continue;
        }
        if derived.is_some_and(|it| it.at == index) {
            let form = derived.map_or(SuperForm::Return, |it| it.form);
            let (call, rest) = split_super(statement, form, cx);
            let (statement, forward) = super_statement_of(call, cx);
            forwards = forward && rest.is_none();
            body.statements.push(statement);
            if let Some(mut rest) = rest {
                replace_this(&mut rest, plan.alias, cx);
                body.statements.push(rest);
            }
            continue;
        }
        let in_fields = index >= start && index < start + count;
        if in_fields || derived.is_some() {
            replace_this(&mut statement, plan.alias, cx);
        }
        if in_fields {
            fields.extend(field_of(statement, cx));
        } else {
            body.statements.push(statement);
        }
    }
    let no_params = ctor.params.items.is_empty() && ctor.params.rest.is_none();
    let Some(body) = ctor.body.as_ref() else {
        return (None, fields);
    };
    let default_ctor = no_params
        && match derived {
            None => body.statements.is_empty(),
            Some(_) => forwards && body.statements.len() == 1,
        };
    if default_ctor {
        return (None, fields);
    }
    ctor.id = None;
    ctor.r#type = FunctionType::FunctionExpression;
    let key = PropertyKey::new_static_identifier(SPAN, Ident::from("constructor"), cx);
    let constructor = ClassElement::new_method_definition(
        SPAN,
        MethodDefinitionType::MethodDefinition,
        AVec::new_in(cx),
        key,
        ctor,
        MethodDefinitionKind::Constructor,
        false,
        false,
        false,
        false,
        None,
        cx,
    );
    (Some(constructor), fields)
}
