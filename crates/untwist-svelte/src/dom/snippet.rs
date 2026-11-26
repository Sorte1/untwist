use super::*;

impl<'a, 's> Cx<'a, 's> {
    fn param_default(
        &self,
        statements: &'a [Statement<'a>],
        param: SymbolId,
    ) -> Option<(SymbolId, String)> {
        for statement in statements {
            let Statement::VariableDeclaration(declaration) = statement else {
                return None;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let (Some(symbol), Some(Expression::CallExpression(call))) = (
                binding(&declarator.id),
                declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses),
            ) else {
                return None;
            };
            if self.rt(&call.callee) != Some(Rt::DerivedSafeEqual) {
                return None;
            }
            let Some(Expression::ArrowFunctionExpression(arrow)) =
                arg(call, 0).map(Expression::without_parentheses)
            else {
                return None;
            };
            let Some(Expression::CallExpression(fallback)) =
                arrow.get_expression().map(Expression::without_parentheses)
            else {
                return None;
            };
            if self.rt(&fallback.callee) != Some(Rt::Fallback) {
                return None;
            }
            let read = match arg(fallback, 0)?.without_parentheses() {
                Expression::ChainExpression(chain) => match &chain.expression {
                    oxc_ast::ast::ChainElement::CallExpression(read) => read,
                    _ => return None,
                },
                _ => return None,
            };
            if !read.arguments.is_empty() || self.symbol(&read.callee) != Some(param) {
                continue;
            }
            let value = match fallback.arguments.len() {
                2 => self.render(arg(fallback, 1)?.span()).ok()?,
                3 if boolean(arg(fallback, 2)) == Some(true) => {
                    self.render(thunk_body(arg(fallback, 1)?)?).ok()?
                }
                _ => return None,
            };
            let reads = self.scoping.get_resolved_reference_ids(param).len();
            return (reads == 1).then_some((symbol, value));
        }
        None
    }

    pub fn snippet_block(&self, symbol: SymbolId) -> R<M> {
        self.snippet_named(symbol, self.scoping.symbol_name(symbol))
    }

    pub fn snippet_named(&self, symbol: SymbolId, name: &str) -> R<M> {
        let declarator = self
            .declarator(symbol)
            .ok_or("a snippet that is not declared here")?;
        let init = declarator.init.as_ref().ok_or("a snippet without a body")?;
        self.snippet_arrow(init, name)
    }

    pub fn snippet_arrow(&self, init: &'a Expression<'a>, name: &str) -> R<M> {
        let Expression::ArrowFunctionExpression(arrow) = init.without_parentheses() else {
            return fail("a snippet of an unknown shape");
        };
        let statements = arrow_statements(arrow);
        let mut params = Vec::new();
        for param in arrow.params.items.iter().skip(1) {
            let symbol =
                binding(&param.pattern).ok_or("a snippet with a destructured parameter")?;
            match &param.initializer {
                Some(init) if self.rt(init) == Some(Rt::Noop) => {
                    params.push(self.scoping.symbol_name(symbol).to_string());
                }
                Some(_) => return fail("a snippet parameter with a default value"),
                None => {
                    let (local, value) = self
                        .param_default(statements, symbol)
                        .ok_or("a snippet parameter of an unknown shape")?;
                    self.rw.forbid(symbol);
                    self.consumed.borrow_mut().insert(local);
                    params.push(format!("{} = {value}", self.scoping.symbol_name(local)));
                }
            }
        }
        let mut scratch = self.scratch();
        let (nodes, _) = self.render_fn(init, &mut scratch)?;
        let header = format!("{{#snippet {name}({})}}", params.join(", "));
        Ok(M::Block {
            sections: vec![(header, nodes)],
            close: "{/snippet}".to_string(),
        })
    }
}
