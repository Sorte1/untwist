use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use untwist_core::ast_util::global::references_are_global;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};
use untwist_eval::fold::{const_to_expression, fold_literal_chain, fold_literal_chain_expression};

use crate::calls::agnostic::{aroot_of, fold_record, ARoot, FoldRecord};
use crate::proof::has_with_statement;
use crate::proof::ProtoRoot;
use crate::string_array::bare_template_text;

pub const PASS: &str = "fold-literal-chains";

const IMPLICIT_KEYS: &[&str] = &[
    "toString",
    "valueOf",
    "@@toPrimitive",
    "constructor",
    "@@isConcatSpreadable",
    "@@iterator",
    "@@split",
    "@@replace",
    "@@match",
    "@@matchAll",
    "@@search",
    "join",
    "length",
    "exec",
    "flags",
    "lastIndex",
];

const SPECIES_CREATING: &[&str] = &[
    "concat", "slice", "splice", "map", "filter", "flat", "flatMap",
];

fn chain_root<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let (callee, _) = call_parts(expr)?;
    let (object, _) = member_parts(callee)?;
    match chain_root(object) {
        Some(root) => Some(root),
        None => Some(object),
    }
}

fn is_literal_element(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    ) || bare_template_text(expr).is_some()
}

fn is_literal_root(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::StringLiteral(_) => true,
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(is_literal_element)),
        other => bare_template_text(other).is_some(),
    }
}

struct ChainKeys<'k> {
    methods: Vec<std::borrow::Cow<'k, str>>,
    plain: bool,
}

fn chain_keys<'k>(expr: &'k Expression<'_>) -> ChainKeys<'k> {
    let mut keys = ChainKeys {
        methods: Vec::new(),
        plain: true,
    };
    let mut current = expr;
    while let Some((callee, arguments)) = call_parts(current) {
        let Some((object, key)) = member_parts(callee) else {
            break;
        };
        keys.methods.push(key);
        keys.plain &= arguments
            .iter()
            .all(|argument| argument.as_expression().is_some_and(is_literal_element));
        current = object;
    }
    keys
}

const STRING_PROTOTYPE_OWN: [&str; 53] = [
    "length",
    "constructor",
    "anchor",
    "at",
    "big",
    "blink",
    "bold",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "concat",
    "endsWith",
    "fontcolor",
    "fontsize",
    "fixed",
    "includes",
    "indexOf",
    "isWellFormed",
    "italics",
    "lastIndexOf",
    "link",
    "localeCompare",
    "match",
    "matchAll",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "replace",
    "replaceAll",
    "search",
    "slice",
    "small",
    "split",
    "strike",
    "sub",
    "substr",
    "substring",
    "sup",
    "startsWith",
    "toString",
    "toWellFormed",
    "trim",
    "trimStart",
    "trimLeft",
    "trimEnd",
    "trimRight",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "toLowerCase",
    "toUpperCase",
    "valueOf",
    "@@iterator",
];

const ARRAY_PROTOTYPE_OWN: [&str; 42] = [
    "length",
    "constructor",
    "at",
    "concat",
    "copyWithin",
    "fill",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "lastIndexOf",
    "pop",
    "push",
    "reverse",
    "shift",
    "unshift",
    "slice",
    "sort",
    "splice",
    "includes",
    "indexOf",
    "join",
    "keys",
    "entries",
    "values",
    "forEach",
    "filter",
    "flat",
    "flatMap",
    "map",
    "every",
    "some",
    "reduce",
    "reduceRight",
    "toReversed",
    "toSorted",
    "toSpliced",
    "with",
    "toLocaleString",
    "toString",
    "@@iterator",
    "@@unscopables",
];

fn any_demanded(keys: &ChainKeys<'_>, written: impl FnMut(&str) -> bool) -> bool {
    let species = keys
        .methods
        .iter()
        .any(|key| SPECIES_CREATING.contains(&key.as_ref()));
    keys.methods
        .iter()
        .map(|key| key.as_ref())
        .chain(IMPLICIT_KEYS.iter().copied())
        .chain(species.then_some("@@species"))
        .any(written)
}

const SYMBOL_ARGUMENT_METHODS: [(&str, &str); 6] = [
    ("split", "@@split"),
    ("replace", "@@replace"),
    ("replaceAll", "@@replace"),
    ("match", "@@match"),
    ("matchAll", "@@matchAll"),
    ("search", "@@search"),
];

fn root_hits(root: ProtoRoot, marker: &str, own: bool) -> bool {
    match aroot_of(root) {
        ARoot::Any => true,
        ARoot::Str => marker == "s",
        ARoot::Arr => marker == "a",
        ARoot::Obj => !own,
        ARoot::Other => false,
    }
}

fn a_hits(record: &FoldRecord, marker: &str, key: &str, own: bool) -> bool {
    record.keys.iter().any(|(name, root)| {
        root_hits(*root, marker, own)
            && match name.as_deref() {
                None => true,
                Some("__proto__") => !own,
                Some(name) => name == key,
            }
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Str,
    Arr,
    Done,
}

const STRING_RETURNS_STRING: [&str; 27] = [
    "at",
    "charAt",
    "concat",
    "padEnd",
    "padStart",
    "repeat",
    "slice",
    "substr",
    "substring",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "trim",
    "trimStart",
    "trimEnd",
    "trimLeft",
    "trimRight",
    "toString",
    "valueOf",
    "normalize",
    "toWellFormed",
    "replace",
    "replaceAll",
    "anchor",
    "big",
    "bold",
];

fn step_demand(
    kind: Kind,
    method: &str,
    demand: &mut Vec<(&'static str, String, bool)>,
) -> Option<Kind> {
    match kind {
        Kind::Str => {
            if !STRING_PROTOTYPE_OWN.contains(&method)
                || matches!(method, "match" | "matchAll" | "search")
            {
                return None;
            }
            demand.push(("s", method.to_string(), true));
            if let Some((_, symbol)) = SYMBOL_ARGUMENT_METHODS
                .iter()
                .find(|(name, _)| *name == method)
            {
                demand.push(("s", (*symbol).to_string(), false));
            }
            Some(if method == "split" {
                Kind::Arr
            } else if STRING_RETURNS_STRING.contains(&method) {
                Kind::Str
            } else {
                Kind::Done
            })
        }
        Kind::Arr => {
            if !ARRAY_PROTOTYPE_OWN.contains(&method) {
                return None;
            }
            demand.push(("a", method.to_string(), true));
            match method {
                "join" => Some(Kind::Str),
                "toString" => {
                    demand.push(("a", "join".to_string(), true));
                    Some(Kind::Str)
                }
                "indexOf" | "lastIndexOf" | "includes" => Some(Kind::Done),
                "reverse" => Some(Kind::Arr),
                "slice" | "concat" => {
                    demand.push(("a", "constructor".to_string(), true));
                    demand.push(("a", "@@species".to_string(), false));
                    if method == "concat" {
                        demand.push(("a", "@@isConcatSpreadable".to_string(), false));
                    }
                    Some(Kind::Arr)
                }
                _ => None,
            }
        }
        Kind::Done => None,
    }
}

fn chain_demand(
    keys: &ChainKeys<'_>,
    root: &Expression<'_>,
) -> Option<Vec<(&'static str, String, bool)>> {
    let mut kind = match root {
        Expression::ArrayExpression(_) => Kind::Arr,
        _ => Kind::Str,
    };
    let mut demand = Vec::new();
    for method in keys.methods.iter().rev() {
        kind = step_demand(kind, method.as_ref(), &mut demand)?;
    }
    Some(demand)
}

fn a_patched(record: &FoldRecord, expr: &Expression<'_>, root: &Expression<'_>) -> bool {
    if record.code_text {
        return true;
    }
    let keys = chain_keys(expr);
    if !keys.plain {
        return true;
    }
    let Some(demand) = chain_demand(&keys, root) else {
        return any_demanded(&keys, |key| {
            record.keys.iter().any(|(name, _)| {
                name.as_deref()
                    .is_none_or(|name| name == key || name == "__proto__")
            })
        });
    };
    demand
        .iter()
        .any(|(marker, key, own)| a_hits(record, marker, key, *own))
}

#[cold]
#[inline(never)]
fn may_be_patched(
    expr: &Expression<'_>,
    root: &Expression<'_>,
    cx: &Cx<'_, '_>,
    a: Option<&FoldRecord>,
) -> bool {
    let code_text = cx.opts.code_text.is_marked();
    code_text || a.is_none_or(|record| a_patched(record, expr, root))
}

#[inline(never)]
fn fold_call<'a>(expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>, a: Option<&FoldRecord>) {
    let Some(root) = chain_root(expr) else {
        return;
    };
    if !is_literal_root(root) {
        return;
    }
    let Some(folded) = fold_literal_chain(expr) else {
        return;
    };
    if !references_are_global(expr, cx.scoping()) || may_be_patched(expr, root, cx, a) {
        return;
    }
    let span = expr.span();
    let Some(replacement) = const_to_expression(&folded, span, &*cx) else {
        return;
    };
    *expr = replacement;
    cx.changed();
}

fn char_code_argument(argument: &Argument<'_>) -> bool {
    match argument
        .as_expression()
        .map(Expression::without_parentheses)
    {
        Some(Expression::NumericLiteral(_) | Expression::StringLiteral(_)) => true,
        Some(Expression::UnaryExpression(unary)) => {
            unary.operator == UnaryOperator::UnaryNegation
                && matches!(
                    unary.argument.without_parentheses(),
                    Expression::NumericLiteral(_)
                )
        }
        Some(Expression::Identifier(id)) => matches!(id.name.as_str(), "NaN" | "Infinity"),
        _ => false,
    }
}

fn is_char_code_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return false;
    };
    !call.optional
        && !member.optional
        && member.property.name == "fromCharCode"
        && matches!(member.object.without_parentheses(), Expression::Identifier(id) if id.name == "String")
        && call.arguments.iter().all(char_code_argument)
}

const CHAR_CODE_KEYS: [&str; 3] = ["String", "fromCharCode", "__proto__"];

fn char_code_patched(cx: &Cx<'_, '_>, a: Option<&FoldRecord>) -> bool {
    cx.opts.code_text.is_marked()
        || a.is_none_or(|record| {
            record.code_text
                || record.keys.iter().any(|(name, _)| {
                    name.as_deref()
                        .is_none_or(|name| CHAR_CODE_KEYS.contains(&name))
                })
        })
}

#[inline(never)]
fn fold_char_code<'a>(expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>, a: Option<&FoldRecord>) {
    if !is_char_code_call(expr) || !references_are_global(expr, cx.scoping()) {
        return;
    }
    if !cx.opts.unsafe_transforms && char_code_patched(cx, a) {
        return;
    }
    let Some(replacement) = fold_literal_chain_expression(expr, expr.span(), &*cx) else {
        return;
    };
    *expr = replacement;
    cx.changed();
}

#[derive(Default)]
pub struct FoldLiteralChains {
    eval_checked: bool,
    a: Option<FoldRecord>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FoldLiteralChains::default())
}

fn has_direct_eval(scoping: &Scoping) -> bool {
    scoping
        .scope_descendants_from_root()
        .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::DirectEval))
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::CallExpression]);

impl<'a> Pass<'a> for FoldLiteralChains {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION).on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if !self.eval_checked {
            self.eval_checked = true;
            if has_direct_eval(cx.scoping()) || has_with_statement(cx.scoping()) {
                cx.opts.code_text.mark();
                return;
            }
        }
        self.a = Some(fold_record(program, cx.scoping()));
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if matches!(expr, Expression::CallExpression(_)) {
            fold_char_code(expr, cx, self.a.as_ref());
        }
        if matches!(expr, Expression::CallExpression(_)) {
            fold_call(expr, cx, self.a.as_ref());
        }
    }
}
