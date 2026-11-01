use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtins {
    Unknown,
    Intact,
    Aliased,
    Written,
}

impl Builtins {
    pub fn allows_safe(self, unsafe_transforms: bool) -> bool {
        self == Builtins::Intact || (unsafe_transforms && self == Builtins::Aliased)
    }

    pub fn allows_unsafe(self) -> bool {
        matches!(self, Builtins::Intact | Builtins::Aliased)
    }
}

const ROOTS: [&str; 5] = ["Object", "Array", "Function", "Symbol", "String"];

const GLOBAL_OBJECTS: [&str; 4] = ["globalThis", "window", "self", "global"];

const DEFINERS: [(&str, &str); 8] = [
    ("Object", "defineProperty"),
    ("Object", "defineProperties"),
    ("Object", "setPrototypeOf"),
    ("Object", "assign"),
    ("Reflect", "defineProperty"),
    ("Reflect", "set"),
    ("Reflect", "setPrototypeOf"),
    ("Reflect", "deleteProperty"),
];

const RELIED_ON: [(&str, bool, &[&str]); 7] = [
    (
        "Object",
        false,
        &[
            "assign",
            "keys",
            "create",
            "defineProperty",
            "defineProperties",
            "getOwnPropertyDescriptor",
            "getOwnPropertyDescriptors",
            "getOwnPropertySymbols",
            "getPrototypeOf",
            "setPrototypeOf",
            "prototype",
        ],
    ),
    ("Array", false, &["isArray", "from", "of", "prototype"]),
    (
        "Array",
        true,
        &[
            "concat",
            "slice",
            "push",
            "filter",
            "forEach",
            "indexOf",
            "join",
            "values",
            "keys",
            "entries",
            "constructor",
        ],
    ),
    ("String", false, &["prototype"]),
    ("String", true, &["slice", "constructor"]),
    ("Function", false, &["prototype"]),
    ("Function", true, &["apply", "call", "bind", "constructor"]),
];

fn relied_on(root: &str, on_prototype: bool, key: Option<&str>) -> bool {
    let Some(key) = key else {
        return true;
    };
    match RELIED_ON
        .iter()
        .find(|(owner, prototype, _)| *owner == root && *prototype == on_prototype)
    {
        Some((_, _, keys)) => keys.contains(&key),
        None => true,
    }
}

const LEGACY_DEFINERS: [&str; 2] = ["__defineGetter__", "__defineSetter__"];

const PROMISE: &str = "Promise";

fn literal_key<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

fn member_path<'k>(expression: &'k Expression<'_>, path: &mut Vec<&'k str>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => {
            path.push(id.name.as_str());
            true
        }
        Expression::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        Expression::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn target_path<'k>(target: &'k SimpleAssignmentTarget<'_>, path: &mut Vec<&'k str>) -> bool {
    match target {
        SimpleAssignmentTarget::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        SimpleAssignmentTarget::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn installed_value<'k, 'a>(
    read: &Expression<'_>,
    write: &'k Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assignment) = write.without_parentheses() else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let target = assignment.left.as_simple_assignment_target()?;
    let (mut read_path, mut written) = (Vec::new(), Vec::new());
    let same = member_path(read, &mut read_path)
        && target_path(target, &mut written)
        && read_path.len() > 1
        && read_path == written;
    same.then_some(&assignment.right)
}

fn installs_if_missing<'k, 'a>(logical: &'k LogicalExpression<'a>) -> Option<&'k Expression<'a>> {
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    installed_value(&logical.left, &logical.right)
}

fn only_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(expression) => Some(&expression.expression),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only_expression(only),
            _ => None,
        },
        _ => None,
    }
}

fn guarded_install<'k, 'a>(statement: &'k IfStatement<'a>) -> Option<&'k Expression<'a>> {
    if statement.alternate.is_some() {
        return None;
    }
    let Expression::UnaryExpression(test) = statement.test.without_parentheses() else {
        return None;
    };
    if test.operator != UnaryOperator::LogicalNot {
        return None;
    }
    installed_value(&test.argument, only_expression(&statement.consequent)?)
}

struct Scan<'s> {
    scoping: &'s Scoping,
    module: bool,
    writes_only: bool,
    first_only: bool,
    functions: u32,
    verdict: Builtins,
    promise_written: bool,
}

pub struct Scanned {
    pub verdict: Builtins,
    pub promise_written: bool,
}

pub fn scan(program: &Program<'_>, scoping: &Scoping, module: bool, writes_only: bool) -> Scanned {
    let mut scan = Scan {
        scoping,
        module,
        writes_only,
        first_only: false,
        functions: 0,
        verdict: Builtins::Intact,
        promise_written: false,
    };
    scan.visit_program(program);
    Scanned {
        verdict: scan.verdict,
        promise_written: scan.promise_written,
    }
}

pub fn intact(program: &Program<'_>, scoping: &Scoping, module: bool) -> bool {
    let mut scan = Scan {
        scoping,
        module,
        writes_only: false,
        first_only: true,
        functions: 0,
        verdict: Builtins::Intact,
        promise_written: false,
    };
    scan.visit_program(program);
    scan.verdict == Builtins::Intact
}

impl Scan<'_> {
    fn settled(&self) -> bool {
        self.verdict == Builtins::Written || (self.first_only && self.verdict != Builtins::Intact)
    }

    fn global_named(&self, id: &IdentifierReference<'_>, names: &[&str]) -> bool {
        names.contains(&id.name.as_str())
            && id
                .reference_id
                .get()
                .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }

    fn is_global_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, &GLOBAL_OBJECTS),
            Expression::ThisExpression(_) => self.functions == 0 && !self.module,
            _ => false,
        }
    }

    fn is_builtin(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, &ROOTS),
            Expression::StaticMemberExpression(member) => {
                let name = member.property.name.as_str();
                (name == "prototype" && self.is_builtin(&member.object))
                    || (ROOTS.contains(&name) && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn is_root(&self, expression: &Expression<'_>) -> bool {
        if self.is_builtin(expression) || self.is_global_object(expression) {
            return true;
        }
        matches!(
            expression.without_parentheses(),
            Expression::StaticMemberExpression(member)
                if GLOBAL_OBJECTS.contains(&member.property.name.as_str())
                    && self.is_global_object(&member.object)
        )
    }

    fn reaches_builtin(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            if self.is_builtin(current) {
                return true;
            }
            current = match current {
                Expression::StaticMemberExpression(member) => member.object.without_parentheses(),
                Expression::ComputedMemberExpression(member) => member.object.without_parentheses(),
                _ => return false,
            };
        }
    }

    fn global_key(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        let named = |name: &str| ROOTS.contains(&name) || GLOBAL_OBJECTS.contains(&name);
        (self.is_global_object(object) || self.is_root(object) && !self.is_builtin(object))
            && key.is_none_or(named)
    }

    fn builtin_slot<'e>(&self, object: &'e Expression<'_>) -> Option<(&'e str, bool)> {
        match object.without_parentheses() {
            Expression::Identifier(id) if self.global_named(id, &ROOTS) => {
                Some((id.name.as_str(), false))
            }
            Expression::StaticMemberExpression(member)
                if member.property.name.as_str() == "prototype" =>
            {
                match member.object.without_parentheses() {
                    Expression::Identifier(id) if self.global_named(id, &ROOTS) => {
                        Some((id.name.as_str(), true))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn writes_member(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if let Some((root, on_prototype)) = self.builtin_slot(object) {
            return relied_on(root, on_prototype, key);
        }
        self.reaches_builtin(object) || self.global_key(object, key)
    }

    fn member_write(&self, target: &Expression<'_>) -> bool {
        match target.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                self.writes_member(&member.object, Some(member.property.name.as_str()))
            }
            Expression::ComputedMemberExpression(member) => {
                self.writes_member(&member.object, literal_key(&member.expression))
            }
            _ => false,
        }
    }

    fn written(&mut self) {
        self.verdict = Builtins::Written;
    }

    fn reaches_promise(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            current = match current {
                Expression::Identifier(id) => return self.global_named(id, &[PROMISE]),
                Expression::StaticMemberExpression(member) => {
                    if member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                Expression::ComputedMemberExpression(member) => {
                    if literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                _ => return false,
            };
        }
    }

    fn promise_target(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, &[PROMISE])
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn defines_on_promise(&self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return false;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.reaches_promise(target) || (key == Some(PROMISE) && self.is_global_object(target))
    }

    fn aliased(&mut self) {
        if self.verdict == Builtins::Intact {
            self.verdict = Builtins::Aliased;
        }
    }

    fn is_definer(&self, callee: &Expression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        let name = member.property.name.as_str();
        let Expression::Identifier(root) = member.object.without_parentheses() else {
            return false;
        };
        DEFINERS
            .iter()
            .any(|(owner, method)| *method == name && self.global_named(root, &[owner]))
    }

    fn defines_on_builtin(&self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return true;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.writes_member(target, key)
    }

    fn legacy_definition(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        if !LEGACY_DEFINERS.contains(&member.property.name.as_str()) {
            return false;
        }
        let key = arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(literal_key);
        self.writes_member(&member.object, key)
    }

    fn visit_unless_root(&mut self, expression: &Expression<'_>) {
        if self.writes_only || !self.is_root(expression) {
            self.visit_expression(expression);
        }
    }

    fn visit_callee(&mut self, callee: &Expression<'_>) {
        match callee.without_parentheses() {
            Expression::ConditionalExpression(conditional) => {
                self.visit_expression(&conditional.test);
                self.visit_unless_root(&conditional.consequent);
                self.visit_unless_root(&conditional.alternate);
            }
            _ => self.visit_unless_root(callee),
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.settled() {
            return;
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.settled() {
            return;
        }
        if !self.writes_only && self.is_root(it) {
            self.aliased();
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.visit_unless_root(&it.object);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.visit_unless_root(&it.object);
        self.visit_expression(&it.expression);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.is_definer(&it.callee) && self.defines_on_promise(&it.arguments) {
            self.promise_written = true;
        }
        let defines = self.is_definer(&it.callee) && self.defines_on_builtin(&it.arguments);
        if defines || self.legacy_definition(&it.callee, &it.arguments) {
            self.written();
            return;
        }
        self.visit_callee(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.visit_callee(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Typeof if self.is_root(&it.argument) => {}
            UnaryOperator::Delete if self.member_write(&it.argument) => self.written(),
            UnaryOperator::Delete if self.reaches_promise(&it.argument) => {
                self.promise_written = true;
            }
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_equality() {
            self.visit_unless_root(&it.left);
            self.visit_unless_root(&it.right);
            return;
        }
        self.visit_expression(&it.left);
        if matches!(it.operator, BinaryOperator::Instanceof | BinaryOperator::In) {
            self.visit_unless_root(&it.right);
        } else {
            self.visit_expression(&it.right);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let installs = matches!(
            it.operator,
            AssignmentOperator::LogicalOr | AssignmentOperator::LogicalNullish
        ) && it
            .left
            .as_simple_assignment_target()
            .is_some_and(|target| target_path(target, &mut Vec::new()));
        if installs {
            self.visit_expression(&it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match guarded_install(it) {
            Some(value) => self.visit_expression(value),
            None => walk_js::walk_if_statement(self, it),
        }
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if let Some(value) = installs_if_missing(it) {
            self.visit_expression(value);
            return;
        }
        if it.operator == LogicalOperator::And {
            self.visit_unless_root(&it.left);
        } else {
            self.visit_expression(&it.left);
        }
        self.visit_expression(&it.right);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if self.promise_target(it) {
            self.promise_written = true;
        }
        let written = match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, &ROOTS) || self.global_named(id, &GLOBAL_OBJECTS)
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.writes_member(&member.object, Some(member.property.name.as_str()))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.writes_member(&member.object, literal_key(&member.expression))
            }
            _ => false,
        };
        if written {
            self.written();
            return;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.functions -= 1;
    }
}
