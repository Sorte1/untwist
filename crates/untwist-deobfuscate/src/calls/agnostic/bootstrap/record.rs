use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;

use super::{binding_symbol, key_name, names, symbol_of, Resolve, Shape, Table};

fn leads(expr: &Expression<'_>, span: Span) -> bool {
    match expr.without_parentheses() {
        Expression::CallExpression(call) => call.span == span,
        Expression::SequenceExpression(seq) => seq
            .expressions
            .first()
            .is_some_and(|first| leads(first, span)),
        _ => false,
    }
}

fn statement_leads(statement: &Statement<'_>, span: Span) -> bool {
    match statement {
        Statement::ReturnStatement(ret) => {
            ret.argument.as_ref().is_some_and(|arg| leads(arg, span))
        }
        Statement::ExpressionStatement(stmt) => leads(&stmt.expression, span),
        _ => false,
    }
}

fn created_literal<'a>(
    init: &'a Expression<'a>,
    caches: &mut Vec<&'a IdentifierReference<'a>>,
) -> Option<&'a ObjectExpression<'a>> {
    match init.without_parentheses() {
        Expression::ObjectExpression(object) => Some(object),
        Expression::AssignmentExpression(assign)
            if assign.operator == AssignmentOperator::Assign =>
        {
            let member = assign.left.as_member_expression()?;
            let Expression::Identifier(cache) = member.object().without_parentheses() else {
                return None;
            };
            caches.push(cache);
            created_literal(&assign.right, caches)
        }
        _ => None,
    }
}

struct CacheUses<'s> {
    scoping: &'s Scoping,
    cache: SymbolId,
    require: SymbolId,
    inside: Span,
    stored: Vec<Span>,
    escapes: bool,
}

impl<'a> VisitJs<'a> for CacheUses<'_> {
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let stores = it.left.as_member_expression().is_some_and(|member| {
            names(self.scoping, member.object(), self.require)
                && matches!(
                    member.static_property_name().and_then(|name| name.as_str()),
                    Some("c")
                )
        });
        if stores && names(self.scoping, &it.right, self.cache) {
            self.stored.push(it.right.span());
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let outside = it.span.start < self.inside.start || it.span.end > self.inside.end;
        if symbol_of(self.scoping, it) == Some(self.cache)
            && outside
            && !self.stored.contains(&it.span)
        {
            self.escapes = true;
        }
    }
}

fn caches_confined(
    program: &Program<'_>,
    scoping: &Scoping,
    shape: &Shape<'_>,
    caches: &[&IdentifierReference<'_>],
) -> bool {
    caches.iter().all(|cache| {
        let Some(symbol) = symbol_of(scoping, cache) else {
            return false;
        };
        let mut uses = CacheUses {
            scoping,
            cache: symbol,
            require: shape.require,
            inside: shape.body.span,
            stored: Vec::new(),
            escapes: false,
        };
        uses.visit_program(program);
        !uses.escapes && !scoping.symbol_is_mutated(symbol)
    })
}

fn fresh_exports(object: &ObjectExpression<'_>) -> bool {
    let mut exports = 0;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(p) = property else {
            return false;
        };
        let name = match &p.key {
            PropertyKey::StaticIdentifier(id) if !p.computed => Cow::Borrowed(id.name.as_str()),
            PropertyKey::StringLiteral(text) => js_text(text.value),
            PropertyKey::NumericLiteral(_) if !p.computed => continue,
            _ => return false,
        };
        if p.kind != PropertyKind::Init || name == "__proto__" {
            return false;
        }
        if name == "exports" {
            if !matches!(
                p.value.without_parentheses(),
                Expression::ObjectExpression(_)
            ) {
                return false;
            }
            exports += 1;
        }
    }
    exports == 1
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Passed {
    Object,
    Read,
    Unknown,
}

pub(super) struct Record<'a> {
    symbol: SymbolId,
    literal: &'a ObjectExpression<'a>,
    caches: Vec<&'a IdentifierReference<'a>>,
}

fn sequenced<'a>(expr: &'a Expression<'a>, span: Span) -> Option<&'a AssignmentExpression<'a>> {
    match expr.without_parentheses() {
        Expression::SequenceExpression(seq) => {
            seq.expressions
                .windows(2)
                .find_map(|pair| match (&pair[0], &pair[1]) {
                    (
                        Expression::AssignmentExpression(assign),
                        Expression::CallExpression(call),
                    ) if call.span == span => Some(&**assign),
                    _ => None,
                })
        }
        Expression::LogicalExpression(logical) => sequenced(&logical.right, span),
        Expression::StaticMemberExpression(member) => sequenced(&member.object, span),
        Expression::ComputedMemberExpression(member) => sequenced(&member.object, span),
        _ => None,
    }
}

fn sequenced_record<'a>(scoping: &Scoping, shape: &Shape<'a>) -> Option<Record<'a>> {
    let assign = shape
        .body
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::ReturnStatement(ret) => {
                sequenced(ret.argument.as_ref()?, shape.dispatch.span)
            }
            _ => None,
        })?;
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    let symbol = symbol_of(scoping, target)?;
    let declared = shape.body.statements.iter().any(|statement| {
        matches!(statement, Statement::VariableDeclaration(decl)
            if decl.kind == VariableDeclarationKind::Var
                && decl.declarations.iter().any(|declarator| declarator.init.is_none()
                    && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                        if id.symbol_id.get() == Some(symbol))))
    });
    let writes = scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .filter(|reference| scoping.get_reference(**reference).is_write())
        .count();
    if !declared || writes != 1 || !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }
    let mut caches = Vec::new();
    let literal = created_literal(&assign.right, &mut caches)?;
    Some(Record {
        symbol,
        literal,
        caches,
    })
}

pub(super) fn module_record<'a>(scoping: &Scoping, shape: &Shape<'a>) -> Option<Record<'a>> {
    let statements = &shape.body.statements;
    let Some(at) = statements
        .iter()
        .position(|statement| statement_leads(statement, shape.dispatch.span))
    else {
        return sequenced_record(scoping, shape);
    };
    let Statement::VariableDeclaration(decl) = statements.get(at.checked_sub(1)?)? else {
        return None;
    };
    let [declarator] = decl.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    if scoping.symbol_is_mutated(symbol) {
        return None;
    }
    let mut caches = Vec::new();
    let literal = created_literal(declarator.init.as_ref()?, &mut caches)?;
    Some(Record {
        symbol,
        literal,
        caches,
    })
}

fn passed_values(
    program: &Program<'_>,
    scoping: &Scoping,
    resolve: Resolve<'_>,
    (shape, cache_read): (&Shape<'_>, bool),
) -> Vec<Passed> {
    let Some(found) = module_record(scoping, shape) else {
        return Vec::new();
    };
    let record = found.symbol;
    let exports = fresh_exports(found.literal)
        && (found.caches.is_empty()
            || (!cache_read && caches_confined(program, scoping, shape, &found.caches)));
    let mut out = Vec::new();
    for argument in &shape.dispatch.arguments {
        let passed = match argument
            .as_expression()
            .map(Expression::without_parentheses)
        {
            Some(Expression::Identifier(id)) => match symbol_of(scoping, id) {
                Some(symbol) if symbol == record => Passed::Object,
                Some(_) => Passed::Read,
                None => Passed::Unknown,
            },
            Some(expr) => match expr.as_member_expression() {
                Some(member) if names(scoping, member.object(), record) => {
                    match key_name(member, resolve) {
                        Some(key) if key == "exports" && exports => Passed::Object,
                        Some(_) => Passed::Read,
                        None => Passed::Unknown,
                    }
                }
                _ => Passed::Unknown,
            },
            None => Passed::Unknown,
        };
        if passed == Passed::Unknown {
            break;
        }
        out.push(passed);
    }
    out
}

pub(super) fn dispatched_objects(
    (program, scoping, resolve): (&Program<'_>, &Scoping, Resolve<'_>),
    (shape, cache_read): (&Shape<'_>, bool),
    table: &Table<'_>,
) -> Vec<SymbolId> {
    let passed = passed_values(program, scoping, resolve, (shape, cache_read));
    let mut objects = Vec::new();
    for params in &table.functions {
        for (index, param) in params.items.iter().enumerate() {
            if passed.get(index + shape.offset) != Some(&Passed::Object)
                || param.initializer.is_some()
            {
                continue;
            }
            if let Some(symbol) = binding_symbol(param) {
                if !scoping.symbol_is_mutated(symbol) {
                    objects.push(symbol);
                }
            }
        }
    }
    objects
}
