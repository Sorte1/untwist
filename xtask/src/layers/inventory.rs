use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::parse_program;

#[derive(clap::Args, Debug)]
pub struct Request {
    #[arg(long, default_value = "input")]
    pub label: String,
    #[arg(long)]
    pub each: bool,
    pub paths: Vec<PathBuf>,
}

pub type Counts = BTreeMap<String, usize>;

pub fn run(request: &Request) -> Result<()> {
    let mut files = Vec::new();
    for path in &request.paths {
        collect(path, &mut files)?;
    }
    files.sort();
    let mut total = Counts::new();
    for file in &files {
        let source =
            std::fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
        let counts = count(&source);
        if request.each && !counts.is_empty() {
            println!("file {} {}", file.display(), render(&counts));
        }
        for (form, n) in counts {
            *total.entry(form).or_default() += n;
        }
    }
    println!(
        "inventory {} files {} {}",
        request.label,
        files.len(),
        render(&total)
    );
    Ok(())
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    for entry in std::fs::read_dir(path).with_context(|| format!("read {}", path.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "js" | "mjs" | "cjs"))
        {
            out.push(path);
        }
    }
    Ok(())
}

pub fn render(counts: &Counts) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .iter()
        .map(|(form, n)| format!("{form}={n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn count(source: &str) -> Counts {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut helpers = HelperFinder::default();
    helpers.visit_program(&parsed.program);
    let mut counter = Counter {
        counts: Counts::new(),
        helpers: helpers.names,
        links: BTreeSet::new(),
    };
    counter.visit_program(&parsed.program);
    if !parsed.diagnostics.is_empty() {
        counter.bump("parse-diagnostics", false);
    }
    counter.counts
}

fn ident_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn member_name<'a>(expr: &'a Expression<'a>) -> Option<(&'a Expression<'a>, &'a str)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(key) => Some((&member.object, key.value.as_str()?)),
            _ => None,
        },
        _ => None,
    }
}

fn is_text_literal(expr: &Expression) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

fn is_stringish(expr: &Expression) -> bool {
    match expr.without_parentheses() {
        Expression::BinaryExpression(binary) => {
            is_stringish(&binary.left) || is_stringish(&binary.right)
        }
        Expression::TemplateLiteral(_) | Expression::StringLiteral(_) => true,
        _ => false,
    }
}

fn all_text(arguments: &[Argument]) -> bool {
    !arguments.is_empty()
        && arguments
            .iter()
            .all(|arg| arg.as_expression().is_some_and(is_text_literal))
}

fn first_arg<'a>(arguments: &'a [Argument<'a>]) -> Option<&'a Expression<'a>> {
    arguments.first().and_then(Argument::as_expression)
}

fn object_kind(expr: &Expression) -> &'static str {
    match expr.without_parentheses() {
        Expression::ThisExpression(_) => "this",
        Expression::Identifier(id) => match id.name.as_str() {
            "window" => "window",
            "globalThis" => "globalThis",
            "self" => "self",
            "global" => "global",
            _ => "other",
        },
        _ => "other",
    }
}

fn is_split_bar(arg: Option<&Argument>) -> bool {
    let Some(Expression::CallExpression(call)) = arg.and_then(Argument::as_expression) else {
        return false;
    };
    member_name(&call.callee).is_some_and(|(_, name)| name == "split")
        && matches!(first_arg(&call.arguments), Some(Expression::StringLiteral(s)) if s.value == "|")
}

fn is_packer_call(call: &CallExpression) -> bool {
    let Expression::FunctionExpression(func) = call.callee.without_parentheses() else {
        return false;
    };
    func.params.items.len() == 6 && call.arguments.len() == 6 && is_split_bar(call.arguments.get(3))
}

struct Counter {
    counts: Counts,
    helpers: BTreeSet<String>,
    links: BTreeSet<u32>,
}

impl Counter {
    fn bump(&mut self, form: &str, literal: bool) {
        *self.counts.entry(form.to_string()).or_default() += 1;
        if literal {
            *self.counts.entry(format!("{form}:literal")).or_default() += 1;
        }
    }

    fn classify_call(&mut self, call: &CallExpression) {
        let literal = all_text(&call.arguments);
        if is_packer_call(call) {
            self.bump("packer", false);
        }
        if let Some(name) = ident_name(&call.callee) {
            self.classify_named_call(name, call, literal);
            return;
        }
        if let Expression::SequenceExpression(seq) = call.callee.without_parentheses() {
            if seq.expressions.last().and_then(ident_name) == Some("eval") {
                self.bump("eval-indirect", literal);
            }
            return;
        }
        if let Some((object, name)) = member_name(&call.callee) {
            self.classify_member_call(object, name, call, literal);
        }
    }

    fn classify_named_call(&mut self, name: &str, call: &CallExpression, literal: bool) {
        match name {
            "eval" => self.bump("eval", literal),
            "Function" => self.bump("Function", literal),
            "atob" => self.bump("atob", literal),
            "setTimeout" | "setInterval"
                if first_arg(&call.arguments).is_some_and(is_stringish) =>
            {
                self.bump("timer-string", false);
            }
            _ if self.helpers.contains(name) => self.classify_helper_call(name, call),
            _ => {}
        }
    }

    fn classify_member_call(
        &mut self,
        object: &Expression,
        name: &str,
        call: &CallExpression,
        literal: bool,
    ) {
        match name {
            "eval" => self.bump(&format!("member-eval:{}", object_kind(object)), literal),
            "Function" => self.bump("member-Function", literal),
            "setTimeout" | "setInterval"
                if first_arg(&call.arguments).is_some_and(is_stringish) =>
            {
                self.bump("timer-string", false);
            }
            "defineProperty" if is_prototype(first_arg(&call.arguments)) => {
                self.bump("prototype-define", false)
            }
            _ => {}
        }
    }

    fn note_chain(&mut self, call: &CallExpression) {
        let Some(name) = ident_name(&call.callee).filter(|name| self.helpers.contains(*name))
        else {
            return;
        };
        if let Some(Expression::CallExpression(arg)) = first_arg(&call.arguments) {
            if ident_name(&arg.callee) == Some(name) {
                self.links.insert(arg.span.start);
            }
        }
        if self.links.contains(&call.span.start) {
            return;
        }
        self.bump("define-chain", false);
        let (depth, base) = chain_depth(name, call);
        *self
            .counts
            .entry(format!("define-chain-depth{depth:02}"))
            .or_default() += 1;
        *self
            .counts
            .entry(format!("define-chain-base-{base}"))
            .or_default() += 1;
    }

    fn classify_helper_call(&mut self, name: &str, call: &CallExpression) {
        self.bump("define-helper-call", false);
        self.bump(&format!("define-helper-call<{name}>"), false);
        let inner = matches!(first_arg(&call.arguments),
            Some(Expression::CallExpression(arg)) if ident_name(&arg.callee) == Some(name));
        if inner {
            self.bump("define-helper-nested", false);
        }
    }
}

fn is_prototype(expr: Option<&Expression>) -> bool {
    expr.and_then(member_name)
        .is_some_and(|(_, name)| name == "prototype")
}

fn chain_depth<'a>(name: &str, call: &'a CallExpression<'a>) -> (usize, &'static str) {
    match first_arg(&call.arguments) {
        Some(Expression::CallExpression(arg)) if ident_name(&arg.callee) == Some(name) => {
            let (depth, base) = chain_depth(name, arg);
            (depth + 1, base)
        }
        Some(Expression::ObjectExpression(_)) => (1, "object"),
        Some(Expression::AssignmentExpression(_)) => (1, "assign"),
        _ => (1, "other"),
    }
}

impl<'a> Visit<'a> for Counter {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.classify_call(it);
        self.note_chain(it);
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if ident_name(&it.callee) == Some("Function") {
            self.bump("new-Function", all_text(&it.arguments));
        }
        walk::walk_new_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::StaticMemberExpression(member) = &it.left {
            if member.property.name == "prototype" {
                self.bump("prototype-assign", false);
            } else if member_name(&member.object).is_some_and(|(_, name)| name == "prototype") {
                self.bump("prototype-member-write", false);
            }
        }
        walk::walk_assignment_expression(self, it);
    }
}

#[derive(Default)]
struct HelperFinder {
    names: BTreeSet<String>,
    stack: Vec<(Option<String>, usize, bool)>,
    pending: Option<String>,
}

fn is_descriptor_define(call: &CallExpression) -> bool {
    let is_define = member_name(&call.callee).is_some_and(|(object, name)| {
        name == "defineProperty" && ident_name(object) == Some("Object")
    });
    let Some(Expression::ObjectExpression(descriptor)) =
        call.arguments.get(2).and_then(Argument::as_expression)
    else {
        return false;
    };
    let has_key = |wanted: &str| {
        descriptor.properties.iter().any(|prop| {
            matches!(prop, ObjectPropertyKind::ObjectProperty(p)
                if p.key.static_name().and_then(|name| name.into_cow_str()).is_some_and(|key| key == wanted))
        })
    };
    is_define && has_key("writable") && has_key("value")
}

impl<'a> Visit<'a> for HelperFinder {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.pending = matches!(&it.init, Some(Expression::FunctionExpression(_)))
            .then(|| it.id.get_identifier_name().map(|name| name.to_string()))
            .flatten();
        walk::walk_variable_declarator(self, it);
        self.pending = None;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let name = it
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .or_else(|| self.pending.take());
        self.stack.push((name, it.params.items.len(), false));
        walk::walk_function(self, it, flags);
        if let Some((Some(name), 3, true)) = self.stack.pop() {
            self.names.insert(name);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_descriptor_define(it) {
            if let Some(top) = self.stack.last_mut() {
                top.2 = true;
            }
        }
        walk::walk_call_expression(self, it);
    }
}
