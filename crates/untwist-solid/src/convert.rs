use std::cell::RefCell;

use oxc_ast::ast::{
    ArrayExpressionElement, AssignmentTarget, BinaryOperator, CallExpression, Expression,
    ObjectExpression, ObjectPropertyKind, PropertyKind, Statement, UnaryOperator,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_restore::html::Node;
use untwist_restore::text::{Piece, Rewriter, Site};

use crate::ast::{arg, args, binding, expr_symbol, is_identifier, is_simple, string, thunk};
use crate::element::Block;
use crate::jsx::{self, Child};
use crate::runtime::{Rt, Runtime};

pub type R<T> = Result<T, String>;

pub fn fail<T>(message: impl Into<String>) -> R<T> {
    Err(message.into())
}

pub struct Template {
    pub nodes: Vec<Node>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Jsx,
    Expr,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ctx {
    Insert,
    Children,
}

pub struct Cx<'a, 's> {
    pub source: &'s str,
    pub scoping: &'a Scoping,
    pub nodes: &'a AstNodes<'a>,
    pub runtime: &'a Runtime,
    pub templates: &'a FxHashMap<SymbolId, Template>,
    pub rw: &'a Rewriter<'s>,
    pub roots: RefCell<FxHashMap<Span, Kind>>,
    pub idents: Vec<(Span, SymbolId)>,
    pub ignored: RefCell<Vec<Span>>,
    pub inline: RefCell<FxHashMap<Span, String>>,
    pub deleted: RefCell<Vec<Span>>,
    pub library: RefCell<rustc_hash::FxHashSet<SymbolId>>,
    pub candidates: Vec<(Span, oxc_syntax::node::NodeId)>,
}

pub fn unwrap_parens(text: String) -> String {
    match text
        .strip_prefix("(\n")
        .and_then(|rest| rest.strip_suffix(")"))
        .map(str::trim_end)
    {
        Some(inner) if inner.trim_start().starts_with('<') => {
            jsx::indent_rest(inner.trim_start(), "")
        }
        _ => text,
    }
}

pub fn strip_bang<'e, 'a>(expr: &'e Expression<'a>) -> &'e Expression<'a> {
    if let Expression::UnaryExpression(outer) = expr.without_parentheses() {
        if outer.operator == UnaryOperator::LogicalNot {
            if let Expression::UnaryExpression(inner) = outer.argument.without_parentheses() {
                if inner.operator == UnaryOperator::LogicalNot {
                    return inner.argument.without_parentheses();
                }
            }
        }
    }
    expr
}

fn line_indent(source: &str, at: u32) -> &str {
    let start = source[..at as usize].rfind('\n').map_or(0, |at| at + 1);
    let line = &source[start..];
    &line[..line.len() - line.trim_start().len()]
}

pub fn candidates(nodes: &AstNodes<'_>) -> Vec<(Span, oxc_syntax::node::NodeId)> {
    let mut out: Vec<(Span, oxc_syntax::node::NodeId)> = nodes
        .iter()
        .filter(|inner| {
            matches!(
                inner.kind(),
                AstKind::CallExpression(_)
                    | AstKind::ArrayExpression(_)
                    | AstKind::SequenceExpression(_)
                    | AstKind::FunctionBody(_)
            )
        })
        .map(|inner| (inner.kind().span(), inner.id()))
        .collect();
    out.sort_by_key(|(span, _)| span.start);
    out
}

impl<'a, 's> Cx<'a, 's> {
    pub fn referenced_at(&self, span: Span) -> impl Iterator<Item = (Span, SymbolId)> + '_ {
        let first = self.idents.partition_point(|(at, _)| at.start < span.start);
        let ignored = self.ignored.borrow().clone();
        self.idents[first..]
            .iter()
            .take_while(move |(at, _)| at.start < span.end)
            .filter(move |(at, _)| at.end <= span.end)
            .filter(move |(at, _)| {
                !ignored
                    .iter()
                    .any(|skip| skip.start <= at.start && at.end <= skip.end)
            })
            .copied()
    }

    pub fn name_inline(&self, span: Span, name: &str) {
        self.inline.borrow_mut().insert(span, name.to_string());
        self.rw.add_site(Site {
            span,
            pieces: vec![Piece::Lit(name.to_string())],
        });
    }

    pub fn render(&self, expr: &Expression<'_>) -> R<String> {
        self.rw.render(expr.span())
    }

    pub fn render_value(&self, expr: &Expression<'_>) -> R<String> {
        let text = self.render(expr)?;
        Ok(
            match is_simple(expr)
                || matches!(expr.without_parentheses(), Expression::UnaryExpression(_))
            {
                true => text,
                false => format!("({text})"),
            },
        )
    }

    pub fn rt(&self, expr: &Expression<'_>) -> Option<Rt> {
        self.runtime.resolve(expr, self.scoping)
    }

    pub fn is(&self, expr: &Expression<'_>, rt: Rt) -> bool {
        self.rt(expr) == Some(rt)
    }

    pub fn template_of(&self, expr: &Expression<'_>) -> Option<&'a Template> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if !call.arguments.is_empty() {
            return None;
        }
        self.templates
            .get(&expr_symbol(&call.callee, self.scoping)?)
    }

    fn kind_of(&self, expr: &Expression<'_>) -> Option<Kind> {
        self.roots
            .borrow()
            .get(&expr.without_parentheses().span())
            .copied()
    }

    fn tail_block(
        &self,
        body: oxc_syntax::node::NodeId,
        statements: &[Statement<'_>],
    ) -> R<Option<(Span, String)>> {
        let Some(at) = Block::element_start(self, statements) else {
            return Ok(None);
        };
        let parent = self.nodes.parent_id(body);
        let called = matches!(self.nodes.kind(self.nodes.parent_id(parent)), AstKind::CallExpression(call) if call.callee.without_parentheses().span() == self.nodes.kind(parent).span());
        let getter = matches!(self.nodes.kind(parent), AstKind::Function(_))
            && matches!(
                self.nodes.kind(self.nodes.parent_id(parent)),
                AstKind::ObjectProperty(_)
            );
        if at == 0 && (called || getter) {
            return Ok(None);
        }
        let tail = &statements[at..];
        let Some(last) = tail.last() else {
            return Ok(None);
        };
        let text = Block::element(self, tail)?;
        let span = Span::new(tail[0].span().start, last.span().end);
        Ok(Some((span, text)))
    }

    fn memo_sequence(&self, items: &[Expression<'_>], span: Span) -> R<Option<String>> {
        let Some((last, memos)) = items.split_last() else {
            return Ok(None);
        };
        let mut bound = Vec::new();
        for item in memos {
            let Expression::AssignmentExpression(assign) = item.without_parentheses() else {
                return Ok(None);
            };
            let Some(symbol) = target_symbol(&assign.left, self.scoping) else {
                return Ok(None);
            };
            let Expression::CallExpression(call) = assign.right.without_parentheses() else {
                return Ok(None);
            };
            if !self.is(&call.callee, Rt::Memo) {
                return Ok(None);
            }
            bound.push(symbol);
        }
        if bound.is_empty() {
            return Ok(None);
        }
        let Some(body) = thunk(last) else {
            return Ok(None);
        };
        let text = self.render(body)?;
        self.drop_declarations(&bound, span);
        Ok(Some(text))
    }

    pub fn is_deleted(&self, statement: &Statement<'_>) -> bool {
        self.deleted.borrow().contains(&statement.span())
    }

    fn drop_declarations(&self, bound: &[SymbolId], inside: Span) {
        let mut seen: Vec<Span> = Vec::new();
        for symbol in bound {
            let declaration = self.scoping.symbol_declaration(*symbol);
            let AstKind::VariableDeclarator(declarator) = self.nodes.kind(declaration) else {
                continue;
            };
            if declarator.init.is_some() {
                continue;
            }
            let parent = self.nodes.parent_id(declaration);
            let AstKind::VariableDeclaration(statement) = self.nodes.kind(parent) else {
                continue;
            };
            let all = statement.declarations.iter().all(|declarator| {
                declarator.init.is_none()
                    && binding(&declarator.id).is_some_and(|symbol| bound.contains(&symbol))
            });
            let span = statement.span;
            let within = inside.start <= span.start && span.end <= inside.end;
            if !all || within || seen.contains(&span) {
                continue;
            }
            seen.push(span);
            self.deleted.borrow_mut().push(span);
            let start = self.source[..span.start as usize]
                .rfind('\n')
                .map_or(span.start, |at| at as u32);
            let used = self
                .scoping
                .get_resolved_references(*symbol)
                .any(|reference| {
                    let at = self.nodes.kind(reference.node_id()).span();
                    !(inside.start <= at.start && at.end <= inside.end)
                });
            if used {
                continue;
            }
            self.rw.add_site(Site {
                span: Span::new(start, span.end),
                pieces: Vec::new(),
            });
        }
    }

    fn register_tail(&self, span: Span, text: String) {
        let text = match text.contains('\n') {
            true => format!("return (\n  {}\n);", jsx::indent_rest(&text, "  ")),
            false => format!("return {text};"),
        };
        let text = jsx::indent_rest(&text, line_indent(self.source, span.start));
        self.roots.borrow_mut().insert(span, Kind::Jsx);
        self.rw.add_site(Site {
            span,
            pieces: vec![Piece::Lit(text)],
        });
    }

    fn register(&self, span: Span, kind: Kind, text: String) {
        let before = self.source[..span.start as usize].trim_end();
        let statement_value = before.ends_with("return")
            || before.ends_with("=>")
            || (before.ends_with('=')
                && !before.ends_with("==")
                && !before.ends_with("!=")
                && !before.ends_with("<=")
                && !before.ends_with(">="));
        let text = match kind == Kind::Jsx && statement_value && text.contains('\n') {
            true => format!("(\n  {}\n)", jsx::indent_rest(&text, "  ")),
            false => text,
        };
        let text = jsx::indent_rest(&text, line_indent(self.source, span.start));
        self.roots.borrow_mut().insert(span, kind);
        self.rw.add_site(Site {
            span,
            pieces: vec![Piece::Lit(text)],
        });
    }

    pub fn is_template_call(&self, call: &CallExpression<'_>) -> bool {
        call.arguments.is_empty()
            && expr_symbol(&call.callee, self.scoping)
                .is_some_and(|symbol| self.templates.contains_key(&symbol))
    }

    pub fn collect_node(&self, node: oxc_syntax::node::NodeId) -> R<()> {
        let owner = self.nodes.kind(node).span();
        let first = self
            .candidates
            .partition_point(|(span, _)| span.start < owner.start);
        let mut candidates: Vec<(Span, oxc_syntax::node::NodeId)> = self.candidates[first..]
            .iter()
            .take_while(|(span, _)| span.start < owner.end)
            .filter(|(span, _)| span.end <= owner.end)
            .copied()
            .collect();
        candidates.sort_by_key(|(span, _)| (span.size(), span.start));
        for (span, id) in candidates {
            if self.roots.borrow().contains_key(&span) {
                continue;
            }
            let found = match self.nodes.kind(id) {
                AstKind::CallExpression(call) => self.root_call(call, id)?,
                AstKind::ArrayExpression(array) => self.root_array(array)?,
                AstKind::SequenceExpression(sequence) => {
                    if let Some(found) = self.memo_sequence(&sequence.expressions, span)? {
                        self.register(span, Kind::Expr, found);
                        continue;
                    }
                    match Block::sequence(self, &sequence.expressions)? {
                        Some((text, bound)) => {
                            self.drop_declarations(&bound, span);
                            Some((Kind::Jsx, text))
                        }
                        None => None,
                    }
                }
                AstKind::FunctionBody(body) => {
                    if let Some((tail, text)) = self.tail_block(id, &body.statements)? {
                        self.register_tail(tail, text);
                    }
                    None
                }
                _ => None,
            };
            if let Some((kind, text)) = found {
                self.register(span, kind, text);
            }
        }
        Ok(())
    }

    fn root_array(&self, array: &oxc_ast::ast::ArrayExpression<'_>) -> R<Option<(Kind, String)>> {
        {
            {
                let mut strong = false;
                for element in &array.elements {
                    let Some(element) = element.as_expression() else {
                        return Ok(None);
                    };
                    if self.kind_of(element) == Some(Kind::Jsx) {
                        strong = true;
                    }
                }
                if !strong {
                    return Ok(None);
                }
                let mut children = Vec::new();
                for element in &array.elements {
                    if let ArrayExpressionElement::SpreadElement(_)
                    | ArrayExpressionElement::Elision(_) = element
                    {
                        return Ok(None);
                    }
                    if let Some(element) = element.as_expression() {
                        children.extend(self.child(element, Ctx::Children, true)?);
                    }
                }
                Ok(Some((Kind::Jsx, jsx::fragment(&children))))
            }
        }
    }

    fn condition(&self, value: &Expression<'_>, node: oxc_syntax::node::NodeId) -> R<String> {
        let at = self.nodes.kind(node).span();
        let counted = !std::ptr::eq(strip_bang(value), value.without_parentheses())
            || matches!(value.without_parentheses(), Expression::BinaryExpression(_));
        let tested = match self.nodes.parent_kind(node) {
            AstKind::ConditionalExpression(branch) => {
                branch.test.without_parentheses().span() == at
            }
            AstKind::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
            AstKind::LogicalExpression(logical) => {
                counted
                    && logical.operator == oxc_ast::ast::LogicalOperator::And
                    && logical.left.without_parentheses().span() == at
            }
            _ => false,
        };
        if !tested {
            return fail("a memo condition is read as a value");
        }
        self.render_value(strip_bang(value))
    }

    fn root_call(
        &self,
        call: &CallExpression<'_>,
        node: oxc_syntax::node::NodeId,
    ) -> R<Option<(Kind, String)>> {
        if self.is_template_call(call) {
            let symbol = expr_symbol(&call.callee, self.scoping);
            if let Some(template) = symbol.and_then(|symbol| self.templates.get(&symbol)) {
                return Ok(Some((Kind::Jsx, Block::static_element(self, template)?)));
            }
        }
        let callee = call.callee.without_parentheses();
        if let Expression::ArrowFunctionExpression(arrow) = callee {
            if call.arguments.is_empty()
                && arrow.params.items.is_empty()
                && arrow.params.rest.is_none()
            {
                if let Some(body) = arrow.get_function_body() {
                    let statements = body.statements.as_slice();
                    if Block::starts_element(self, statements) {
                        return Ok(Some((Kind::Jsx, Block::element(self, statements)?)));
                    }
                    if let Some(text) = self.memo_iife(statements)? {
                        return Ok(Some((Kind::Expr, text)));
                    }
                }
            }
            return Ok(None);
        }
        match self.rt(callee) {
            Some(Rt::CreateComponent) => Ok(Some((Kind::Jsx, self.component(call)?))),
            Some(Rt::Memo) => match (args(call).as_deref(), call.arguments.len()) {
                (Some([body]), 1) => match thunk(body) {
                    Some(value) => {
                        let text = self.render(value)?;
                        Ok(Some((Kind::Jsx, jsx::fragment(&[Child::Expr(text)]))))
                    }
                    None => Ok(None),
                },
                _ => Ok(None),
            },
            _ => {
                if !call.arguments.is_empty() {
                    return Ok(None);
                }
                if let Expression::CallExpression(inner) = callee {
                    if self.is(&inner.callee, Rt::Memo) && inner.arguments.len() == 1 {
                        if let Some(value) = arg(inner, 0).and_then(thunk) {
                            return Ok(Some((Kind::Expr, self.condition(value, node)?)));
                        }
                    }
                }
                if let Some(value) = self.memo_local(callee) {
                    return Ok(Some((Kind::Expr, self.condition(value, node)?)));
                }
                Ok(None)
            }
        }
    }

    fn memo_value(&self, symbol: SymbolId) -> Option<&'a Expression<'a>> {
        let AstKind::VariableDeclarator(declarator) =
            self.nodes.kind(self.scoping.symbol_declaration(symbol))
        else {
            return None;
        };
        if let Some(init) = &declarator.init {
            return Some(init);
        }
        let mut writes = self
            .scoping
            .get_resolved_references(symbol)
            .filter(|reference| reference.is_write());
        let write = writes.next()?;
        if writes.next().is_some() {
            return None;
        }
        match self.nodes.parent_kind(write.node_id()) {
            AstKind::AssignmentExpression(assign) => Some(&assign.right),
            _ => None,
        }
    }

    fn memo_local<'e>(&self, callee: &'e Expression<'_>) -> Option<&'a Expression<'a>> {
        let symbol = expr_symbol(callee, self.scoping)?;
        let Some(Expression::CallExpression(call)) =
            self.memo_value(symbol).map(Expression::without_parentheses)
        else {
            return None;
        };
        if !self.is(&call.callee, Rt::Memo) || call.arguments.len() != 1 {
            return None;
        }
        arg(call, 0).and_then(thunk)
    }

    fn memo_iife(&self, statements: &[Statement<'_>]) -> R<Option<String>> {
        let Some((last, decls)) = statements.split_last() else {
            return Ok(None);
        };
        if decls.is_empty() {
            return Ok(None);
        }
        for statement in decls {
            let Statement::VariableDeclaration(declaration) = statement else {
                return Ok(None);
            };
            for declarator in &declaration.declarations {
                let Some(Expression::CallExpression(call)) = declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses)
                else {
                    return Ok(None);
                };
                if !self.is(&call.callee, Rt::Memo) || binding(&declarator.id).is_none() {
                    return Ok(None);
                }
            }
        }
        let Statement::ReturnStatement(ret) = last else {
            return Ok(None);
        };
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            ret.argument.as_ref().map(Expression::without_parentheses)
        else {
            return Ok(None);
        };
        if !arrow.params.items.is_empty() || arrow.params.rest.is_some() {
            return Ok(None);
        }
        if let Some(value) = arrow.get_expression() {
            return Ok(Some(self.render(value)?));
        }
        let statements = crate::ast::arrow_statements(arrow);
        self.returned_text(statements).map(Some)
    }

    pub fn returned_text(&self, statements: &[Statement<'_>]) -> R<String> {
        let skip = statements
            .iter()
            .take_while(|statement| self.is_deleted(statement))
            .count();
        match &statements[skip..] {
            [Statement::ReturnStatement(ret)] => match &ret.argument {
                Some(value) => self.render(value),
                None => Ok("undefined".to_string()),
            },
            [Statement::BlockStatement(block)] => self.returned_text(&block.body),
            [Statement::IfStatement(branch)] => {
                let Some(alternate) = &branch.alternate else {
                    return fail("a getter returns only on one branch");
                };
                self.ternary(
                    &branch.test,
                    &branch.consequent,
                    std::slice::from_ref(alternate),
                )
            }
            [Statement::IfStatement(branch), rest @ ..] if branch.alternate.is_none() => {
                self.ternary(&branch.test, &branch.consequent, rest)
            }
            _ => fail("a getter or render function body is not a single returned value"),
        }
    }

    fn ternary(
        &self,
        test: &Expression<'_>,
        consequent: &Statement<'_>,
        alternate: &[Statement<'_>],
    ) -> R<String> {
        let yes = self.returned_text(std::slice::from_ref(consequent))?;
        let no = self.returned_text(alternate)?;
        let wrap = |text: String| match text.contains(" ? ") || text.contains("=>") {
            true => format!("({text})"),
            false => text,
        };
        Ok(format!(
            "{} ? {} : {}",
            self.render_value(test)?,
            wrap(yes),
            no
        ))
    }

    pub fn child(&self, expr: &Expression<'_>, ctx: Ctx, fixed: bool) -> R<Vec<Child>> {
        let expr = expr.without_parentheses();
        if let Some(text) = string(expr) {
            return Ok(vec![Child::Str(text.to_string())]);
        }
        if let Expression::CallExpression(call) = expr {
            if self.is(&call.callee, Rt::Memo) && call.arguments.len() == 1 {
                if let Some(value) = arg(call, 0).and_then(thunk) {
                    return Ok(vec![Child::Expr(self.render(value)?)]);
                }
            }
        }
        if let (Ctx::Children, Expression::ArrayExpression(array)) = (ctx, expr) {
            let mut out = Vec::new();
            for element in &array.elements {
                let Some(element) = element.as_expression() else {
                    return fail("a children array holds a spread or a hole");
                };
                out.extend(self.child(element, ctx, true)?);
            }
            return Ok(out);
        }
        match self.kind_of(expr) {
            Some(Kind::Jsx) => return Ok(vec![Child::Jsx(unwrap_parens(self.render(expr)?))]),
            Some(Kind::Expr) => return Ok(vec![Child::Expr(self.render(expr)?)]),
            None => {}
        }
        if ctx == Ctx::Insert {
            if let Some(value) = thunk(expr) {
                return Ok(vec![Child::Expr(self.render(value)?)]);
            }
            if self.is_accessor(expr) {
                return Ok(vec![Child::Expr(format!("{}()", self.render(expr)?))]);
            }
        }
        let text = self.render(expr)?;
        Ok(vec![Child::Expr(match fixed {
            true => crate::ast::once(expr, text),
            false => text,
        })])
    }

    pub fn api_name(&self, callee: &Expression<'_>) -> Option<String> {
        let symbol = expr_symbol(callee, self.scoping)?;
        if let Some(rt) = self.runtime.symbols.get(&symbol) {
            return rt.spelling().map(|(name, _)| name.to_string());
        }
        let declaration = self.scoping.symbol_declaration(symbol);
        let AstKind::ImportSpecifier(named) = self.nodes.kind(declaration) else {
            return None;
        };
        let AstKind::ImportDeclaration(import) = self.nodes.kind(self.nodes.parent_id(declaration))
        else {
            return None;
        };
        import
            .source
            .value
            .as_str()
            .is_some_and(|source| source.starts_with("solid-js"))
            .then(|| named.imported.name().to_string())
    }

    fn is_accessor(&self, expr: &Expression<'_>) -> bool {
        let Some(symbol) = expr_symbol(expr, self.scoping) else {
            return false;
        };
        if self.scoping.symbol_is_mutated(symbol) {
            return false;
        }
        let declaration = self.scoping.symbol_declaration(symbol);
        let AstKind::VariableDeclarator(declarator) = self.nodes.kind(declaration) else {
            return false;
        };
        let Some(Expression::CallExpression(call)) = declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        else {
            return false;
        };
        let Some(api) = self.api_name(&call.callee) else {
            return false;
        };
        match &declarator.id {
            oxc_ast::ast::BindingPattern::BindingIdentifier(_) => {
                matches!(
                    api.as_str(),
                    "createMemo" | "children" | "createDeferred" | "createSelector"
                )
            }
            oxc_ast::ast::BindingPattern::ArrayPattern(array) => {
                matches!(api.as_str(), "createSignal" | "createResource")
                    && array
                        .elements
                        .first()
                        .and_then(Option::as_ref)
                        .and_then(binding)
                        == Some(symbol)
            }
            _ => false,
        }
    }

    pub fn component(&self, call: &CallExpression<'_>) -> R<String> {
        let Some(callee) = arg(call, 0) else {
            return fail("createComponent has no component");
        };
        let name = self.render(callee)?;
        let inline = self
            .inline
            .borrow()
            .contains_key(&callee.without_parentheses().span());
        let named = inline
            || match callee.without_parentheses() {
                Expression::Identifier(_) => name.starts_with(|c: char| c.is_ascii_uppercase()),
                Expression::StaticMemberExpression(_) => name.split('.').all(is_identifier),
                _ => false,
            };
        if !named {
            return fail(format!("the component `{name}` has no JSX spelling"));
        }
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        match arg(call, 1).map(Expression::without_parentheses) {
            None => {}
            Some(Expression::ObjectExpression(object)) => {
                self.props_object(object, &mut attrs, &mut children)?
            }
            Some(Expression::CallExpression(merge)) if self.is(&merge.callee, Rt::MergeProps) => {
                for part in args(merge).unwrap_or_default() {
                    match part.without_parentheses() {
                        Expression::ObjectExpression(object) => {
                            self.props_object(object, &mut attrs, &mut children)?
                        }
                        other => attrs.push(format!("{{...{}}}", self.render(other)?)),
                    }
                }
            }
            Some(other) => attrs.push(format!("{{...{}}}", self.render(other)?)),
        }
        Ok(jsx::element(&name, &attrs, &children))
    }

    pub fn props_object(
        &self,
        object: &ObjectExpression<'_>,
        attrs: &mut Vec<String>,
        children: &mut Vec<Child>,
    ) -> R<()> {
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return fail("a props object holds a spread");
            };
            if property.computed {
                return fail("a props object holds a computed key");
            }
            let Some(key) = property
                .key
                .static_name()
                .and_then(|key| key.as_str().map(str::to_string))
            else {
                return fail("a props object holds a key with no name");
            };
            if !is_attribute_name(&key) {
                return fail(format!("the prop `{key}` has no JSX spelling"));
            }
            let getter = property.kind == PropertyKind::Get;
            if property.method || getter {
                let Expression::FunctionExpression(function) = &property.value else {
                    return fail(format!("the prop `{key}` is not a function"));
                };
                let statements = function
                    .body
                    .as_ref()
                    .map_or(&[][..], |body| body.statements.as_slice());
                if property.method {
                    if key != "ref" {
                        return fail(format!("the prop `{key}` is a method"));
                    }
                    attrs.push(format!(
                        "ref={{{}}}",
                        self.component_ref(function, statements)?
                    ));
                    continue;
                }
                let value = self.getter(statements)?;
                match (key.as_str(), value) {
                    ("children", Getter::Value(expr)) => {
                        children.extend(self.child(expr, Ctx::Children, false)?)
                    }
                    ("children", Getter::Jsx(text)) => children.push(Child::Jsx(text)),
                    ("children", Getter::Text(text)) => children.push(Child::Expr(text)),
                    (_, Getter::Value(expr)) => {
                        attrs.push(format!("{key}={{{}}}", unwrap_parens(self.render(expr)?)))
                    }
                    (_, Getter::Jsx(text) | Getter::Text(text)) => {
                        attrs.push(format!("{key}={{{text}}}"))
                    }
                }
                continue;
            }
            if key == "children" {
                children.extend(self.child(&property.value, Ctx::Children, true)?);
                continue;
            }
            attrs.push(self.static_prop(&key, &property.value)?);
        }
        Ok(())
    }

    fn static_prop(&self, key: &str, value: &Expression<'_>) -> R<String> {
        if let Some(text) = string(value) {
            if !text.contains(['"', '\\', '\n', '\r', '&', '{', '}', '<', '>']) {
                return Ok(format!("{key}=\"{text}\""));
            }
        }
        if let Expression::BooleanLiteral(literal) = value.without_parentheses() {
            if literal.value {
                return Ok(key.to_string());
            }
        }
        Ok(format!(
            "{key}={{{}}}",
            crate::ast::once(value, self.render(value)?)
        ))
    }

    fn getter<'e, 'b>(&self, statements: &'e [Statement<'b>]) -> R<Getter<'e, 'b>> {
        let skip = statements
            .iter()
            .take_while(|statement| self.is_deleted(statement))
            .count();
        let statements = &statements[skip..];
        if Block::starts_element(self, statements) {
            return Ok(Getter::Jsx(Block::element(self, statements)?));
        }
        if let [Statement::ReturnStatement(ret)] = statements {
            if let Some(value) = &ret.argument {
                return Ok(Getter::Value(value));
            }
        }
        Ok(Getter::Text(self.returned_text(statements)?))
    }

    fn component_ref(
        &self,
        function: &oxc_ast::ast::Function<'_>,
        statements: &[Statement<'_>],
    ) -> R<String> {
        let param = function
            .params
            .items
            .first()
            .and_then(|param| binding(&param.pattern));
        let (Some(param), [Statement::VariableDeclaration(declaration), branch]) =
            (param, statements)
        else {
            return fail("a ref prop has an unknown shape");
        };
        let [declarator] = declaration.declarations.as_slice() else {
            return fail("a ref prop has an unknown shape");
        };
        let (Some(holder), Some(init)) = (binding(&declarator.id), declarator.init.as_ref()) else {
            return fail("a ref prop has an unknown shape");
        };
        let target = self.rw.text(init.span());
        let calls_holder = |expr: &Expression<'_>| match expr.without_parentheses() {
            Expression::CallExpression(call) => {
                expr_symbol(&call.callee, self.scoping) == Some(holder)
                    && call.arguments.len() == 1
                    && arg(call, 0).and_then(|a| expr_symbol(a, self.scoping)) == Some(param)
            }
            _ => false,
        };
        let assigns = |expr: &Expression<'_>| assigns_to(self, expr, target, param);
        if ref_branch(self, branch, holder, &calls_holder, &assigns) {
            return self.render(init);
        }
        fail("a ref prop has an unknown shape")
    }
}

pub enum Getter<'e, 'a> {
    Value(&'e Expression<'a>),
    Jsx(String),
    Text(String),
}

pub fn assigns_to(cx: &Cx<'_, '_>, expr: &Expression<'_>, target: &str, value: SymbolId) -> bool {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return false;
    };
    assign.operator.as_str() == "="
        && cx.rw.text(assign.left.span()) == target
        && expr_symbol(&assign.right, cx.scoping) == Some(value)
}

pub fn ref_branch(
    cx: &Cx<'_, '_>,
    statement: &Statement<'_>,
    holder: SymbolId,
    yes: &dyn Fn(&Expression<'_>) -> bool,
    no: &dyn Fn(&Expression<'_>) -> bool,
) -> bool {
    let single = |statement: &Statement<'_>, test: &dyn Fn(&Expression<'_>) -> bool| -> bool {
        match statement {
            Statement::ExpressionStatement(statement) => test(&statement.expression),
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [Statement::ExpressionStatement(statement)] => test(&statement.expression),
                _ => false,
            },
            _ => false,
        }
    };
    match statement {
        Statement::IfStatement(branch) => {
            is_function_test(cx, &branch.test, holder)
                && single(&branch.consequent, yes)
                && branch
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| single(alternate, no))
        }
        Statement::ExpressionStatement(statement) => {
            match statement.expression.without_parentheses() {
                Expression::ConditionalExpression(cond) => {
                    is_function_test(cx, &cond.test, holder)
                        && yes(&cond.consequent)
                        && no(&cond.alternate)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_function_test(cx: &Cx<'_, '_>, test: &Expression<'_>, holder: SymbolId) -> bool {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return false;
    };
    if !matches!(
        binary.operator,
        BinaryOperator::StrictEquality | BinaryOperator::Equality
    ) {
        return false;
    }
    let Expression::UnaryExpression(unary) = binary.left.without_parentheses() else {
        return false;
    };
    unary.operator == UnaryOperator::Typeof
        && expr_symbol(&unary.argument, cx.scoping) == Some(holder)
        && string(&binary.right) == Some("function")
}

pub fn is_attribute_name(name: &str) -> bool {
    let mut parts = name.splitn(2, ':');
    let head = parts.next().unwrap_or_default();
    let ok = |part: &str| {
        part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-'))
    };
    ok(head) && parts.next().is_none_or(ok)
}

pub fn target_symbol(target: &AssignmentTarget<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(ident) => {
            crate::ast::symbol_of(ident, scoping)
        }
        _ => None,
    }
}

pub fn member_target<'e, 'a>(
    target: &'e AssignmentTarget<'a>,
) -> Option<(&'e Expression<'a>, &'e str)> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        _ => None,
    }
}
