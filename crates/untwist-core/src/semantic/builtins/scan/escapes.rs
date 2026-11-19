use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) type Escaping = FxHashSet<usize>;

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

const HANDLER_PREFIX: &str = "on";

#[derive(Clone, Copy)]
enum Flow {
    Out,
    Into(SymbolId),
}

struct Escapes<'s> {
    scoping: &'s Scoping,
    module: bool,
    functions: Escaping,
    seen: Vec<usize>,
    placed: FxHashSet<usize>,
    symbols: FxHashSet<SymbolId>,
    held_functions: FxHashMap<SymbolId, Vec<usize>>,
    held_symbols: FxHashMap<SymbolId, Vec<SymbolId>>,
    literal_holders: FxHashSet<SymbolId>,
    derived: Vec<bool>,
}

pub(super) fn escaping_functions(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
) -> Escaping {
    let mut found = Escapes {
        scoping,
        module,
        functions: FxHashSet::default(),
        seen: Vec::new(),
        placed: FxHashSet::default(),
        symbols: FxHashSet::default(),
        held_functions: FxHashMap::default(),
        held_symbols: FxHashMap::default(),
        literal_holders: literal_holders(program, scoping),
        derived: Vec::new(),
    };
    found.visit_program(program);
    found.script_globals_escape();
    found.close()
}

pub(super) fn function_address(function: &Function<'_>) -> usize {
    std::ptr::from_ref(function).addr()
}

impl Escapes<'_> {
    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn script_globals_escape(&mut self) {
        if self.module {
            return;
        }
        let root = self.scoping.root_scope_id();
        let globals: Vec<SymbolId> = self
            .scoping
            .get_bindings(root)
            .iter()
            .map(|(_, symbol)| *symbol)
            .filter(|symbol| {
                self.scoping
                    .symbol_flags(*symbol)
                    .intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function)
            })
            .collect();
        self.symbols.extend(globals);
    }

    fn function_flows(&mut self, address: usize, flow: Flow) {
        self.placed.insert(address);
        match flow {
            Flow::Out => {
                self.functions.insert(address);
            }
            Flow::Into(holder) => self.held_functions.entry(holder).or_default().push(address),
        }
    }

    fn symbol_flows(&mut self, symbol: SymbolId, flow: Flow) {
        match flow {
            Flow::Out => {
                self.symbols.insert(symbol);
            }
            Flow::Into(holder) => self.held_symbols.entry(holder).or_default().push(symbol),
        }
    }

    fn flows(&mut self, expression: &Expression<'_>, flow: Flow) {
        match expression.without_parentheses() {
            Expression::FunctionExpression(function) => {
                self.function_flows(function_address(function), flow);
            }
            Expression::Identifier(id) => {
                if let Some(symbol) = self.symbol_of(id) {
                    self.symbol_flows(symbol, flow);
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            self.flows(&property.value, flow);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.flows(&spread.argument, flow);
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.flows(&spread.argument, flow);
                        }
                        other => {
                            if let Some(expression) = other.as_expression() {
                                self.flows(expression, flow);
                            }
                        }
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.flows(&conditional.consequent, flow);
                self.flows(&conditional.alternate, flow);
            }
            Expression::LogicalExpression(logical) => {
                self.flows(&logical.left, flow);
                self.flows(&logical.right, flow);
            }
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    self.flows(last, flow);
                }
            }
            Expression::AssignmentExpression(assignment) => self.flows(&assignment.right, flow),
            Expression::AwaitExpression(awaited) => self.flows(&awaited.argument, flow),
            Expression::StaticMemberExpression(member) => self.flows(&member.object, flow),
            Expression::ComputedMemberExpression(member) => self.flows(&member.object, flow),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    if let Some(object) = forwarded(&call.callee) {
                        self.flows(object, flow);
                    }
                }
                ChainElement::TSNonNullExpression(non_null) => {
                    self.flows(&non_null.expression, flow);
                }
                ChainElement::StaticMemberExpression(_)
                | ChainElement::ComputedMemberExpression(_)
                | ChainElement::PrivateFieldExpression(_) => {
                    if let Some(member) = chain.expression.as_member_expression() {
                        self.flows(member.object(), flow);
                    }
                }
            },
            Expression::CallExpression(call) => {
                if let Some(object) = forwarded(&call.callee) {
                    self.flows(object, flow);
                }
            }
            _ => {}
        }
    }

    fn escapes(&mut self, expression: &Expression<'_>) {
        self.flows(expression, Flow::Out);
    }

    fn argument_escapes(&mut self, argument: &Argument<'_>) {
        match argument {
            Argument::SpreadElement(spread) => self.escapes(&spread.argument),
            other => {
                if let Some(expression) = other.as_expression() {
                    self.escapes(expression);
                }
            }
        }
    }

    fn member_holder(&self, object: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = object.without_parentheses() else {
            return None;
        };
        self.symbol_of(id)
            .filter(|symbol| self.literal_holders.contains(symbol))
    }

    fn target_flow(&self, target: &AssignmentTarget<'_>) -> Flow {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.symbol_of(id).map_or(Flow::Out, Flow::Into)
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                if member.property.name.as_str().starts_with(HANDLER_PREFIX) {
                    return Flow::Out;
                }
                self.member_holder(&member.object)
                    .map_or(Flow::Out, Flow::Into)
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                let handler = match member.expression.without_parentheses() {
                    Expression::StringLiteral(literal) => literal
                        .value
                        .as_str()
                        .is_none_or(|name| name.starts_with(HANDLER_PREFIX)),
                    _ => true,
                };
                if handler {
                    return Flow::Out;
                }
                self.member_holder(&member.object)
                    .map_or(Flow::Out, Flow::Into)
            }
            _ => Flow::Out,
        }
    }

    fn called_directly(&mut self, callee: &Expression<'_>) {
        if let Expression::FunctionExpression(function) = callee.without_parentheses() {
            self.placed.insert(function_address(function));
        }
    }

    fn close(mut self) -> Escaping {
        let unplaced: Vec<usize> = self
            .seen
            .iter()
            .copied()
            .filter(|address| !self.placed.contains(address))
            .collect();
        self.functions.extend(unplaced);
        let mut pending: Vec<SymbolId> = self.symbols.iter().copied().collect();
        while let Some(symbol) = pending.pop() {
            if let Some(functions) = self.held_functions.get(&symbol) {
                self.functions.extend(functions.iter().copied());
            }
            if let Some(held) = self.held_symbols.get(&symbol) {
                for inner in held {
                    if self.symbols.insert(*inner) {
                        pending.push(*inner);
                    }
                }
            }
        }
        self.functions
    }
}

fn forwarded<'e, 'a>(callee: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    match callee.without_parentheses() {
        Expression::ChainExpression(chain) => match chain.expression.as_member_expression()? {
            MemberExpression::StaticMemberExpression(member) => FORWARDERS
                .contains(&member.property.name.as_str())
                .then_some(&member.object),
            MemberExpression::ComputedMemberExpression(member) => {
                forwarded_key(&member.expression).then_some(&member.object)
            }
            MemberExpression::PrivateFieldExpression(_) => None,
        },
        Expression::StaticMemberExpression(member)
            if FORWARDERS.contains(&member.property.name.as_str()) =>
        {
            Some(&member.object)
        }
        Expression::ComputedMemberExpression(member) => {
            match member.expression.without_parentheses() {
                Expression::StringLiteral(literal)
                    if literal
                        .value
                        .as_str()
                        .is_some_and(|name| FORWARDERS.contains(&name)) =>
                {
                    Some(&member.object)
                }
                Expression::StringLiteral(_) => None,
                _ => Some(&member.object),
            }
        }
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Escapes<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.called_directly(&it.callee);
        if let Some(object) = forwarded(&it.callee) {
            self.escapes(object);
        }
        it.arguments
            .iter()
            .for_each(|argument| self.argument_escapes(argument));
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.called_directly(&it.callee);
        it.arguments
            .iter()
            .for_each(|argument| self.argument_escapes(argument));
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if let Some(object) = forwarded(&it.tag) {
            self.escapes(object);
        }
        it.quasi
            .expressions
            .iter()
            .for_each(|expression| self.escapes(expression));
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let Some(argument) = &it.argument {
            self.escapes(argument);
        }
        walk_js::walk_return_statement(self, it);
    }

    fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
        self.escapes(&it.argument);
        walk_js::walk_throw_statement(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        if let Some(argument) = &it.argument {
            self.escapes(argument);
        }
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if let Some(body) = it.get_expression() {
            self.escapes(body);
        }
        walk_js::walk_arrow_function_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        self.escapes_array(it);
        walk_js::walk_array_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let flow = self.target_flow(&it.left);
        self.flows(&it.right, flow);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            let flow = match &it.id {
                BindingPattern::BindingIdentifier(id) => {
                    id.symbol_id.get().map_or(Flow::Out, Flow::Into)
                }
                _ => Flow::Out,
            };
            self.flows(init, flow);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let address = function_address(it);
        self.seen.push(address);
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            match it.is_declaration() {
                true => self.function_flows(address, Flow::Into(symbol)),
                false => self.held_functions.entry(symbol).or_default().push(address),
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.derived.push(it.heritage.is_some());
        walk_js::walk_class(self, it);
        self.derived.pop();
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        let derived = self.derived.last().copied().unwrap_or(true);
        if it.kind == MethodDefinitionKind::Constructor && !derived {
            self.placed.insert(function_address(&it.value));
        }
        walk_js::walk_method_definition(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        if let Some(value) = &it.value {
            self.escapes(value);
        }
        walk_js::walk_property_definition(self, it);
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        if let Some(value) = &it.value {
            self.escapes(value);
        }
        walk_js::walk_accessor_property(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let Some(initializer) = &it.initializer {
            self.escapes(initializer);
        }
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        self.escapes(&it.right);
        walk_js::walk_assignment_pattern(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        self.escapes(&it.init);
        walk_js::walk_assignment_target_with_default(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                self.function_flows(function_address(function), Flow::Out);
            }
            other => {
                if let Some(expression) = other.as_expression() {
                    self.escapes(expression);
                }
            }
        }
        walk_js::walk_export_default_declaration(self, it);
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        self.declaration_escapes(&it.declaration);
        walk_js::walk_export_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        for specifier in &it.specifiers {
            if let ModuleExportName::IdentifierReference(id) = &specifier.local {
                if let Some(symbol) = self.symbol_of(id) {
                    self.symbols.insert(symbol);
                }
            }
        }
        walk_js::walk_export_named_declaration(self, it);
    }
}

impl Escapes<'_> {
    fn declaration_escapes(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::FunctionDeclaration(function) => {
                self.function_flows(function_address(function), Flow::Out);
            }
            Declaration::VariableDeclaration(variables) => {
                for declarator in &variables.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                        self.symbols.extend(id.symbol_id.get());
                    }
                }
            }
            _ => {}
        }
    }

    fn escapes_array(&mut self, it: &ArrayExpression<'_>) {
        for element in &it.elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => self.escapes(&spread.argument),
                other => {
                    if let Some(expression) = other.as_expression() {
                        self.escapes(expression);
                    }
                }
            }
        }
    }
}

fn forwarded_key(key: &Expression<'_>) -> bool {
    match key.without_parentheses() {
        Expression::StringLiteral(literal) => literal
            .value
            .as_str()
            .is_none_or(|name| FORWARDERS.contains(&name)),
        _ => true,
    }
}

struct LiteralHolders<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<SymbolId>,
}

fn literal_holders(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut holders = LiteralHolders {
        scoping,
        found: FxHashSet::default(),
    };
    holders.visit_program(program);
    holders.found
}

impl<'a> VisitJs<'a> for LiteralHolders<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let literal = matches!(
            it.init.as_ref().map(Expression::without_parentheses),
            Some(Expression::ObjectExpression(_) | Expression::ArrayExpression(_))
        );
        if let (true, BindingPattern::BindingIdentifier(id)) = (literal, &it.id) {
            let symbol = id
                .symbol_id
                .get()
                .filter(|symbol| !self.scoping.symbol_is_mutated(*symbol));
            self.found.extend(symbol);
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
