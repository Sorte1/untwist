use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::identify::Table;
use super::shapes::Kind;

const PROTOTYPE: &str = "prototype";

const LINK_DEPTH: u32 = 8;

#[derive(Clone, Copy)]
struct Use {
    symbol: SymbolId,
    callee: SymbolId,
    index: u8,
    statics: bool,
    key: bool,
}

#[derive(Default)]
pub struct Prototypes {
    proven: bool,
    open: Vec<bool>,
    uses: Vec<Use>,
    links: Vec<(SymbolId, SymbolId)>,
}

impl Prototypes {
    pub fn closed(&self, symbol: SymbolId, table: &Table) -> bool {
        self.proven && self.closed_within(symbol, table, LINK_DEPTH)
    }

    fn closed_within(&self, symbol: SymbolId, table: &Table, depth: u32) -> bool {
        if depth == 0 || self.open.get(symbol.index()).copied().unwrap_or(true) {
            return false;
        }
        let uses = slice_of(&self.uses, symbol, |it| it.symbol);
        let inherits = uses
            .iter()
            .filter(|it| it.index == 0 && inherits_kind(table.kind(it.callee)))
            .count();
        inherits <= 1
            && uses.iter().all(|it| allowed(table.kind(it.callee), it))
            && slice_of(&self.links, symbol, |it| it.0)
                .iter()
                .all(|(_, to)| self.closed_within(*to, table, depth - 1))
    }
}

fn slice_of<T>(items: &[T], symbol: SymbolId, key: impl Fn(&T) -> SymbolId) -> &[T] {
    let from = items.partition_point(|it| key(it) < symbol);
    let to = items.partition_point(|it| key(it) <= symbol);
    items.get(from..to).unwrap_or_default()
}

fn inherits_kind(kind: Option<Kind>) -> bool {
    matches!(
        kind,
        Some(Kind::Inherits | Kind::InheritsOpen | Kind::Extends)
    )
}

fn allowed(kind: Option<Kind>, used: &Use) -> bool {
    let Some(kind) = kind else {
        return false;
    };
    match (kind, used.index) {
        (Kind::ClassCallCheck | Kind::CallSuper, 1) => true,
        (Kind::CreateClass | Kind::CreateClassOpen, 0) => used.statics,
        (Kind::Inherits | Kind::InheritsOpen | Kind::Extends, 0 | 1) => true,
        (Kind::GetPrototypeOf | Kind::Typeof, 0) => true,
        (Kind::DefineProperty | Kind::DefinePropertyPlain | Kind::PublicField, 0) => used.key,
        _ => false,
    }
}

#[derive(Default)]
struct Frame {
    arrow: bool,
    reads_arguments: bool,
    passed: Vec<SymbolId>,
    returns_to: Option<SymbolId>,
}

struct Scan<'s> {
    scoping: &'s Scoping,
    found: Prototypes,
    frames: Vec<Frame>,
    declared_into: Option<SymbolId>,
    next: Option<Frame>,
}

pub fn gather(program: &Program<'_>, scoping: &Scoping) -> Prototypes {
    let mut scan = Scan {
        scoping,
        found: Prototypes {
            proven: true,
            open: vec![false; scoping.symbols_len()],
            uses: Vec::new(),
            links: Vec::new(),
        },
        frames: Vec::new(),
        declared_into: None,
        next: None,
    };
    scan.visit_program(program);
    let mut found = scan.found;
    found.uses.sort_unstable_by_key(|it| it.symbol);
    found.links.sort_unstable_by_key(|it| it.0);
    found
}

fn string_key<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn is_prototype_literal(expression: &Expression<'_>) -> bool {
    string_key(expression) == Some(PROTOTYPE)
}

fn literal_key_other_than_prototype(expression: Option<&Expression<'_>>) -> bool {
    match expression.map(Expression::without_parentheses) {
        Some(Expression::StringLiteral(literal)) => literal.value != PROTOTYPE,
        Some(Expression::NumericLiteral(_)) => true,
        _ => false,
    }
}

fn descriptor_keyed(element: &ArrayExpressionElement<'_>) -> bool {
    let Some(Expression::ObjectExpression(object)) = element.as_expression() else {
        return false;
    };
    object.properties.iter().all(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) => {
            property
                .key
                .static_name()
                .and_then(|name| name.into_cow_str())
                .as_deref()
                != Some("key")
                || literal_key_other_than_prototype(Some(&property.value))
        }
        _ => false,
    })
}

fn statics_keyed(statics: Option<&Expression<'_>>) -> bool {
    match statics.map(Expression::without_parentheses) {
        None | Some(Expression::NullLiteral(_)) => true,
        Some(Expression::ArrayExpression(array)) => array.elements.iter().all(descriptor_keyed),
        _ => false,
    }
}

fn keyed(call: &CallExpression<'_>) -> (bool, bool) {
    let at = |index: usize| call.arguments.get(index).and_then(Argument::as_expression);
    (
        statics_keyed(at(2)),
        literal_key_other_than_prototype(at(1)),
    )
}

fn iife_callee<'k, 'a>(callee: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match callee.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

impl Scan<'_> {
    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn open(&mut self, symbol: SymbolId) {
        if let Some(slot) = self.found.open.get_mut(symbol.index()) {
            *slot = true;
        }
    }

    fn open_all(&mut self) {
        self.found.proven = false;
    }

    fn plain_symbol(&self, expression: &Expression<'_>) -> Option<Option<SymbolId>> {
        match expression.without_parentheses() {
            Expression::Identifier(id) => Some(self.symbol_of(id)),
            _ => None,
        }
    }

    fn visit_unless_plain(&mut self, expression: &Expression<'_>) {
        if self.plain_symbol(expression).is_none() {
            self.visit_expression(expression);
        }
    }

    fn written_member(&mut self, object: &Expression<'_>, key: Option<&str>, dynamic: bool) {
        let prototype = key == Some(PROTOTYPE);
        match self.plain_symbol(object) {
            Some(Some(symbol)) if prototype || dynamic => self.open(symbol),
            Some(_) => {}
            None => {
                if prototype {
                    self.open_all();
                }
                self.visit_expression(object);
            }
        }
    }

    fn written(&mut self, member: &MemberExpression<'_>) {
        match member {
            MemberExpression::StaticMemberExpression(member) => {
                self.written_member(&member.object, Some(member.property.name.as_str()), false);
            }
            MemberExpression::ComputedMemberExpression(member) => {
                let key = string_key(&member.expression);
                let numeric = matches!(
                    member.expression.without_parentheses(),
                    Expression::NumericLiteral(_)
                );
                self.written_member(&member.object, key, key.is_none() && !numeric);
                self.visit_expression(&member.expression);
            }
            MemberExpression::PrivateFieldExpression(member) => {
                self.visit_expression(&member.object);
            }
        }
    }

    fn argument_uses(&mut self, call: &CallExpression<'_>, callee: Option<SymbolId>) {
        let (statics, key) = keyed(call);
        for (index, argument) in call.arguments.iter().enumerate() {
            let Some(expression) = argument.as_expression() else {
                self.visit_argument(argument);
                continue;
            };
            if is_prototype_literal(expression) {
                self.open_all();
                continue;
            }
            match (self.plain_symbol(expression), callee) {
                (Some(Some(symbol)), Some(callee)) => self.found.uses.push(Use {
                    symbol,
                    callee,
                    index: u8::try_from(index).unwrap_or(u8::MAX),
                    statics,
                    key,
                }),
                (Some(Some(symbol)), None) => self.open(symbol),
                (Some(None), _) => {}
                (None, _) => self.visit_expression(expression),
            }
        }
    }

    fn iife_call(&mut self, call: &CallExpression<'_>, function: &Function<'_>) {
        let returns_to = self.declared_into.take();
        let mut passed = Vec::new();
        for (index, argument) in call.arguments.iter().enumerate() {
            let parameter = function
                .params
                .items
                .get(index)
                .filter(|_| function.params.rest.is_none())
                .and_then(|param| match &param.pattern {
                    BindingPattern::BindingIdentifier(id) if param.initializer.is_none() => {
                        id.symbol_id.get()
                    }
                    _ => None,
                });
            let expression = argument.as_expression();
            match (expression.and_then(|it| self.plain_symbol(it)), parameter) {
                (Some(Some(symbol)), Some(parameter)) => {
                    self.found.links.push((symbol, parameter));
                    passed.push(symbol);
                }
                (Some(Some(symbol)), None) => self.open(symbol),
                (Some(None), _) => {}
                (None, _) => {
                    if expression.is_some_and(is_prototype_literal) {
                        self.open_all();
                    }
                    self.visit_argument(argument);
                }
            }
        }
        self.next = Some(Frame {
            arrow: false,
            reads_arguments: false,
            passed,
            returns_to,
        });
        self.visit_expression(&call.callee);
        self.next = None;
    }

    fn push(&mut self, arrow: bool) {
        let frame = match self.next.take() {
            Some(frame) if !arrow => frame,
            _ => Frame {
                arrow,
                ..Frame::default()
            },
        };
        self.frames.push(frame);
    }

    fn pop(&mut self) {
        if let Some(frame) = self.frames.pop() {
            if frame.reads_arguments {
                for symbol in frame.passed {
                    self.open(symbol);
                }
            }
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        match self.symbol_of(it) {
            Some(symbol) => self.open(symbol),
            None if it.name.as_str() == "arguments" => {
                if let Some(frame) = self.frames.iter_mut().rev().find(|frame| !frame.arrow) {
                    frame.reads_arguments = true;
                }
            }
            None => {}
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.visit_unless_plain(&it.object);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.visit_unless_plain(&it.object);
        self.visit_expression(&it.expression);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        match it.as_member_expression() {
            Some(member) => self.written(member),
            None => walk_js::walk_simple_assignment_target(self, it),
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Delete => match it.argument.without_parentheses().as_member_expression()
            {
                Some(member) => self.written(member),
                None => self.visit_expression(&it.argument),
            },
            UnaryOperator::Typeof | UnaryOperator::Void => self.visit_unless_plain(&it.argument),
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_equality()
            || matches!(it.operator, BinaryOperator::Instanceof | BinaryOperator::In)
        {
            self.visit_unless_plain(&it.left);
            self.visit_unless_plain(&it.right);
            return;
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(function) = iife_callee(&it.callee) {
            self.iife_call(it, function);
            return;
        }
        self.declared_into = None;
        let callee = match self.plain_symbol(&it.callee) {
            Some(symbol) => symbol,
            None => {
                self.visit_expression(&it.callee);
                None
            }
        };
        self.argument_uses(it, callee);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.visit_unless_plain(&it.callee);
        for argument in &it.arguments {
            if argument.as_expression().is_some_and(is_prototype_literal) {
                self.open_all();
            }
            self.visit_argument(argument);
        }
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it
            .key
            .static_name()
            .and_then(|name| name.into_cow_str())
            .as_deref()
            == Some(PROTOTYPE)
        {
            self.open_all();
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        let Some(argument) = &it.argument else {
            return;
        };
        let target = self.frames.last().and_then(|frame| frame.returns_to);
        match (self.plain_symbol(argument), target) {
            (Some(Some(symbol)), Some(target)) => self.found.links.push((symbol, target)),
            _ => self.visit_expression(argument),
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.visit_binding_pattern(&it.id);
        let Some(init) = &it.init else {
            return;
        };
        let into = match &it.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        };
        if let (Some(into), Expression::CallExpression(call)) = (into, init.without_parentheses()) {
            if iife_callee(&call.callee).is_some() {
                self.declared_into = Some(into);
            } else if let Some(Expression::FunctionExpression(function)) =
                call.arguments.first().and_then(Argument::as_expression)
            {
                if let Some(own) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
                    self.found.links.push((own, into));
                }
            }
        }
        self.visit_expression(init);
        self.declared_into = None;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.push(false);
        walk_js::walk_function(self, it, flags);
        self.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.push(true);
        walk_js::walk_arrow_function_expression(self, it);
        self.pop();
    }
}
