mod common;

use oxc_ast::ast::Statement;
use untwist_eval::fold::{fold_to_const, is_pure, Const, FoldOptions};

fn fold(source: &str) -> Option<Const> {
    common::with_program(source, |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        fold_to_const(&stmt.expression, &FoldOptions::default())
    })
}

fn fold_with(source: &str, options: &FoldOptions) -> Option<Const> {
    common::with_program(source, |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        fold_to_const(&stmt.expression, options)
    })
}

fn purity(source: &str) -> bool {
    common::with_program(source, |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        is_pure(&stmt.expression)
    })
}

#[test]
fn folds_the_numbers_to_expressions_and_split_strings_shapes() {
    assert_eq!(
        fold("-0x1021e + -0x7eac8 + 0x17 * 0xac9c"),
        Some(Const::Num(431390.0))
    );
    assert_eq!(
        fold("'He' + 'll' + 'o'"),
        Some(Const::Str("Hello".to_string()))
    );
    assert_eq!(fold("!![]"), Some(Const::Bool(true)));
    assert_eq!(fold("typeof 1"), Some(Const::Str("number".to_string())));
}

#[test]
fn folds_the_modelled_pure_builtins() {
    assert_eq!(
        fold("String.fromCharCode(72, 105)"),
        Some(Const::Str("Hi".to_string()))
    );
    assert_eq!(
        fold("decodeURIComponent('%48%69')"),
        Some(Const::Str("Hi".to_string()))
    );
    assert_eq!(fold("unescape('%41')"), Some(Const::Str("A".to_string())));
    assert_eq!(fold("'a'.toUpperCase()"), Some(Const::Str("A".to_string())));
    assert_eq!(fold("parseInt('590708XPLBWj')"), Some(Const::Num(590708.0)));
}

#[test]
fn refuses_anything_it_cannot_prove() {
    assert_eq!(fold("someIdentifier"), None);
    assert_eq!(fold("f(1)"), None);
    assert_eq!(fold("(function () { return 1; })()"), None);
    assert_eq!(fold("[].push(1)"), None);
    assert_eq!(fold("decodeURIComponent('%FF')"), None);
}

#[test]
fn dead_code_guards_fold_to_booleans() {
    assert_eq!(fold("'abc' === 'abc'"), Some(Const::Bool(true)));
    assert_eq!(fold("'abc' === 'abd'"), Some(Const::Bool(false)));
    assert_eq!(
        fold("0x1 === 0x2 ? 'a' : 'b'"),
        Some(Const::Str("b".to_string()))
    );
}

#[test]
fn the_purity_predicate_is_ours_not_the_oxc_side_effect_guard() {
    assert!(purity("'a'.toUpperCase()"));
    assert!(purity("1 + 2"));
    assert!(purity("[1, 2]"));
    assert!(!purity("f()"));
    assert!(!purity("a = 1"));
    assert!(!purity("a++"));
    assert!(!purity("delete a.b"));
    assert!(!purity("[].push(1)"));
    assert!(!purity("someIdentifier"));
}

#[test]
fn provided_bindings_fold_like_evaluate_globals_needs() {
    let mut options = FoldOptions::default();
    options
        .bindings
        .insert("SHIFT".to_string(), Const::Num(460.0));
    let folded = common::with_program("0x1cc - SHIFT", |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        fold_to_const(&stmt.expression, &options)
    });
    assert_eq!(folded, Some(Const::Num(0.0)));
}

#[test]
fn purity_is_decided_by_native_identity_not_by_syntax() {
    assert_eq!(fold("[]['pu'+'sh'](1)"), None);
    assert_eq!(fold("[1].push.call([1], 2)"), None);
    assert_eq!(fold("Math.random()"), None);
    assert_eq!(fold("'a'.concat('b')"), Some(Const::Str("ab".to_string())));
    assert_eq!(fold("[1,2].join()"), Some(Const::Str("1,2".to_string())));
    assert_eq!(fold("Math.max(1, 2)"), Some(Const::Num(2.0)));
    assert_eq!(fold("parseInt('0x1f')"), Some(Const::Num(31.0)));
    assert_eq!(
        fold("String.fromCharCode(65)"),
        Some(Const::Str("A".to_string()))
    );
    assert_eq!(fold("JSON.parse('1')"), Some(Const::Num(1.0)));
    assert_eq!(fold("JSON.parse('[1]')"), None);
    assert_eq!(fold("[1,2].map(function(x){return x})"), None);
}

#[test]
fn allow_pure_calls_false_refuses_every_call() {
    let options = FoldOptions {
        allow_pure_calls: false,
        ..FoldOptions::default()
    };
    for source in [
        "'a'.concat('b')",
        "Math.max(1, 2)",
        "parseInt('0x1f')",
        "String.fromCharCode(65)",
        "JSON.parse('1')",
    ] {
        assert!(
            fold_with(source, &options).is_none(),
            "expected `{source}` to refuse folding when allow_pure_calls is false"
        );
    }
}

#[test]
fn a_catastrophic_regexp_is_left_unfolded() {
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings: Default::default(),
    };
    let source = format!("'{}c'.replace(/^(a+)+b$/, 'x')", "a".repeat(43));
    assert!(fold_with(&source, &options).is_none());
}
