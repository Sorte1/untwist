use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast::AstType;
use oxc_ast_visit::{walk, walk_js, Visit, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;

use super::hash::{fnv, mix, Exact, PolyHash};

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
const ABSENT: u64 = 0x5bd1_e995_0000_0009;
const FINALLY: u64 = 0x5bd1_e995_0000_000a;
const LEXICAL_THIS: u64 = 0x5bd1_e995_0000_000b;

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
    let mut scan = DefScan::new(scoping);
    scan.visit_program(program);
    let mut emitter = Emitter::new(scoping, Sink::Prefix(PolyHash::new()), scan.single_definitions());
    emitter.collect_units = true;
    emitter.visit_program_body(program);
    let len = emitter.count;
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
}

pub fn exact_shape(unit: &Unit<'_, '_>, scoping: &Scoping) -> ExactShape {
    let sink = Sink::Exact(ExactSink {
        hash: Exact::new(),
        loose: PolyHash::new(),
        scope: unit.scope,
        inner: Vec::new(),
        outer: Vec::new(),
        placeholders: FxHashMap::default(),
        labels: FxHashMap::default(),
        specific: 0,
    });
    let mut scan = DefScan::new(scoping);
    match unit.node {
        UnitNode::Function(function) => scan.visit_function(function, ScopeFlags::Function),
        UnitNode::Arrow(arrow) => scan.visit_arrow_function_expression(arrow),
        UnitNode::Program(program) => scan.visit_program(program),
    }
    let mut emitter = Emitter::new(scoping, sink, scan.single_definitions());
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
    }
}

struct ExactSink {
    hash: Exact,
    loose: PolyHash,
    scope: Option<ScopeId>,
    inner: Vec<SymbolId>,
    outer: Vec<SymbolId>,
    placeholders: FxHashMap<SymbolId, u64>,
    labels: FxHashMap<String, u64>,
    specific: u32,
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
        if let Some(&known) = self.placeholders.get(&symbol) {
            return known;
        }
        let token = if is_within(scoping, symbol, self.scope) {
            self.inner.push(symbol);
            mix(INNER ^ (self.inner.len() - 1) as u64)
        } else {
            self.outer.push(symbol);
            mix(OUTER ^ (self.outer.len() - 1) as u64)
        };
        self.placeholders.insert(symbol, token);
        token
    }

    fn label(&mut self, name: &str) -> u64 {
        let next = self.labels.len() as u64;
        *self
            .labels
            .entry(name.to_string())
            .or_insert_with(|| mix(LABEL ^ next.rotate_left(32)))
    }
}

struct Emitter<'n, 'a> {
    scoping: &'n Scoping,
    sink: Sink,
    count: u32,
    collect_units: bool,
    units: Vec<Unit<'n, 'a>>,
    pending: Option<(Span, SymbolId)>,
    single: FxHashSet<SymbolId>,
    frames: Vec<Frame>,
}

#[derive(Clone, Copy)]
struct Frame {
    arrow: bool,
    lexical_this: bool,
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

fn is_string(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::StringLiteral(_))
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
) -> Option<(VariableDeclarationKind, SymbolId, &'d Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    match declaration.declarations.as_slice() {
        [declarator] => {
            let (symbol, init) = declared_value(declarator)?;
            Some((declaration.kind, symbol, init))
        }
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
    fn new(scoping: &'n Scoping, sink: Sink, single: FxHashSet<SymbolId>) -> Self {
        Self {
            scoping,
            sink,
            count: 0,
            collect_units: false,
            units: Vec::new(),
            pending: None,
            single,
            frames: Vec::new(),
        }
    }

    fn push_label(&mut self, name: &str) {
        self.count += 1;
        match &mut self.sink {
            Sink::Prefix(prefix) => prefix.push(LABEL),
            Sink::Exact(sink) => {
                let token = sink.label(name);
                sink.hash.push(token);
                sink.loose.push(LABEL);
            }
        }
    }

    fn uses_lexical_this(&mut self) {
        for frame in self.frames.iter_mut().rev() {
            if !frame.arrow || frame.lexical_this {
                break;
            }
            frame.lexical_this = true;
        }
    }

    fn is_single_definition(&self, kind: VariableDeclarationKind, symbol: SymbolId) -> bool {
        kind == VariableDeclarationKind::Var || self.single.contains(&symbol)
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
        if let Some((kind, symbol, init)) = single_declared_value(statement) {
            if returned_symbol(next, self.scoping) != Some(symbol)
                || !self.is_single_definition(kind, symbol)
            {
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
        let var = declaration.kind == VariableDeclarationKind::Var;
        for declarator in &declaration.declarations {
            if let Some((symbol, init)) = declared_value(declarator) {
                if self.is_single_definition(declaration.kind, symbol) {
                    self.emit_initialiser(symbol, init);
                    continue;
                }
            }
            let bare = declarator.init.is_none() && declarator.id.is_binding_identifier();
            let single = declared_symbol(declarator)
                .is_some_and(|symbol| self.is_single_definition(declaration.kind, symbol));
            if bare && (var || single) {
                continue;
            }
            self.open(AstType::VariableDeclaration);
            self.push(flag(declaration.kind.as_str()));
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
        self.frames.push(Frame {
            arrow: false,
            lexical_this: false,
        });
        self.open(AstType::Function);
        if function.r#async {
            self.push(flag("async"));
        }
        if function.generator {
            self.push(flag("generator"));
        }
        self.visit_formal_parameters(&function.params);
        if let Some(body) = &function.body {
            self.emit_list(&body.statements);
        }
        self.close();
        self.frames.pop();
    }

    fn arrow_unit(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.frames.push(Frame {
            arrow: true,
            lexical_this: false,
        });
        self.open(AstType::Function);
        if arrow.r#async {
            self.push(flag("async"));
        }
        self.visit_formal_parameters(&arrow.params);
        match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => self.emit_list(&body.statements),
            body => {
                let expr = body.to_expression();
                self.open(AstType::ReturnStatement);
                self.visit_expression(expr);
                self.close();
            }
        }
        if self.frames.pop().is_some_and(|frame| frame.lexical_this) {
            self.push(LEXICAL_THIS);
        }
        self.close();
    }

    fn identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        match crate::collect::reference_symbol(id, self.scoping) {
            Some(symbol) => self.push_symbol(symbol),
            None => {
                if matches!(id.name.as_str(), "arguments" | "eval") {
                    self.uses_lexical_this();
                }
                self.push_specific(tagged(TAG_GLOBAL, id.name.as_str()))
            }
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
            AstKind::LabelIdentifier(label) => self.push_label(label.name.as_str()),
            AstKind::ThisExpression(_) | AstKind::Super(_) | AstKind::NewTarget(_) => {
                self.uses_lexical_this()
            }
            AstKind::VariableDeclaration(declaration) => {
                self.push(flag(declaration.kind.as_str()))
            }
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
                let proto = property
                    .key
                    .static_name()
                    .is_some_and(|name| name.as_str() == Some("__proto__"));
                let string_key = matches!(property.key, PropertyKey::StringLiteral(_));
                if property.computed && (!string_key || proto) {
                    self.push(COMPUTED);
                }
                if property.method {
                    self.push(flag("method"));
                }
                if property.shorthand && proto {
                    self.push(flag("shorthand"));
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
            if (is_typeof(left) && is_string(right)) || (is_string(left) && is_typeof(right)) {
                operator = loose_operator(operator);
            }
        }
        self.open(AstType::BinaryExpression);
        self.push(tagged(TAG_OP, operator.as_str()));
        self.visit_expression(left);
        self.visit_expression(right);
        self.close();
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let kind = AstKind::ForStatement(self.alloc(it));
        self.enter_node(kind);
        match &it.init {
            Some(init) => self.visit_for_statement_init(init),
            None => self.push(ABSENT),
        }
        match &it.test {
            Some(test) => self.visit_expression(test),
            None => self.push(ABSENT),
        }
        match &it.update {
            Some(update) => self.visit_expression(update),
            None => self.push(ABSENT),
        }
        self.visit_statement(&it.body);
        self.leave_node(kind);
    }

    fn visit_try_statement(&mut self, it: &TryStatement<'a>) {
        let kind = AstKind::TryStatement(self.alloc(it));
        self.enter_node(kind);
        self.visit_block_statement(&it.block);
        if let Some(handler) = &it.handler {
            self.visit_catch_clause(handler);
        }
        if let Some(finalizer) = &it.finalizer {
            self.push(FINALLY);
            self.visit_block_statement(finalizer);
        }
        self.leave_node(kind);
    }

    fn visit_span(&mut self, _it: &Span) {}
}

#[derive(Default)]
struct DefState {
    defined_in: Option<u32>,
    defined_hoisted: u32,
    defined_in_loop: bool,
    early: bool,
    captured: bool,
    written: bool,
    constant: bool,
    definitions: u32,
}

impl DefState {
    fn single(&self) -> bool {
        self.defined_in.is_some()
            && self.definitions == 1
            && !self.early
            && !self.captured
            && !(self.constant && self.written)
    }
}

#[derive(Clone, Copy)]
struct ScanFrame {
    id: u32,
    loops: u32,
    hoisted: u32,
}

struct DefScan<'s> {
    scoping: &'s Scoping,
    frames: Vec<ScanFrame>,
    next_frame: u32,
    states: FxHashMap<SymbolId, DefState>,
}

fn declared_symbol(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

impl<'s> DefScan<'s> {
    fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            frames: vec![ScanFrame {
                id: 0,
                loops: 0,
                hoisted: 0,
            }],
            next_frame: 1,
            states: FxHashMap::default(),
        }
    }

    fn frame(&self) -> ScanFrame {
        self.frames.last().copied().unwrap_or(ScanFrame {
            id: 0,
            loops: 0,
            hoisted: 0,
        })
    }

    fn in_frame(&mut self, hoisted: bool, walk: impl FnOnce(&mut Self)) {
        let outer = self.frame().hoisted;
        self.frames.push(ScanFrame {
            id: self.next_frame,
            loops: 0,
            hoisted: outer + u32::from(hoisted),
        });
        self.next_frame += 1;
        walk(self);
        self.frames.pop();
    }

    fn in_loop(&mut self, walk: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.frames.last_mut() {
            frame.loops += 1;
        }
        walk(self);
        if let Some(frame) = self.frames.last_mut() {
            frame.loops -= 1;
        }
    }

    fn lexical_declaration(&mut self, declaration: &VariableDeclaration<'_>, definite: bool) {
        let constant = declaration.kind != VariableDeclarationKind::Let;
        for declarator in &declaration.declarations {
            self.visit_variable_declarator(declarator);
            let Some(symbol) = declared_symbol(declarator) else {
                continue;
            };
            let frame = self.frame();
            let resets = declarator.init.is_none() && frame.loops > 0;
            let state = self.states.entry(symbol).or_default();
            state.definitions += 1;
            state.constant |= constant;
            if definite && !resets {
                state.defined_in = Some(frame.id);
                state.defined_hoisted = frame.hoisted;
                state.defined_in_loop = frame.loops > 0;
            }
        }
    }

    fn single_definitions(self) -> FxHashSet<SymbolId> {
        self.states
            .into_iter()
            .filter(|(_, state)| state.single())
            .map(|(symbol, _)| symbol)
            .collect()
    }
}

fn lexical<'s, 'a>(statement: &'s Statement<'a>) -> Option<&'s VariableDeclaration<'a>> {
    match statement {
        Statement::VariableDeclaration(declaration)
            if declaration.kind != VariableDeclarationKind::Var =>
        {
            Some(declaration)
        }
        _ => None,
    }
}

impl<'a> Visit<'a> for DefScan<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        match lexical(it) {
            Some(declaration) => self.lexical_declaration(declaration, true),
            None => walk::walk_statement(self, it),
        }
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        for statement in &it.consequent {
            match lexical(statement) {
                Some(declaration) => self.lexical_declaration(declaration, false),
                None => self.visit_statement(statement),
            }
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.in_frame(it.is_declaration(), |scan| walk::walk_function(scan, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.in_frame(false, |scan| walk::walk_arrow_function_expression(scan, it));
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.in_frame(false, |scan| walk::walk_class(scan, it));
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.in_loop(|scan| walk::walk_for_statement(scan, it));
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.in_loop(|scan| walk::walk_for_in_statement(scan, it));
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.in_loop(|scan| walk::walk_for_of_statement(scan, it));
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.in_loop(|scan| walk::walk_while_statement(scan, it));
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.in_loop(|scan| walk::walk_do_while_statement(scan, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let Some(symbol) = reference.symbol_id() else {
            return;
        };
        let written = reference.is_write();
        let frame = self.frame();
        let state = self.states.entry(symbol).or_default();
        state.written |= written;
        match state.defined_in {
            None => state.early = true,
            Some(defined) if defined != frame.id => {
                state.captured |= state.defined_in_loop || frame.hoisted > state.defined_hoisted;
            }
            Some(_) => {}
        }
    }
}
