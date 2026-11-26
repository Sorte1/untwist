use super::*;

fn is_empty_list(expr: Option<&Expression<'_>>) -> bool {
    match expr.map(Expression::without_parentheses) {
        Some(Expression::ArrayExpression(array)) => array.elements.is_empty(),
        other => is_nullish(other),
    }
}

impl<'a, 's> Cx<'a, 's> {
    pub(super) fn object_attrs(
        &self,
        object: &'a oxc_ast::ast::ObjectExpression<'a>,
    ) -> R<Vec<String>> {
        let mut attrs = Vec::new();
        for property in &object.properties {
            match property {
                ObjectPropertyKind::SpreadProperty(spread) => {
                    attrs.push(format!("{{...{}}}", self.code(&spread.argument)?));
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    if property.computed || property.method || property.kind != PropertyKind::Init {
                        return fail("a spread attribute object with a computed or accessor key");
                    }
                    let key = key_name(&property.key).ok_or("an attribute with a computed name")?;
                    match string(&property.value) {
                        Some(text) => attrs.push(format!(
                            "{key}=\"{}\"",
                            braces(&text.replace('&', "&amp;").replace('"', "&quot;"))
                        )),
                        None => attrs.push(attr(&key, &self.code(&property.value)?)),
                    }
                }
            }
        }
        Ok(attrs)
    }

    pub(super) fn spread_attrs(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        let shape = "a spread attribute effect of an unknown shape";
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail(shape);
        };
        if !arrow.params.items.is_empty() || arrow.params.rest.is_some() {
            return fail("a spread attribute effect with derived inputs");
        }
        let Some(Expression::ObjectExpression(object)) =
            arrow_value(arrow).map(Expression::without_parentheses)
        else {
            return fail(shape);
        };
        if call.arguments.len() > 8 || !(2..5).all(|index| is_empty_list(arg(call, index))) {
            return fail("a spread attribute effect with async or blocked inputs");
        }
        match arg(call, 5).map(Expression::without_parentheses) {
            None => {}
            Some(hash) if is_nullish(Some(hash)) => {}
            Some(hash) if string(hash).is_some() && string(hash) == self.hash.as_deref() => {}
            Some(_) => return fail("a spread attribute effect with a foreign style hash"),
        }
        if boolean(arg(call, 6)).is_none() || boolean(arg(call, 7)).is_none() {
            return fail(shape);
        }
        for text in self.object_attrs(object)? {
            self.push_attr(body, node, text);
        }
        Ok(())
    }

    pub(super) fn set_attributes(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        let Some(Expression::ObjectExpression(object)) =
            arg(call, 2).map(Expression::without_parentheses)
        else {
            return fail("set_attributes() with props that are not an object literal");
        };
        match arg(call, 3).map(Expression::without_parentheses) {
            None => {}
            Some(hash) if is_nullish(Some(hash)) => {}
            Some(hash) if string(hash).is_some() && string(hash) == self.hash.as_deref() => {}
            Some(_) => return fail("set_attributes() with a foreign style hash"),
        }
        for text in self.object_attrs(object)? {
            self.push_attr(body, node, text);
        }
        Ok(())
    }

    fn directive_target(&self, expr: Option<&'a Expression<'a>>) -> R<String> {
        let body = expr
            .and_then(thunk_body)
            .ok_or("a directive whose function is not a getter")?;
        let text = self.render(body)?;
        let simple = text.split('.').all(is_identifier);
        match simple {
            true => Ok(text),
            false => fail("a directive whose function is not a plain name"),
        }
    }

    fn directive_params(&self, expr: Option<&'a Expression<'a>>) -> R<Option<String>> {
        match expr {
            None => Ok(None),
            Some(expr) if is_nullish(Some(expr)) => Ok(None),
            Some(expr) => {
                let body = thunk_body(expr).ok_or("directive parameters of an unknown shape")?;
                Ok(Some(self.render(body)?))
            }
        }
    }

    pub(super) fn transition(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let flags = arg(call, 0)
            .and_then(number)
            .ok_or("a transition with computed flags")?;
        if flags & !7 != 0 || call.arguments.len() > 4 {
            return fail("a transition with unknown flags");
        }
        let kind = match flags & 3 {
            3 => "transition",
            1 => "in",
            2 => "out",
            _ => return fail("a transition that is neither in nor out"),
        };
        let node = self.element(body, arg(call, 1))?;
        let name = self.directive_target(arg(call, 2))?;
        let modifier = if flags & 4 != 0 { "|global" } else { "" };
        let mut text = format!("{kind}:{name}{modifier}");
        if let Some(params) = self.directive_params(arg(call, 3))? {
            text.push_str(&format!("={{{params}}}"));
        }
        self.push_attr(body, node, text);
        Ok(())
    }

    pub(super) fn animation(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        if call.arguments.len() > 3 {
            return fail("an animation with extra arguments");
        }
        let node = self.element(body, arg(call, 0))?;
        let name = self.directive_target(arg(call, 1))?;
        let mut text = format!("animate:{name}");
        if let Some(params) = self.directive_params(arg(call, 2))? {
            text.push_str(&format!("={{{params}}}"));
        }
        self.push_attr(body, node, text);
        Ok(())
    }

    pub(super) fn dynamic_element(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(
            body,
            arg(call, 0).ok_or("svelte:element without an anchor")?,
        )?;
        let tag = self.thunk(arg(call, 1).ok_or("svelte:element without a tag")?)?;
        if boolean(arg(call, 2)) != Some(false) || call.arguments.len() > 4 {
            return fail("svelte:element with a namespace");
        }
        let mut attrs = vec![format!("this={{{tag}}}")];
        let mut children = Vec::new();
        match arg(call, 3) {
            None => {}
            Some(render) if is_nullish(Some(render)) => {}
            Some(render) => {
                let Expression::ArrowFunctionExpression(arrow) = render.without_parentheses()
                else {
                    return fail("svelte:element content of an unknown shape");
                };
                let symbols = param_symbols(&arrow.params)
                    .ok_or("svelte:element content of an unknown shape")?;
                let [element, anchor] = symbols.as_slice() else {
                    return fail("svelte:element content of an unknown shape");
                };
                let (nodes, _, own) =
                    self.body_seeded(Code::of(arrow), *anchor, false, Some(*element))?;
                attrs.extend(own);
                children = nodes;
            }
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Element {
                name: "svelte:element".to_string(),
                attrs,
                children,
                svg: true,
            }],
        )
    }

    pub(super) fn boundary(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(
            body,
            arg(call, 0).ok_or("svelte:boundary without an anchor")?,
        )?;
        if call.arguments.len() != 3 {
            return fail("svelte:boundary with an error transform");
        }
        let Some(Expression::ObjectExpression(object)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail("svelte:boundary props of an unknown shape");
        };
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return fail("svelte:boundary props of an unknown shape");
            };
            let key = key_name(&property.key)
                .filter(|_| !property.computed && property.kind == PropertyKind::Init)
                .ok_or("svelte:boundary props of an unknown shape")?;
            match key.as_str() {
                "failed" | "pending"
                    if matches!(
                        property.value.without_parentheses(),
                        Expression::ArrowFunctionExpression(_)
                    ) =>
                {
                    children.push(self.snippet_arrow(&property.value, &key)?);
                    children.push(M::Text(" ".to_string()));
                }
                "failed" | "pending" => {
                    let symbol = self
                        .symbol(&property.value)
                        .filter(|symbol| body.deferred.contains(symbol))
                        .ok_or("a svelte:boundary snippet that is not declared beside it")?;
                    if self.scoping.get_resolved_reference_ids(symbol).len() != 1 {
                        return fail("a svelte:boundary snippet that is used elsewhere");
                    }
                    body.used.insert(symbol);
                    children.push(self.snippet_named(symbol, &key)?);
                    children.push(M::Text(" ".to_string()));
                }
                _ => attrs.push(attr(&key, &self.code(&property.value)?)),
            }
        }
        let render = arg(call, 2).ok_or("svelte:boundary without content")?;
        children.extend(self.render_fn(render, body)?.0);
        self.fill_anchor(
            body,
            node,
            vec![M::Element {
                name: "svelte:boundary".to_string(),
                attrs,
                children,
                svg: true,
            }],
        )
    }
}
