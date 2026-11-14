use oxc_ast::ast::*;
use oxc_ast::CommentKind;
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;

use crate::matcher::{call_parts, const_member};

const LINE_START: &str = "//";
const MODULES_START: &str = ": ";
const ALTERNATIVE: &str = " | ";
const SAME_CODE: &str = ", same code in ";
const SCRIPT_SUFFIX: &str = ".js";
const REQUIRE: &str = "require";
const COVERED_PERCENT: u64 = 60;

fn package_name(label: &str) -> bool {
    let mut parts = label.splitn(2, '/');
    let first = parts.next().unwrap_or_default();
    let scoped = first.starts_with('@');
    let plain = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-._".contains(&byte)
            })
            && !part.starts_with('.')
    };
    match (scoped, parts.next()) {
        (true, Some(rest)) => plain(&first[1..]) && plain(rest),
        (false, None) => plain(first),
        _ => false,
    }
}

fn module_path(path: &str) -> bool {
    path.ends_with(SCRIPT_SUFFIX)
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-._@".contains(&byte))
        })
}

pub fn parse(comment: &str) -> Option<String> {
    let text = comment.strip_prefix(LINE_START)?.trim();
    let (head, modules) = text.split_once(MODULES_START)?;
    let (label, range) = head.split_once(' ')?;
    if !range.starts_with(|ch: char| ch.is_ascii_digit()) || !package_name(label) {
        return None;
    }
    let own = modules.split(SAME_CODE).next()?;
    if own.contains(ALTERNATIVE) || !module_path(own) {
        return None;
    }
    Some(format!("{label}/{own}"))
}

fn is_require_call(expr: &Expression<'_>) -> bool {
    let expr = expr.without_parentheses();
    let callee = match call_parts(expr) {
        Some((callee, _)) => callee,
        None => match const_member(expr, "default").or_else(|| member_object(expr)) {
            Some(object) => return is_require_call(object),
            None => return false,
        },
    };
    match callee {
        Expression::Identifier(id) => id.name == REQUIRE,
        other => const_member(other, "n").is_some_and(
            |object| matches!(object, Expression::Identifier(id) if id.name == REQUIRE),
        ),
    }
}

fn member_object<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr {
        Expression::StaticMemberExpression(member) => Some(&member.object),
        _ => None,
    }
}

fn is_runtime_call(expr: &Expression<'_>) -> bool {
    let Some((callee, _)) = call_parts(expr.without_parentheses()) else {
        return false;
    };
    let helper = |name: &str| {
        const_member(callee, name).is_some_and(
            |object| matches!(object, Expression::Identifier(id) if id.name == REQUIRE),
        )
    };
    helper("r") || helper("d") || is_require_call(expr)
}

fn trivial(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ImportDeclaration(_)
        | Statement::EmptyStatement(_)
        | Statement::ExportNamedDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .all(|declarator| declarator.init.as_ref().is_none_or(is_require_call)),
        Statement::ExpressionStatement(statement) => is_runtime_call(&statement.expression),
        _ => false,
    }
}

pub fn library_path(program: &Program<'_>) -> Option<String> {
    let starts: FxHashMap<u32, usize> = program
        .body
        .iter()
        .enumerate()
        .map(|(index, statement)| (statement.span().start, index))
        .collect();
    let mut covered: FxHashMap<usize, String> = FxHashMap::default();
    for comment in &program.comments {
        if comment.kind != CommentKind::Line || !comment.is_leading() {
            continue;
        }
        let Some(&index) = starts.get(&comment.attached_to) else {
            continue;
        };
        let text = comment.span.source_text(program.source_text);
        if let Some(path) = parse(text) {
            covered.insert(index, path);
        }
    }
    let mut paths: Vec<&String> = covered.values().collect();
    paths.sort_unstable();
    paths.dedup();
    let [path] = paths.as_slice() else {
        return None;
    };
    let mut total = 0u64;
    let mut inside = 0u64;
    for (index, statement) in program.body.iter().enumerate() {
        if trivial(statement) && !covered.contains_key(&index) {
            continue;
        }
        let size = u64::from(statement.span().size());
        total += size;
        if covered.contains_key(&index) {
            inside += size;
        }
    }
    (total > 0 && inside * 100 >= total * COVERED_PERCENT).then(|| (*path).clone())
}
