use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentOperator, AssignmentTarget, BindingIdentifier,
    BindingPattern, CatchClause, Class, Expression, Function, FunctionType, IfStatement,
    MemberExpression, ReturnStatement, Statement, VariableDeclaration, VariableDeclarationKind,
    VariableDeclarator,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use super::prim::fold_constant;
use super::{ContainerRef, GateRef, GateTest, KeyRecipe, KeyTerm};
use crate::names::Name;
use crate::ops::string_key;
use crate::stack;
use crate::value::{JsString, PropKey};

const UNDEFINED: &str = "undefined";
const ARGUMENTS: &str = "arguments";

#[derive(Default)]
struct PatternNames<'a> {
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for PatternNames<'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.push(it.name.as_str());
    }
}

fn pattern_names<'a>(pattern: &BindingPattern<'a>) -> Vec<&'a str> {
    let mut names = PatternNames::default();
    names.visit_binding_pattern(pattern);
    names.names
}

#[derive(Default)]
struct OwnNames<'a> {
    names: FxHashSet<&'a str>,
    vars: FxHashSet<&'a str>,
    lexical: FxHashSet<&'a str>,
    declares_functions: bool,
}

impl<'a> VisitJs<'a> for OwnNames<'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let var = it.kind == VariableDeclarationKind::Var;
        for declarator in &it.declarations {
            let names = pattern_names(&declarator.id);
            let target = if var {
                &mut self.vars
            } else {
                &mut self.lexical
            };
            target.extend(names);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        if let Some(param) = &it.param {
            self.lexical.extend(pattern_names(&param.pattern));
        }
        walk_js::walk_catch_clause(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.lexical.insert(id.name.as_str());
        }
        walk_js::walk_class(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.declares_functions |= it.r#type == FunctionType::FunctionDeclaration;
        if let Some(id) = &it.id {
            self.names.insert(id.name.as_str());
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}

struct Locals<'a> {
    own: FxHashSet<&'a str>,
    writable: FxHashSet<&'a str>,
}

fn own_names<'a>(function: &Function<'a>, params: &[&'a str]) -> Option<Locals<'a>> {
    let mut own = OwnNames::default();
    own.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        own.visit_function_body(body);
    }
    if own.declares_functions {
        return None;
    }
    if let (Some(id), FunctionType::FunctionExpression) = (&function.id, function.r#type) {
        own.names.insert(id.name.as_str());
        own.lexical.insert(id.name.as_str());
    }
    own.names.insert(ARGUMENTS);
    let writable = params
        .iter()
        .copied()
        .chain(own.vars.iter().copied())
        .filter(|name| !own.lexical.contains(name) && *name != ARGUMENTS)
        .collect();
    Some(Locals {
        own: own.names,
        writable,
    })
}

fn simple_params<'a>(function: &Function<'a>) -> Option<Vec<&'a str>> {
    if function.params.rest.is_some() || function.params.items.len() > usize::from(u8::MAX) {
        return None;
    }
    let mut names: Vec<&'a str> = Vec::with_capacity(function.params.items.len());
    for param in &function.params.items {
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            return None;
        };
        if param.initializer.is_some() || names.contains(&id.name.as_str()) {
            return None;
        }
        names.push(id.name.as_str());
    }
    Some(names)
}

fn identifier<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

struct Tail<'n, 'a> {
    prologue: &'n [Statement<'a>],
    leading: &'n [VariableDeclarator<'a>],
    memo: &'n Expression<'a>,
}

fn memo_tail<'n, 'a>(
    statements: &'n [Statement<'a>],
    own: &FxHashSet<&'a str>,
) -> Option<Tail<'n, 'a>> {
    let (rest, hit) = match statements {
        [rest @ .., Statement::IfStatement(branch), Statement::ReturnStatement(ret)] => {
            (rest, Hit::Branch(branch, returned(ret)?))
        }
        [rest @ .., Statement::ExpressionStatement(node), Statement::ReturnStatement(ret)] => {
            (rest, Hit::Choice(&node.expression, returned(ret)?))
        }
        [rest @ .., Statement::ReturnStatement(ret)] => {
            (rest, Hit::Ternary(ret.argument.as_ref()?))
        }
        _ => return None,
    };
    let declared = rest
        .iter()
        .rev()
        .take_while(|statement| uninitialised(statement))
        .count();
    let [prologue @ .., Statement::VariableDeclaration(declaration)] =
        rest.get(..rest.len() - declared)?
    else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var {
        return None;
    }
    let (memo_declarator, leading) = declaration.declarations.split_last()?;
    let BindingPattern::BindingIdentifier(read) = &memo_declarator.id else {
        return None;
    };
    hit.copies(read.name.as_str(), own).then_some(Tail {
        prologue,
        leading,
        memo: memo_declarator.init.as_ref()?,
    })
}

enum Hit<'n, 'a> {
    Branch(&'n IfStatement<'a>, &'a str),
    Ternary(&'n Expression<'a>),
    Choice(&'n Expression<'a>, &'a str),
}

impl Hit<'_, '_> {
    fn copies(&self, read: &str, own: &FxHashSet<&str>) -> bool {
        match self {
            Hit::Branch(branch, result) => own.contains(result) && hit_branch(branch, read, result),
            Hit::Ternary(returned) => hit_ternary(returned, read, own),
            Hit::Choice(choice, result) => own.contains(result) && hit_choice(choice, read, result),
        }
    }
}

fn hit_ternary(returned: &Expression<'_>, read: &str, own: &FxHashSet<&str>) -> bool {
    let Expression::SequenceExpression(sequence) = returned.without_parentheses() else {
        return false;
    };
    let [Expression::ConditionalExpression(choice), last] = sequence.expressions.as_slice() else {
        return false;
    };
    let Some(result) = identifier(last).filter(|result| own.contains(result)) else {
        return false;
    };
    misses_on(&choice.test, read) && assigns(&choice.alternate, read, result)
}

fn hit_choice(choice: &Expression<'_>, read: &str, result: &str) -> bool {
    let Expression::ConditionalExpression(choice) = choice.without_parentheses() else {
        return false;
    };
    misses_on(&choice.test, read) && assigns(&choice.alternate, read, result)
}

fn misses_on(test: &Expression<'_>, read: &str) -> bool {
    let Expression::UnaryExpression(test) = test.without_parentheses() else {
        return false;
    };
    test.operator == UnaryOperator::LogicalNot && identifier(&test.argument) == Some(read)
}

fn uninitialised(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::VariableDeclaration(declaration)
    if declaration.kind == VariableDeclarationKind::Var
        && declaration.declarations.iter().all(|declarator| {
            declarator.init.is_none()
                && matches!(declarator.id, BindingPattern::BindingIdentifier(_))
        }))
}

fn returned<'a>(ret: &ReturnStatement<'a>) -> Option<&'a str> {
    identifier(ret.argument.as_ref()?)
}

fn hit_branch(branch: &IfStatement<'_>, read: &str, result: &str) -> bool {
    misses_on(&branch.test, read)
        && branch
            .alternate
            .as_ref()
            .is_some_and(|alternate| copies(alternate, read, result))
}

fn copies(statement: &Statement<'_>, read: &str, result: &str) -> bool {
    let statement = match statement {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only,
            _ => return false,
        },
        other => other,
    };
    let Statement::ExpressionStatement(node) = statement else {
        return false;
    };
    assigns(&node.expression, read, result)
}

fn assigns(expr: &Expression<'_>, read: &str, result: &str) -> bool {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return false;
    };
    assign.operator == AssignmentOperator::Assign
        && target.name.as_str() == result
        && identifier(&assign.right) == Some(read)
}

#[derive(Clone)]
struct Def {
    term: KeyTerm,
    deps: Vec<Span>,
}

struct Compiler<'a> {
    params: Vec<&'a str>,
    own: FxHashSet<&'a str>,
    writable: FxHashSet<&'a str>,
    self_name: &'a str,
    defs: FxHashMap<&'a str, Def>,
    written: FxHashSet<&'a str>,
    history: Vec<(Span, KeyTerm)>,
    array: Option<&'a str>,
    gate: Option<GateRef>,
}

pub(crate) fn compile_recipe<'a>(
    function: &'a Function<'a>,
    self_name: &'a str,
    intern: &mut dyn FnMut(&'a str) -> Name,
) -> Option<KeyRecipe> {
    if function.generator || function.r#async {
        return None;
    }
    let params = simple_params(function)?;
    let locals = own_names(function, &params)?;
    let body = function.body.as_ref()?;
    let tail = memo_tail(&body.statements, &locals.writable)?;
    let mut compiler = Compiler::new(params, locals, self_name);
    for statement in tail.prologue {
        compiler.statement(statement)?;
    }
    for declarator in tail.leading {
        compiler.declarator(declarator)?;
    }
    compiler.finish(tail.memo, intern)
}

impl<'a> Compiler<'a> {
    fn new(params: Vec<&'a str>, locals: Locals<'a>, self_name: &'a str) -> Self {
        let defs = params
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let index = u8::try_from(index).ok()?;
                let term = KeyTerm::Argument(index);
                Some((
                    *name,
                    Def {
                        term,
                        deps: Vec::new(),
                    },
                ))
            })
            .collect();
        Self {
            params,
            own: locals.own,
            writable: locals.writable,
            self_name,
            defs,
            written: FxHashSet::default(),
            history: Vec::new(),
            array: None,
            gate: None,
        }
    }

    fn statement(&mut self, statement: &Statement<'a>) -> Option<()> {
        match statement {
            Statement::ExpressionStatement(node) => self.assignment(&node.expression, node.span),
            Statement::VariableDeclaration(declaration) => self.declaration(declaration),
            Statement::IfStatement(branch) => self.gate(branch),
            Statement::EmptyStatement(_) => Some(()),
            _ => None,
        }
    }

    fn declaration(&mut self, declaration: &VariableDeclaration<'a>) -> Option<()> {
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        declaration
            .declarations
            .iter()
            .try_for_each(|declarator| self.declarator(declarator))
    }

    fn declarator(&mut self, declarator: &VariableDeclarator<'a>) -> Option<()> {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        match &declarator.init {
            Some(init) => self.define(id.name.as_str(), init, declarator.span),
            None => Some(()),
        }
    }

    fn assignment(&mut self, expr: &Expression<'a>, span: Span) -> Option<()> {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return None;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
            return None;
        };
        if assign.operator != AssignmentOperator::Assign {
            return None;
        }
        let name = target.name.as_str();
        if self.params.contains(&name) && !self_update(name, &assign.right) {
            return None;
        }
        self.define(name, &assign.right, span)
    }

    fn define(&mut self, name: &'a str, value: &Expression<'a>, span: Span) -> Option<()> {
        if !self.writable.contains(name) || !self.written.insert(name) {
            return None;
        }
        let mut def = self.term(value)?;
        def.deps.push(span);
        self.history.push((span, def.term.clone()));
        self.defs.insert(name, def);
        Some(())
    }

    fn term(&mut self, expr: &Expression<'a>) -> Option<Def> {
        if let Some(value) = fold_constant(expr, &[]) {
            let term = KeyTerm::Literal(value.value());
            return Some(Def {
                term,
                deps: Vec::new(),
            });
        }
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.defs.get(id.name.as_str()).cloned(),
            Expression::BinaryExpression(node) => {
                let left = self.term(&node.left)?;
                let right = self.term(&node.right)?;
                let (l, r) = (Box::new(left.term), Box::new(right.term));
                let term = match node.operator {
                    BinaryOperator::Addition => KeyTerm::Add(l, r),
                    BinaryOperator::Subtraction => KeyTerm::Sub(l, r),
                    _ => return None,
                };
                Some(Def {
                    term,
                    deps: merged(left.deps, right.deps),
                })
            }
            Expression::ComputedMemberExpression(node) if !node.optional => {
                let array = identifier(&node.object)?;
                self.claim_array(array)?;
                let index = self.term(&node.expression)?;
                let term = KeyTerm::ArrayAt(Box::new(index.term));
                Some(Def {
                    term,
                    deps: index.deps,
                })
            }
            _ => None,
        }
    }

    fn claim_array(&mut self, name: &'a str) -> Option<()> {
        if self.own.contains(name) || name == self.self_name || name == UNDEFINED {
            return None;
        }
        match self.array {
            Some(array) if array != name => None,
            _ => {
                self.array = Some(name);
                Some(())
            }
        }
    }
}

fn self_update(name: &str, value: &Expression<'_>) -> bool {
    let Expression::BinaryExpression(node) = value.without_parentheses() else {
        return false;
    };
    let shifted = matches!(
        node.operator,
        BinaryOperator::Addition | BinaryOperator::Subtraction
    );
    shifted && identifier(&node.left) == Some(name) && fold_constant(&node.right, &[]).is_some()
}

fn merged(mut left: Vec<Span>, right: Vec<Span>) -> Vec<Span> {
    left.extend(right);
    left
}

impl<'a> Compiler<'a> {
    fn gate(&mut self, branch: &IfStatement<'a>) -> Option<()> {
        if self.gate.is_some() || branch.alternate.is_some() || !self.self_is_free() {
            return None;
        }
        let (flag, test) = self.gate_test(&branch.test)?;
        self.gate = Some(GateRef {
            span: branch.span,
            flag,
            test,
        });
        Some(())
    }

    fn self_is_free(&self) -> bool {
        !self.own.contains(self.self_name)
    }

    fn gate_test(&self, test: &Expression<'a>) -> Option<(PropKey, GateTest)> {
        match test.without_parentheses() {
            Expression::BinaryExpression(node) => {
                let loose = match node.operator {
                    BinaryOperator::StrictEquality => false,
                    BinaryOperator::Equality => true,
                    _ => return None,
                };
                let member = match (identifier(&node.left), identifier(&node.right)) {
                    (Some(UNDEFINED), _) => &node.right,
                    (_, Some(UNDEFINED)) => &node.left,
                    _ => return None,
                };
                if self.own.contains(UNDEFINED) {
                    return None;
                }
                Some((self.self_member(member)?, GateTest::Undefined { loose }))
            }
            Expression::UnaryExpression(node) if node.operator == UnaryOperator::LogicalNot => {
                Some((self.self_member(&node.argument)?, GateTest::Falsy))
            }
            _ => None,
        }
    }

    fn self_member(&self, expr: &Expression<'a>) -> Option<PropKey> {
        let member = expr.without_parentheses().as_member_expression()?;
        if member.optional() || identifier(member.object()) != Some(self.self_name) {
            return None;
        }
        let name = static_name(member)?;
        (name != "prototype").then(|| string_key(JsString::from_str(name)))
    }

    fn container(
        &self,
        object: &Expression<'a>,
        intern: &mut dyn FnMut(&'a str) -> Name,
    ) -> Option<ContainerRef> {
        if let Some(name) = identifier(object) {
            let free = !self.own.contains(name) && Some(name) != self.array;
            return free.then(|| ContainerRef::Binding(intern(name)));
        }
        if !self.self_is_free() {
            return None;
        }
        self.self_member(object).map(ContainerRef::SelfMember)
    }

    fn finish(
        mut self,
        memo: &Expression<'a>,
        intern: &mut dyn FnMut(&'a str) -> Name,
    ) -> Option<KeyRecipe> {
        let Expression::ComputedMemberExpression(read) = memo.without_parentheses() else {
            return None;
        };
        if read.optional {
            return None;
        }
        let key = self.term(&read.expression)?;
        let container = self.container(&read.object, intern)?;
        let mut slice = key.deps.clone();
        slice.extend(self.gate.as_ref().map(|gate| gate.span));
        slice.sort_unstable_by_key(|span| (span.start, span.end));
        slice.dedup();
        let reads = self.outside_reads(&slice);
        Some(KeyRecipe {
            container,
            key: key.term,
            gate: self.gate,
            array: self.array.map(&mut *intern),
            reads,
            #[cfg(test)]
            slice,
            self_name: intern(self.self_name),
            undefined: intern(UNDEFINED),
        })
    }

    fn outside_reads(&self, slice: &[Span]) -> Vec<KeyTerm> {
        self.history
            .iter()
            .filter(|(span, term)| !slice.contains(span) && reads_array(term))
            .map(|(_, term)| term.clone())
            .collect()
    }
}

fn static_name<'a>(member: &MemberExpression<'a>) -> Option<&'a str> {
    match member {
        MemberExpression::StaticMemberExpression(node) => Some(node.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(node) => match &node.expression {
            Expression::StringLiteral(literal) if !literal.lone_surrogates => {
                Some(literal.value.as_str())
            }
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn reads_array(term: &KeyTerm) -> bool {
    match term {
        KeyTerm::ArrayAt(_) => true,
        KeyTerm::Add(left, right) | KeyTerm::Sub(left, right) => {
            reads_array(left) || reads_array(right)
        }
        KeyTerm::Argument(_) | KeyTerm::Literal(_) => false,
    }
}
