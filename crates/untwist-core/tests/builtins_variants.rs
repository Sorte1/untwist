use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_parser::{ParseOptions, Parser};
use untwist_core::parse::SourceType;
use untwist_core::semantic::builtins::{scan, Builtins, Reliance};
use untwist_core::semantic::{build_scoping, has_module_syntax};

const MATH_ROUND: Reliance = Reliance {
    roots: &["Math"],
    members: &[("Math", false, &["round"])],
};

const VARIANTS: usize = 44;

fn variant(expression: &Expression<'_>) -> usize {
    match expression {
        Expression::BooleanLiteral(_) => 0,
        Expression::NullLiteral(_) => 1,
        Expression::NumericLiteral(_) => 2,
        Expression::BigIntLiteral(_) => 3,
        Expression::RegExpLiteral(_) => 4,
        Expression::StringLiteral(_) => 5,
        Expression::TemplateLiteral(_) => 6,
        Expression::Identifier(_) => 7,
        Expression::Super(_) => 8,
        Expression::ArrayExpression(_) => 9,
        Expression::ArrowFunctionExpression(_) => 10,
        Expression::AssignmentExpression(_) => 11,
        Expression::AwaitExpression(_) => 12,
        Expression::BinaryExpression(_) => 13,
        Expression::CallExpression(_) => 14,
        Expression::ChainExpression(_) => 15,
        Expression::ClassExpression(_) => 16,
        Expression::ConditionalExpression(_) => 17,
        Expression::FunctionExpression(_) => 18,
        Expression::ImportExpression(_) => 19,
        Expression::LogicalExpression(_) => 20,
        Expression::NewExpression(_) => 21,
        Expression::ObjectExpression(_) => 22,
        Expression::ParenthesizedExpression(_) => 23,
        Expression::SequenceExpression(_) => 24,
        Expression::TaggedTemplateExpression(_) => 25,
        Expression::ThisExpression(_) => 26,
        Expression::UnaryExpression(_) => 27,
        Expression::UpdateExpression(_) => 28,
        Expression::YieldExpression(_) => 29,
        Expression::PrivateInExpression(_) => 30,
        Expression::ImportMeta(_) => 31,
        Expression::NewTarget(_) => 32,
        Expression::JSXElement(_) => 33,
        Expression::JSXFragment(_) => 34,
        Expression::TSAsExpression(_) => 35,
        Expression::TSSatisfiesExpression(_) => 36,
        Expression::TSTypeAssertion(_) => 37,
        Expression::TSNonNullExpression(_) => 38,
        Expression::TSInstantiationExpression(_) => 39,
        Expression::V8IntrinsicExpression(_) => 40,
        Expression::ComputedMemberExpression(_) => 41,
        Expression::StaticMemberExpression(_) => 42,
        Expression::PrivateFieldExpression(_) => 43,
    }
}

#[derive(Clone, Copy)]
enum Dialect {
    Js,
    Ts,
    V8,
}

struct Shape {
    around: &'static str,
    shape: &'static str,
    dialect: Dialect,
}

const fn js(shape: &'static str) -> Shape {
    Shape {
        around: "BODY",
        shape,
        dialect: Dialect::Js,
    }
}

const fn inside(around: &'static str, shape: &'static str) -> Shape {
    Shape {
        around,
        shape,
        dialect: Dialect::Js,
    }
}

const fn ts(shape: &'static str) -> Shape {
    Shape {
        around: "BODY",
        shape,
        dialect: Dialect::Ts,
    }
}

const ASYNC: &str = "async function g() { BODY }";
const GENERATOR: &str = "function* g() { BODY }";
const DERIVED: &str = "class A extends Object { #x; m() { BODY } }";
const ORDINARY: &str = "function g() { BODY }";
const MODULE: &str = "export {}; BODY";

const SHAPES: &[Shape] = &[
    js("true"),
    js("null"),
    js("1"),
    js("1n"),
    js("/a/"),
    js("'s'"),
    js("`a${SLOT}b`"),
    js("o"),
    js("this"),
    inside(DERIVED, "super[SLOT]"),
    inside(DERIVED, "super.m(SLOT)"),
    js("[SLOT]"),
    js("[0, ...SLOT]"),
    js("(() => SLOT)"),
    js("((a = SLOT) => 0)"),
    js("(() => { SLOT; })"),
    js("(o.y = SLOT)"),
    js("(o[SLOT] = 0)"),
    js("((SLOT).y = 0)"),
    js("(o.y ||= SLOT)"),
    js("(o[SLOT] ??= 0)"),
    js("((SLOT).y &&= 0)"),
    js("(o.y += SLOT)"),
    js("([o.y = SLOT] = [])"),
    js("({ [SLOT]: o.y } = {})"),
    js("({ y: o[SLOT] } = {})"),
    inside(ASYNC, "(await SLOT)"),
    js("(SLOT + 0)"),
    js("(0 in SLOT)"),
    js("(SLOT instanceof o)"),
    js("o(SLOT)"),
    js("(SLOT)()"),
    js("o.y(SLOT)"),
    js("o[SLOT]()"),
    js("(SLOT).y()"),
    js("o?.[SLOT]"),
    js("o?.(SLOT)"),
    js("(SLOT)?.y"),
    js("o?.y(SLOT)"),
    js("(SLOT)?.()"),
    js("(class { [SLOT]() {} })"),
    js("(class extends (SLOT) {})"),
    js("(class { y = SLOT; })"),
    js("(class { static { SLOT; } })"),
    js("(SLOT ? 0 : 1)"),
    js("(0 ? SLOT : 1)"),
    js("(0 ? 1 : SLOT)"),
    js("(function () { SLOT; })"),
    js("(function (a = SLOT) {})"),
    js("(function n() { return SLOT; })"),
    js("import(SLOT)"),
    js("import('x', SLOT)"),
    js("(SLOT || 0)"),
    js("(0 && SLOT)"),
    js("(0 ?? SLOT)"),
    js("new o(SLOT)"),
    js("new (SLOT)()"),
    js("new (SLOT).y()"),
    js("({ y: SLOT })"),
    js("({ [SLOT]: 0 })"),
    js("({ ...SLOT })"),
    js("({ get y() { return SLOT; } })"),
    js("({ y() { return SLOT; } })"),
    js("(SLOT)"),
    js("(SLOT, 0)"),
    js("(0, SLOT)"),
    js("o`${SLOT}`"),
    js("(SLOT)``"),
    js("(SLOT).y``"),
    js("typeof SLOT"),
    js("typeof (SLOT).y"),
    js("typeof o[SLOT]"),
    js("!SLOT"),
    js("void SLOT"),
    js("-SLOT"),
    js("~SLOT"),
    js("delete o[SLOT]"),
    js("delete (SLOT).y"),
    js("delete (SLOT)[0]"),
    js("o[SLOT]++"),
    js("--(SLOT).y"),
    inside(GENERATOR, "(yield SLOT)"),
    inside(GENERATOR, "(yield* SLOT)"),
    inside(DERIVED, "(#x in SLOT)"),
    inside(DERIVED, "(SLOT).#x"),
    inside(DERIVED, "(SLOT)?.#x"),
    inside(MODULE, "import.meta"),
    inside(ORDINARY, "new.target"),
    js("<a b={SLOT}>{SLOT}</a>"),
    js("<>{SLOT}</>"),
    ts("(SLOT as any)"),
    ts("(SLOT satisfies any)"),
    ts("(<any>SLOT)"),
    ts("(SLOT)!"),
    ts("(SLOT)!.y"),
    ts("((SLOT)<string>)"),
    Shape {
        around: "BODY",
        shape: "%DebugPrint(SLOT)",
        dialect: Dialect::V8,
    },
    js("(SLOT)[0]"),
    js("(SLOT).y"),
    js("o[SLOT]"),
];

const CONTEXTS: &[&str] = &[
    "EXPR;",
    "typeof EXPR;",
    "typeof (EXPR);",
    "!EXPR;",
    "void EXPR;",
    "EXPR == 0;",
    "0 !== EXPR;",
    "EXPR instanceof Object;",
    "0 in EXPR;",
    "if (EXPR) {}",
    "while (EXPR) break;",
    "do {} while (EXPR);",
    "x = EXPR;",
    "var v = EXPR;",
    "x = (EXPR, 0);",
    "x = (0, EXPR);",
    "(EXPR, 0);",
    "(EXPR).y;",
    "(EXPR)[0];",
    "(EXPR)();",
    "(EXPR).y();",
    "new (EXPR)();",
    "(EXPR)``;",
    "(EXPR)?.y;",
    "(EXPR)?.();",
    "(EXPR).y = 1;",
    "(EXPR)[0] = 1;",
    "delete (EXPR).y;",
    "delete (EXPR)[0];",
    "(EXPR).y ||= 0;",
    "x = ((EXPR).y ||= 0);",
    "x = ((EXPR).y += 0);",
    "[(EXPR).y] = [];",
    "for ((EXPR).y in {});",
    "(EXPR).y++;",
    "Object.getPrototypeOf(EXPR);",
    "(EXPR, Object).getPrototypeOf(0);",
    "(EXPR, Reflect).get(0, 0);",
    "(EXPR, Object).getPrototypeOf``;",
    "0[EXPR];",
    "typeof 0[EXPR];",
    "typeof (EXPR).constructor;",
    "typeof Object.getPrototypeOf(EXPR);",
    "typeof [].constructor(EXPR);",
    "typeof (EXPR, 0).constructor;",
    "typeof globalThis[EXPR];",
    "delete Promise.resolve[EXPR];",
    "x = EXPR || 0;",
    "x = 0 ?? EXPR;",
    "x = EXPR ? 0 : 1;",
    "if (!EXPR) {}",
    "setTimeout(EXPR, 0);",
    "x = `${EXPR}`;",
    "for (var k in EXPR);",
    "for (var k of EXPR);",
    "x = [...EXPR];",
    "f(...EXPR);",
];

const WRITE: &str = "(Math.round = 0)";

const READS: [&str; 3] = ["(p = eval)", "(p = [].__proto__)", "(p = Function)"];

fn source_type(dialect: Dialect) -> SourceType {
    match dialect {
        Dialect::Js | Dialect::V8 => SourceType::cjs().with_jsx(true),
        Dialect::Ts => SourceType::ts(),
    }
}

fn verdicts(source: &str, dialect: Dialect) -> Option<(Builtins, Builtins)> {
    let alloc = Allocator::default();
    let options = ParseOptions {
        allow_v8_intrinsics: matches!(dialect, Dialect::V8),
        ..ParseOptions::default()
    };
    let mut parsed = Parser::new(&alloc, source, source_type(dialect))
        .with_options(options)
        .parse();
    if parsed.diagnostics.has_errors() {
        let module = SourceType::mjs().with_jsx(!matches!(dialect, Dialect::Ts));
        let module = match dialect {
            Dialect::Ts => module.with_typescript(true),
            Dialect::Js | Dialect::V8 => module,
        };
        parsed = Parser::new(&alloc, source, module)
            .with_options(options)
            .parse();
    }
    if parsed.diagnostics.has_errors() {
        return None;
    }
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    let full = scan(&parsed.program, &sem.scoping, module, false, &MATH_ROUND).verdict;
    let writes = scan(&parsed.program, &sem.scoping, module, true, &MATH_ROUND).verdict;
    Some((full, writes))
}

fn fill(shape: &Shape, context: &str, payload: &str) -> String {
    let expression = shape.shape.replace("SLOT", payload);
    shape
        .around
        .replace("BODY", &context.replace("EXPR", &expression))
}

struct Seen(Vec<bool>);

impl<'a> VisitJs<'a> for Seen {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        self.0[variant(it)] = true;
        walk_js::walk_expression(self, it);
    }
}

fn record(seen: &mut Seen, source: &str, dialect: Dialect) {
    let alloc = Allocator::default();
    let options = ParseOptions {
        allow_v8_intrinsics: matches!(dialect, Dialect::V8),
        ..ParseOptions::default()
    };
    let mut parsed = Parser::new(&alloc, source, source_type(dialect))
        .with_options(options)
        .parse();
    if parsed.diagnostics.has_errors() {
        parsed = Parser::new(&alloc, source, SourceType::mjs().with_jsx(true))
            .with_options(options)
            .parse();
    }
    assert!(
        !parsed.diagnostics.has_errors(),
        "shape does not parse: {source}"
    );
    seen.visit_program(&parsed.program);
}

#[test]
fn the_shapes_cover_every_expression_variant() {
    let mut seen = Seen(vec![false; VARIANTS]);
    for shape in SHAPES {
        record(&mut seen, &fill(shape, "x = EXPR;", "0"), shape.dialect);
    }
    let missing: Vec<usize> = (0..VARIANTS).filter(|index| !seen.0[*index]).collect();
    assert!(missing.is_empty(), "variants with no shape: {missing:?}");
}

#[test]
fn a_write_inside_any_variant_in_any_position_is_written() {
    let mut failures = Vec::new();
    for shape in SHAPES.iter().filter(|shape| shape.shape.contains("SLOT")) {
        for context in CONTEXTS {
            let source = fill(shape, context, WRITE);
            let Some((full, writes)) = verdicts(&source, shape.dialect) else {
                continue;
            };
            if full != Builtins::Written || writes != Builtins::Written {
                failures.push(format!("{source}  =>  {full:?} / {writes:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_read_inside_any_variant_in_any_position_is_not_intact() {
    let mut failures = Vec::new();
    for shape in SHAPES.iter().filter(|shape| shape.shape.contains("SLOT")) {
        for context in CONTEXTS {
            for read in READS {
                let source = fill(shape, context, read);
                let Some((full, _)) = verdicts(&source, shape.dialect) else {
                    continue;
                };
                if full == Builtins::Intact {
                    failures.push(source);
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_shape_parses_in_most_positions() {
    let mut thin = Vec::new();
    for shape in SHAPES.iter().filter(|shape| shape.shape.contains("SLOT")) {
        let parsed = CONTEXTS
            .iter()
            .filter(|context| verdicts(&fill(shape, context, WRITE), shape.dialect).is_some())
            .count();
        if parsed * 2 < CONTEXTS.len() {
            thin.push(format!("{} ({parsed})", shape.shape));
        }
    }
    assert!(thin.is_empty(), "{}", thin.join("\n"));
}
