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

impl<'a, 's> Cx<'a, 's> {
    pub fn referenced(&self, span: Span) -> impl Iterator<Item = SymbolId> + '_ {
        let first = self.idents.partition_point(|(at, _)| at.start < span.start);
        self.idents[first..]
            .iter()
            .take_while(move |(at, _)| at.end <= span.end)
            .map(|(_, symbol)| *symbol)
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
        let mut candidates: Vec<(Span, oxc_syntax::node::NodeId)> = self
            .nodes
            .iter()
            .filter(|inner| {
                matches!(
                    inner.kind(),
                    AstKind::CallExpression(_) | AstKind::ArrayExpression(_)
                )
            })
            .map(|inner| (inner.kind().span(), inner.id()))
            .filter(|(span, _)| owner.start <= span.start && span.end <= owner.end)
            .collect();
        candidates.sort_by_key(|(span, _)| (span.size(), span.start));
        for (span, id) in candidates {
            if self.roots.borrow().contains_key(&span) {
                continue;
            }
            let found = match self.nodes.kind(id) {
                AstKind::CallExpression(call) => self.root_call(call)?,
                AstKind::ArrayExpression(array) => self.root_array(array)?,
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
                        children.extend(self.child(element, Ctx::Children)?);
                    }
                }
                Ok(Some((Kind::Jsx, jsx::fragment(&children))))
            }
        }
    }

    fn root_call(&self, call: &CallExpression<'_>) -> R<Option<(Kind, String)>> {
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
                            return Ok(Some((Kind::Expr, self.render_value(strip_bang(value))?)));
                        }
                    }
                }
                if let Some(value) = self.memo_local(callee) {
                    return Ok(Some((Kind::Expr, self.render_value(strip_bang(value))?)));
                }
                Ok(None)
            }
        }
    }

    fn memo_local<'e>(&self, callee: &'e Expression<'_>) -> Option<&'a Expression<'a>> {
        let symbol = expr_symbol(callee, self.scoping)?;
        let AstKind::VariableDeclarator(declarator) =
            self.nodes.kind(self.scoping.symbol_declaration(symbol))
        else {
            return None;
        };
        let Some(Expression::CallExpression(call)) = declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
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
        match statements {
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

    pub fn child(&self, expr: &Expression<'_>, ctx: Ctx) -> R<Vec<Child>> {
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
                out.extend(self.child(element, ctx)?);
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
        Ok(vec![Child::Expr(self.render(expr)?)])
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
        let named = match callee.without_parentheses() {
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
                        children.extend(self.child(expr, Ctx::Children)?)
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
                children.extend(self.child(&property.value, Ctx::Children)?);
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
        Ok(format!("{key}={{{}}}", self.render(value)?))
    }

    fn getter<'e, 'b>(&self, statements: &'e [Statement<'b>]) -> R<Getter<'e, 'b>> {
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
