use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentTarget, BindingIdentifier, CallExpression, Class,
    Expression, ForInStatement, ForOfStatement, Function, IdentifierReference, Program,
    SimpleAssignmentTarget, Statement, UnaryExpression, WithStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::semantic::{other_inputs_given, program_is_strict};
use untwist_eval::{function_text, run_sink, SinkCallee, SinkOutcome, SinkScript};

use super::decode::sloppy_script;
use super::eval::verdict;
use super::function::{joined, pooled};
use super::model::{DecodeCx, Decoded, Layer, LayerForm, Refusal, Refused};

const POOL_CAP: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Called,
    Evaluated,
}

impl Shape {
    fn form(self) -> LayerForm {
        match self {
            Shape::Called => LayerForm::LoaderFunction,
            Shape::Evaluated => LayerForm::LoaderEval,
        }
    }

    fn callee(self) -> SinkCallee {
        match self {
            Shape::Called => SinkCallee::Function,
            Shape::Evaluated => SinkCallee::Eval,
        }
    }
}

struct Site<'p, 'a> {
    source: &'a str,
    start: u32,
    prefix: &'p [Statement<'a>],
    call: &'p CallExpression<'a>,
}

struct Parts<'p, 'a> {
    shape: Shape,
    callee: &'p Expression<'a>,
    args: Vec<&'p Expression<'a>>,
    kept: &'p [Argument<'a>],
}

enum Probed {
    Peeled(Layer),
    Refused(Refusal),
    Missed,
}

struct Spent {
    steps: u64,
    heap: u64,
    clean: bool,
}

pub fn decode(program: &Program<'_>, cx: &DecodeCx) -> Decoded {
    let site = site(program)?;
    for shape in [Shape::Called, Shape::Evaluated] {
        let Some(parts) = parts(site.call, shape) else {
            continue;
        };
        match probed(&site, &parts, cx) {
            Probed::Peeled(layer) => return Some(Ok(layer)),
            Probed::Refused(reason) => {
                return Some(Err(Refused {
                    form: shape.form(),
                    reason,
                }))
            }
            Probed::Missed => {}
        }
    }
    None
}

fn site<'p, 'a>(program: &'p Program<'a>) -> Option<Site<'p, 'a>> {
    if program.source_type.is_module() || !sloppy_script(program) {
        return None;
    }
    let (index, last) = program
        .body
        .iter()
        .enumerate()
        .rev()
        .find(|(_, statement)| !matches!(statement, Statement::EmptyStatement(_)))?;
    let Statement::ExpressionStatement(last) = last else {
        return None;
    };
    let Expression::CallExpression(call) = last.expression.without_parentheses() else {
        return None;
    };
    let prefix = &program.body[..index];
    if call.optional
        || prefix
            .iter()
            .any(|statement| Syntax::of_statement(statement, false).functions)
    {
        return None;
    }
    Some(Site {
        source: program.source_text,
        start: last.span.start,
        prefix,
        call,
    })
}

fn parts<'p, 'a>(call: &'p CallExpression<'a>, shape: Shape) -> Option<Parts<'p, 'a>> {
    let (callee, args, kept) = match shape {
        Shape::Called => {
            let (callee, args) = match call.callee.without_parentheses() {
                Expression::CallExpression(inner) if !inner.optional => {
                    (&inner.callee, &inner.arguments)
                }
                Expression::NewExpression(inner) => (&inner.callee, &inner.arguments),
                _ => return None,
            };
            (callee, plain(args)?, call.arguments.as_slice())
        }
        Shape::Evaluated => {
            let args = plain(&call.arguments)?;
            if args.len() != 1 {
                return None;
            }
            (&call.callee, args, &[][..])
        }
    };
    plain(kept)?;
    let mut callee_syntax = Syntax::default();
    callee_syntax.visit_expression(callee);
    if callee_syntax.functions {
        return None;
    }
    Some(Parts {
        shape,
        callee,
        args,
        kept,
    })
}

fn plain<'p, 'a>(arguments: &'p [Argument<'a>]) -> Option<Vec<&'p Expression<'a>>> {
    arguments.iter().map(Argument::as_expression).collect()
}

fn probed(site: &Site<'_, '_>, parts: &Parts<'_, '_>, cx: &DecodeCx) -> Probed {
    let prefix = &site.source[..site.start as usize];
    let region = region_text(site.source, parts);
    let mut pool: Vec<Vec<String>> = Vec::new();
    let mut spent = Spent {
        steps: 0,
        heap: 0,
        clean: false,
    };
    loop {
        let left = cx.fuel.saturating_sub(spent.steps);
        if left == 0 {
            return Probed::Refused(Refusal::Fuel);
        }
        let outcome = match oracle(prefix, &region, &pool, (left, cx.heap), &mut spent) {
            Some(outcome) => outcome,
            None => return Probed::Missed,
        };
        match outcome {
            SinkOutcome::Unknown(texts) => {
                if pool.len() >= POOL_CAP || pool.contains(&texts) {
                    return Probed::Refused(Refusal::Cap);
                }
                if pooled(&texts).is_none() {
                    return Probed::Refused(Refusal::Injection);
                }
                pool.push(texts);
            }
            SinkOutcome::Reached { callee, texts } if callee == parts.shape.callee() => {
                if !cx.unsafe_transforms && other_inputs_given() {
                    return Probed::Refused(Refusal::NeedsUnsafe);
                }
                let prefix = match spent.clean && cx.unsafe_transforms {
                    true => kept_prefix(site, prefix, parts, &texts),
                    false => prefix,
                };
                return match peeled_text((site, parts), prefix, &texts, cx) {
                    Ok(text) => Probed::Peeled(Layer {
                        form: parts.shape.form(),
                        text,
                        steps: spent.steps,
                        heap: spent.heap,
                    }),
                    Err(reason) => Probed::Refused(reason),
                };
            }
            SinkOutcome::Reached { .. } | SinkOutcome::Refused => return Probed::Missed,
        }
    }
}

fn region_text(source: &str, parts: &Parts<'_, '_>) -> String {
    let pieces: Vec<&str> = std::iter::once(parts.callee)
        .chain(parts.args.iter().copied())
        .map(|expr| span_text(source, expr.span()))
        .collect();
    format!("[({})]", pieces.join("),("))
}

fn span_text(source: &str, span: oxc_span::Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

fn oracle(
    prefix: &str,
    region: &str,
    pool: &[Vec<String>],
    (fuel, heap): (u64, u64),
    spent: &mut Spent,
) -> Option<SinkOutcome> {
    let mut script = format!("{prefix}\n;{region};\nif (0) {{\n");
    let region_start = u32::try_from(prefix.len() + 2).ok()?;
    for texts in pool {
        script.push_str(&pooled(texts)?);
        script.push_str(";\n");
    }
    script.push_str("}\n");
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, &script, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let program = &parsed.program;
    let sink = program
        .body
        .iter()
        .position(|statement| statement.span().start == region_start)?;
    let functions = pool_functions(program.body.last()?)?;
    if functions.len() != pool.len() || !stays_inside(program, region_start) {
        return None;
    }
    let pool = pool
        .iter()
        .map(|texts| function_text(texts))
        .zip(functions)
        .collect();
    let run = run_sink(
        SinkScript {
            program,
            sink,
            region: region_start,
            pool,
        },
        fuel,
        heap,
    );
    spent.steps = spent.steps.saturating_add(run.steps);
    spent.heap = spent.heap.max(run.heap);
    spent.clean = run.prefix_clean;
    Some(run.outcome)
}

fn pool_functions<'p, 'a>(last: &'p Statement<'a>) -> Option<Vec<&'p Function<'a>>> {
    let Statement::IfStatement(pool) = last else {
        return None;
    };
    let Statement::BlockStatement(block) = &pool.consequent else {
        return None;
    };
    block
        .body
        .iter()
        .map(|statement| match statement {
            Statement::ExpressionStatement(only) => match &only.expression {
                Expression::FunctionExpression(function) => Some(&**function),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn stays_inside(program: &Program<'_>, region: u32) -> bool {
    let mut writes = Syntax::default();
    for statement in &program.body {
        if statement.span().start >= region {
            writes.visit_statement(statement);
        }
    }
    if writes.writes_out {
        return false;
    }
    let built = SemanticBuilder::new().with_build_nodes(true).build(program);
    bindings_stay_inside(&built.semantic, region)
}

fn bindings_stay_inside(semantic: &Semantic<'_>, region: u32) -> bool {
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let written_in_region = |id: &oxc_syntax::reference::ReferenceId| {
        let reference = scoping.get_reference(*id);
        reference.is_write() && nodes.get_node(reference.node_id()).span().start >= region
    };
    let unresolved = scoping
        .root_unresolved_references()
        .values()
        .flatten()
        .any(written_in_region);
    if unresolved {
        return false;
    }
    scoping.symbol_ids().all(|symbol| {
        scoping.symbol_span(symbol).start >= region
            || !scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .any(written_in_region)
    })
}

#[derive(Default)]
struct Syntax {
    functions: bool,
    writes_out: bool,
    unwatched: bool,
    names: Option<Vec<String>>,
}

impl Syntax {
    fn of_statement(statement: &Statement<'_>, named: bool) -> Self {
        let mut syntax = Syntax {
            names: named.then(Vec::new),
            ..Syntax::default()
        };
        syntax.visit_statement(statement);
        syntax
    }

    fn name(&mut self, name: &str) {
        if let Some(names) = self.names.as_mut() {
            names.push(name.to_string());
        }
    }
}

impl<'a> Visit<'a> for Syntax {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions = true;
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.functions = true;
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.functions = true;
        self.writes_out = true;
        walk::walk_class(self, it);
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        self.writes_out |= it.as_member_expression().is_some();
        walk::walk_assignment_target(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.writes_out |= it.as_member_expression().is_some();
        walk::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let deletes = it.operator == UnaryOperator::Delete;
        self.writes_out |= deletes;
        self.unwatched |= deletes;
        walk::walk_unary_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.writes_out = true;
        self.unwatched = true;
        walk::walk_with_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.unwatched = true;
        walk::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.unwatched = true;
        walk::walk_for_of_statement(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.name(&it.name);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.name(&it.name);
    }
}

fn peeled_text(
    (site, parts): (&Site<'_, '_>, &Parts<'_, '_>),
    prefix: &str,
    texts: &[String],
    cx: &DecodeCx,
) -> Result<String, Refusal> {
    let opened = prefix.trim().is_empty();
    let body = match parts.shape {
        Shape::Called => {
            let kept = kept_text(site.source, parts.kept);
            joined(texts, kept, parts.kept.len()).ok_or(Refusal::Injection)?
        }
        Shape::Evaluated => {
            let [text] = texts else {
                return Err(Refusal::Shape);
            };
            evaluated(text, cx)?
        }
    };
    Ok(match opened {
        true => body,
        false => format!("{prefix}\n;{body}"),
    })
}

fn kept_prefix<'s>(
    site: &Site<'_, '_>,
    prefix: &'s str,
    parts: &Parts<'_, '_>,
    texts: &[String],
) -> &'s str {
    let mut written = Syntax {
        names: Some(Vec::new()),
        ..Syntax::default()
    };
    for statement in site.prefix {
        written.visit_statement(statement);
    }
    if other_inputs_given() || written.unwatched {
        return prefix;
    }
    let mut read = Syntax {
        names: Some(Vec::new()),
        ..Syntax::default()
    };
    for kept in parts.kept {
        read.visit_argument(kept);
    }
    let written = written.names.unwrap_or_default();
    let mut read = read.names.unwrap_or_default();
    let decoded = match parts.shape {
        Shape::Called => texts
            .split_last()
            .map(|(body, params)| format!("(function ({}) {{\n{body}\n}})", params.join(","))),
        Shape::Evaluated => texts.first().cloned(),
    };
    let Some(names) = decoded.as_deref().and_then(outside_names) else {
        return prefix;
    };
    read.extend(names);
    let touches = read
        .iter()
        .any(|name| written.contains(name) || REFLECTIVE.contains(&name.as_str()));
    match touches {
        true => prefix,
        false => "",
    }
}

const REFLECTIVE: [&str; 11] = [
    "eval",
    "Function",
    "globalThis",
    "window",
    "self",
    "global",
    "frames",
    "top",
    "parent",
    "setTimeout",
    "setInterval",
];

fn outside_names(text: &str) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut with = Syntax::default();
    with.visit_program(&parsed.program);
    if with.unwatched {
        return None;
    }
    let built = SemanticBuilder::new().build(&parsed.program);
    let scoping = built.semantic.scoping();
    let root = scoping.root_scope_id();
    let mut names: Vec<String> = scoping
        .root_unresolved_references()
        .keys()
        .map(|name| name.to_string())
        .collect();
    names.extend(
        scoping
            .get_bindings(root)
            .iter()
            .map(|(name, _)| name.to_string()),
    );
    Some(names)
}

fn kept_text<'s>(source: &'s str, kept: &[Argument<'_>]) -> &'s str {
    match (kept.first(), kept.last()) {
        (Some(first), Some(last)) => source
            .get(first.span().start as usize..last.span().end as usize)
            .unwrap_or(""),
        _ => "",
    }
}

fn evaluated(text: &str, cx: &DecodeCx) -> Result<String, Refusal> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return Err(Refusal::Unparsable);
    }
    let program = &parsed.program;
    if program_is_strict(program) {
        return Ok(format!("(() => {{\n{text}\n}})();"));
    }
    match verdict(program, cx.unsafe_transforms, other_inputs_given())? {
        true => Ok(format!("{{\n{text}\n}}")),
        false => Ok(text.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::decode;
    use crate::layers::model::{DecodeCx, LayerForm, Refusal};
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;

    const CX: DecodeCx = DecodeCx {
        unsafe_transforms: false,
        fuel: 2_000_000,
        heap: 1 << 24,
    };

    fn peeled(source: &str) -> Option<Result<(LayerForm, String), Refusal>> {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        decode(&parsed.program, &CX).map(|decoded| {
            decoded
                .map(|layer| (layer.form, layer.text))
                .map_err(|refused| refused.reason)
        })
    }

    #[test]
    fn a_function_text_built_by_an_inner_function_is_peeled() {
        let source = "var F = [].constructor.constructor;\nF(F('return \"f(\" + 1 + \")\"')())();";
        let text = "var F = [].constructor.constructor;\n\n;(function anonymous(\n) {\nf(1)\n})();";
        assert_eq!(
            peeled(source),
            Some(Ok((LayerForm::LoaderFunction, text.to_string())))
        );
    }

    #[test]
    fn an_eval_text_from_a_callback_is_peeled_in_place() {
        let source = "var k = 1;\neval([103, 41, 42].map(function (c) { return String.fromCharCode(c - k); }).join(''));";
        let text = "var k = 1;\n\n;f()";
        assert_eq!(
            peeled(source),
            Some(Ok((LayerForm::LoaderEval, text.to_string())))
        );
    }

    #[test]
    fn a_strict_eval_text_keeps_its_own_scope() {
        let source =
            "(0, [].constructor.constructor('return eval')())('\"use strict\"; var a = 1;');";
        let text = "(() => {\n\"use strict\"; var a = 1;\n})();";
        assert_eq!(
            peeled(source),
            Some(Ok((LayerForm::LoaderEval, text.to_string())))
        );
    }

    #[test]
    fn a_region_that_writes_outside_itself_is_not_peeled() {
        let rows = [
            "var o = {};\neval((o.x = 1, 'f()'));",
            "var n = 0;\neval((n = 1, 'f()'));",
            "eval((m = 1, 'f()'));",
            "var a = ['f()'];\neval(a.reverse()[0]);",
            "var o = { a: 'f()' };\neval((delete o.b, o.a));",
        ];
        for source in rows {
            assert_eq!(peeled(source), None, "{source}");
        }
    }

    #[test]
    fn a_prefix_with_a_function_is_not_tried() {
        let rows = [
            "function g() { return 'f()'; }\neval(g());",
            "var g = () => 'f()';\neval(g());",
            "eval((function () { return 'f()'; }));",
        ];
        for source in rows {
            assert_eq!(peeled(source), None, "{source}");
        }
    }

    #[test]
    fn a_function_text_that_names_itself_is_refused() {
        let source = "var F = [].constructor.constructor;\nF(F('return anonymous')())();";
        assert_eq!(peeled(source), Some(Err(Refusal::Injection)));
    }

    #[test]
    fn a_callee_that_runs_no_code_is_no_layer() {
        let rows = [
            "console.log('x');",
            "var s = 'x';\nString(s);",
            "[].concat('a')('b');",
        ];
        for source in rows {
            assert_eq!(peeled(source), None, "{source}");
        }
    }
}
