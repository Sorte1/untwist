use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, SourceType, Span, SPAN};
use oxc_str::{Ident, Str};
use oxc_syntax::symbol::SymbolId;
use untwist_core::semantic::build_scoping;

use rustc_hash::{FxHashMap, FxHashSet};
use sha2::{Digest, Sha256};
use untwist_core::ast_util::prop::number_key;

use crate::model::{body_key, Bodies, BodyKey, Runtime};
use crate::runtime::synthetic_statements;
use crate::webpack::canon::canonical_statement;
pub use crate::webpack::machinery_forms::{ENSURE_FORMS, HELPER_FORMS, LOADER_FORMS};

const MARK: char = '\0';
const INSTALLED: &str = "\0installed";
const GLOBAL: &str = "\0global";
const CHUNK: &str = "\0chunk";
const IMPORTS: &str = "\0imports";
const GLOBAL_PREFIXES: [&str; 2] = ["webpackChunk", "webpackJsonp"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Helper,
    Loader,
    Ensure,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Known {
    pub role: Role,
    pub installed: Vec<String>,
}

#[derive(Default)]
struct Marked {
    found: bool,
}

impl<'a> VisitJs<'a> for Marked {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found |= it.value.as_str().is_none_or(|text| text.starts_with(MARK));
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.found |= it.value.raw.contains(MARK);
    }
}

struct Abstract<'a> {
    builder: AstBuilder<'a>,
    installed: Vec<String>,
    global: Option<String>,
    mixed: bool,
}

fn installed_table(object: &ObjectExpression<'_>) -> Option<Vec<String>> {
    if object.properties.is_empty() {
        return None;
    }
    object
        .properties
        .iter()
        .map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return None;
            };
            if property.computed || property.kind != PropertyKind::Init || property.method {
                return None;
            }
            let Expression::NumericLiteral(value) = &property.value else {
                return None;
            };
            if value.value != 0.0 && value.value != 1.0 {
                return None;
            }
            match &property.key {
                PropertyKey::NumericLiteral(key) => Some(number_key(key.value)),
                PropertyKey::StringLiteral(key) => key.value.as_str().map(str::to_string),
                _ => None,
            }
        })
        .collect()
}

fn import_table(object: &ObjectExpression<'_>) -> bool {
    !object.properties.is_empty()
        && object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            let key = matches!(property.key, PropertyKey::NumericLiteral(_) | PropertyKey::StringLiteral(_));
            let Expression::ArrowFunctionExpression(arrow) = property.value.without_parentheses() else {
                return false;
            };
            let imports = arrow.params.items.is_empty()
                && matches!(arrow.body.as_expression().map(Expression::without_parentheses),
                    Some(Expression::ImportExpression(import))
                        if matches!(import.source, Expression::StringLiteral(_)) && import.options.is_none());
            key && !property.computed && property.kind == PropertyKind::Init && imports
        })
}

fn is_chunk_test(regex: &RegExpLiteral<'_>) -> bool {
    let pattern = regex.regex.pattern.text.as_str();
    let Some(inner) = pattern
        .strip_prefix("^(")
        .and_then(|rest| rest.strip_suffix(")$"))
    else {
        return false;
    };
    !inner.is_empty()
        && inner
            .split('|')
            .all(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
}

fn loose_id_compare(compare: &BinaryExpression<'_>) -> bool {
    matches!(
        compare.operator,
        BinaryOperator::Equality | BinaryOperator::Inequality
    ) && matches!(
        compare.left,
        Expression::NumericLiteral(_) | Expression::StringLiteral(_)
    ) && matches!(compare.right, Expression::Identifier(_))
}

impl<'a> Abstract<'a> {
    fn global_name(&mut self, text: &str) -> bool {
        if !GLOBAL_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
        {
            return false;
        }
        match &self.global {
            Some(seen) if seen != text => self.mixed = true,
            _ => self.global = Some(text.to_string()),
        }
        true
    }

    fn blocked(&self, body: &mut Statement<'a>) {
        if matches!(body, Statement::BlockStatement(_)) {
            return;
        }
        let inner = std::mem::replace(body, Statement::new_empty_statement(SPAN, &self.builder));
        let mut list = oxc_allocator::Vec::with_capacity_in(1, &self.builder);
        list.push(inner);
        *body = Statement::new_block_statement(SPAN, list, &self.builder);
    }

    fn placeholder(&self, text: &str) -> Expression<'a> {
        Expression::new_string_literal(
            SPAN,
            Str::from_str_in(text, &self.builder),
            None,
            &self.builder,
        )
    }
}

impl<'a> VisitMut<'a> for Abstract<'a> {
    fn visit_statements(&mut self, it: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        let old = std::mem::replace(it, oxc_allocator::Vec::new_in(&self.builder));
        for statement in old {
            match statement {
                Statement::VariableDeclaration(declaration)
                    if declaration.declarations.len() > 1 =>
                {
                    let declaration = declaration.unbox();
                    for declarator in declaration.declarations {
                        let mut one = oxc_allocator::Vec::with_capacity_in(1, &self.builder);
                        one.push(declarator);
                        it.push(Statement::new_variable_declaration(
                            declaration.span,
                            declaration.kind,
                            one,
                            declaration.declare,
                            &self.builder,
                        ));
                    }
                }
                other => it.push(other),
            }
        }
        walk_mut::walk_statements(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        match it {
            Expression::ObjectExpression(object) if import_table(object) => {
                *it = self.placeholder(IMPORTS);
                return;
            }
            Expression::ObjectExpression(object) => {
                if let Some(ids) = installed_table(object) {
                    if !self.installed.is_empty() {
                        self.mixed = true;
                    }
                    self.installed = ids;
                    *it = self.placeholder(INSTALLED);
                    return;
                }
            }
            Expression::RegExpLiteral(regex) if is_chunk_test(regex) => {
                *it = self.placeholder(CHUNK);
                return;
            }
            Expression::BinaryExpression(compare) if loose_id_compare(compare) => {
                compare.left = self.placeholder(CHUNK);
            }
            _ => {}
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_for_statement(&mut self, it: &mut ForStatement<'a>) {
        self.blocked(&mut it.body);
        walk_mut::walk_for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &mut ForInStatement<'a>) {
        self.blocked(&mut it.body);
        walk_mut::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &mut ForOfStatement<'a>) {
        self.blocked(&mut it.body);
        walk_mut::walk_for_of_statement(self, it);
    }

    fn visit_while_statement(&mut self, it: &mut WhileStatement<'a>) {
        self.blocked(&mut it.body);
        walk_mut::walk_while_statement(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &mut ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(name) = &it.expression {
            let text = name.value.as_str().unwrap_or_default().to_string();
            if self.global_name(&text) {
                it.expression = self.placeholder(GLOBAL);
            }
        }
        walk_mut::walk_computed_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &mut StaticMemberExpression<'a>) {
        let text = it.property.name.to_string();
        if self.global_name(&text) {
            it.property.name = Ident::from_str_in(GLOBAL, &self.builder);
        }
        walk_mut::walk_static_member_expression(self, it);
    }
}

fn abstracted(expr: &Expression<'_>, require: &str) -> Option<(String, Vec<String>)> {
    let mut marked = Marked::default();
    marked.visit_expression(expr);
    if marked.found {
        return None;
    }
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut copy = Statement::new_expression_statement(SPAN, expr.clone_in(&alloc), &builder);
    let mut abstraction = Abstract {
        builder: AstBuilder::new(&alloc),
        installed: Vec::new(),
        global: None,
        mixed: false,
    };
    abstraction.visit_statement(&mut copy);
    if abstraction.mixed {
        return None;
    }
    let copy = symbols_renamed(&alloc, copy, require);
    let text = canonical_statement(&copy, REQUIRE_MARK)?;
    Some((text, abstraction.installed))
}

const REQUIRE_MARK: &str = "__untwist_machinery_require__";

struct Symbols<'s, 'a> {
    builder: AstBuilder<'a>,
    scoping: &'s Scoping,
    require: &'s str,
    names: FxHashMap<SymbolId, Ident<'a>>,
    early: FxHashSet<SymbolId>,
    loops: u32,
}

struct Order<'s> {
    scoping: &'s Scoping,
    seen: FxHashSet<SymbolId>,
    early: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Order<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.seen.extend(it.symbol_id.get());
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol.filter(|symbol| !self.seen.contains(symbol)) {
            self.early.insert(symbol);
        }
    }
}

impl Symbols<'_, '_> {
    fn settled(&self, declarator: &VariableDeclarator<'_>) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        !self.early.contains(&symbol)
            && !self
                .scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
    }
}

impl<'a> Symbols<'_, 'a> {
    fn name(&mut self, symbol: SymbolId) -> Ident<'a> {
        let next = format!("$s{}", self.names.len());
        *self
            .names
            .entry(symbol)
            .or_insert_with(|| Ident::from_str_in(&next, &self.builder))
    }
}

impl<'a> VisitMut<'a> for Symbols<'_, 'a> {
    fn visit_variable_declaration(&mut self, it: &mut VariableDeclaration<'a>) {
        let lexical = matches!(
            it.kind,
            VariableDeclarationKind::Let | VariableDeclarationKind::Const
        );
        if lexical && self.loops == 0 && it.declarations.iter().all(|d| self.settled(d)) {
            it.kind = VariableDeclarationKind::Var;
        }
        walk_mut::walk_variable_declaration(self, it);
    }

    fn visit_for_statement(&mut self, it: &mut ForStatement<'a>) {
        self.loops += 1;
        walk_mut::walk_for_statement(self, it);
        self.loops -= 1;
    }

    fn visit_for_in_statement(&mut self, it: &mut ForInStatement<'a>) {
        self.loops += 1;
        walk_mut::walk_for_in_statement(self, it);
        self.loops -= 1;
    }

    fn visit_for_of_statement(&mut self, it: &mut ForOfStatement<'a>) {
        self.loops += 1;
        walk_mut::walk_for_of_statement(self, it);
        self.loops -= 1;
    }

    fn visit_while_statement(&mut self, it: &mut WhileStatement<'a>) {
        self.loops += 1;
        walk_mut::walk_while_statement(self, it);
        self.loops -= 1;
    }

    fn visit_do_while_statement(&mut self, it: &mut DoWhileStatement<'a>) {
        self.loops += 1;
        walk_mut::walk_do_while_statement(self, it);
        self.loops -= 1;
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            it.name = self.name(symbol);
        }
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        match symbol {
            Some(symbol) => it.name = self.name(symbol),
            None if it.name == self.require => {
                it.name = Ident::from_str_in(REQUIRE_MARK, &self.builder);
            }
            None => {}
        }
    }
}

fn symbols_renamed<'a>(
    alloc: &'a Allocator,
    statement: Statement<'a>,
    require: &str,
) -> Statement<'a> {
    let builder = AstBuilder::new(alloc);
    let mut body = oxc_allocator::Vec::with_capacity_in(1, &builder);
    body.push(statement);
    let mut program = Program::new(
        SPAN,
        SourceType::cjs(),
        "",
        oxc_allocator::Vec::new_in(&builder),
        None,
        oxc_allocator::Vec::new_in(&builder),
        body,
        &builder,
    );
    let scoping = build_scoping(&program).scoping;
    let mut order = Order {
        scoping: &scoping,
        seen: FxHashSet::default(),
        early: FxHashSet::default(),
    };
    order.visit_program(&program);
    let mut symbols = Symbols {
        builder: AstBuilder::new(alloc),
        scoping: &scoping,
        require,
        names: FxHashMap::default(),
        early: order.early,
        loops: 0,
    };
    symbols.visit_program(&mut program);
    program
        .body
        .pop()
        .unwrap_or_else(|| Statement::new_empty_statement(SPAN, &builder))
}

pub fn form_digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn recorded(table: &[&str], text: &str) -> bool {
    let digest = form_digest(text);
    table.contains(&digest.as_str())
}

pub fn form_text(expr: &Expression<'_>, require: &str) -> Option<String> {
    abstracted(expr, require).map(|(text, _)| text)
}

pub fn known(expr: &Expression<'_>, require: &str) -> Option<Known> {
    let (text, installed) = abstracted(expr, require)?;
    let text = text.as_str();
    let role = if recorded(LOADER_FORMS, text) && loads_javascript_only(text) {
        Role::Loader
    } else if recorded(ENSURE_FORMS, text) {
        Role::Ensure
    } else if recorded(HELPER_FORMS, text) {
        Role::Helper
    } else {
        return None;
    };
    if role != Role::Loader && !installed.is_empty() {
        return None;
    }
    Some(Known { role, installed })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Loading {
    pub installed: Vec<String>,
    handlers: Vec<String>,
    called: Option<Vec<String>>,
}

impl Loading {
    pub fn settles_ensure(&self) -> bool {
        !self.handlers.is_empty()
            && self
                .called
                .as_ref()
                .is_some_and(|called| called.iter().all(|name| self.handlers.contains(name)))
    }
}

fn named_after<'t>(text: &'t str, mark: &str, end: char) -> Vec<&'t str> {
    text.match_indices(mark)
        .filter_map(|(at, _)| {
            let rest = &text[at + mark.len()..];
            let name_end =
                rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))?;
            (rest[name_end..].starts_with(end) && name_end > 0).then(|| &rest[..name_end])
        })
        .collect()
}

const HANDLER: &str = "$R.f.";
const JAVASCRIPT_HANDLERS: [&str; 4] = ["j", "require", "readFileVm", "import"];
const SHARING: [&str; 3] = ["$R.S", "$R.I", "$R.R"];

fn loads_javascript_only(text: &str) -> bool {
    let handlers = named_after(text, HANDLER, '=');
    !handlers.is_empty()
        && handlers
            .iter()
            .all(|name| JAVASCRIPT_HANDLERS.contains(name))
        && !SHARING.iter().any(|mark| text.contains(mark))
}
const SPREAD: &str = "Object.keys($R.f)";
const SELF_LOADING: &str = "\0self";

fn statement_elements<'k, 'a>(statement: &'k Statement<'a>) -> Vec<&'k Expression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return Vec::new();
    };
    match statement.expression.without_parentheses() {
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
        other => vec![other],
    }
}

struct Refill<'p, 'a, 'b> {
    alloc: &'b Allocator,
    pool: &'p FxHashMap<BodyKey, oxc_allocator::Vec<'a, Statement<'a>>>,
}

impl<'b> VisitMut<'b> for Refill<'_, '_, 'b> {
    fn visit_function_body(&mut self, it: &mut FunctionBody<'b>) {
        if it.statements.is_empty() {
            if let Some(taken) = self.pool.get(&body_key(it)) {
                it.statements = oxc_allocator::Vec::from_iter_in(
                    taken.iter().map(|statement| statement.clone_in(self.alloc)),
                    &self.alloc,
                );
            }
        }
        walk_mut::walk_function_body(self, it);
    }
}

pub fn refilled<'b>(
    alloc: &'b Allocator,
    statements: &[Statement<'_>],
    skip: usize,
    bodies: &Bodies<'_>,
) -> Option<oxc_allocator::Vec<'b, Statement<'b>>> {
    let Bodies::Taken(pool) = bodies else {
        return None;
    };
    let mut copies = oxc_allocator::Vec::new_in(&alloc);
    for (index, statement) in statements.iter().enumerate() {
        if index == skip {
            copies.push(Statement::new_empty_statement(
                SPAN,
                &AstBuilder::new(alloc),
            ));
            continue;
        }
        let mut copy = statement.clone_in(alloc);
        Refill { alloc, pool }.visit_statement(&mut copy);
        copies.push(copy);
    }
    Some(copies)
}

fn whole(
    expr: &Expression<'_>,
    require: &str,
    bodies: &Bodies<'_>,
) -> Option<(String, Vec<String>)> {
    let Bodies::Taken(pool) = bodies else {
        return abstracted(expr, require);
    };
    let alloc = Allocator::default();
    let mut copy = expr.clone_in(&alloc);
    Refill {
        alloc: &alloc,
        pool,
    }
    .visit_expression(&mut copy);
    abstracted(&copy, require)
}

pub fn installed_declared(statements: &[Statement<'_>]) -> Vec<String> {
    statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) => Some(declaration),
            _ => None,
        })
        .flat_map(|declaration| &declaration.declarations)
        .filter_map(
            |declarator| match declarator.init.as_ref()?.without_parentheses() {
                Expression::ObjectExpression(object) => installed_table(object),
                _ => None,
            },
        )
        .flatten()
        .collect()
}

pub fn loading_of(statements: &[Statement<'_>], require: &str, bodies: &Bodies<'_>) -> Loading {
    let mut loading = Loading::default();
    for expr in statements.iter().flat_map(statement_elements) {
        let Some((text, installed)) = whole(expr, require, bodies) else {
            continue;
        };
        let text = text.as_str();
        if recorded(LOADER_FORMS, text) && loads_javascript_only(text) {
            loading.installed.extend(installed);
            loading.handlers.extend(
                named_after(text, HANDLER, '=')
                    .into_iter()
                    .map(str::to_string),
            );
        } else if recorded(ENSURE_FORMS, text) && installed.is_empty() {
            let called = match (text.contains(SPREAD), text.contains(HANDLER)) {
                (true, _) => Vec::new(),
                (false, true) => named_after(text, HANDLER, '(')
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                (false, false) => {
                    loading.handlers.push(SELF_LOADING.to_string());
                    vec![SELF_LOADING.to_string()]
                }
            };
            loading.called = Some(called);
        }
    }
    loading
}

const JSONP_BLOCK: usize = 6;

fn jsonp_array(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    let [declarator] = &declaration.declarations[..] else {
        return false;
    };
    matches!(declarator.init.as_ref().map(Expression::without_parentheses),
        Some(Expression::AssignmentExpression(assignment))
            if matches!(assignment.right.without_parentheses(), Expression::LogicalExpression(_)))
}

fn jsonp_form(
    source: &str,
    statements: &[Statement<'_>],
    require: &str,
) -> Option<(String, Vec<String>, Vec<Span>)> {
    let at = statements.iter().position(jsonp_array)?;
    let block = statements.get(at..at + JSONP_BLOCK)?;
    let text_of = |statement: &Statement<'_>| statement.span().source_text(source).to_string();
    let functions: Vec<&Statement<'_>> = statements
        .iter()
        .filter(|statement| {
            matches!(statement, Statement::FunctionDeclaration(function)
                if function.id.as_ref().is_some_and(|id| id.name != require))
        })
        .collect();
    let body: Vec<String> = functions
        .iter()
        .copied()
        .chain(block)
        .map(text_of)
        .collect();
    let text = format!("(() => {{\n{}\n}})();", body.join("\n"));
    let alloc = Allocator::default();
    let parsed = synthetic_statements(&alloc, text)?;
    let [Statement::ExpressionStatement(wrapper)] = &parsed[..] else {
        return None;
    };
    let (form, installed) = abstracted(&wrapper.expression, require)?;
    let spans = functions
        .into_iter()
        .chain(block)
        .map(GetSpan::span)
        .collect();
    Some((form, installed, spans))
}

pub fn jsonp_block(source: &str, statements: &[Statement<'_>], require: &str) -> Option<Vec<Span>> {
    let (form, installed, spans) = jsonp_form(source, statements, require)?;
    (recorded(LOADER_FORMS, &form) && installed.is_empty()).then_some(spans)
}

pub fn jsonp_block_text(
    source: &str,
    statements: &[Statement<'_>],
    require: &str,
) -> Option<String> {
    jsonp_form(source, statements, require).map(|(form, _, _)| form)
}

const ENSURE: &str = "e";
const LOADER_STATE: [&str; 2] = ["f", "O"];

pub fn settle<'a>(alloc: &'a Allocator, runtime: &mut Runtime<'a>, loading: &Loading) {
    if loading.handlers.is_empty() {
        return;
    }
    let settled = loading
        .settles_ensure()
        .then(|| {
            let text = format!(
                "{}.e = function () {{ return Promise.all([]); }};",
                runtime.require_name
            );
            synthetic_statements(alloc, text)?.pop()
        })
        .flatten();
    for helper in &mut runtime.helpers {
        if LOADER_STATE.contains(&helper.name.as_str()) {
            helper.movable = false;
        }
        if helper.name == ENSURE {
            match &settled {
                Some(statement) => {
                    helper.statement = statement.clone_in(alloc);
                    helper.movable = true;
                }
                None => helper.movable = false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use untwist_core::parse::parse_program;

    fn first<'k, 'a>(program: &'k Program<'a>) -> &'k Expression<'a> {
        match &program.body[0] {
            Statement::ExpressionStatement(statement) => &statement.expression,
            _ => panic!("not an expression"),
        }
    }

    #[test]
    fn chunk_tables_and_globals_are_abstracted() {
        let alloc = Allocator::default();
        let source = "(() => { var t = { 57: 0, 8: 0 }; var g = self[\"webpackChunkapp\"] = self[\"webpackChunkapp\"] || []; if (9 != c) {} })();";
        let parsed = parse_program(&alloc, source);
        let (text, installed) = abstracted(first(&parsed.program), "r").expect("abstracted");
        assert_eq!(installed, ["57", "8"]);
        assert!(!text.contains("webpackChunkapp"), "{text}");
        assert!(!text.contains("57"), "{text}");
        assert!(!text.contains('9'), "{text}");
    }

    #[test]
    fn two_global_names_are_not_abstracted() {
        let alloc = Allocator::default();
        let source =
            "(() => { var g = self[\"webpackChunka\"] = self[\"webpackChunkb\"] || []; })();";
        let parsed = parse_program(&alloc, source);
        assert!(abstracted(first(&parsed.program), "r").is_none());
    }

    #[test]
    fn a_marked_string_is_refused() {
        let alloc = Allocator::default();
        let source = "(() => { var g = \"\\0installed\"; })();";
        let parsed = parse_program(&alloc, source);
        assert!(abstracted(first(&parsed.program), "r").is_none());
    }
}
