use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BlockStatement, CallExpression, Class, Expression,
    FormalParameters, Function, FunctionBody, LabeledStatement, Program, PropertyDefinition,
    Statement, StaticBlock, SwitchCase, WithStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;

use super::super::decode::{fold_text, FreeNames};
use super::super::model::{DecodeCx, Refusal};

const GLOBAL_OBJECTS: [&str; 3] = ["window", "globalThis", "self"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum List {
    FunctionBody,
    Program,
    Block,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Statement(Option<List>),
    Expression,
    Indirect,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Context {
    pub strict: bool,
    pub function: Option<Span>,
    pub simple_params: bool,
    pub async_or_generator: bool,
    pub class_member: bool,
    pub labelled: bool,
    pub in_with: bool,
}

pub(super) struct Site<'a> {
    pub span: Span,
    pub kind: Kind,
    pub context: Context,
    pub text: Result<String, Refusal>,
    pub names: Vec<&'a str>,
    pub over_cap: bool,
}

#[derive(Clone, Copy)]
struct Frame {
    span: Option<Span>,
    strict: bool,
    simple_params: bool,
    async_or_generator: bool,
    class_member: bool,
    labels: u32,
}

pub(super) struct Finder<'a> {
    frames: Vec<Frame>,
    with_depth: u32,
    next_list: Option<List>,
    listed: Option<List>,
    cx: DecodeCx,
    pub steps: u64,
    pub heap: u64,
    pub sites: Vec<Site<'a>>,
}

impl<'a> Finder<'a> {
    pub fn new(program: &Program<'a>, cx: DecodeCx) -> Self {
        let strict = program.source_type.is_module() || program.has_use_strict_directive();
        Self {
            frames: vec![Frame {
                span: None,
                strict,
                simple_params: true,
                async_or_generator: false,
                class_member: false,
                labels: 0,
            }],
            with_depth: 0,
            next_list: None,
            listed: None,
            cx,
            steps: 0,
            heap: 0,
            sites: Vec::new(),
        }
    }

    fn top(&self) -> Frame {
        self.frames.last().copied().unwrap_or(Frame {
            span: None,
            strict: true,
            simple_params: false,
            async_or_generator: true,
            class_member: true,
            labels: 1,
        })
    }

    fn context(&self) -> Context {
        let top = self.top();
        Context {
            strict: top.strict,
            function: top.span,
            simple_params: top.simple_params,
            async_or_generator: self.frames.iter().any(|frame| frame.async_or_generator),
            class_member: top.class_member,
            labelled: top.labels > 0,
            in_with: self.with_depth > 0,
        }
    }

    fn push_function(
        &mut self,
        span: Span,
        strict: bool,
        params: &FormalParameters<'_>,
        async_or_generator: bool,
    ) {
        let strict = strict || self.top().strict;
        self.frames.push(Frame {
            span: Some(span),
            strict,
            simple_params: simple(params),
            async_or_generator,
            class_member: false,
            labels: 0,
        });
    }

    fn push_class_member(&mut self) {
        self.frames.push(Frame {
            span: None,
            strict: true,
            simple_params: false,
            async_or_generator: false,
            class_member: true,
            labels: 0,
        });
    }

    fn visit_list(&mut self, statements: &ArenaVec<'a, Statement<'a>>, list: List) {
        self.next_list = Some(list);
        self.visit_statements(statements);
    }

    fn record(&mut self, span: Span, kind: Kind, call: &CallExpression<'a>) {
        let mut free = FreeNames::default();
        for argument in &call.arguments {
            if let Some(expression) = argument.as_expression() {
                free.visit_expression(expression);
            }
        }
        let text = match kind {
            Kind::Indirect => Err(Refusal::Shape),
            _ => self.text_of(call),
        };
        self.sites.push(Site {
            span,
            kind,
            context: self.context(),
            text,
            names: free.names,
            over_cap: free.over_cap,
        });
    }

    fn text_of(&mut self, call: &CallExpression<'a>) -> Result<String, Refusal> {
        let [argument] = call.arguments.as_slice() else {
            return Err(Refusal::Shape);
        };
        let Some(expression) = argument.as_expression() else {
            return Err(Refusal::Shape);
        };
        if let Expression::StringLiteral(literal) = expression {
            return literal
                .value
                .as_str()
                .map(str::to_string)
                .ok_or(Refusal::Bailed);
        }
        let left = DecodeCx {
            fuel: self.cx.fuel.saturating_sub(self.steps),
            heap: self.cx.heap.saturating_sub(self.heap),
            ..self.cx
        };
        let (text, steps, heap) = fold_text(expression, &left)?;
        self.steps = self.steps.saturating_add(steps);
        self.heap = self.heap.max(heap);
        Ok(text)
    }
}

fn simple(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params
            .items
            .iter()
            .all(|item| item.initializer.is_none() && item.pattern.is_binding_identifier())
}

pub(super) fn direct_eval<'p, 'a>(
    expression: &'p Expression<'a>,
) -> Option<&'p CallExpression<'a>> {
    match expression.without_parentheses() {
        Expression::CallExpression(call) if is_direct_eval(call) => Some(call),
        _ => None,
    }
}

fn is_direct_eval(call: &CallExpression<'_>) -> bool {
    let named = matches!(
        call.callee.without_parentheses(),
        Expression::Identifier(id) if id.name == "eval"
    );
    named && !call.optional && !call.arguments.is_empty()
}

fn indirect_eval(call: &CallExpression<'_>) -> bool {
    let literal = matches!(call.arguments.as_slice(), [Argument::StringLiteral(_)]);
    literal
        && match call.callee.without_parentheses() {
            Expression::SequenceExpression(seq) => matches!(
                seq.expressions.last(),
                Some(Expression::Identifier(id)) if id.name == "eval"
            ),
            Expression::StaticMemberExpression(member) => {
                member.property.name == "eval"
                    && match member.object.without_parentheses() {
                        Expression::Identifier(id) => GLOBAL_OBJECTS.contains(&id.name.as_str()),
                        Expression::ThisExpression(_) => true,
                        _ => false,
                    }
            }
            _ => false,
        }
}

impl<'a> Visit<'a> for Finder<'a> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.visit_list(&it.body, List::Program);
    }

    fn visit_statements(&mut self, it: &ArenaVec<'a, Statement<'a>>) {
        let list = self.next_list.take();
        for statement in it {
            self.listed = list;
            self.visit_statement(statement);
        }
        self.listed = None;
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let listed = self.listed.take();
        if let Statement::ExpressionStatement(statement) = it {
            if let Some(call) = direct_eval(&statement.expression) {
                self.record(statement.span, Kind::Statement(listed), call);
                return;
            }
        }
        walk::walk_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(it) {
            self.record(it.span, Kind::Expression, it);
            return;
        }
        if indirect_eval(it) {
            self.record(it.span, Kind::Indirect, it);
            return;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.visit_list(&it.statements, List::FunctionBody);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.visit_list(&it.body, List::Block);
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        self.visit_list(&it.consequent, List::Block);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let special = it.r#async || it.generator;
        self.push_function(it.span, it.has_use_strict_directive(), &it.params, special);
        walk::walk_function(self, it, flags);
        self.frames.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.push_function(
            it.span,
            it.has_use_strict_directive(),
            &it.params,
            it.r#async,
        );
        walk::walk_arrow_function_expression(self, it);
        self.frames.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let top = self.top();
        self.frames.push(Frame {
            strict: true,
            labels: 0,
            ..top
        });
        walk::walk_class(self, it);
        self.frames.pop();
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.push_class_member();
        walk::walk_property_definition(self, it);
        self.frames.pop();
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.push_class_member();
        walk::walk_static_block(self, it);
        self.frames.pop();
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.visit_expression(&it.object);
        self.with_depth += 1;
        self.visit_statement(&it.body);
        self.with_depth -= 1;
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        if let Some(top) = self.frames.last_mut() {
            top.labels += 1;
        }
        walk::walk_labeled_statement(self, it);
        if let Some(top) = self.frames.last_mut() {
            top.labels -= 1;
        }
    }
}
