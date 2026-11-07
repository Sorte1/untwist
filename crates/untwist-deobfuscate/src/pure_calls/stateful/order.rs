use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::fold::Const;

use super::super::eval::literal_argument;
use super::body::{Func, State};
use crate::bindings::resolved_symbol;
use crate::mutants::straight_line_guards_dropped;

const MAX_FOLDS: usize = 10_000;

pub(super) struct Fold {
    pub(super) span: Span,
    pub(super) value: Const,
    pub(super) statement: Option<Span>,
}

pub(super) struct Walk<'s, 'p> {
    scoping: &'s Scoping,
    funcs: &'s FxHashMap<SymbolId, &'s Func<'p>>,
    states: &'s FxHashMap<SymbolId, State>,
    env: FxHashMap<SymbolId, Const>,
    live: FxHashSet<SymbolId>,
    pub(super) folds: Vec<Fold>,
    pub(super) seen: FxHashMap<SymbolId, usize>,
}

struct Mentions<'s, 'k> {
    scoping: &'s Scoping,
    tracked: &'k dyn Fn(SymbolId) -> bool,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if resolved_symbol(it, self.scoping).is_some_and(|symbol| (self.tracked)(symbol)) {
            self.found = true;
        }
    }
}

impl<'s, 'p> Walk<'s, 'p> {
    pub(super) fn new(
        scoping: &'s Scoping,
        funcs: &'s FxHashMap<SymbolId, &'s Func<'p>>,
        states: &'s FxHashMap<SymbolId, State>,
    ) -> Self {
        let live = funcs
            .values()
            .filter(|func| func.hoisted)
            .map(|func| func.symbol)
            .collect();
        Self {
            scoping,
            funcs,
            states,
            env: FxHashMap::default(),
            live,
            folds: Vec::new(),
            seen: FxHashMap::default(),
        }
    }

    fn tracked(&self, symbol: SymbolId) -> bool {
        self.funcs.contains_key(&symbol) || self.states.contains_key(&symbol)
    }

    fn quiet(&self, visit: impl FnOnce(&mut Mentions<'_, '_>)) -> Option<()> {
        if straight_line_guards_dropped() {
            return Some(());
        }
        let tracked = |symbol: SymbolId| self.tracked(symbol);
        let mut mentions = Mentions {
            scoping: self.scoping,
            tracked: &tracked,
            found: false,
        };
        visit(&mut mentions);
        (!mentions.found).then_some(())
    }

    fn quiet_expr(&self, expr: &Expression<'_>) -> Option<()> {
        self.quiet(|mentions| mentions.visit_expression(expr))
    }

    fn quiet_statement(&self, statement: &Statement<'_>) -> Option<()> {
        self.quiet(|mentions| mentions.visit_statement(statement))
    }

    fn count(&mut self, symbol: SymbolId) {
        *self.seen.entry(symbol).or_default() += 1;
    }

    pub(super) fn program(&mut self, program: &Program<'_>) -> Option<()> {
        let declared: FxHashMap<Span, SymbolId> = self
            .funcs
            .values()
            .map(|func| (func.statement, func.symbol))
            .chain(
                self.states
                    .iter()
                    .map(|(symbol, state)| (state.statement, *symbol)),
            )
            .collect();
        for statement in &program.body {
            let Some(symbol) = declared.get(&statement.span()) else {
                self.statement(statement)?;
                continue;
            };
            if let Some(state) = self.states.get(symbol) {
                self.env.insert(*symbol, state.init.clone());
            } else {
                self.live.insert(*symbol);
            }
        }
        Some(())
    }

    fn statement(&mut self, statement: &Statement<'_>) -> Option<()> {
        match statement {
            Statement::ExpressionStatement(statement) => {
                let folds = self.folds.len();
                self.expr(&statement.expression)?;
                let whole = statement.expression.without_parentheses().span();
                let single = self.folds.len() == folds + 1;
                if let Some(last) = self
                    .folds
                    .last_mut()
                    .filter(|last| single && last.span == whole)
                {
                    last.statement = Some(statement.span);
                }
                Some(())
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
                        self.quiet(|mentions| mentions.visit_variable_declarator(declarator))?;
                        continue;
                    }
                    if let Some(init) = &declarator.init {
                        self.expr(init)?;
                    }
                }
                Some(())
            }
            Statement::IfStatement(statement) => {
                self.expr(&statement.test)?;
                self.quiet_statement(&statement.consequent)?;
                match &statement.alternate {
                    Some(alternate) => self.quiet_statement(alternate),
                    None => Some(()),
                }
            }
            Statement::BlockStatement(block) => {
                for inner in &block.body {
                    self.statement(inner)?;
                }
                Some(())
            }
            Statement::EmptyStatement(_) => Some(()),
            other => self.quiet_statement(other),
        }
    }

    fn read(&mut self, id: &IdentifierReference<'_>) -> Option<()> {
        let Some(symbol) = resolved_symbol(id, self.scoping) else {
            return Some(());
        };
        if self.funcs.contains_key(&symbol) {
            return straight_line_guards_dropped().then_some(());
        }
        if !self.states.contains_key(&symbol) {
            return Some(());
        }
        let value = self.env.get(&symbol)?.clone();
        self.count(symbol);
        self.push(id.span, value)
    }

    fn push(&mut self, span: Span, value: Const) -> Option<()> {
        if self.folds.len() >= MAX_FOLDS {
            return None;
        }
        self.folds.push(Fold {
            span,
            value,
            statement: None,
        });
        Some(())
    }

    fn site(&mut self, call: &CallExpression<'_>) -> Option<Option<()>> {
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        let symbol = resolved_symbol(callee, self.scoping)?;
        let func = *self.funcs.get(&symbol)?;
        if call.optional || !self.live.contains(&symbol) {
            return Some(None);
        }
        let args: Option<Vec<Const>> = call
            .arguments
            .iter()
            .map(|argument| literal_argument(argument.as_expression()?))
            .collect();
        let Some(args) = args else {
            return Some(None);
        };
        let Some(value) = func.run(self.scoping, &args, &mut self.env) else {
            return Some(None);
        };
        self.count(symbol);
        Some(self.push(call.span, value))
    }

    fn arguments(&mut self, arguments: &[Argument<'_>]) -> Option<()> {
        for argument in arguments {
            match argument {
                Argument::SpreadElement(spread) => self.expr(&spread.argument)?,
                other => self.expr(other.to_expression())?,
            }
        }
        Some(())
    }

    fn member(&mut self, member: &MemberExpression<'_>) -> Option<()> {
        self.expr(member.object())?;
        if let MemberExpression::ComputedMemberExpression(computed) = member {
            self.expr(&computed.expression)?;
        }
        Some(())
    }

    fn target(&mut self, target: &AssignmentTarget<'_>) -> Option<()> {
        if let Some(member) = target.as_member_expression() {
            return self.member(member);
        }
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let tracked = resolved_symbol(id, self.scoping).is_some_and(|s| self.tracked(s));
                (!tracked || straight_line_guards_dropped()).then_some(())
            }
            _ => self.quiet(|mentions| mentions.visit_assignment_target(target)),
        }
    }

    fn object(&mut self, object: &ObjectExpression<'_>) -> Option<()> {
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(entry) = property else {
                let ObjectPropertyKind::SpreadProperty(spread) = property else {
                    continue;
                };
                self.expr(&spread.argument)?;
                continue;
            };
            if entry.shorthand || entry.method || entry.kind != PropertyKind::Init {
                self.quiet(|mentions| mentions.visit_object_property(entry))?;
                continue;
            }
            if let Some(key) = entry.key.as_expression().filter(|_| entry.computed) {
                self.expr(key)?;
            }
            self.expr(&entry.value)?;
        }
        Some(())
    }

    fn expr(&mut self, expr: &Expression<'_>) -> Option<()> {
        match expr {
            Expression::Identifier(id) => self.read(id),
            Expression::ParenthesizedExpression(inner) => self.expr(&inner.expression),
            Expression::CallExpression(call) => {
                if let Some(folded) = self.site(call) {
                    return folded;
                }
                if call.optional {
                    return self.quiet_expr(expr);
                }
                self.expr(&call.callee)?;
                self.arguments(&call.arguments)
            }
            Expression::NewExpression(new) => {
                self.expr(&new.callee)?;
                self.arguments(&new.arguments)
            }
            Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_) => {
                let member = expr.as_member_expression()?;
                if member.optional() {
                    return self.quiet_expr(expr);
                }
                self.member(member)
            }
            Expression::BinaryExpression(binary) => {
                self.expr(&binary.left)?;
                self.expr(&binary.right)
            }
            Expression::LogicalExpression(logical) => {
                self.expr(&logical.left)?;
                self.quiet_expr(&logical.right)
            }
            Expression::ConditionalExpression(conditional) => {
                self.expr(&conditional.test)?;
                self.quiet_expr(&conditional.consequent)?;
                self.quiet_expr(&conditional.alternate)
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                self.quiet_expr(expr)
            }
            Expression::UnaryExpression(unary) => self.expr(&unary.argument),
            Expression::AwaitExpression(inner) => self.expr(&inner.argument),
            Expression::SequenceExpression(sequence) => {
                for part in &sequence.expressions {
                    self.expr(part)?;
                }
                Some(())
            }
            Expression::TemplateLiteral(template) => {
                for part in &template.expressions {
                    self.expr(part)?;
                }
                Some(())
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.expr(&tagged.tag)?;
                for part in &tagged.quasi.expressions {
                    self.expr(part)?;
                }
                Some(())
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.expr(&spread.argument)?
                        }
                        ArrayExpressionElement::Elision(_) => {}
                        other => self.expr(other.to_expression())?,
                    }
                }
                Some(())
            }
            Expression::ObjectExpression(object) => self.object(object),
            Expression::AssignmentExpression(assign) if !assign.operator.is_logical() => {
                self.target(&assign.left)?;
                self.expr(&assign.right)
            }
            Expression::UpdateExpression(update) => match update.argument.as_member_expression() {
                Some(member) => self.member(member),
                None => self.quiet_expr(expr),
            },
            other if literal_argument(other).is_some() => Some(()),
            other => self.quiet_expr(other),
        }
    }
}
