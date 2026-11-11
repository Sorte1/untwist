use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use untwist_core::semantic::{is_constant_binding, scope_hazard};

use super::tables::{at, id, some, Sites, SpanSlots, SymbolSet, NONE};
use super::{capacity_hint, record_lengths, with_slack, ScanLengths};
use super::{Frame, FunctionValue, InitOrder, StatementList, ValueOwner};
use crate::fused::{Fused, Member, Walk};
use crate::mutants::inert_statements_unchecked;
use crate::stamp::reached_outside_its_block;

pub(super) struct Scan<'s> {
    scoping: &'s Scoping,
    order: InitOrder,
    disqualified: SymbolSet,
    initialised: SymbolSet,
    current: Option<u32>,
    value: Option<u32>,
    class_value: Option<u32>,
    element_key: bool,
    pending: Option<ValueOwner>,
    functions: u32,
    overflowed: bool,
}

fn hoisted_symbol(stmt: &Statement<'_>) -> Option<SymbolId> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    function.id.as_ref()?.symbol_id.get()
}

fn bound_function_value(declarator: &VariableDeclarator<'_>) -> Option<ValueOwner> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let init = declarator.init.as_ref()?;
    if !matches!(
        init,
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    ) {
        return None;
    }
    Some(ValueOwner::Binding {
        symbol: id.symbol_id.get()?,
        hoisted: false,
    })
}

impl<'s> Scan<'s> {
    pub(super) fn new(scoping: &'s Scoping) -> Self {
        let symbols = scoping.symbols_len();
        let hint = capacity_hint();
        let requested = ScanLengths {
            frames: with_slack(hint.frames),
            lists: with_slack(hint.lists),
            slots: with_slack(hint.slots),
            values: with_slack(hint.values),
        };
        Self {
            scoping,
            order: InitOrder {
                frames: Vec::with_capacity(requested.frames),
                lists: Vec::with_capacity(requested.lists),
                slots: SpanSlots::with_capacity(requested.slots),
                values: Vec::with_capacity(requested.values),
                sites: Sites::with_capacity(scoping.references_len(), symbols),
                requested,
                ..InitOrder::default()
            },
            disqualified: SymbolSet::with_capacity(symbols),
            initialised: SymbolSet::with_capacity(symbols),
            current: None,
            value: None,
            class_value: None,
            element_key: false,
            pending: None,
            functions: 0,
            overflowed: false,
        }
    }

    pub(super) fn run(self, program: &Program<'_>) -> Option<InitOrder> {
        let mut walk = Walk(self);
        walk.visit_program(program);
        walk.0.finish().map(|order| order.with_reentry(program))
    }

    pub(super) fn run_with<'a, M: Member<'a>>(
        self,
        program: &Program<'a>,
        other: M,
    ) -> (Option<InitOrder>, M) {
        let mut walk = Walk(Fused::new(self, other));
        walk.visit_program(program);
        let Fused { first, second, .. } = walk.0;
        (
            first.finish().map(|order| order.with_reentry(program)),
            second,
        )
    }

    fn finish(self) -> Option<InitOrder> {
        let Self {
            scoping,
            mut order,
            disqualified,
            initialised,
            overflowed,
            ..
        } = self;
        drop(initialised);
        if overflowed {
            return None;
        }
        order.hazard = scope_hazard(scoping).is_some();
        for value in &mut order.values {
            let ValueOwner::Binding { symbol, .. } = value.owner else {
                continue;
            };
            if disqualified.contains(symbol)
                || !is_constant_binding(scoping, symbol)
                || reached_outside_its_block(scoping, symbol).is_some()
            {
                value.owner = ValueOwner::Opaque;
            }
        }
        drop(disqualified);
        order.sites.seal();
        record_lengths(order.lengths());
        Some(order)
    }

    fn fresh(&mut self, len: usize) -> Option<u32> {
        let fresh = id(len);
        self.overflowed |= fresh.is_none();
        fresh
    }

    fn enter_value(&mut self, owner: ValueOwner) -> Option<u32> {
        let fresh = self.fresh(self.order.values.len());
        self.order.values.push(FunctionValue {
            owner,
            outer: self.value.unwrap_or(NONE),
            frame: self.current.unwrap_or(NONE),
        });
        self.functions = self.functions.saturating_add(1);
        std::mem::replace(&mut self.value, fresh)
    }

    fn leave_value(&mut self, outer: Option<u32>) {
        self.functions = self.functions.saturating_sub(1);
        self.value = outer;
    }

    fn note_parameters(&mut self, pattern: &BindingPattern<'_>) {
        for id in pattern.get_binding_identifiers() {
            if let Some(symbol) = id.symbol_id.get() {
                self.disqualified.insert(symbol);
            }
        }
    }

    fn note_redeclaration(&mut self, declarator: &VariableDeclarator<'_>) {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        let Some(symbol) = id.symbol_id.get().filter(|_| declarator.init.is_some()) else {
            return;
        };
        if !self.initialised.insert(symbol) {
            self.disqualified.insert(symbol);
        }
    }

    fn suspend_class_value(&mut self) -> Option<u32> {
        let current = self.value?;
        if !std::mem::take(&mut self.element_key) {
            return None;
        }
        if self.class_value != Some(current) {
            return None;
        }
        self.value = self
            .order
            .values
            .get(at(current))
            .and_then(|value| some(value.outer));
        self.functions = self.functions.saturating_sub(1);
        Some(current)
    }

    fn resume_class_value(&mut self, suspended: Option<u32>) {
        let Some(current) = suspended else {
            return;
        };
        self.functions = self.functions.saturating_add(1);
        self.value = Some(current);
    }

    #[inline(always)]
    fn enter_list(&mut self, it: &AVec<'_, Statement<'_>>) {
        let leading = it
            .iter()
            .take_while(|stmt| statement_cannot_call(stmt))
            .count();
        let (Some(start), Some(leading_inert)) =
            (self.fresh(self.order.slots.len()), self.fresh(leading))
        else {
            return;
        };
        self.order.lists.push(StatementList {
            start,
            leading_inert,
            functions: self.functions,
        });
        for statement in it {
            if self.order.slots.insert(statement.span()).is_none() {
                self.overflowed = true;
            }
        }
    }

    #[inline(always)]
    fn enter_frame(&mut self, it: &Statement<'_>) -> Option<u32> {
        let ordinal = self.order.slots.get(it.span())?;
        let slot = self.order.slot_of_ordinal(ordinal)?;
        let frame = self.fresh(self.order.frames.len())?;
        self.order.frames.push(Frame {
            list: slot.list,
            index: slot.index,
            hoisted: hoisted_symbol(it),
            functions: self.functions,
            parent: self.current.unwrap_or(NONE),
        });
        Some(frame)
    }
}

type Saved = (Option<u32>, Option<u32>);

impl Scan<'_> {
    fn enter_function_value(&mut self) -> Saved {
        let owner = self.pending.take().unwrap_or(ValueOwner::Anonymous);
        (self.enter_value(owner), None)
    }
}

impl<'a> Member<'a> for Scan<'_> {
    type Saved = Saved;

    #[inline(always)]
    fn enter_statements(&mut self, it: &AVec<'a, Statement<'a>>) -> Saved {
        self.enter_list(it);
        (None, None)
    }

    #[inline(always)]
    fn enter_statement(&mut self, it: &Statement<'a>) -> Saved {
        self.pending = hoisted_symbol(it).map(|symbol| ValueOwner::Binding {
            symbol,
            hoisted: true,
        });
        let outer = self.current;
        if let Some(frame) = self.enter_frame(it) {
            self.current = Some(frame);
        }
        (outer, None)
    }

    #[inline(always)]
    fn leave_statement(&mut self, (outer, _): Saved) {
        self.current = outer;
    }

    #[inline(always)]
    fn enter_variable_declarator(&mut self, it: &VariableDeclarator<'a>) -> Saved {
        self.pending = bound_function_value(it);
        self.note_redeclaration(it);
        (None, None)
    }

    #[inline(always)]
    fn leave_variable_declarator(&mut self, _saved: Saved) {
        self.pending = None;
    }

    #[inline(always)]
    fn identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(frame) = self.current else {
            return;
        };
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let value = self.value.unwrap_or(NONE);
        let pushed = self
            .order
            .sites
            .push(frame, self.functions, value, it.span, symbol);
        self.overflowed |= pushed.is_none();
    }

    #[inline(always)]
    fn formal_parameter(&mut self, it: &FormalParameter<'a>) {
        self.note_parameters(&it.pattern);
    }

    #[inline(always)]
    fn catch_parameter(&mut self, it: &CatchParameter<'a>) {
        self.note_parameters(&it.pattern);
    }

    #[inline(always)]
    fn enter_function(&mut self, _it: &Function<'a>) -> Saved {
        self.enter_function_value()
    }

    #[inline(always)]
    fn leave_function(&mut self, (outer, _): Saved) {
        self.leave_value(outer);
    }

    #[inline(always)]
    fn enter_arrow(&mut self, _it: &ArrowFunctionExpression<'a>) -> Saved {
        self.enter_function_value()
    }

    #[inline(always)]
    fn leave_arrow(&mut self, (outer, _): Saved) {
        self.leave_value(outer);
    }

    #[inline(always)]
    fn enter_class(&mut self, _it: &Class<'a>) -> Saved {
        self.pending = None;
        let outer = self.enter_value(ValueOwner::Anonymous);
        let enclosing = std::mem::replace(&mut self.class_value, self.value);
        (outer, enclosing)
    }

    #[inline(always)]
    fn leave_class(&mut self, (outer, enclosing): Saved) {
        self.class_value = enclosing;
        self.leave_value(outer);
    }

    #[inline(always)]
    fn enter_class_heritage(&mut self) -> Saved {
        self.element_key = true;
        (self.suspend_class_value(), None)
    }

    #[inline(always)]
    fn leave_class_heritage(&mut self, (suspended, _): Saved) {
        self.resume_class_value(suspended);
    }

    #[inline(always)]
    fn enter_class_element(&mut self) -> Saved {
        self.element_key = true;
        (None, None)
    }

    #[inline(always)]
    fn leave_class_element(&mut self, _saved: Saved) {
        self.element_key = false;
    }

    #[inline(always)]
    fn enter_property_key(&mut self) -> Saved {
        (self.suspend_class_value(), None)
    }

    #[inline(always)]
    fn leave_property_key(&mut self, (suspended, _): Saved) {
        self.resume_class_value(suspended);
    }
}

fn statement_cannot_call(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().all(declarator_cannot_call)
        }
        Statement::ExpressionStatement(statement) => {
            inert_statements_unchecked() || expression_cannot_call(&statement.expression)
        }
        Statement::BlockStatement(block) => {
            inert_statements_unchecked()
                || block.body.iter().all(|inner| {
                    !matches!(inner, Statement::FunctionDeclaration(_))
                        && statement_cannot_call(inner)
                })
        }
        _ => false,
    }
}

fn declarator_cannot_call(declarator: &VariableDeclarator<'_>) -> bool {
    if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
        return false;
    }
    declarator.init.as_ref().is_none_or(expression_cannot_call)
}

fn expression_cannot_call(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_primitive(&unary.argument)
        }
        Expression::ParenthesizedExpression(inner) => expression_cannot_call(&inner.expression),
        Expression::ArrayExpression(array) => array.elements.iter().all(element_cannot_call),
        Expression::ObjectExpression(object) => object.properties.iter().all(property_cannot_call),
        _ => false,
    }
}

fn is_primitive(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_primitive(&unary.argument)
        }
        _ => false,
    }
}

fn element_cannot_call(element: &ArrayExpressionElement<'_>) -> bool {
    if matches!(element, ArrayExpressionElement::Elision(_)) {
        return true;
    }
    element.as_expression().is_some_and(expression_cannot_call)
}

fn property_cannot_call(property: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    !property.computed && expression_cannot_call(&property.value)
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;
    use untwist_core::semantic::build_scoping;

    use super::Scan;

    #[test]
    fn an_index_past_u32_refuses_the_whole_scan() {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, "var m = { alpha: 1 };\nm.alpha;");
        let sem = build_scoping(&parsed.program);
        assert!(Scan::new(&sem.scoping).run(&parsed.program).is_some());
        let mut scan = Scan::new(&sem.scoping);
        assert_eq!(scan.fresh(u32::MAX as usize), None);
        assert!(scan.run(&parsed.program).is_none());
    }
}
