use super::*;

impl<'a, 's> Cx<'a, 's> {
    fn old_if_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("an if block without an anchor")?)?;
        let test = arg(call, 1)
            .and_then(thunk_body)
            .ok_or("an if block of an unknown shape")?;
        let elseif = match arg(call, 4) {
            None => false,
            Some(flag) => boolean(Some(flag)).ok_or("an if block with a computed else-if flag")?,
        };
        if call.arguments.len() > 5 {
            return fail("an if block of an unknown shape");
        }
        let opener = if elseif { "{:else if" } else { "{#if" };
        let (nodes, _) =
            self.render_fn(arg(call, 2).ok_or("an if block without a branch")?, body)?;
        let mut sections = vec![(format!("{opener} {}}}", self.render(test)?), nodes)];
        if let Some(alternate) = arg(call, 3).filter(|alternate| !is_nullish(Some(alternate))) {
            let (nodes, _) = self.render_fn(alternate, body)?;
            match nodes.as_slice() {
                [M::Block {
                    sections: inner,
                    close,
                }] if close == "{/if}"
                    && inner
                        .first()
                        .is_some_and(|(header, _)| header.starts_with("{:else if")) =>
                {
                    sections.extend(inner.iter().cloned());
                }
                _ => sections.push(("{:else}".to_string(), nodes)),
            }
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Block {
                sections,
                close: "{/if}".to_string(),
            }],
        )
    }

    pub(super) fn if_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        if arg(call, 1).and_then(thunk_body).is_some() {
            return self.old_if_block(body, call);
        }
        if call.arguments.len() != 2 {
            return fail("an if block with an else-if flag");
        }
        let node = self.node_expr(body, arg(call, 0).ok_or("an if block without an anchor")?)?;
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail("an if block of an unknown shape");
        };
        let params = param_symbols(&arrow.params).ok_or("an if block of an unknown shape")?;
        let [render] = params.as_slice() else {
            return fail("an if block of an unknown shape");
        };
        let render = *render;
        self.rw.forbid(render);
        let mut branches: Vec<(Option<Span>, &'a Expression<'a>)> = Vec::new();
        match (arrow.get_expression(), arrow_statements(arrow)) {
            (None, [Statement::IfStatement(stmt)]) => self.branches(stmt, render, &mut branches)?,
            (None, [Statement::ExpressionStatement(stmt)]) => {
                self.ternary(&stmt.expression, render, &mut branches)?
            }
            (Some(expr), _) => self.ternary(expr, render, &mut branches)?,
            _ => return fail("an if block of an unknown shape"),
        }
        let mut sections = Vec::new();
        for (index, (test, branch)) in branches.into_iter().enumerate() {
            let header = match (index, test) {
                (0, Some(test)) => format!("{{#if {}}}", self.render(test)?),
                (0, None) => return fail("an if block without a condition"),
                (_, Some(test)) => format!("{{:else if {}}}", self.render(test)?),
                (_, None) => "{:else}".to_string(),
            };
            let (nodes, _) = self.render_fn(branch, body)?;
            sections.push((header, nodes));
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Block {
                sections,
                close: "{/if}".to_string(),
            }],
        )
    }

    pub(super) fn render_call(
        &self,
        expr: &'a Expression<'a>,
        render: SymbolId,
    ) -> R<&'a Expression<'a>> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return fail("an if branch of an unknown shape");
        };
        if self.symbol(&call.callee) != Some(render)
            || call.arguments.is_empty()
            || call.arguments.len() > 2
        {
            return fail("an if branch of an unknown shape");
        }
        arg(call, 0).ok_or_else(|| "an if branch of an unknown shape".to_string())
    }

    pub(super) fn branch_statement(
        &self,
        statement: &'a Statement<'a>,
        render: SymbolId,
    ) -> R<&'a Expression<'a>> {
        match statement {
            Statement::ExpressionStatement(stmt) => self.render_call(&stmt.expression, render),
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [only] => self.branch_statement(only, render),
                _ => fail("an if branch of an unknown shape"),
            },
            _ => fail("an if branch of an unknown shape"),
        }
    }

    pub(super) fn branches(
        &self,
        stmt: &'a IfStatement<'a>,
        render: SymbolId,
        out: &mut Vec<(Option<Span>, &'a Expression<'a>)>,
    ) -> R<()> {
        out.push((
            Some(stmt.test.span()),
            self.branch_statement(&stmt.consequent, render)?,
        ));
        match &stmt.alternate {
            None => Ok(()),
            Some(Statement::IfStatement(next)) => self.branches(next, render, out),
            Some(other) => {
                out.push((None, self.branch_statement(other, render)?));
                Ok(())
            }
        }
    }

    pub(super) fn ternary(
        &self,
        expr: &'a Expression<'a>,
        render: SymbolId,
        out: &mut Vec<(Option<Span>, &'a Expression<'a>)>,
    ) -> R<()> {
        match expr.without_parentheses() {
            Expression::ConditionalExpression(conditional) => {
                out.push((
                    Some(conditional.test.span()),
                    self.render_call(&conditional.consequent, render)?,
                ));
                match conditional.alternate.without_parentheses() {
                    Expression::ConditionalExpression(_) => {
                        self.ternary(&conditional.alternate, render, out)
                    }
                    _ => {
                        out.push((None, self.render_call(&conditional.alternate, render)?));
                        Ok(())
                    }
                }
            }
            Expression::LogicalExpression(logical) if logical.operator.as_str() == "&&" => {
                out.push((
                    Some(logical.left.span()),
                    self.render_call(&logical.right, render)?,
                ));
                Ok(())
            }
            _ => fail("an if block of an unknown shape"),
        }
    }

    pub(super) fn each_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let flags = arg(call, 1)
            .and_then(number)
            .ok_or("an each block with computed flags")?;
        let node = self.node_expr(body, arg(call, 0).ok_or("an each block without an anchor")?)?;
        let collection = self.thunk(arg(call, 2).ok_or("an each block without a collection")?)?;
        let render = arg(call, 4).ok_or("an each block without a body")?;
        let fields = match render.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => arrow
                .params
                .items
                .get(1)
                .and_then(|param| binding(&param.pattern))
                .and_then(|item| self.destructure(arrow_statements(arrow), item)),
            _ => None,
        };
        let (nodes, params) = self.render_fn(render, body)?;
        let item = match (&fields, params.get(1)) {
            (Some(fields), _) => Some(self.pattern(fields, &[])?),
            (None, Some(symbol)) => Some(self.scoping.symbol_name(*symbol).to_string()),
            (None, None) => None,
        };
        let index = params
            .get(2)
            .filter(|symbol| !self.scoping.get_resolved_reference_ids(**symbol).is_empty())
            .map(|symbol| self.scoping.symbol_name(*symbol).to_string())
            .filter(|name| name != "$$index");
        let key = match arg(call, 3) {
            Some(key) if self.rt(key) == Some(Rt::Index) => None,
            Some(Expression::ArrowFunctionExpression(arrow)) => {
                let scope = arrow
                    .scope_id
                    .get()
                    .ok_or("a key function without a scope")?;
                let [param] = arrow.params.items.as_slice() else {
                    return fail("a key function of an unknown shape");
                };
                if arrow.params.rest.is_some() {
                    return fail("a key function of an unknown shape");
                }
                match (&fields, params.get(1)) {
                    (Some(fields), _) => self.key_fields(&param.pattern, fields, scope)?,
                    (None, Some(item)) => {
                        let symbol =
                            binding(&param.pattern).ok_or("a key function of an unknown shape")?;
                        self.key_rename(symbol, self.scoping.symbol_name(*item), scope)?;
                    }
                    (None, None) => return fail("a keyed each block without an item"),
                }
                Some(
                    self.render(
                        thunk_body_any(arrow)
                            .ok_or("a key function of an unknown shape")?
                            .span(),
                    )?,
                )
            }
            _ => return fail("an each block with a key of an unknown shape"),
        };
        let mut header = format!("{{#each {collection}");
        if let Some(item) = &item {
            header.push_str(&format!(" as {item}"));
        }
        if let Some(index) = &index {
            if item.is_none() {
                return fail("an each block with an index but no item");
            }
            header.push_str(&format!(", {index}"));
        }
        if let Some(key) = key {
            header.push_str(&format!(" ({key})"));
        }
        header.push('}');
        let mut sections = vec![(header, nodes)];
        if let Some(fallback) = arg(call, 5).filter(|fallback| !is_nullish(Some(fallback))) {
            let (nodes, _) = self.render_fn(fallback, body)?;
            sections.push(("{:else}".to_string(), nodes));
        }
        let block = M::Block {
            sections,
            close: "{/each}".to_string(),
        };
        match flags & 4 != 0 {
            true => {
                if !body.dom[node].children.is_empty()
                    || !matches!(body.dom[node].kind, DKind::Element { .. })
                {
                    return fail("a controlled each block on a node with children");
                }
                self.fill(body, node, Fill::Children(vec![block]))
            }
            false => self.fill_anchor(body, node, vec![block]),
        }
    }

    fn within_scope(&self, span: Span, scope: ScopeId) -> bool {
        let node = self.scoping.get_node_id(scope);
        let outer = self.nodes.kind(node).span();
        outer.start <= span.start && span.end <= outer.end
    }

    fn key_rename(&self, symbol: SymbolId, name: &str, scope: ScopeId) -> R<()> {
        if self.scoping.symbol_name(symbol) == name {
            return Ok(());
        }
        let captured = self.scoping.symbol_ids().any(|other| {
            other != symbol
                && self.scoping.symbol_name(other) == name
                && self
                    .scoping
                    .get_resolved_references(other)
                    .any(|reference| {
                        let at = self.nodes.kind(reference.node_id()).span();
                        self.within_scope(at, scope)
                    })
        });
        if captured {
            return fail("a key function names its item differently");
        }
        self.rw.rename(symbol, name.to_string());
        Ok(())
    }

    fn key_fields(
        &self,
        pattern: &oxc_ast::ast::BindingPattern<'_>,
        fields: &pattern::Destructure,
        scope: ScopeId,
    ) -> R<()> {
        let mut bound = Vec::new();
        if !pattern::key_bindings(pattern, Vec::new(), &mut bound) {
            return fail("a key function of an unknown shape");
        }
        for (path, symbol) in bound {
            let leaf = fields
                .leaves
                .iter()
                .find(|leaf| leaf.path == path)
                .ok_or("a key function reads a field the each block does not")?;
            self.key_rename(symbol, self.scoping.symbol_name(leaf.local), scope)?;
        }
        Ok(())
    }

    pub(super) fn key_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("a key block without an anchor")?)?;
        let value = self.thunk(arg(call, 1).ok_or("a key block without a value")?)?;
        let (nodes, _) = self.render_fn(arg(call, 2).ok_or("a key block without a body")?, body)?;
        let block = M::Block {
            sections: vec![(format!("{{#key {value}}}"), nodes)],
            close: "{/key}".to_string(),
        };
        self.fill_anchor(body, node, vec![block])
    }

    pub(super) fn await_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(
            body,
            arg(call, 0).ok_or("an await block without an anchor")?,
        )?;
        let value = self.thunk(arg(call, 1).ok_or("an await block without a value")?)?;
        let part = |index: usize, body: &mut Body| -> R<Option<(Vec<M>, Option<String>)>> {
            match arg(call, index).filter(|expr| !is_nullish(Some(expr))) {
                None => Ok(None),
                Some(callback) => {
                    let (nodes, params) = self.render_fn(callback, body)?;
                    Ok(Some((
                        nodes,
                        params
                            .get(1)
                            .map(|symbol| self.scoping.symbol_name(*symbol).to_string()),
                    )))
                }
            }
        };
        let pending = part(2, body)?;
        let then = part(3, body)?;
        let catch = part(4, body)?;
        let named = |word: &str, name: Option<String>| match name {
            Some(name) => format!("{word} {name}"),
            None => word.to_string(),
        };
        let mut sections = Vec::new();
        match (pending, then) {
            (Some((nodes, _)), then) => {
                sections.push((format!("{{#await {value}}}"), nodes));
                if let Some((nodes, name)) = then {
                    sections.push((format!("{{:{}}}", named("then", name)), nodes));
                }
            }
            (None, Some((nodes, name))) => {
                sections.push((format!("{{#await {value} {}}}", named("then", name)), nodes))
            }
            (None, None) => {}
        }
        if let Some((nodes, name)) = catch {
            match sections.is_empty() {
                true => sections.push((
                    format!("{{#await {value} {}}}", named("catch", name)),
                    nodes,
                )),
                false => sections.push((format!("{{:{}}}", named("catch", name)), nodes)),
            }
        }
        if sections.is_empty() {
            return fail("an await block without branches");
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Block {
                sections,
                close: "{/await}".to_string(),
            }],
        )
    }

    pub(super) fn html_tag(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("an html tag without an anchor")?)?;
        let value = self.thunk(arg(call, 1).ok_or("an html tag without a value")?)?;
        if call.arguments.len() > 5 {
            return fail("an html tag with extra arguments");
        }
        let controlled = boolean(arg(call, 2)).ok_or("an html tag with a computed flag")?;
        let tag = M::Tag(format!("{{@html {value}}}"));
        match controlled {
            true => {
                if !body.dom[node].children.is_empty() {
                    return fail("a controlled html tag on a node with children");
                }
                self.fill(body, node, Fill::Children(vec![tag]))
            }
            false => self.fill_anchor(body, node, vec![tag]),
        }
    }

    pub(super) fn render_args(&self, call: &'a CallExpression<'a>, from: usize) -> R<String> {
        let mut args = Vec::new();
        for argument in call.arguments.iter().skip(from) {
            let expr = argument
                .as_expression()
                .ok_or("a spread argument to a snippet")?;
            args.push(self.thunk(expr)?);
        }
        Ok(args.join(", "))
    }

    pub(super) fn dynamic_render(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("a render tag without an anchor")?)?;
        let getter = arg(call, 1).ok_or("a render tag without a snippet")?;
        let args = self.render_args(call, 2)?;
        let optional = match getter.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
                match arrow_value(arrow).map(Expression::without_parentheses) {
                    Some(Expression::LogicalExpression(logical))
                        if logical.operator.as_str() == "??"
                            && self.rt(&logical.right) == Some(Rt::Noop) =>
                    {
                        Some(self.code(&logical.left)?)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let tag = match optional {
            Some(snippet) => format!("{{@render {snippet}?.({args})}}"),
            None => format!("{{@render {}({args})}}", self.thunk(getter)?),
        };
        self.fill_anchor(body, node, vec![M::Tag(tag)])
    }

    fn component_object(
        &self,
        body: &mut Body,
        name: &str,
        object: &'a oxc_ast::ast::ObjectExpression<'a>,
        attrs: &mut Vec<String>,
        children: &mut Vec<M>,
    ) -> R<()> {
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return fail(format!("the component `{name}` gets spread props"));
            };
            let key = key_name(&property.key)
                .filter(|_| !property.computed)
                .ok_or("a computed component prop")?;
            match (property.kind, key.as_str()) {
                (PropertyKind::Init, "$$slots") => {}
                (PropertyKind::Init, "children")
                    if matches!(
                        property.value.without_parentheses(),
                        Expression::ArrowFunctionExpression(_)
                    ) =>
                {
                    *children = self.render_fn(&property.value, body)?.0;
                }
                (PropertyKind::Init, _) => match property.value.without_parentheses() {
                    Expression::StringLiteral(_) => attrs.push(format!(
                        "{key}=\"{}\"",
                        braces(
                            &string(&property.value)
                                .ok_or("a prop text with an invalid escape")?
                                .replace('&', "&amp;")
                                .replace('"', "&quot;")
                        )
                    )),
                    _ => attrs.push(attr(&key, &self.code(&property.value)?)),
                },
                (PropertyKind::Get, _) => {
                    let Expression::FunctionExpression(getter) = &property.value else {
                        return fail("a component prop getter of an unknown shape");
                    };
                    let statements = &getter
                        .body
                        .as_ref()
                        .ok_or("an empty prop getter")?
                        .statements;
                    let [Statement::ReturnStatement(ret)] = statements.as_slice() else {
                        return fail("a component prop getter of an unknown shape");
                    };
                    let code = self.code(
                        ret.argument
                            .as_ref()
                            .ok_or("a prop getter returns nothing")?,
                    )?;
                    attrs.push(attr(&key, &code));
                }
                (PropertyKind::Set, _) => {
                    let target = self.setter_function(&property.value)?;
                    let at = attrs
                        .iter()
                        .position(|text| *text == attr(&key, &target))
                        .ok_or_else(|| {
                            format!(
                                "the component prop `{key}` is written but not read the same way"
                            )
                        })?;
                    attrs[at] = match target == key {
                        true => format!("bind:{key}"),
                        false => format!("bind:{key}={{{target}}}"),
                    };
                }
            }
        }
        Ok(())
    }

    pub(super) fn user_call(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return fail(format!(
                "the call `{}` has no template meaning",
                first_line(self.rw.text(call.span))
            ));
        };
        let symbol = symbol_of(callee, self.scoping).ok_or_else(|| {
            format!(
                "the call to the unbound `{}` has no template meaning",
                callee.name
            )
        })?;
        let node = self.node_expr(body, arg(call, 0).ok_or("a call in the template code")?)?;
        if self.arrow(symbol).is_some() && !body.deferred.contains(&symbol) {
            let args = self.render_args(call, 1)?;
            if !self.snippets.borrow().contains(&symbol) {
                self.snippets.borrow_mut().push(symbol);
            }
            return self.fill_anchor(
                body,
                node,
                vec![M::Tag(format!("{{@render {}({args})}}", callee.name))],
            );
        }
        let name = match self.component_name(symbol) {
            Some(name) => {
                if self
                    .scoping
                    .symbol_ids()
                    .any(|other| other != symbol && self.scoping.symbol_name(other) == name)
                {
                    return fail(format!(
                        "the child component name `{name}` is taken by another binding"
                    ));
                }
                name.to_string()
            }
            None => callee.name.to_string(),
        };
        let name = name.as_str();
        if !name.starts_with(|c: char| c.is_ascii_uppercase()) || call.arguments.len() != 2 {
            return fail(format!(
                "the call to `{name}` is neither a snippet nor a component"
            ));
        }
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        match arg(call, 1).map(Expression::without_parentheses) {
            Some(Expression::ObjectExpression(object)) => {
                self.component_object(body, name, object, &mut attrs, &mut children)?
            }
            Some(Expression::CallExpression(spread))
                if self.rt(&spread.callee) == Some(Rt::SpreadProps) =>
            {
                for part in &spread.arguments {
                    match part.as_expression().map(Expression::without_parentheses) {
                        Some(Expression::ObjectExpression(object)) => {
                            self.component_object(body, name, object, &mut attrs, &mut children)?
                        }
                        Some(thunk) if thunk_body(thunk).is_some() => {
                            attrs.push(format!("{{...{}}}", self.thunk(thunk)?));
                        }
                        _ => {
                            return fail(format!(
                                "the component `{name}` gets spread props of an unknown shape"
                            ))
                        }
                    }
                }
            }
            _ => {
                return fail(format!(
                    "the component `{name}` gets props that are not an object literal"
                ))
            }
        }
        if !self.children.borrow().contains(&symbol) {
            self.children.borrow_mut().push(symbol);
        }
        let element = M::Element {
            name: name.to_string(),
            attrs,
            children,
            svg: true,
        };
        self.fill_anchor(body, node, vec![element])
    }
}
