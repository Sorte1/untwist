use super::*;

impl<'a, 's> Cx<'a, 's> {
    pub(super) fn push_attr(&self, body: &mut Body, node: usize, attr: String) {
        body.dom[node].attrs.push(attr);
    }

    pub(super) fn update(&self, body: &mut Body, expr: &'a Expression<'a>) -> R<()> {
        match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                for inner in &sequence.expressions {
                    self.update(body, inner)?;
                }
                Ok(())
            }
            Expression::CallExpression(call) => self.update_call(body, call),
            Expression::AssignmentExpression(assign) if assign.operator.as_str() == "=" => {
                match &assign.left {
                    AssignmentTarget::AssignmentTargetIdentifier(_) => {
                        match assign.right.without_parentheses() {
                            Expression::CallExpression(call)
                                if matches!(
                                    self.rt(&call.callee),
                                    Some(Rt::SetClass | Rt::SetStyle)
                                ) =>
                            {
                                self.update_call(body, call)
                            }
                            Expression::CallExpression(call)
                                if self.rt(&call.callee) == Some(Rt::SetAttributes) =>
                            {
                                self.set_attributes(body, call)
                            }
                            _ => fail("an assignment in the template code"),
                        }
                    }
                    AssignmentTarget::StaticMemberExpression(member) => {
                        let node = self.node_expr(body, &member.object)?;
                        self.property(body, node, member.property.name.as_str(), &assign.right)
                    }
                    _ => fail("an assignment in the template code"),
                }
            }
            _ => fail(format!(
                "`{}` has no template meaning",
                first_line(self.rw.text(expr.span()))
            )),
        }
    }

    pub(super) fn property(
        &self,
        body: &mut Body,
        node: usize,
        name: &str,
        value: &'a Expression<'a>,
    ) -> R<()> {
        if let (Some(event), DKind::Element { .. }) =
            (name.strip_prefix("__"), &body.dom[node].kind)
        {
            if event.is_empty() || !event.bytes().all(|b| b.is_ascii_lowercase()) {
                return fail(format!(
                    "the element property `{name}` has no attribute spelling"
                ));
            }
            let code = self.hoisted_handler(value)?;
            self.push_attr(body, node, attr(&format!("on{event}"), &code));
            return Ok(());
        }
        match (name, &body.dom[node].kind) {
            ("textContent", DKind::Element { .. }) => {
                let text = self.text_value(value)?;
                self.fill(body, node, Fill::Children(vec![M::Text(text)]))
            }
            ("nodeValue", DKind::Text(_)) => {
                let text = self.text_value(value)?;
                self.fill(body, node, Fill::Text(text))
            }
            (_, DKind::Element { .. }) => {
                let attribute = PROPERTIES
                    .iter()
                    .find(|(property, _)| *property == name)
                    .map(|row| row.1)
                    .ok_or_else(|| {
                        format!("the element property `{name}` has no attribute spelling")
                    })?;
                let code = self.code(value)?;
                self.push_attr(body, node, attr(attribute, &code));
                Ok(())
            }
            _ => fail(format!(
                "the property `{name}` is written on a node that is not an element"
            )),
        }
    }

    pub(super) fn update_call(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        match self.rt(&call.callee) {
            Some(Rt::SetText) => {
                let node =
                    self.node_expr(body, arg(call, 0).ok_or("set_text() without a node")?)?;
                if !matches!(body.dom[node].kind, DKind::Text(_)) {
                    return fail("set_text() on a node that is not text");
                }
                let text = self.text_value(arg(call, 1).ok_or("set_text() without a value")?)?;
                self.fill(body, node, Fill::Text(text))
            }
            Some(Rt::SetAttribute) => {
                let node = self.element(body, arg(call, 0))?;
                let name = arg(call, 1)
                    .and_then(string)
                    .ok_or("set_attribute() with a computed name")?;
                if call.arguments.len() > 3 {
                    return fail("set_attribute() with extra arguments");
                }
                let text =
                    self.attr_value(name, arg(call, 2).ok_or("set_attribute() without a value")?)?;
                self.push_attr(body, node, text);
                Ok(())
            }
            Some(rt @ (Rt::SetValue | Rt::SetChecked)) => {
                let node = self.element(body, arg(call, 0))?;
                let code = self.code(arg(call, 1).ok_or("a property update without a value")?)?;
                self.push_attr(
                    body,
                    node,
                    attr(
                        if rt == Rt::SetValue {
                            "value"
                        } else {
                            "checked"
                        },
                        &code,
                    ),
                );
                Ok(())
            }
            Some(Rt::SetClass) => self.set_class(body, call),
            Some(Rt::SetStyle) => self.set_style(body, call),
            Some(Rt::ToggleClass) => {
                let node = self.element(body, arg(call, 0))?;
                let name = arg(call, 1)
                    .and_then(string)
                    .ok_or("toggle_class() with a computed name")?;
                if call.arguments.len() != 3 {
                    return fail("toggle_class() with extra arguments");
                }
                let code = self.code(arg(call, 2).ok_or("toggle_class() without a value")?)?;
                self.push_attr(body, node, directive("class", name, &code));
                Ok(())
            }
            _ => fail(format!(
                "`{}` has no template meaning",
                first_line(self.rw.text(call.span))
            )),
        }
    }

    pub(super) fn set_class(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        if call.arguments.len() > 6 {
            return fail("set_class() with extra arguments");
        }
        let old = call.arguments.len() == 2;
        let value = arg(call, if old { 1 } else { 2 }).ok_or("set_class() without a value")?;
        if let Some(text) = string(value) {
            if let Some(attr) = self.static_attr("class", Some(text)) {
                self.push_attr(body, node, attr);
            }
        } else if !is_nullish(Some(value)) {
            let inner = match value.without_parentheses() {
                Expression::CallExpression(clsx)
                    if self.rt(&clsx.callee) == Some(Rt::Clsx) && clsx.arguments.len() == 1 =>
                {
                    arg(clsx, 0).ok_or("clsx() without a value")?
                }
                _ => value,
            };
            let text = self.attr_value("class", inner)?;
            self.push_attr(body, node, text);
        }
        let directives = match arg(call, 5).map(Expression::without_parentheses) {
            None => None,
            Some(Expression::ObjectExpression(object)) => Some(&**object),
            Some(Expression::Identifier(_)) => match arg(call, 5)
                .and_then(|dep| self.symbol(dep))
                .and_then(|symbol| self.facts.classes.get(&symbol))
            {
                Some(Class::Dep(span)) => Some(
                    self.object_at(*span)
                        .ok_or("class directives that are not an object literal")?,
                ),
                _ => return fail("class directives that are not an object literal"),
            },
            Some(_) => return fail("class directives that are not an object literal"),
        };
        match directives {
            None => Ok(()),
            Some(object) => {
                for property in &object.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        return fail("a spread in the class directives");
                    };
                    let name = key_name(&property.key)
                        .filter(|_| !property.computed)
                        .ok_or("a computed class directive")?;
                    let code = self.code(&property.value)?;
                    self.push_attr(body, node, directive("class", &name, &code));
                }
                Ok(())
            }
        }
    }

    pub(super) fn set_style(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        if call.arguments.len() > 4 {
            return fail("set_style() with extra arguments");
        }
        let value = arg(call, 1).ok_or("set_style() without a value")?;
        match string(value) {
            Some("") => {}
            Some(text) => self.push_attr(body, node, format!("style=\"{}\"", braces(text))),
            None if is_nullish(Some(value)) => {}
            None => {
                let text = self.attr_value("style", value)?;
                self.push_attr(body, node, text);
            }
        }
        match arg(call, 3).map(Expression::without_parentheses) {
            None => Ok(()),
            Some(Expression::ObjectExpression(object)) => {
                for property in &object.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        return fail("a spread in the style directives");
                    };
                    let name = key_name(&property.key)
                        .filter(|_| !property.computed)
                        .ok_or("a computed style directive")?;
                    let directive_text = match string(&property.value) {
                        Some(text) => format!("style:{name}=\"{}\"", braces(text)),
                        None => directive("style", &name, &self.code(&property.value)?),
                    };
                    self.push_attr(body, node, directive_text);
                }
                Ok(())
            }
            Some(_) => fail("style directives that are not an object literal"),
        }
    }

    pub(super) fn radio_value(&self, body: &mut Body, stmt: &'a IfStatement<'a>) -> R<()> {
        let shape = "an if statement in the template code";
        if stmt.alternate.is_some() {
            return fail(shape);
        }
        let Expression::BinaryExpression(test) = stmt.test.without_parentheses() else {
            return fail(shape);
        };
        let Expression::AssignmentExpression(cache) = test.right.without_parentheses() else {
            return fail(shape);
        };
        let consequent = match &stmt.consequent {
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [only] => only,
                _ => return fail(shape),
            },
            other => other,
        };
        let Statement::ExpressionStatement(write) = consequent else {
            return fail(shape);
        };
        let Expression::AssignmentExpression(write) = write.expression.without_parentheses() else {
            return fail(shape);
        };
        let AssignmentTarget::StaticMemberExpression(member) = &write.left else {
            return fail(shape);
        };
        if test.operator.as_str() != "!==" || member.property.name != "value" {
            return fail(shape);
        }
        let node = self.node_expr(body, &member.object)?;
        let code = self.code(&cache.right)?;
        self.push_attr(body, node, attr("value", &code));
        Ok(())
    }

    pub(super) fn event(&self, body: &mut Body, call: &'a CallExpression<'a>, rt: Rt) -> R<()> {
        let name = arg(call, 0)
            .and_then(string)
            .ok_or("an event with a computed name")?;
        let special = match arg(call, 1).and_then(|target| self.rt(target)) {
            Some(Rt::Window) => Some("svelte:window"),
            Some(Rt::Document) => Some("svelte:document"),
            _ => None,
        };
        if let Some(special) = special {
            if rt != Rt::Event || call.arguments.len() > 3 {
                return fail("a window or document event with flags");
            }
            let code = self.code(arg(call, 2).ok_or("an event without a handler")?)?;
            self.specials
                .borrow_mut()
                .push((special, attr(&format!("on{name}"), &code)));
            return Ok(());
        }
        let node = self.element(body, arg(call, 1))?;
        let handler = arg(call, 2).ok_or("an event without a handler")?;
        let capture = match rt {
            Rt::Event => boolean(arg(call, 3)).ok_or("an event with a computed capture flag")?,
            _ => false,
        };
        if rt == Rt::Delegated && call.arguments.len() > 3 {
            return fail("a delegated event with extra arguments");
        }
        let code = self.code(handler)?;
        let attribute = format!("on{name}{}", if capture { "capture" } else { "" });
        self.push_attr(body, node, attr(&attribute, &code));
        Ok(())
    }

    fn object_at(&self, span: Span) -> Option<&'a oxc_ast::ast::ObjectExpression<'a>> {
        self.nodes.iter().find_map(|node| match node.kind() {
            AstKind::ObjectExpression(object)
                if span.start <= object.span.start
                    && object.span.end <= span.end
                    && span.size() - object.span.size() <= 2 =>
            {
                Some(object)
            }
            _ => None,
        })
    }

    fn hoisted_handler(&self, value: &'a Expression<'a>) -> R<String> {
        let Expression::ArrayExpression(array) = value.without_parentheses() else {
            return self.code(value);
        };
        let shape = "a hoisted event handler of an unknown shape";
        let mut items = array.elements.iter().map(|element| element.as_expression());
        let handler = items.next().flatten().ok_or(shape)?;
        let symbol = self.symbol(handler).ok_or(shape)?;
        let function = self
            .declarator(symbol)
            .and_then(|declarator| declarator.init.as_ref())
            .ok_or(shape)?;
        let Expression::ArrowFunctionExpression(arrow) = function.without_parentheses() else {
            return fail(shape);
        };
        let params = param_symbols(&arrow.params).ok_or(shape)?;
        let args: Vec<SymbolId> = items
            .map(|item| item.and_then(|item| self.symbol(item)))
            .collect::<Option<_>>()
            .ok_or(shape)?;
        if params.len() != args.len() + 1 {
            return fail(shape);
        }
        let scope = arrow.scope_id.get().ok_or(shape)?;
        for (param, arg) in params.iter().skip(1).zip(&args) {
            let name = self.scoping.symbol_name(*arg);
            if self.scoping.symbol_name(*param) != name {
                if self.hoisted_clash(scope, name, *param) {
                    return fail("a hoisted event handler whose names would clash");
                }
                self.rw.rename(*param, name.to_string());
            }
        }
        let event = params[0];
        let head = match self.scoping.get_resolved_reference_ids(event).is_empty() {
            true => "()".to_string(),
            false => format!("({})", self.scoping.symbol_name(event)),
        };
        let text = match arrow.get_expression() {
            Some(expr) => self.render(expr.span())?,
            None => self.render(arrow.body.span())?,
        };
        Ok(format!("{head} => {text}"))
    }

    fn hoisted_clash(&self, scope: ScopeId, name: &str, except: SymbolId) -> bool {
        self.scoping.symbol_ids().any(|other| {
            other != except
                && self.scoping.symbol_name(other) == name
                && self
                    .scoping
                    .scope_ancestors(self.scoping.symbol_scope_id(other))
                    .any(|ancestor| ancestor == scope)
        })
    }

    pub(super) fn getter_code(&self, expr: &'a Expression<'a>) -> R<String> {
        if let Some(body) = thunk_body(expr) {
            return self.render(body);
        }
        let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
            return fail("a binding getter of an unknown shape");
        };
        if !arrow.params.items.is_empty() {
            return fail("a binding getter takes parameters");
        }
        let (last, reads) = arrow_statements(arrow)
            .split_last()
            .ok_or("an empty binding getter")?;
        for read in reads {
            let Statement::ExpressionStatement(read) = read else {
                return fail("a binding getter with statements");
            };
            if !matches!(read.expression.without_parentheses(), Expression::CallExpression(call) if self.rt(&call.callee) == Some(Rt::Get))
            {
                return fail("a binding getter with statements");
            }
        }
        match last {
            Statement::ReturnStatement(ret) => self.code(
                ret.argument
                    .as_ref()
                    .ok_or("a binding getter returns nothing")?,
            ),
            _ => fail("a binding getter of an unknown shape"),
        }
    }

    pub(super) fn setter_target(&self, expr: &'a Expression<'a>) -> R<String> {
        let shape = "a binding setter of an unknown shape";
        let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
            return fail(shape);
        };
        let params = param_symbols(&arrow.params).ok_or(shape)?;
        let [param] = params.as_slice() else {
            return fail(shape);
        };
        self.rw.forbid(*param);
        self.assigned(thunk_body_any(arrow).ok_or(shape)?, *param, shape, 2)
    }

    pub(super) fn setter_function(&self, expr: &'a Expression<'a>) -> R<String> {
        let shape = "a component prop setter of an unknown shape";
        let Expression::FunctionExpression(setter) = expr.without_parentheses() else {
            return fail(shape);
        };
        let params = param_symbols(&setter.params).ok_or(shape)?;
        let [param] = params.as_slice() else {
            return fail(shape);
        };
        let statements = &setter.body.as_ref().ok_or(shape)?.statements;
        let [Statement::ExpressionStatement(stmt)] = statements.as_slice() else {
            return fail(shape);
        };
        self.rw.forbid(*param);
        self.assigned(&stmt.expression, *param, shape, 3)
    }

    pub(super) fn assigned(
        &self,
        body: &'a Expression<'a>,
        param: SymbolId,
        shape: &str,
        set_args: usize,
    ) -> R<String> {
        let is_param =
            |expr: Option<&Expression<'_>>| expr.and_then(|expr| self.symbol(expr)) == Some(param);
        match body.without_parentheses() {
            Expression::CallExpression(call)
                if self.rt(&call.callee) == Some(Rt::Set)
                    && is_param(arg(call, 1))
                    && call.arguments.len() <= set_args =>
            {
                let Some(Expression::Identifier(target)) = arg(call, 0) else {
                    return fail(shape);
                };
                Ok(target.name.to_string())
            }
            Expression::CallExpression(call)
                if is_param(arg(call, 0)) && call.arguments.len() == 1 =>
            {
                let Expression::Identifier(target) = &call.callee else {
                    return fail(shape);
                };
                match self
                    .symbol(&call.callee)
                    .and_then(|symbol| self.facts.classes.get(&symbol))
                {
                    Some(Class::Getter) => Ok(target.name.to_string()),
                    _ => fail(shape),
                }
            }
            Expression::AssignmentExpression(assign)
                if assign.operator.as_str() == "=" && is_param(Some(&assign.right)) =>
            {
                self.render(assign.left.span())
            }
            _ => fail(shape),
        }
    }

    pub(super) fn attr_value(&self, name: &str, expr: &Expression<'_>) -> R<String> {
        if let Some(value) = string(expr) {
            return Ok(format!(
                "{name}=\"{}\"",
                braces(&value.replace('&', "&amp;").replace('"', "&quot;"))
            ));
        }
        let Expression::TemplateLiteral(template) = expr.without_parentheses() else {
            return Ok(attr(name, &self.code(expr)?));
        };
        if template.expressions.is_empty() {
            return Ok(attr(name, &self.code(expr)?));
        }
        let mut out = String::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            let cooked = quasi
                .value
                .cooked
                .and_then(|cooked| cooked.as_str())
                .ok_or("a template text with an invalid escape")?;
            out.push_str(&braces(
                &cooked.replace('&', "&amp;").replace('"', "&quot;"),
            ));
            if let Some(expr) = template.expressions.get(index) {
                out.push_str(&format!("{{{}}}", self.code(nullish_empty(expr))?));
            }
        }
        Ok(format!("{name}=\"{out}\""))
    }

    fn function_binding(
        &self,
        name: &str,
        getter: &'a Expression<'a>,
        setter: &'a Expression<'a>,
    ) -> R<Option<String>> {
        let Expression::ArrowFunctionExpression(arrow) = setter.without_parentheses() else {
            return Ok(None);
        };
        let params = param_symbols(&arrow.params);
        let Some(&[param]) = params.as_deref() else {
            return Ok(None);
        };
        let canonical =
            thunk_body_any(arrow).is_some_and(|body| self.assigned(body, param, "", 2).is_ok());
        if canonical || thunk_body(getter).is_none() {
            return Ok(None);
        }
        let get = self.render(getter.span())?;
        let set = self.render(setter.span())?;
        Ok(Some(format!("bind:{name}={{{get}, {set}}}")))
    }

    pub(super) fn bind(&self, body: &mut Body, call: &'a CallExpression<'a>, rt: Rt) -> R<()> {
        let (node, getter, setter, name) = match rt {
            Rt::BindValue => (arg(call, 0), arg(call, 1), arg(call, 2), "value"),
            Rt::BindChecked => (arg(call, 0), arg(call, 1), arg(call, 2), "checked"),
            Rt::BindGroup => {
                if !matches!(arg(call, 1), Some(Expression::ArrayExpression(array)) if array.elements.is_empty())
                {
                    return fail("a group binding inside a keyed or nested each");
                }
                (arg(call, 2), arg(call, 3), arg(call, 4), "group")
            }
            _ => {
                if call.arguments.len() > 3 {
                    return fail("bind:this inside an each block");
                }
                (arg(call, 0), arg(call, 2), arg(call, 1), "this")
            }
        };
        let node = self.element(body, node)?;
        let (getter, setter) = (
            getter.ok_or("a binding without a getter")?,
            setter.ok_or("a binding without a setter")?,
        );
        if rt != Rt::BindThis {
            if let Some(text) = self.function_binding(name, getter, setter)? {
                self.push_attr(body, node, text);
                return Ok(());
            }
        }
        let getter = self.getter_code(getter)?;
        let target = self.setter_target(setter)?;
        if getter != target {
            return fail(format!(
                "bind:{name} reads `{getter}` but writes `{target}`"
            ));
        }
        let text = match getter == name {
            true => format!("bind:{name}"),
            false => format!("bind:{name}={{{getter}}}"),
        };
        self.push_attr(body, node, text);
        Ok(())
    }
}
