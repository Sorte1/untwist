use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::SymbolId;

use super::identify::resolved;

const OBJECT_ARRAYS: [&str; 4] = ["keys", "values", "entries", "getOwnPropertyNames"];
const ARRAY_STATICS: [&str; 2] = ["from", "of"];
const QUIET_METHODS: [&str; 14] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "concat",
    "join",
    "indexOf",
    "lastIndexOf",
    "includes",
    "splice",
    "reverse",
    "at",
    "fill",
];

pub struct Context<'k, 'a> {
    pub statements: &'k [Statement<'a>],
    pub at: usize,
    pub scoping: &'k Scoping,
    pub module: bool,
}

fn global(expression: &Expression<'_>, name: &str, scoping: &Scoping) -> bool {
    matches!(expression.without_parentheses(), Expression::Identifier(id)
        if id.name.as_str() == name
            && id.reference_id.get().is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none()))
}

fn static_call<'k, 'a>(
    expression: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, &'k str, &'k CallExpression<'a>)> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    (!call.optional && !member.optional).then_some((
        &member.object,
        member.property.name.as_str(),
        call,
    ))
}

fn plain_string(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

fn split_separator(call: &CallExpression<'_>) -> bool {
    match call.arguments.as_slice() {
        [] => true,
        [separator] | [separator, Argument::NumericLiteral(_)] => matches!(
            separator,
            Argument::StringLiteral(_) | Argument::RegExpLiteral(_)
        ),
        _ => false,
    }
}

fn no_spread(call: &CallExpression<'_>) -> bool {
    call.arguments.iter().all(|argument| !argument.is_spread())
}

pub fn fresh_array(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::ArrayExpression(_) => true,
        Expression::NewExpression(new) => {
            global(&new.callee, "Array", scoping) && {
                new.arguments.iter().all(|argument| !argument.is_spread())
            }
        }
        Expression::CallExpression(call) if global(&call.callee, "Array", scoping) => {
            !call.optional && no_spread(call)
        }
        _ => match static_call(expression) {
            Some((object, name, call)) if global(object, "Object", scoping) => {
                OBJECT_ARRAYS.contains(&name) && call.arguments.len() == 1 && no_spread(call)
            }
            Some((object, name, call)) if global(object, "Array", scoping) => {
                ARRAY_STATICS.contains(&name) && no_spread(call)
            }
            Some((object, "split", call)) => plain_string(object) && split_separator(call),
            _ => false,
        },
    }
}

fn fresh_collection(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    matches!(expression.without_parentheses(), Expression::NewExpression(new)
        if (global(&new.callee, "Map", scoping) || global(&new.callee, "Set", scoping))
            && new.arguments.len() <= 1
            && new.arguments.iter().all(|argument| !argument.is_spread()))
}

fn declared_before<'k, 'a>(
    context: &Context<'k, 'a>,
    symbol: SymbolId,
) -> Option<&'k Expression<'a>> {
    context.statements[..context.at]
        .iter()
        .find_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) if !declaration.declare => declaration
                .declarations
                .iter()
                .find(|declarator| {
                    matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                        if id.symbol_id.get() == Some(symbol))
                })
                .and_then(|declarator| declarator.init.as_ref()),
            _ => None,
        })
}

struct Uses<'s> {
    symbol: SymbolId,
    scoping: &'s Scoping,
    source: ReferenceId,
    seen: usize,
    escaped: bool,
}

impl Uses<'_> {
    fn is_symbol(&self, expression: &Expression<'_>) -> bool {
        resolved(expression, self.scoping) == Some(self.symbol)
    }

    fn quiet_reference(&mut self) {
        self.seen += 1;
    }
}

impl<'a> Visit<'a> for Uses<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        if self.scoping.get_reference(reference).symbol_id() != Some(self.symbol) {
            return;
        }
        self.seen += 1;
        self.escaped |= reference != self.source;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() {
            if self.is_symbol(&member.object) {
                if QUIET_METHODS.contains(&member.property.name.as_str()) {
                    self.quiet_reference();
                } else {
                    self.escaped = true;
                }
                for argument in &it.arguments {
                    self.visit_argument(argument);
                }
                return;
            }
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_symbol(&it.object) {
            self.quiet_reference();
            return;
        }
        walk::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_symbol(&it.object) {
            self.quiet_reference();
            self.visit_expression(&it.expression);
            return;
        }
        walk::walk_computed_member_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::ComputedMemberExpression(member) = it {
            if self.is_symbol(&member.object) {
                self.quiet_reference();
                self.escaped |= !matches!(member.expression, Expression::NumericLiteral(_));
                self.visit_expression(&member.expression);
                return;
            }
        }
        walk::walk_simple_assignment_target(self, it);
    }
}

fn quiet_binding(context: &Context<'_, '_>, id: &IdentifierReference<'_>) -> bool {
    let scoping = context.scoping;
    let Some(source) = id.reference_id.get() else {
        return false;
    };
    let Some(symbol) = scoping.get_reference(source).symbol_id() else {
        return false;
    };
    let script_root = !context.module && scoping.symbol_scope_id(symbol) == scoping.root_scope_id();
    if script_root || !scoping.symbol_redeclarations(symbol).is_empty() {
        return false;
    }
    let Some(init) = declared_before(context, symbol) else {
        return false;
    };
    if !fresh_array(init, scoping) {
        return false;
    }
    let total = scoping.get_resolved_reference_ids(symbol).len();
    let mut uses = Uses {
        symbol,
        scoping,
        source,
        seen: 0,
        escaped: false,
    };
    for statement in context.statements {
        uses.visit_statement(statement);
        if uses.escaped {
            return false;
        }
    }
    uses.seen == total
        && scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| !scoping.get_reference(*reference).is_write())
}

pub fn proven_array(context: &Context<'_, '_>, source: &Expression<'_>) -> bool {
    match source.without_parentheses() {
        Expression::Identifier(id) => quiet_binding(context, id),
        other => fresh_array(other, context.scoping),
    }
}

pub fn returnless(context: &Context<'_, '_>, source: &Expression<'_>) -> bool {
    plain_string(source)
        || fresh_collection(source, context.scoping)
        || proven_array(context, source)
}
