use std::cell::RefCell;

use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::builtins::{self, Owner};
use crate::infer::{global_name, resolved, Env};
use crate::ty::{Kind, Ty};

const VIEW_DEPTH: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Any,
    Known(Ty),
    Unknown,
}

impl View {
    pub fn join(&self, other: &View) -> View {
        match (self, other) {
            (View::Any, _) | (_, View::Any) => View::Any,
            (View::Unknown, _) | (_, View::Unknown) => View::Unknown,
            (View::Known(a), View::Known(b)) => View::Known(a.join(b)),
        }
    }

    pub fn widened(&self) -> View {
        match self {
            View::Known(ty) => View::Known(ty.widened()),
            other => other.clone(),
        }
    }

    pub fn fits(&self, target: &Ty) -> bool {
        match self {
            View::Any => true,
            View::Known(ty) => target.contains(ty),
            View::Unknown => false,
        }
    }

    pub fn only(&self, kind: Kind) -> bool {
        matches!(self, View::Known(ty) if ty.only(kind))
    }

    pub fn has(&self, kind: Kind) -> bool {
        matches!(self, View::Known(ty) if ty.any_of(kind))
    }
}

pub struct Annotations {
    pub symbols: FxHashMap<SymbolId, Ty>,
    pub returns: FxHashMap<usize, Ty>,
}

pub struct Viewer<'e, 'f, 'a> {
    pub env: &'e Env<'f, 'a>,
    pub notes: &'e Annotations,
    memo: RefCell<FxHashMap<SymbolId, View>>,
    open: RefCell<FxHashSet<SymbolId>>,
}

impl<'e, 'f, 'a> Viewer<'e, 'f, 'a> {
    pub fn new(env: &'e Env<'f, 'a>, notes: &'e Annotations) -> Self {
        Self {
            env,
            notes,
            memo: RefCell::default(),
            open: RefCell::default(),
        }
    }

    pub fn view(&self, expression: &Expression<'a>) -> View {
        self.view_at(expression, 0)
    }

    fn view_at(&self, expression: &Expression<'a>, depth: usize) -> View {
        if depth > VIEW_DEPTH {
            return View::Unknown;
        }
        let next = depth + 1;
        match expression {
            Expression::StringLiteral(literal) => View::Known(match literal.value.as_str() {
                Some(text) => Ty::string_literal(text),
                None => Ty::string(),
            }),
            Expression::NumericLiteral(literal) => View::Known(Ty::number_literal(literal.value)),
            Expression::BigIntLiteral(_) => View::Known(Ty::bigint()),
            Expression::BooleanLiteral(literal) => View::Known(Ty::boolean_literal(literal.value)),
            Expression::TemplateLiteral(_) => View::Known(Ty::string()),
            Expression::Identifier(ident) => self.ident(ident, next),
            Expression::ParenthesizedExpression(inner) => self.view_at(&inner.expression, next),
            Expression::UnaryExpression(unary) => self.unary(unary, next),
            Expression::BinaryExpression(binary) => binary_view(
                binary.operator,
                &self.view_at(&binary.left, next),
                &self.view_at(&binary.right, next),
            ),
            Expression::LogicalExpression(logical) => self
                .view_at(&logical.left, next)
                .join(&self.view_at(&logical.right, next)),
            Expression::ConditionalExpression(conditional) => self
                .view_at(&conditional.consequent, next)
                .join(&self.view_at(&conditional.alternate, next)),
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .map_or(View::Unknown, |last| self.view_at(last, next)),
            Expression::AssignmentExpression(assign) => {
                match assign.operator.to_binary_operator() {
                    None if assign.operator.is_assign() => self.view_at(&assign.right, next),
                    None => View::Unknown,
                    Some(operator) => {
                        let left = match &assign.left {
                            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                                self.ident(ident, next)
                            }
                            _ => View::Unknown,
                        };
                        binary_view(operator, &left, &self.view_at(&assign.right, next))
                    }
                }
            }
            Expression::UpdateExpression(_) => View::Known(Ty::number()),
            Expression::AwaitExpression(awaited) => match self.view_at(&awaited.argument, next) {
                View::Known(ty) if ty.promise_free() => View::Known(ty),
                View::Any => View::Any,
                _ => View::Unknown,
            },
            Expression::CallExpression(call) => self.call(call),
            Expression::StaticMemberExpression(member) => self.member(member, next),
            Expression::ComputedMemberExpression(member) => {
                match self.view_at(&member.object, next) {
                    View::Any => View::Any,
                    _ => View::Unknown,
                }
            }
            _ => View::Unknown,
        }
    }

    fn unary(&self, unary: &UnaryExpression<'a>, depth: usize) -> View {
        match unary.operator {
            UnaryOperator::LogicalNot | UnaryOperator::Delete => View::Known(Ty::boolean()),
            UnaryOperator::Typeof => View::Known(Ty::string()),
            UnaryOperator::Void => View::Known(Ty::undefined()),
            UnaryOperator::UnaryPlus => View::Known(Ty::number()),
            UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot => {
                match self.view_at(&unary.argument, depth) {
                    View::Known(ty) if ty.only(Kind::BigInt) => View::Known(Ty::bigint()),
                    View::Known(ty) if ty.any_of(Kind::BigInt) => View::Unknown,
                    _ => View::Known(Ty::number()),
                }
            }
        }
    }

    fn member(&self, member: &StaticMemberExpression<'a>, depth: usize) -> View {
        let key = member.property.name.as_str();
        if let Some(root) = self.env.static_owner(&member.object) {
            return match builtins::find(Owner::Static(static_name(root)), key, false) {
                Some(row) => View::Known(row.lib.ty()),
                None => View::Unknown,
            };
        }
        match self.view_at(&member.object, depth) {
            View::Any => View::Any,
            View::Known(ty) if key == "length" && ty.only(Kind::String) => {
                View::Known(Ty::number())
            }
            _ => View::Unknown,
        }
    }

    fn call(&self, call: &CallExpression<'a>) -> View {
        if call.optional {
            return View::Unknown;
        }
        match call.callee.without_parentheses() {
            Expression::Identifier(ident) => {
                if let Some(symbol) = resolved(self.env.cx, ident) {
                    let function = self.env.facts.by_symbol.get(&symbol);
                    return match function.and_then(|index| self.notes.returns.get(index)) {
                        Some(ty) => View::Known(ty.clone()),
                        None => View::Unknown,
                    };
                }
                match global_name(self.env.cx, ident)
                    .and_then(|name| builtins::find(Owner::Global, name, true))
                {
                    Some(row) => View::Known(row.lib.ty()),
                    None => View::Unknown,
                }
            }
            Expression::FunctionExpression(function) => self.iife(function.node_id.get()),
            Expression::ArrowFunctionExpression(arrow) => self.iife(arrow.node_id.get()),
            Expression::StaticMemberExpression(member) => {
                let key = member.property.name.as_str();
                if let Some(root) = self.env.static_owner(&member.object) {
                    return match builtins::find(Owner::Static(static_name(root)), key, true) {
                        Some(row) => View::Known(row.lib.ty()),
                        None => View::Unknown,
                    };
                }
                match self.view(&member.object) {
                    View::Any => View::Any,
                    View::Known(ty) => match ty.single_kind() {
                        Some(kind) => builtins::find(Owner::Proto(kind), key, true)
                            .map_or(View::Unknown, |row| View::Known(row.lib.ty())),
                        None => View::Unknown,
                    },
                    View::Unknown => View::Unknown,
                }
            }
            _ => View::Unknown,
        }
    }

    fn iife(&self, node: oxc_syntax::node::NodeId) -> View {
        match self
            .env
            .facts
            .by_node
            .get(&node)
            .and_then(|index| self.notes.returns.get(index))
        {
            Some(ty) => View::Known(ty.clone()),
            None => View::Unknown,
        }
    }

    fn ident(&self, ident: &IdentifierReference<'a>, depth: usize) -> View {
        let Some(symbol) = resolved(self.env.cx, ident) else {
            return match global_name(self.env.cx, ident) {
                Some("undefined") => View::Known(Ty::undefined()),
                Some("NaN" | "Infinity") => View::Known(Ty::number()),
                _ => View::Unknown,
            };
        };
        self.symbol(symbol, depth)
    }

    pub fn symbol(&self, symbol: SymbolId, depth: usize) -> View {
        if let Some(ty) = self.notes.symbols.get(&symbol) {
            return View::Known(ty.clone());
        }
        if let Some(view) = self.memo.borrow().get(&symbol) {
            return view.clone();
        }
        if !self.open.borrow_mut().insert(symbol) {
            return View::Unknown;
        }
        let view = self.declared(symbol, depth);
        self.open.borrow_mut().remove(&symbol);
        self.memo.borrow_mut().insert(symbol, view.clone());
        view
    }

    pub fn declared_without(&self, symbol: SymbolId) -> View {
        self.declared(symbol, 0)
    }

    fn declared(&self, symbol: SymbolId, depth: usize) -> View {
        let cx = self.env.cx;
        if !cx.scoping.symbol_redeclarations(symbol).is_empty() {
            return View::Unknown;
        }
        let declaration = cx.scoping.symbol_declaration(symbol);
        match cx.nodes.kind(declaration) {
            AstKind::VariableDeclarator(declarator) => {
                if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
                    return View::Unknown;
                }
                match &declarator.init {
                    Some(init) => {
                        let view = self.view_at(init, depth);
                        let nullish = matches!(&view, View::Known(ty) if ty.is_nullable());
                        match (crate::facts::declaration_kind(cx.nodes, declarator).is_const(), nullish) {
                            (_, true) => View::Unknown,
                            (true, false) => view,
                            (false, false) => view.widened(),
                        }
                    }
                    None => View::Unknown,
                }
            }
            AstKind::FormalParameter(param) => self.param_view(declaration, param, depth),
            _ => View::Unknown,
        }
    }

    fn param_view(
        &self,
        declaration: oxc_syntax::node::NodeId,
        param: &FormalParameter<'a>,
        depth: usize,
    ) -> View {
        if let Some(default) = &param.initializer {
            return self.view_at(default, depth).widened();
        }
        let nodes = self.env.cx.nodes;
        let Some(function_node) = nodes
            .ancestor_ids(declaration)
            .skip(1)
            .find(|id| matches!(nodes.kind(*id), AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)))
        else {
            return View::Unknown;
        };
        let Some(&index) = self.env.facts.by_node.get(&function_node) else {
            return View::Unknown;
        };
        let fact = &self.env.facts.functions[index];
        if fact.closed && fact.symbol.is_none() {
            return View::Unknown;
        }
        match nodes.parent_kind(function_node) {
            AstKind::Program(_) | AstKind::FunctionBody(_) | AstKind::BlockStatement(_) => {
                View::Any
            }
            AstKind::VariableDeclarator(_) => View::Any,
            AstKind::MethodDefinition(_) => View::Any,
            AstKind::ExportNamedDeclaration(_) | AstKind::ExportDefaultDeclaration(_) => View::Any,
            _ => View::Unknown,
        }
    }
}

pub fn static_name(root: &str) -> &'static str {
    builtins::ROWS
        .iter()
        .find_map(|row| match row.owner {
            Owner::Static(name) if name == root => Some(name),
            _ => None,
        })
        .unwrap_or("")
}

pub fn binary_view(operator: BinaryOperator, left: &View, right: &View) -> View {
    if operator.is_equality()
        || operator.is_compare()
        || matches!(operator, BinaryOperator::In | BinaryOperator::Instanceof)
    {
        return View::Known(Ty::boolean());
    }
    if operator == BinaryOperator::Addition {
        if left.only(Kind::String) || right.only(Kind::String) {
            return View::Known(Ty::string());
        }
        if matches!(left, View::Any) || matches!(right, View::Any) {
            return View::Any;
        }
        if left.only(Kind::Number) && right.only(Kind::Number) {
            return View::Known(Ty::number());
        }
        if left.only(Kind::BigInt) && right.only(Kind::BigInt) {
            return View::Known(Ty::bigint());
        }
        return View::Unknown;
    }
    if operator == BinaryOperator::ShiftRightZeroFill {
        return View::Known(Ty::number());
    }
    if left.only(Kind::BigInt) && right.only(Kind::BigInt) {
        return View::Known(Ty::bigint());
    }
    if left.has(Kind::BigInt) || right.has(Kind::BigInt) {
        return View::Unknown;
    }
    View::Known(Ty::number())
}
