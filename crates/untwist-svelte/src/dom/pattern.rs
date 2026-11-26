use oxc_ast::ast::BindingPattern;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Index(u32),
}

pub struct Leaf {
    pub path: Vec<Seg>,
    pub local: SymbolId,
    pub fallback: Option<Span>,
}

pub struct Destructure {
    pub leaves: Vec<Leaf>,
    pub arrays: Vec<(Vec<Seg>, u32)>,
}

impl<'a, 's> Cx<'a, 's> {
    fn path_of(
        &self,
        expr: &Expression<'_>,
        item: SymbolId,
        arrays: &FxHashMap<SymbolId, Vec<Seg>>,
    ) -> Option<Vec<Seg>> {
        match expr.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                let mut path = self.path_of(&member.object, item, arrays)?;
                path.push(Seg::Key(member.property.name.to_string()));
                Some(path)
            }
            Expression::ComputedMemberExpression(member) => {
                let mut path = self.path_of(&member.object, item, arrays)?;
                match member.expression.without_parentheses() {
                    Expression::StringLiteral(literal) => {
                        path.push(Seg::Key(literal.value.as_str()?.to_string()))
                    }
                    other => path.push(Seg::Index(number(other)?)),
                }
                Some(path)
            }
            Expression::CallExpression(call)
                if self.rt(&call.callee) == Some(Rt::Get) && call.arguments.len() == 1 =>
            {
                let symbol = self.symbol(arg(call, 0)?)?;
                match symbol == item {
                    true => Some(Vec::new()),
                    false => arrays.get(&symbol).cloned(),
                }
            }
            Expression::Identifier(_) if self.symbol(expr) == Some(item) => Some(Vec::new()),
            _ => None,
        }
    }

    pub fn destructure(&self, statements: &[Statement<'_>], item: SymbolId) -> Option<Destructure> {
        let mut arrays: FxHashMap<SymbolId, Vec<Seg>> = FxHashMap::default();
        let mut out = Destructure {
            leaves: Vec::new(),
            arrays: Vec::new(),
        };
        let mut consumed = Vec::new();
        for statement in statements {
            let Statement::VariableDeclaration(declaration) = statement else {
                break;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                break;
            };
            let (Some(symbol), Some(init)) = (binding(&declarator.id), declarator.init.as_ref())
            else {
                break;
            };
            match init.without_parentheses() {
                Expression::ArrowFunctionExpression(arrow)
                    if arrow.params.items.is_empty() && arrow.params.rest.is_none() =>
                {
                    let path = self.path_of(arrow.get_expression()?, item, &arrays)?;
                    out.leaves.push(Leaf {
                        path,
                        local: symbol,
                        fallback: None,
                    });
                }
                Expression::CallExpression(call) => match self.rt(&call.callee) {
                    Some(Rt::Derived) => {
                        let inner = arg(call, 0).and_then(thunk_body).and_then(|_| {
                            let Expression::ArrowFunctionExpression(arrow) =
                                arg(call, 0)?.without_parentheses()
                            else {
                                return None;
                            };
                            arrow.get_expression()
                        })?;
                        let Expression::CallExpression(to_array) = inner.without_parentheses()
                        else {
                            return None;
                        };
                        if self.rt(&to_array.callee) != Some(Rt::ToArray) {
                            return None;
                        }
                        let path = self.path_of(arg(to_array, 0)?, item, &arrays)?;
                        let length = number(arg(to_array, 1)?)?;
                        out.arrays.push((path.clone(), length));
                        arrays.insert(symbol, path);
                    }
                    Some(Rt::DerivedSafeEqual) => {
                        let Expression::ArrowFunctionExpression(arrow) =
                            arg(call, 0)?.without_parentheses()
                        else {
                            return None;
                        };
                        let Expression::CallExpression(fallback) =
                            arrow.get_expression()?.without_parentheses()
                        else {
                            return None;
                        };
                        if self.rt(&fallback.callee) != Some(Rt::Fallback)
                            || fallback.arguments.len() != 2
                        {
                            return None;
                        }
                        let path = self.path_of(arg(fallback, 0)?, item, &arrays)?;
                        out.leaves.push(Leaf {
                            path,
                            local: symbol,
                            fallback: Some(arg(fallback, 1)?.span()),
                        });
                    }
                    _ => break,
                },
                _ => break,
            }
            consumed.push(symbol);
        }
        if out.leaves.is_empty() {
            return None;
        }
        let uses = self.scoping.get_resolved_reference_ids(item).len();
        let mut reads = 0;
        for statement in statements.iter().take(consumed.len()) {
            reads += self.reads_of(statement, item);
        }
        if reads != uses {
            return None;
        }
        for symbol in arrays.keys() {
            let reads: usize = statements
                .iter()
                .take(consumed.len())
                .map(|statement| self.reads_of(statement, *symbol))
                .sum();
            if reads != self.scoping.get_resolved_reference_ids(*symbol).len() {
                return None;
            }
            self.rw.forbid(*symbol);
        }
        self.rw.forbid(item);
        self.consumed.borrow_mut().extend(consumed);
        Some(out)
    }

    fn reads_of(&self, statement: &Statement<'_>, symbol: SymbolId) -> usize {
        let span = statement.span();
        self.scoping
            .get_resolved_references(symbol)
            .filter(|reference| {
                let at = self.nodes.kind(reference.node_id()).span();
                span.start <= at.start && at.end <= span.end
            })
            .count()
    }

    pub fn pattern(&self, destructure: &Destructure, prefix: &[Seg]) -> R<String> {
        if let Some(leaf) = destructure.leaves.iter().find(|leaf| leaf.path == prefix) {
            let mut text = self.scoping.symbol_name(leaf.local).to_string();
            if let Some(fallback) = leaf.fallback {
                text.push_str(&format!(" = {}", self.render(fallback)?));
            }
            return Ok(text);
        }
        let below = |path: &[Seg]| path.len() > prefix.len() && path.starts_with(prefix);
        if let Some((_, length)) = destructure.arrays.iter().find(|(path, _)| path == prefix) {
            let mut items = Vec::new();
            for index in 0..*length {
                let mut next = prefix.to_vec();
                next.push(Seg::Index(index));
                let used = destructure
                    .leaves
                    .iter()
                    .any(|leaf| leaf.path.starts_with(&next));
                items.push(match used {
                    true => self.pattern(destructure, &next)?,
                    false => String::new(),
                });
            }
            return Ok(format!("[{}]", items.join(", ")));
        }
        let mut keys: Vec<String> = Vec::new();
        for path in destructure
            .leaves
            .iter()
            .map(|leaf| leaf.path.as_slice())
            .chain(destructure.arrays.iter().map(|(path, _)| path.as_slice()))
            .filter(|path| below(path))
        {
            match &path[prefix.len()] {
                Seg::Key(key) if !keys.contains(key) => keys.push(key.clone()),
                Seg::Key(_) => {}
                Seg::Index(_) => {
                    return fail("an each item read by index outside an array pattern")
                }
            }
        }
        if keys.is_empty() {
            return fail("an each item pattern with no bindings");
        }
        let mut entries = Vec::new();
        for key in keys {
            if !is_identifier(&key) {
                return fail("an each item pattern with a quoted key");
            }
            let mut next = prefix.to_vec();
            next.push(Seg::Key(key.clone()));
            let inner = self.pattern(destructure, &next)?;
            let bare = inner.split(" = ").next().unwrap_or_default() == key;
            entries.push(match bare {
                true => inner,
                false => format!("{key}: {inner}"),
            });
        }
        Ok(format!("{{ {} }}", entries.join(", ")))
    }
}

pub fn key_bindings(
    pattern: &BindingPattern<'_>,
    path: Vec<Seg>,
    out: &mut Vec<(Vec<Seg>, SymbolId)>,
) -> bool {
    match pattern {
        BindingPattern::BindingIdentifier(ident) => {
            out.push((path, ident.symbol_id()));
            true
        }
        BindingPattern::AssignmentPattern(assign) => key_bindings(&assign.left, path, out),
        BindingPattern::ObjectPattern(object) => {
            object.rest.is_none()
                && object.properties.iter().all(|property| {
                    let Some(key) = key_name(&property.key).filter(|_| !property.computed) else {
                        return false;
                    };
                    let mut next = path.clone();
                    next.push(Seg::Key(key));
                    key_bindings(&property.value, next, out)
                })
        }
        BindingPattern::ArrayPattern(array) => {
            array.rest.is_none()
                && array
                    .elements
                    .iter()
                    .enumerate()
                    .all(|(index, element)| match element {
                        None => true,
                        Some(element) => {
                            let mut next = path.clone();
                            next.push(Seg::Index(index as u32));
                            key_bindings(element, next, out)
                        }
                    })
        }
    }
}
