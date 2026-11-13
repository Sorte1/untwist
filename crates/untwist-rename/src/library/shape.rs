use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast::AstType;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::js_text::js_text;

use super::hash::{fnv, mix, Exact, PolyHash};

mod chain;
mod inline;

const TAG_TYPE: u8 = 1;
const TAG_STR: u8 = 2;
const TAG_GLOBAL: u8 = 3;
const TAG_OP: u8 = 4;
const TAG_FLAG: u8 = 5;
const TAG_NUMBER: u8 = 6;

const END: u64 = 0x5bd1_e995_0000_0001;
const ID: u64 = 0x5bd1_e995_0000_0002;
const ELSE: u64 = 0x5bd1_e995_0000_0003;
const DECL_ID: u64 = 0x5bd1_e995_0000_0004;
const LABEL: u64 = 0x5bd1_e995_0000_0005;
const COMPUTED: u64 = 0x5bd1_e995_0000_0006;
const INNER: u64 = 0x5bd1_e995_0000_0007;
const OUTER: u64 = 0x5bd1_e995_0000_0008;

fn tagged(tag: u8, text: &str) -> u64 {
    fnv(tag, text.as_bytes())
}

thread_local! {
    static TYPE_TOKENS: std::cell::RefCell<[u64; 256]> = const { std::cell::RefCell::new([0; 256]) };
}

fn type_token(ty: AstType) -> u64 {
    TYPE_TOKENS.with(|tokens| {
        let mut tokens = tokens.borrow_mut();
        let slot = &mut tokens[ty as usize];
        if *slot == 0 {
            *slot = tagged(TAG_TYPE, &format!("{ty:?}"));
        }
        *slot
    })
}

fn property_kind(kind: PropertyKind) -> &'static str {
    match kind {
        PropertyKind::Init => "Init",
        PropertyKind::Get => "Get",
        PropertyKind::Set => "Set",
    }
}

fn method_kind(kind: MethodDefinitionKind) -> &'static str {
    match kind {
        MethodDefinitionKind::Constructor => "Constructor",
        MethodDefinitionKind::Method => "Method",
        MethodDefinitionKind::Get => "Get",
        MethodDefinitionKind::Set => "Set",
    }
}

fn flag(text: &str) -> u64 {
    tagged(TAG_FLAG, text)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Function,
    Program,
}

#[derive(Clone, Copy, Debug)]
pub enum UnitNode<'n, 'a> {
    Function(&'n Function<'a>),
    Arrow(&'n ArrowFunctionExpression<'a>),
    Program(&'n Program<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct Unit<'n, 'a> {
    pub node: UnitNode<'n, 'a>,
    pub kind: UnitKind,
    pub start: u32,
    pub len: u32,
    pub scope: Option<ScopeId>,
    pub binding: Option<SymbolId>,
    pub span: Span,
}

impl Unit<'_, '_> {
    pub fn loose_key(&self, prefix: &PolyHash) -> (u32, u64) {
        (self.len, prefix.range(self.start, self.len))
    }
}

pub struct Shaped<'n, 'a> {
    pub units: Vec<Unit<'n, 'a>>,
    pub prefix: PolyHash,
}

pub fn shape_program<'n, 'a>(program: &'n Program<'a>, scoping: &'n Scoping) -> Shaped<'n, 'a> {
    let mut emitter = Emitter::new(scoping, Sink::Prefix(PolyHash::new()));
    emitter.collect_units = true;
    emitter.visit_program_body(program);
    let len = emitter.count;
    emitter.emit_deferred();
    let mut units = std::mem::take(&mut emitter.units);
    units.push(Unit {
        node: UnitNode::Program(program),
        kind: UnitKind::Program,
        start: 0,
        len,
        scope: Some(scoping.root_scope_id()),
        binding: None,
        span: program.span,
    });
    let Sink::Prefix(prefix) = emitter.sink else {
        unreachable!("the program was shaped into a prefix hash")
    };
    Shaped { units, prefix }
}

#[derive(Clone, Debug)]
pub struct ExactShape {
    pub exact: u128,
    pub loose: (u32, u64),
    pub inner: Vec<SymbolId>,
    pub outer: Vec<SymbolId>,
    pub specific: u32,
    pub inlined: Vec<(SymbolId, Span)>,
}

pub fn exact_shape(unit: &Unit<'_, '_>, scoping: &Scoping) -> ExactShape {
    let sink = Sink::Exact(ExactSink {
        hash: Exact::new(),
        loose: PolyHash::new(),
        scope: unit.scope,
        inner: Vec::new(),
        outer: Vec::new(),
        specific: 0,
        inlined: Vec::new(),
    });
    let mut emitter = Emitter::new(scoping, sink);
    match unit.node {
        UnitNode::Function(function) => emitter.function_unit(function),
        UnitNode::Arrow(arrow) => emitter.arrow_unit(arrow),
        UnitNode::Program(program) => emitter.visit_program_body(program),
    }
    let count = emitter.count;
    let Sink::Exact(sink) = emitter.sink else {
        unreachable!("the unit was shaped into an exact hash")
    };
    ExactShape {
        exact: sink.hash.finish(),
        loose: (count, sink.loose.range(0, count)),
        inner: sink.inner,
        outer: sink.outer,
        specific: sink.specific,
        inlined: sink.inlined,
    }
}

struct ExactSink {
    hash: Exact,
    loose: PolyHash,
    scope: Option<ScopeId>,
    inner: Vec<SymbolId>,
    outer: Vec<SymbolId>,
    specific: u32,
    inlined: Vec<(SymbolId, Span)>,
}

enum Sink {
    Prefix(PolyHash),
    Exact(ExactSink),
}

fn is_within(scoping: &Scoping, symbol: SymbolId, root: Option<ScopeId>) -> bool {
    let Some(root) = root else {
        return false;
    };
    let mut scope = Some(scoping.symbol_scope_id(symbol));
    while let Some(current) = scope {
        if current == root {
            return true;
        }
        scope = scoping.scope_parent_id(current);
    }
    false
}

impl ExactSink {
    fn placeholder(&mut self, scoping: &Scoping, symbol: SymbolId) -> u64 {
        if let Some(index) = self.inner.iter().position(|&known| known == symbol) {
            return mix(INNER ^ index as u64);
        }
        if let Some(index) = self.outer.iter().position(|&known| known == symbol) {
            return mix(OUTER ^ index as u64);
        }
        if is_within(scoping, symbol, self.scope) {
            self.inner.push(symbol);
            mix(INNER ^ (self.inner.len() - 1) as u64)
        } else {
            self.outer.push(symbol);
            mix(OUTER ^ (self.outer.len() - 1) as u64)
        }
    }
}

struct Emitter<'n, 'a> {
    scoping: &'n Scoping,
    sink: Sink,
    count: u32,
    collect_units: bool,
    units: Vec<Unit<'n, 'a>>,
    pending: Option<(Span, SymbolId)>,
    deferred: Vec<&'n Function<'a>>,
}

fn function_value_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

fn exits(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(_)
        | Statement::ThrowStatement(_)
        | Statement::BreakStatement(_)
        | Statement::ContinueStatement(_) => true,
        Statement::BlockStatement(block) => block.body.last().is_some_and(exits),
        Statement::IfStatement(branch) => {
            exits(&branch.consequent) && branch.alternate.as_ref().is_some_and(exits)
        }
        _ => false,
    }
}

fn is_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_) => true,
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void && is_literal(&unary.argument)
        }
        _ => false,
    }
}

fn is_typeof(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Typeof)
}

fn is_equality(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equality
            | BinaryOperator::StrictEquality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictInequality
    )
}

fn loose_operator(operator: BinaryOperator) -> BinaryOperator {
    match operator {
        BinaryOperator::StrictEquality => BinaryOperator::Equality,
        BinaryOperator::StrictInequality => BinaryOperator::Inequality,
        other => other,
    }
}

fn assigned_symbol(statement: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Statement::ExpressionStatement(expression) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = expression.expression.without_parentheses()
    else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    crate::collect::reference_symbol(target, scoping)
}

fn declared_value<'d, 'a>(
    declarator: &'d VariableDeclarator<'a>,
) -> Option<(SymbolId, &'d Expression<'a>)> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.symbol_id.get()?, declarator.init.as_ref()?))
}

fn single_declared_value<'d, 'a>(
    statement: &'d Statement<'a>,
) -> Option<(SymbolId, &'d Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    match declaration.declarations.as_slice() {
        [declarator] => declared_value(declarator),
        _ => None,
    }
}

fn returned_symbol(statement: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Statement::ReturnStatement(ret) = statement else {
        return None;
    };
    let Some(Expression::Identifier(id)) =
        ret.argument.as_ref().map(|arg| arg.without_parentheses())
    else {
        return None;
    };
    crate::collect::reference_symbol(id, scoping)
}

impl<'n, 'a: 'n> Emitter<'n, 'a> {
    fn new(scoping: &'n Scoping, sink: Sink) -> Self {
        Self {
            scoping,
            sink,
            count: 0,
            collect_units: false,
            units: Vec::new(),
            pending: None,
            deferred: Vec::new(),
        }
    }

    fn push(&mut self, token: u64) {
        self.count += 1;
        match &mut self.sink {
            Sink::Prefix(prefix) => prefix.push(token),
            Sink::Exact(sink) => {
                sink.hash.push(token);
                sink.loose.push(token);
            }
        }
    }

    fn push_specific(&mut self, token: u64) {
        if let Sink::Exact(sink) = &mut self.sink {
            sink.specific += 1;
        }
        self.push(token);
    }

    fn push_symbol(&mut self, symbol: SymbolId) {
        self.count += 1;
        match &mut self.sink {
            Sink::Prefix(prefix) => prefix.push(ID),
            Sink::Exact(sink) => {
                let placeholder = sink.placeholder(self.scoping, symbol);
                sink.hash.push(placeholder);
                sink.loose.push(ID);
            }
        }
    }

    fn open(&mut self, ty: AstType) {
        self.push(type_token(ty));
    }

    fn close(&mut self) {
        self.push(END);
    }

    fn visit_program_body(&mut self, program: &Program<'a>) {
        self.emit_list(&program.body);
    }

    fn emit_list(&mut self, statements: &[Statement<'a>]) {
        let mut index = 0;
        while index < statements.len() {
            let statement = &statements[index];
            if let Some(next) = statements.get(index + 1) {
                if self.emit_assign_return(statement, next) {
                    index += 2;
                    continue;
                }
            }
            self.emit_statement(statement);
            index += 1;
        }
    }

    fn emit_assign_return(&mut self, statement: &Statement<'a>, next: &Statement<'a>) -> bool {
        if let Some((symbol, init)) = single_declared_value(statement) {
            if returned_symbol(next, self.scoping) != Some(symbol) {
                return false;
            }
            self.open(AstType::ReturnStatement);
            self.emit_assignment(symbol, init);
            self.close();
            return true;
        }
        let Some(assigned) = assigned_symbol(statement, self.scoping) else {
            return false;
        };
        if returned_symbol(next, self.scoping) != Some(assigned) {
            return false;
        }
        let Statement::ExpressionStatement(expression) = statement else {
            return false;
        };
        self.open(AstType::ReturnStatement);
        self.visit_expression(&expression.expression);
        self.close();
        true
    }

    fn emit_expression_statement(&mut self, expr: &Expression<'a>) {
        if let Expression::SequenceExpression(sequence) = expr.without_parentheses() {
            for part in &sequence.expressions {
                self.emit_expression_statement(part);
            }
            return;
        }
        self.open(AstType::ExpressionStatement);
        self.visit_expression(expr);
        self.close();
    }

    fn emit_return(&mut self, ret: &ReturnStatement<'a>) {
        let Some(argument) = &ret.argument else {
            self.open(AstType::ReturnStatement);
            self.close();
            return;
        };
        if let Expression::SequenceExpression(sequence) = argument.without_parentheses() {
            let (last, rest) = sequence
                .expressions
                .split_last()
                .expect("a sequence has expressions");
            for part in rest {
                self.emit_expression_statement(part);
            }
            self.open(AstType::ReturnStatement);
            self.visit_expression(last);
            self.close();
            return;
        }
        self.open(AstType::ReturnStatement);
        self.visit_expression(argument);
        self.close();
    }

    fn emit_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let Some((symbol, init)) = declared_value(declarator) {
                self.emit_initialiser(symbol, init);
                continue;
            }
            if declarator.init.is_none() && declarator.id.is_binding_identifier() {
                continue;
            }
            self.open(AstType::VariableDeclaration);
            self.visit_variable_declarator(declarator);
            self.close();
        }
    }

    fn emit_assignment(&mut self, symbol: SymbolId, init: &Expression<'a>) {
        self.open(AstType::AssignmentExpression);
        self.push(tagged(TAG_OP, AssignmentOperator::Assign.as_str()));
        self.open(AstType::IdentifierReference);
        self.push_symbol(symbol);
        self.close();
        if let Some(span) = function_value_span(init) {
            self.pending = Some((span, symbol));
        }
        self.visit_expression(init);
        self.close();
    }

    fn emit_initialiser(&mut self, symbol: SymbolId, init: &Expression<'a>) {
        self.open(AstType::ExpressionStatement);
        self.emit_assignment(symbol, init);
        self.close();
    }

    fn emit_if(&mut self, branch: &IfStatement<'a>) {
        self.open(AstType::IfStatement);
        self.visit_expression(&branch.test);
        self.emit_statement(&branch.consequent);
        match &branch.alternate {
            Some(alternate) if exits(&branch.consequent) => {
                self.close();
                self.emit_statement(alternate);
            }
            Some(alternate) => {
                self.push(ELSE);
                self.emit_statement(alternate);
                self.close();
            }
            None => self.close(),
        }
    }

    fn emit_statement(&mut self, statement: &Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => self.emit_list(&block.body),
            Statement::EmptyStatement(_) => {}
            Statement::ExpressionStatement(expression) => {
                self.emit_expression_statement(&expression.expression);
            }
            Statement::ReturnStatement(ret) => self.emit_return(ret),
            Statement::VariableDeclaration(declaration) => self.emit_declaration(declaration),
            Statement::IfStatement(branch) => self.emit_if(branch),
            _ => walk_js::walk_statement(self, statement),
        }
    }

    fn record_unit(&mut self, unit: Unit<'n, 'a>) {
        if self.collect_units {
            self.units.push(unit);
        }
    }

    fn take_pending(&mut self, span: Span) -> Option<SymbolId> {
        match self.pending {
            Some((pending, symbol)) if pending == span => {
                self.pending = None;
                Some(symbol)
            }
            _ => None,
        }
    }

    fn function_unit(&mut self, function: &Function<'a>) {
        self.open(AstType::Function);
        if function.r#async {
            self.push(flag("async"));
        }
        if function.generator {
            self.push(flag("generator"));
        }
        self.visit_formal_parameters(&function.params);
        if let Some(body) = &function.body {
            self.emit_body(&body.statements);
        }
        self.close();
    }

    fn arrow_unit(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.open(AstType::Function);
        if arrow.r#async {
            self.push(flag("async"));
        }
        self.visit_formal_parameters(&arrow.params);
        match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => self.emit_body(&body.statements),
            body => {
                let expr = body.to_expression();
                self.open(AstType::ReturnStatement);
                self.visit_expression(expr);
                self.close();
            }
        }
        self.close();
    }

    fn identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        match crate::collect::reference_symbol(id, self.scoping) {
            Some(symbol) => self.push_symbol(symbol),
            None => self.push_specific(tagged(TAG_GLOBAL, id.name.as_str())),
        }
    }

    fn payload(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::IdentifierReference(id) => self.identifier_reference(id),
            AstKind::BindingIdentifier(id) => match id.symbol_id.get() {
                Some(symbol) => self.push_symbol(symbol),
                None => self.push_specific(tagged(TAG_GLOBAL, id.name.as_str())),
            },
            AstKind::IdentifierName(name) => {
                self.push_specific(tagged(TAG_STR, name.name.as_str()))
            }
            AstKind::PrivateIdentifier(name) => {
                self.push_specific(tagged(TAG_STR, &format!("#{}", name.name)))
            }
            AstKind::LabelIdentifier(_) => self.push(LABEL),
            AstKind::StringLiteral(literal) => {
                self.push_specific(tagged(TAG_STR, &js_text(literal.value)))
            }
            AstKind::TemplateElement(element) => {
                self.push_specific(tagged(TAG_STR, element.value.raw.as_str()))
            }
            AstKind::NumericLiteral(literal) => {
                self.push(mix(u64::from(TAG_NUMBER) << 56 ^ literal.value.to_bits()))
            }
            AstKind::BigIntLiteral(literal) => {
                self.push_specific(tagged(TAG_NUMBER, literal.value.as_str()))
            }
            AstKind::RegExpLiteral(literal) => {
                self.push_specific(tagged(TAG_STR, literal.regex.pattern.text.as_str()));
                self.push(tagged(TAG_FLAG, &literal.regex.flags.to_string()));
            }
            AstKind::BooleanLiteral(literal) => {
                self.push(flag(if literal.value { "true" } else { "false" }))
            }
            AstKind::LogicalExpression(logical) => {
                self.push(tagged(TAG_OP, logical.operator.as_str()))
            }
            AstKind::UnaryExpression(unary) => self.push(tagged(TAG_OP, unary.operator.as_str())),
            AstKind::UpdateExpression(update) => {
                self.push(tagged(TAG_OP, update.operator.as_str()));
                if update.prefix {
                    self.push(flag("prefix"));
                }
            }
            AstKind::AssignmentExpression(assignment) => {
                self.push(tagged(TAG_OP, assignment.operator.as_str()))
            }
            AstKind::ObjectProperty(property) => {
                self.push(flag(property_kind(property.kind)));
                if property.computed && !matches!(property.key, PropertyKey::StringLiteral(_)) {
                    self.push(COMPUTED);
                }
            }
            AstKind::MethodDefinition(method) => {
                self.push(flag(method_kind(method.kind)));
                if method.r#static {
                    self.push(flag("static"));
                }
                if method.computed {
                    self.push(COMPUTED);
                }
            }
            AstKind::PropertyDefinition(property) => {
                if property.r#static {
                    self.push(flag("static"));
                }
                if property.computed {
                    self.push(COMPUTED);
                }
            }
            AstKind::CallExpression(call) if call.optional => self.push(flag("optional")),
            AstKind::StaticMemberExpression(member) if member.optional => {
                self.push(flag("optional"))
            }
            AstKind::ComputedMemberExpression(member) if member.optional => {
                self.push(flag("optional"))
            }
            AstKind::ForOfStatement(statement) if statement.r#await => self.push(flag("await")),
            AstKind::YieldExpression(expr) if expr.delegate => self.push(flag("delegate")),
            _ => {}
        }
    }
}

fn skipped(ty: AstType) -> bool {
    matches!(
        ty,
        AstType::ParenthesizedExpression
            | AstType::BlockStatement
            | AstType::FunctionBody
            | AstType::Directive
            | AstType::Hashbang
            | AstType::EmptyStatement
    )
}

fn canonical_type(kind: &AstKind<'_>) -> AstType {
    match kind {
        AstKind::ComputedMemberExpression(member)
            if matches!(member.expression, Expression::StringLiteral(_)) =>
        {
            AstType::StaticMemberExpression
        }
        AstKind::ArrowFunctionExpression(_) => AstType::Function,
        AstKind::IdentifierName(_) => AstType::StringLiteral,
        other => other.ty(),
    }
}

impl<'n, 'a: 'n> VisitJs<'a> for Emitter<'n, 'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let ty = kind.ty();
        if skipped(ty) {
            return;
        }
        self.open(canonical_type(&kind));
        self.payload(kind);
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if skipped(kind.ty()) {
            return;
        }
        self.close();
    }

    fn visit_statements(&mut self, it: &oxc_allocator::Vec<'a, Statement<'a>>) {
        self.emit_list(it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        self.emit_statement(it);
    }

    fn visit_directives(&mut self, _it: &oxc_allocator::Vec<'a, Directive<'a>>) {}

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let (Some(symbol), Some(span)) = (id.symbol_id.get(), function_value_span(init)) {
                self.pending = Some((span, symbol));
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
            let symbol = crate::collect::reference_symbol(target, self.scoping);
            if let (Some(symbol), Some(span)) = (symbol, function_value_span(&it.right)) {
                self.pending = Some((span, symbol));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let declared = it.is_declaration().then_some(it.id.as_ref()).flatten();
        if let Some(id) = declared {
            self.push(DECL_ID);
            match id.symbol_id.get() {
                Some(symbol) => self.push_symbol(symbol),
                None => self.push_specific(tagged(TAG_GLOBAL, id.name.as_str())),
            }
        }
        let binding = declared
            .and_then(|id| id.symbol_id.get())
            .or_else(|| self.take_pending(it.span))
            .or_else(|| it.id.as_ref().and_then(|id| id.symbol_id.get()));
        let start = self.count;
        let node: &'n Function<'a> = self.alloc(it);
        self.function_unit(node);
        self.record_unit(Unit {
            node: UnitNode::Function(node),
            kind: UnitKind::Function,
            start,
            len: self.count - start,
            scope: it.scope_id.get(),
            binding,
            span: it.span,
        });
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let binding = self.take_pending(it.span);
        let start = self.count;
        let node: &'n ArrowFunctionExpression<'a> = self.alloc(it);
        self.arrow_unit(node);
        self.record_unit(Unit {
            node: UnitNode::Arrow(node),
            kind: UnitKind::Function,
            start,
            len: self.count - start,
            scope: it.scope_id.get(),
            binding,
            span: it.span,
        });
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let literal = match it.argument.without_parentheses() {
            Expression::NumericLiteral(number) => Some(number.value),
            _ => None,
        };
        match (it.operator, literal) {
            (UnaryOperator::Void, Some(_)) => {
                self.open(AstType::IdentifierReference);
                self.push_specific(tagged(TAG_GLOBAL, "undefined"));
                self.close();
            }
            (UnaryOperator::LogicalNot, Some(value)) if value == 0.0 || value == 1.0 => {
                self.open(AstType::BooleanLiteral);
                self.push(flag(if value == 0.0 { "true" } else { "false" }));
                self.close();
            }
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let mut operator = it.operator;
        let (mut left, mut right) = (&it.left, &it.right);
        if is_equality(operator) {
            if is_literal(left) && !is_literal(right) {
                std::mem::swap(&mut left, &mut right);
            }
            if is_typeof(left) || is_typeof(right) {
                operator = loose_operator(operator);
            }
        }
        self.open(AstType::BinaryExpression);
        self.push(tagged(TAG_OP, operator.as_str()));
        self.visit_expression(left);
        self.visit_expression(right);
        self.close();
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.emit_logical(it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.emit_call(it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_span(&mut self, _it: &Span) {}
}
