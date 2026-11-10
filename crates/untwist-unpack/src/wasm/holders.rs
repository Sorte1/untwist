use oxc_ast::ast::{
    Argument, AssignmentOperator, AssignmentTarget, CallExpression, Expression, Function,
    MemberExpression, Statement,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::{required_spec, unchained};

#[derive(Debug, Default)]
pub(super) struct Holders {
    pub reads: Vec<(String, String)>,
    pub bindings: Vec<(String, String, String)>,
    pub setters: Vec<(String, String)>,
    pub handoffs: Vec<(String, String, String)>,
    pub loose: Vec<(String, String)>,
    pub calls: Vec<(String, String, String)>,
}

struct Collector<'s> {
    semantic: &'s Semantic<'s>,
    names: FxHashMap<SymbolId, String>,
    called: FxHashSet<Span>,
    out: Holders,
}

fn callee_member<'a>(call: &'a CallExpression<'a>) -> Option<&'a MemberExpression<'a>> {
    let mut callee = call.callee.without_parentheses();
    match callee {
        Expression::SequenceExpression(sequence) => match sequence.expressions.as_slice() {
            [Expression::NumericLiteral(_), last] => callee = last.without_parentheses(),
            _ => return None,
        },
        Expression::CallExpression(wrap) => match (&wrap.callee, wrap.arguments.as_slice()) {
            (Expression::Identifier(id), [only]) if id.name == "Object" => {
                callee = only.as_expression()?.without_parentheses();
            }
            _ => return None,
        },
        _ => {}
    }
    callee.as_member_expression()
}

impl<'s> Collector<'s> {
    fn nodes(&self) -> &'s AstNodes<'s> {
        self.semantic.nodes()
    }

    fn scoping(&self) -> &'s Scoping {
        self.semantic.scoping()
    }

    fn symbol_of(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expression.without_parentheses() else {
            return None;
        };
        self.scoping().get_reference(id.reference_id()).symbol_id()
    }

    fn holder_of(&self, expression: &Expression<'_>) -> Option<&str> {
        self.names
            .get(&self.symbol_of(expression)?)
            .map(String::as_str)
    }

    fn collect_requires(&mut self) {
        for node in self.nodes().iter() {
            let symbol = match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    match (declarator.id.get_binding_identifier(), &declarator.init) {
                        (Some(id), Some(init)) if required_spec(init).is_some() => {
                            Some(id.symbol_id())
                        }
                        _ => None,
                    }
                }
                AstKind::AssignmentExpression(assign) => match &assign.left {
                    AssignmentTarget::AssignmentTargetIdentifier(id)
                        if required_spec(&assign.right).is_some() =>
                    {
                        self.scoping().get_reference(id.reference_id()).symbol_id()
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(symbol) = symbol {
                self.hold(symbol);
            }
        }
    }

    fn hold(&mut self, symbol: SymbolId) {
        let name = self.scoping().symbol_name(symbol).to_string();
        self.names.insert(symbol, name);
    }

    fn setter_binding(&self, function: &Function<'_>) -> Option<SymbolId> {
        let id = function.id.as_ref()?;
        let params = &function.params;
        if !function.is_declaration() || params.rest.is_some() || params.items.len() != 1 {
            return None;
        }
        let param = params.items[0]
            .pattern
            .get_binding_identifier()?
            .symbol_id();
        let [Statement::ExpressionStatement(statement)] =
            function.body.as_ref()?.statements.as_slice()
        else {
            return None;
        };
        let Expression::AssignmentExpression(assign) = statement.expression.without_parentheses()
        else {
            return None;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
            return None;
        };
        if assign.operator != AssignmentOperator::Assign
            || self.symbol_of(&assign.right) != Some(param)
        {
            return None;
        }
        let scoping = self.scoping();
        let binding = scoping.get_reference(target.reference_id()).symbol_id()?;
        let uninitialized = matches!(
            self.nodes().kind(scoping.symbol_declaration(binding)),
            AstKind::VariableDeclarator(declarator) if declarator.init.is_none()
        );
        let writes = scoping
            .get_resolved_references(binding)
            .filter(|reference| reference.is_write())
            .count();
        let uses = scoping.get_resolved_references(id.symbol_id()).count();
        let single = scoping.symbol_redeclarations(id.symbol_id()).is_empty()
            && scoping.symbol_redeclarations(binding).is_empty();
        (uninitialized && writes == 1 && uses == 1 && single).then_some(binding)
    }

    fn collect_setters(&mut self) {
        for node in self.nodes().iter() {
            let AstKind::Function(function) = node.kind() else {
                continue;
            };
            let Some(binding) = self.setter_binding(function) else {
                continue;
            };
            if let Some(id) = &function.id {
                let name = self.scoping().symbol_name(binding).to_string();
                self.out.setters.push((id.name.to_string(), name));
                self.hold(binding);
            }
        }
    }

    fn drop_shared_names(&mut self) {
        let mut seen: FxHashMap<&str, usize> = FxHashMap::default();
        for name in self.names.values() {
            *seen.entry(name.as_str()).or_default() += 1;
        }
        let shared: FxHashSet<String> = seen
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name.to_string())
            .collect();
        self.names.retain(|_, name| !shared.contains(name));
        self.out
            .setters
            .retain(|(_, binding)| !shared.contains(binding));
    }

    fn enclosing_function(&self, id: NodeId) -> Option<String> {
        let nodes = self.nodes();
        let function = nodes.ancestor_ids(id).find(|ancestor| {
            matches!(
                nodes.kind(*ancestor),
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
            )
        })?;
        if let AstKind::Function(function) = nodes.kind(function) {
            if let Some(id) = &function.id {
                return Some(id.name.to_string());
            }
        }
        match nodes.parent_kind(function) {
            AstKind::VariableDeclarator(declarator) => declarator
                .id
                .get_binding_identifier()
                .map(|id| id.name.to_string()),
            _ => None,
        }
    }

    fn call(&mut self, id: NodeId, call: &'s CallExpression<'s>) {
        let Some(member) = callee_member(call) else {
            return;
        };
        let (Some(holder), Some(key)) = (
            self.holder_of(member.object()),
            member.static_property_name().and_then(|name| name.as_str()),
        ) else {
            return;
        };
        let (holder, key) = (holder.to_string(), key.to_string());
        self.called.insert(member.span());
        if let [Argument::Identifier(_)] = call.arguments.as_slice() {
            let argument = call.arguments[0]
                .as_expression()
                .and_then(|arg| self.holder_of(arg));
            match argument {
                Some(argument) => {
                    self.out
                        .handoffs
                        .push((holder.clone(), key.clone(), argument.to_string()))
                }
                None => self.out.loose.push((holder.clone(), key.clone())),
            }
        } else {
            self.out.loose.push((holder.clone(), key.clone()));
        }
        if let Some(function) = self.enclosing_function(id) {
            self.out.calls.push((function, holder, key));
        }
    }

    fn is_write_target(&self, id: NodeId, span: Span) -> bool {
        matches!(self.nodes().parent_kind(id), AstKind::AssignmentExpression(assign) if assign.left.span() == span)
    }

    fn member(&mut self, id: NodeId, object: &Expression<'_>, key: Option<&str>, span: Span) {
        let (Some(holder), Some(key)) = (self.holder_of(object), key) else {
            return;
        };
        if self.is_write_target(id, span) {
            return;
        }
        let pair = (holder.to_string(), key.to_string());
        if !self.called.contains(&span) {
            self.out.loose.push(pair.clone());
        }
        self.out.reads.push(pair);
    }

    fn visit(&mut self) {
        let nodes = self.nodes();
        for node in nodes.iter() {
            if let AstKind::CallExpression(call) = node.kind() {
                self.call(node.id(), call);
            }
        }
        for node in nodes.iter() {
            match node.kind() {
                AstKind::StaticMemberExpression(member) => self.member(
                    node.id(),
                    &member.object,
                    Some(member.property.name.as_str()),
                    member.span,
                ),
                AstKind::ComputedMemberExpression(member) => self.member(
                    node.id(),
                    &member.object,
                    member.static_property_name().and_then(|name| name.as_str()),
                    member.span,
                ),
                AstKind::VariableDeclarator(declarator) => {
                    let (Some(id), Some(init)) =
                        (declarator.id.get_binding_identifier(), &declarator.init)
                    else {
                        continue;
                    };
                    let Some(member) = unchained(init).as_member_expression() else {
                        continue;
                    };
                    if let (Some(holder), Some(key)) = (
                        self.holder_of(member.object()),
                        member.static_property_name().and_then(|name| name.as_str()),
                    ) {
                        self.out.bindings.push((
                            id.name.to_string(),
                            holder.to_string(),
                            key.to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

pub(super) fn holders_of(semantic: &Semantic<'_>) -> Holders {
    let mut collector = Collector {
        semantic,
        names: FxHashMap::default(),
        called: FxHashSet::default(),
        out: Holders::default(),
    };
    collector.collect_requires();
    collector.collect_setters();
    collector.drop_shared_names();
    collector.visit();
    let mut out = collector.out;
    for list in [&mut out.reads, &mut out.loose] {
        list.sort();
        list.dedup();
    }
    out.calls.sort();
    out.calls.dedup();
    out
}
