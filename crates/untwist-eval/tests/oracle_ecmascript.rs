mod common;

use common::oracle_evaluate;
use oxc_ast::ast::Statement;
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};

const AGREEMENT_CORPUS: [&str; 23] = [
    "1 + 2",
    "1 + '2'",
    "'a' + 'b'",
    "-0x1021e + -0x7eac8 + 0x17 * 0xac9c",
    "0x1cc - 0x100",
    "'0x1cc' - 0x100",
    "10 / 4",
    "7 % 3",
    "2 ** 10",
    "-1 >>> 0",
    "255 & -2",
    "1 << 33",
    "~2.9",
    "!![]",
    "![]",
    "typeof 1",
    "typeof 'a'",
    "typeof null",
    "'' == 0",
    "null == undefined",
    "null == 0",
    "NaN === NaN",
    "void 0",
];

fn ours(source: &str) -> Option<String> {
    common::with_program(source, |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            return None;
        };
        let folded = fold_to_const(&stmt.expression, &FoldOptions::default())?;
        Some(match folded {
            Const::Num(n) => format!("number:{}", untwist_eval::coerce::number_to_string(n)),
            Const::Str(text) => format!("string:{text}"),
            Const::Bool(b) => format!("boolean:{b}"),
            Const::Undefined => "undefined".to_string(),
            Const::Null => "null".to_string(),
        })
    })
}

#[test]
fn we_agree_with_oxc_ecmascript_on_primitive_expressions() {
    for source in AGREEMENT_CORPUS {
        assert_eq!(
            ours(source),
            oracle_evaluate(source),
            "disagreement on `{source}`"
        );
    }
}

#[test]
fn documented_differences_where_oxc_cannot_represent_the_value() {
    assert!(ours("[1, 2]").is_none());
    assert_eq!(oracle_evaluate("[1, 2]"), None);
    assert_eq!(
        ours("String.fromCharCode(0xD800).length"),
        Some("number:1".to_string())
    );
    assert_eq!(oracle_evaluate("String.fromCharCode(0xD800).length"), None);
    assert_eq!(ours("-'0xa6'"), Some("number:-166".to_string()));
    assert_eq!(oracle_evaluate("-'0xa6'"), None);
}

#[test]
fn oxc_folds_string_methods_unguarded_which_is_why_our_purity_predicate_exists() {
    assert_eq!(
        oracle_evaluate("'a'.toUpperCase()"),
        Some("string:A".to_string())
    );
    assert_eq!(ours("'a'.toUpperCase()"), Some("string:A".to_string()));
}
