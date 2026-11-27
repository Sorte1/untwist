use oxc_ast::ast::{
    Class, ClassElement, Declaration, ExportDefaultDeclarationKind, Expression, Statement,
    UnaryOperator,
};
use oxc_ast::AstKind;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::ast::expr_symbol;
use crate::convert::Cx;
use crate::module::{helper_owner, inside, Found, Unit};
use crate::runtime::Rt;

pub struct Effect {
    pub line: String,
    span: Span,
    symbols: FxHashSet<SymbolId>,
    owners: Vec<Span>,
}

impl Effect {
    pub fn touches(
        &self,
        cx: &Cx<'_, '_>,
        home: ScopeId,
        built: &Unit,
        shared: &FxHashSet<SymbolId>,
    ) -> bool {
        if built
            .spans
            .iter()
            .any(|span| inside(*span, self.span) || inside(self.span, *span))
            || shared
                .iter()
                .any(|symbol| inside(self.span, cx.scoping.symbol_span(*symbol)))
        {
            return false;
        }
        let seeds: Vec<SymbolId> = built
            .reads
            .iter()
            .filter(|symbol| shared.contains(symbol))
            .copied()
            .collect();
        let through: Vec<Span> = reach(cx, home, &seeds, &[])
            .into_iter()
            .filter_map(|symbol| helper_owner(cx, symbol).map(|(owner, _)| owner))
            .collect();
        self.symbols
            .iter()
            .any(|symbol| built.reads.contains(symbol))
            || self.owners.iter().any(|owner| {
                through.contains(owner) || built.spans.iter().any(|span| inside(*span, *owner))
            })
    }
}

fn runs_in(scoping: &Scoping, from: ScopeId, home: ScopeId) -> bool {
    for scope in scoping.scope_ancestors(from) {
        if scope == home {
            return true;
        }
        if scoping.scope_flags(scope).is_function() {
            return false;
        }
    }
    false
}

fn effect_sites(cx: &Cx<'_, '_>, home: ScopeId) -> Vec<u32> {
    let mut sites: Vec<u32> = cx
        .nodes
        .iter()
        .filter(|node| match node.kind() {
            AstKind::CallExpression(_)
            | AstKind::NewExpression(_)
            | AstKind::AssignmentExpression(_)
            | AstKind::UpdateExpression(_)
            | AstKind::TaggedTemplateExpression(_)
            | AstKind::AwaitExpression(_) => true,
            AstKind::UnaryExpression(unary) => unary.operator == UnaryOperator::Delete,
            _ => false,
        })
        .filter(|node| runs_in(cx.scoping, node.scope_id(), home))
        .map(|node| node.kind().span().start)
        .collect();
    sites.sort_unstable();
    sites
}

fn runs(sites: &[u32], span: Span) -> bool {
    let at = sites.partition_point(|site| *site < span.start);
    sites.get(at).is_some_and(|site| *site < span.end)
}

fn class_runs(sites: &[u32], class: &Class<'_>) -> bool {
    class
        .heritage
        .as_ref()
        .is_some_and(|base| runs(sites, base.expression.span()))
        || class.body.body.iter().any(|element| match element {
            ClassElement::StaticBlock(_) => true,
            ClassElement::PropertyDefinition(property) => {
                property.computed || (property.r#static && property.value.is_some())
            }
            ClassElement::MethodDefinition(method) => method.computed,
            _ => false,
        })
}

fn is_effect(cx: &Cx<'_, '_>, sites: &[u32], statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ImportDeclaration(_)
        | Statement::FunctionDeclaration(_)
        | Statement::ExportAllDeclaration(_)
        | Statement::EmptyStatement(_) => false,
        Statement::ClassDeclaration(class) => class_runs(sites, class),
        Statement::VariableDeclaration(declaration) => runs(sites, declaration.span),
        Statement::ExportNamedDeclaration(_) | Statement::ExportFromDeclaration(_) => false,
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => runs(sites, declaration.span),
            Declaration::ClassDeclaration(class) => class_runs(sites, class),
            _ => false,
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(_)
            | ExportDefaultDeclarationKind::Identifier(_) => false,
            ExportDefaultDeclarationKind::ClassDeclaration(class) => class_runs(sites, class),
            _ => true,
        },
        Statement::ExpressionStatement(statement) => {
            match statement.expression.without_parentheses() {
                Expression::CallExpression(call) => {
                    !matches!(cx.rt(&call.callee), Some(Rt::DelegateEvents))
                        && cx.api_name(&call.callee).as_deref() != Some("render")
                }
                Expression::StringLiteral(_) => false,
                _ => true,
            }
        }
        _ => true,
    }
}

fn object_assigned(cx: &Cx<'_, '_>, call: &oxc_ast::ast::CallExpression<'_>) -> Option<SymbolId> {
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    let global = object.name == "Object" && expr_symbol(&member.object, cx.scoping).is_none();
    let writes = matches!(
        member.property.name.as_str(),
        "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
    );
    match global && writes {
        true => crate::ast::arg(call, 0).and_then(|first| expr_symbol(first, cx.scoping)),
        false => None,
    }
}

fn written_object(cx: &Cx<'_, '_>, statement: &Statement<'_>) -> Option<SymbolId> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    match statement.expression.without_parentheses() {
        Expression::AssignmentExpression(assign) => assign
            .left
            .as_member_expression()
            .and_then(|member| expr_symbol(member.object(), cx.scoping)),
        Expression::UpdateExpression(update) => update
            .argument
            .as_member_expression()
            .and_then(|member| expr_symbol(member.object(), cx.scoping)),
        Expression::CallExpression(call) => object_assigned(cx, call),
        _ => None,
    }
}

fn plain(cx: &Cx<'_, '_>, home: ScopeId, symbol: SymbolId) -> bool {
    cx.scoping.symbol_scope_id(symbol) == home
        && !cx.runtime.symbols.contains_key(&symbol)
        && !cx.templates.contains_key(&symbol)
        && !cx.library.borrow().contains(&symbol)
}

pub fn reach(
    cx: &Cx<'_, '_>,
    home: ScopeId,
    seeds: &[SymbolId],
    found: &[Found],
) -> FxHashSet<SymbolId> {
    let mut reached: FxHashSet<SymbolId> = seeds.iter().copied().collect();
    let mut queue: Vec<SymbolId> = seeds.to_vec();
    while let Some(symbol) = queue.pop() {
        if found
            .iter()
            .any(|component| component.symbol == Some(symbol))
        {
            continue;
        }
        let Some((span, _)) = helper_owner(cx, symbol) else {
            continue;
        };
        for (_, used) in cx.referenced_at(span) {
            let component = found.iter().any(|component| component.symbol == Some(used));
            if plain(cx, home, used) && !component && reached.insert(used) {
                queue.push(used);
            }
        }
    }
    reached
}

fn effect_of(
    cx: &Cx<'_, '_>,
    home: ScopeId,
    found: &[Found],
    statement: &Statement<'_>,
) -> Option<Effect> {
    let span = statement.span();
    let written = written_object(cx, statement);
    let direct: Vec<SymbolId> = cx
        .referenced_at(span)
        .filter(|(at, _)| !found.iter().any(|component| inside(component.span, *at)))
        .map(|(_, symbol)| symbol)
        .filter(|symbol| plain(cx, home, *symbol) && !inside(span, cx.scoping.symbol_span(*symbol)))
        .filter(|symbol| {
            !found
                .iter()
                .any(|component| component.symbol == Some(*symbol))
                || written == Some(*symbol)
        })
        .collect();
    if direct.is_empty() {
        return None;
    }
    let symbols = reach(cx, home, &direct, found);
    let owners = symbols
        .iter()
        .filter_map(|symbol| helper_owner(cx, *symbol).map(|(owner, _)| owner))
        .collect();
    let line = cx.rw.text(span).lines().next().unwrap_or_default();
    Some(Effect {
        line: line.chars().take(60).collect(),
        span,
        symbols,
        owners,
    })
}

pub fn module_effects(cx: &Cx<'_, '_>, home: ScopeId, found: &[Found]) -> Vec<Effect> {
    let sites = effect_sites(cx, home);
    untwist_restore::region::statements(cx.scoping, cx.nodes, home)
        .iter()
        .filter(|statement| is_effect(cx, &sites, statement))
        .filter_map(|statement| effect_of(cx, home, found, statement))
        .collect()
}
