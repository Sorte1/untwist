use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use untwist_core::parse::parse_program;
use untwist_eval::fold::{
    fold_layer_text, fold_literal_chain, fold_to_const, Const, FoldOptions, LayerText,
};

const UNMODELLED_READS: [&str; 25] = [
    "true.toString",
    "false.constructor",
    "true[\"valueOf\"]",
    "(1).constructor",
    "Infinity.constructor",
    "({a: 1}).constructor",
    "({a: 1}).__proto__",
    "Math.SQRT2",
    "Math.LOG10E",
    "Number.NaN",
    "Number.NEGATIVE_INFINITY",
    "String.length",
    "String.name",
    "Math.abs.length",
    "Math.max.length",
    "Math.abs.name",
    "\"abc\".slice.length",
    "\"abc\".indexOf.name",
    "parseInt.length",
    "[].push.length",
    "[].constructor",
    "\"abc\".at",
    "\"abc\".normalize",
    "(1).toLocaleString",
    "true.x",
];

fn with_expr<R>(source: &str, body: impl FnOnce(&Expression<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let Statement::ExpressionStatement(stmt) = &parsed.program.body[0] else {
        panic!("expression statement");
    };
    body(&stmt.expression)
}

fn fold_default(source: &str) -> Option<Const> {
    with_expr(source, |expr| fold_to_const(expr, &FoldOptions::default()))
}

fn fold_chain(source: &str) -> Option<Const> {
    with_expr(source, fold_literal_chain)
}

fn layer_text(source: &str) -> LayerText {
    with_expr(source, |expr| {
        fold_layer_text(expr, 1_000_000, 1 << 24).text
    })
}

fn each_form(read: &str) -> [String; 4] {
    [
        format!("typeof {read};"),
        format!("{read} === undefined;"),
        format!("{read} ? 1 : 2;"),
        format!("\"\" + {read};"),
    ]
}

#[test]
fn an_unmodelled_member_never_folds_to_undefined_in_the_default_folder() {
    for read in UNMODELLED_READS {
        for source in each_form(read) {
            assert_eq!(fold_default(&source), None, "{source}");
        }
    }
}

#[test]
fn an_unmodelled_member_never_folds_to_undefined_in_a_literal_chain() {
    for read in UNMODELLED_READS {
        for source in each_form(read) {
            assert_eq!(fold_chain(&source), None, "{source}");
        }
    }
}

#[test]
fn an_unmodelled_member_never_reaches_a_layer_text() {
    for read in UNMODELLED_READS {
        let source = format!("\"console.log('\" + typeof ({read}) + \"|\" + ({read}) + \"')\";");
        assert_eq!(layer_text(&source), LayerText::Bailed, "{source}");
    }
}

#[test]
fn the_reported_layer_texts_are_refused() {
    let sources = [
        "typeof true.toString === \"undefined\" ? \"console.log('wrong')\" : \"console.log('right')\";",
        "`console.log(${typeof (false).constructor === \"undefined\" ? 1 : 2})`;",
        "\"console.log(\" + (true.valueOf ? 1 : 2) + \")\";",
        "\"console.log(88, '\" + typeof (Math.SQRT2) + \"|\" + (Math.SQRT2) + \"')\";",
        "\"console.log(108, '\" + typeof (Math.abs.length) + \"|\" + (Math.abs.length) + \"')\";",
        "\"console.log(56, '\" + typeof ({a:1}).constructor + \"|\" + ({a:1}).constructor + \"')\";",
    ];
    for source in sources {
        assert_eq!(layer_text(source), LayerText::Bailed, "{source}");
    }
}

#[test]
fn modelled_members_and_plain_misses_still_fold() {
    let cases = [
        ("({a: 1}).a;", Const::Num(1.0)),
        ("typeof ({a: 1}).b;", Const::Str("undefined".to_string())),
        ("\"abc\".length;", Const::Num(3.0)),
        ("\"abc\"[1];", Const::Str("b".to_string())),
        ("typeof \"abc\".slice;", Const::Str("function".to_string())),
        ("typeof Math.abs;", Const::Str("function".to_string())),
        ("Math.PI > 3;", Const::Bool(true)),
        ("[1, 2].length;", Const::Num(2.0)),
        ("typeof [1][5];", Const::Str("undefined".to_string())),
        ("(255).toString(16);", Const::Str("ff".to_string())),
    ];
    for (source, expected) in cases {
        assert_eq!(fold_default(source), Some(expected), "{source}");
    }
}
