use super::*;

impl<'a, 's> Cx<'a, 's> {
    pub(super) fn if_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
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
        if flags & 8 != 0 {
            return fail("an animated each block");
        }
        let node = self.node_expr(body, arg(call, 0).ok_or("an each block without an anchor")?)?;
        let collection = self.thunk(arg(call, 2).ok_or("an each block without a collection")?)?;
        let render = arg(call, 4).ok_or("an each block without a body")?;
        let (nodes, params) = self.render_fn(render, body)?;
        let mut item = params
            .get(1)
            .map(|symbol| self.scoping.symbol_name(*symbol).to_string());
        let index = params
            .get(2)
            .map(|symbol| self.scoping.symbol_name(*symbol).to_string())
            .filter(|name| name != "$$index");
        if item.as_deref() == Some("$$item") {
            item = Some(self.destructured(render)?);
        }
        let key = match arg(call, 3) {
            Some(key) if self.rt(key) == Some(Rt::Index) => None,
            Some(Expression::ArrowFunctionExpression(arrow))
                if item.as_deref().is_some_and(|item| item.starts_with('{')) =>
            {
                Some(
                    self.render(
                        thunk_body_any(arrow)
                            .ok_or("a key function of an unknown shape")?
                            .span(),
                    )?,
                )
            }
            Some(Expression::ArrowFunctionExpression(arrow)) => {
                let params =
                    param_symbols(&arrow.params).ok_or("a key function of an unknown shape")?;
                let [param] = params.as_slice() else {
                    return fail("a key function of an unknown shape");
                };
                if Some(self.scoping.symbol_name(*param)) != item.as_deref() {
                    return fail("a key function names its item differently");
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

    pub(super) fn destructured(&self, render: &'a Expression<'a>) -> R<String> {
        let shape = "an each block that destructures its item in a way with no simple pattern";
        let Expression::ArrowFunctionExpression(arrow) = render.without_parentheses() else {
            return fail(shape);
        };
        let mut names = Vec::new();
        for symbol in crate::script::leading_getters(arrow_statements(arrow)) {
            let getter = self.arrow(symbol).ok_or(shape)?;
            let Some(Expression::StaticMemberExpression(member)) =
                getter.get_expression().map(Expression::without_parentheses)
            else {
                return fail(shape);
            };
            let Expression::CallExpression(read) = member.object.without_parentheses() else {
                return fail(shape);
            };
            let name = self.scoping.symbol_name(symbol);
            if self.rt(&read.callee) != Some(Rt::Get) || member.property.name != name {
                return fail(shape);
            }
            names.push(name.to_string());
        }
        if names.is_empty() {
            return fail(shape);
        }
        Ok(format!("{{ {} }}", names.join(", ")))
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
        let name = callee.name.as_str();
        if !name.starts_with(|c: char| c.is_ascii_uppercase()) || call.arguments.len() != 2 {
            return fail(format!(
                "the call to `{name}` is neither a snippet nor a component"
            ));
        }
        let Some(Expression::ObjectExpression(object)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail(format!(
                "the component `{name}` gets props that are not an object literal"
            ));
        };
        let mut attrs = Vec::new();
        let mut children = Vec::new();
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
                    children = self.render_fn(&property.value, body)?.0;
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
